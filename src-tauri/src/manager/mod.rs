pub mod inventory;
pub mod runner;

use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::AppHandle;

use crate::interface::controller::{
    Controller, DeviceType, MatrixMap, OutputCapabilities, OutputPortDefinition, SegmentDefinition,
    SegmentType,
};

use self::inventory::{default_params_for_effect, scan_controllers};
use self::runner::DeviceRunner;

type ControllerRef = Arc<Mutex<Box<dyn Controller>>>;

// ============================================================================
// DTOs exposed to the frontend
// ============================================================================

#[derive(serde::Serialize, Clone, Debug)]
pub struct ScopeRef {
    pub port: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct ScopeModeState {
    /// Explicit selection at this scope. `None` means "inherit parent".
    pub selected_effect_id: Option<String>,
    /// Resolved effect id after applying inheritance. `None` means "off / no mode".
    pub effective_effect_id: Option<String>,
    /// Resolved parameters for the effective effect (from the origin scope).
    pub effective_params: Option<Map<String, Value>>,
    /// Where `effective_effect_id` is coming from.
    pub effective_from: Option<ScopeRef>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct Segment {
    pub id: String,
    pub name: String,
    pub segment_type: SegmentType,
    pub leds_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix: Option<MatrixMap>,
    pub mode: ScopeModeState,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct OutputPort {
    pub id: String,
    pub name: String,
    pub output_type: SegmentType,
    pub leds_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix: Option<MatrixMap>,
    pub capabilities: OutputCapabilities,
    pub segments: Vec<Segment>,
    pub mode: ScopeModeState,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct Device {
    pub port: String,
    pub model: String,
    pub description: String,
    pub id: String,
    pub device_type: DeviceType,
    pub brightness: u8,
    pub outputs: Vec<OutputPort>,
    pub mode: ScopeModeState,
}

// ============================================================================
// Internal state
// ============================================================================

#[derive(Clone, Debug)]
struct ActiveEffect {
    effect_id: String,
    started_at: Instant,
}

#[derive(Clone, Debug, Default)]
struct ModeConfig {
    active_effect: Option<ActiveEffect>,
    params_by_effect: HashMap<String, Map<String, Value>>,
    rev: u64,
}

impl ModeConfig {
    fn selected_effect_id(&self) -> Option<String> {
        self.active_effect.as_ref().map(|a| a.effect_id.clone())
    }

    fn set_inherit(&mut self) {
        if self.active_effect.is_some() {
            self.rev = self.rev.wrapping_add(1);
        }
        self.active_effect = None;
    }

    fn ensure_params_entry(&mut self, effect_id: &str) -> Result<(), String> {
        if self.params_by_effect.contains_key(effect_id) {
            return Ok(());
        }
        let defaults = default_params_for_effect(effect_id)
            .ok_or_else(|| format!("Effect '{}' not found", effect_id))?;
        self.params_by_effect.insert(effect_id.to_string(), defaults);
        Ok(())
    }

    fn params_for_effect(&self, effect_id: &str) -> Option<Map<String, Value>> {
        if let Some(stored) = self.params_by_effect.get(effect_id) {
            return Some(stored.clone());
        }
        default_params_for_effect(effect_id)
    }

    fn set_effect(&mut self, effect_id: &str, started_at: Instant) -> Result<(), String> {
        self.ensure_params_entry(effect_id)?;
        self.active_effect = Some(ActiveEffect {
            effect_id: effect_id.to_string(),
            started_at,
        });
        self.rev = self.rev.wrapping_add(1);
        Ok(())
    }

    fn merge_params(&mut self, effect_id: &str, params: &Map<String, Value>) -> Result<(), String> {
        self.ensure_params_entry(effect_id)?;
        let entry = self.params_by_effect.entry(effect_id.to_string()).or_default();
        for (k, v) in params {
            entry.insert(k.clone(), v.clone());
        }
        self.rev = self.rev.wrapping_add(1);
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct SegmentConfig {
    id: String,
    name: String,
    segment_type: SegmentType,
    leds_count: usize,
    matrix: Option<MatrixMap>,
    mode: ModeConfig,
}

#[derive(Clone, Debug)]
struct OutputConfig {
    id: String,
    name: String,
    output_type: SegmentType,
    leds_count: usize,
    matrix: Option<MatrixMap>,
    capabilities: OutputCapabilities,
    mode: ModeConfig,
    segments: Vec<SegmentConfig>,
}

#[derive(Clone, Debug)]
struct DeviceConfig {
    brightness: u8,
    mode: ModeConfig,
    outputs: Vec<OutputConfig>,
}

#[derive(Clone, Debug)]
struct ResolvedEffect {
    effect_id: String,
    from: ScopeRef,
    started_at: Instant,
    params: Map<String, Value>,
    origin_rev: u64,
}

impl DeviceConfig {
    fn from_output_defs(defs: Vec<OutputPortDefinition>) -> Self {
        let outputs = defs
            .into_iter()
            .map(|def| OutputConfig {
                id: def.id,
                name: def.name,
                output_type: def.output_type,
                leds_count: def.leds_count.max(1),
                matrix: def.matrix,
                capabilities: def.capabilities,
                mode: ModeConfig::default(),
                // Segments are user-defined and only meaningful for linear outputs (future).
                segments: Vec::new(),
            })
            .collect();

        Self {
            brightness: 100,
            mode: ModeConfig::default(),
            outputs,
        }
    }

    fn sync_with_output_defs(&mut self, defs: Vec<OutputPortDefinition>) {
        let mut old_by_id: HashMap<String, OutputConfig> =
            self.outputs.drain(..).map(|o| (o.id.clone(), o)).collect();

        let mut new_outputs = Vec::with_capacity(defs.len());
        for def in defs {
            let old = old_by_id.remove(&def.id);
            let mut out = if let Some(mut o) = old {
                o.name = def.name;
                o.capabilities = def.capabilities.clone();
                o.output_type = def.output_type;
                o.leds_count = def.leds_count.max(1);
                o.matrix = def.matrix.clone();

                // Segments only apply to linear outputs. If the driver changes output type,
                // drop any existing user segments.
                if o.output_type != SegmentType::Linear {
                    o.segments.clear();
                }

                // Ensure existing user segments still match the driver's LED count.
                if o.output_type == SegmentType::Linear && !o.segments.is_empty() {
                    let total = o.segments.iter().map(|s| s.leds_count).sum::<usize>();
                    if total != o.leds_count {
                        o.segments.clear();
                    }
                }

                o
            } else {
                OutputConfig {
                    id: def.id,
                    name: def.name,
                    output_type: def.output_type,
                    leds_count: def.leds_count.max(1),
                    matrix: def.matrix,
                    capabilities: def.capabilities,
                    mode: ModeConfig::default(),
                    segments: Vec::new(),
                }
            };

            // If driver says this output is not linear, ensure no segments are kept.
            if out.output_type != SegmentType::Linear {
                out.segments.clear();
            }

            new_outputs.push(out);
        }

        self.outputs = new_outputs;
    }
}

struct ManagedDevice {
    controller: ControllerRef,
    config: Arc<Mutex<DeviceConfig>>,
    runner: Option<DeviceRunner>,
}

pub struct LightingManager {
    devices: Mutex<HashMap<String, ManagedDevice>>,
}

impl Default for LightingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LightingManager {
    pub fn new() -> Self {
        Self {
            devices: Mutex::new(HashMap::new()),
        }
    }

    /// Probe hardware and merge newly discovered controllers into the manager.
    pub fn scan_devices(&self) -> Vec<Device> {
        let found = scan_controllers();
        {
            let mut devices = self.devices.lock().unwrap();
            for controller in found {
            let port = controller.port_name();
                devices.entry(port.clone()).or_insert_with(|| {
                    let controller_ref: ControllerRef = Arc::new(Mutex::new(controller));
                    let output_defs = controller_ref.lock().unwrap().outputs();
                    let config = DeviceConfig::from_output_defs(output_defs);

                    ManagedDevice {
                        controller: controller_ref,
                        config: Arc::new(Mutex::new(config)),
                        runner: None,
                    }
                });
            }
        }

        // Always sync output definitions after scan, in case a driver changed its capabilities.
        self.sync_all_output_defs();

        self.get_devices()
    }

    /// Return current devices without probing.
    pub fn get_devices(&self) -> Vec<Device> {
        let devices = self.devices.lock().unwrap();
        devices
            .iter()
            .map(|(port, md)| self.build_device_dto(port, md))
            .collect()
    }

    /// Set effect selection for a scope.
    ///
    /// - `(None, None)` targets the device scope
    /// - `(Some(output), None)` targets an output scope
    /// - `(Some(output), Some(segment))` targets a segment scope
    pub fn set_scope_effect(
        &self,
        port: &str,
        output_id: Option<&str>,
        segment_id: Option<&str>,
        effect_id: Option<&str>,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        let mut devices = self.devices.lock().unwrap();
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;

        {
            let mut cfg = md.config.lock().unwrap();

            // Resolve current effective effect for continuity (before mutation).
            let current_resolved = self.resolve_effect_for_scope(&cfg, port, output_id, segment_id);

            match (output_id, segment_id) {
                (None, None) => {
                    // Device scope: setting device-level should force children to inherit.
                    if let Some(new_id) = effect_id {
                        let started_at = if let Some(res) = &current_resolved {
                            if res.effect_id == new_id {
                                res.started_at
                            } else {
                                Instant::now()
                            }
                        } else {
                            Instant::now()
                        };

                        cfg.mode.set_effect(new_id, started_at)?;

                        // Force outputs + segments to inherit (per spec).
                        for out in &mut cfg.outputs {
                            out.mode.set_inherit();
                            for seg in &mut out.segments {
                                seg.mode.set_inherit();
                            }
                        }
                    } else {
                        cfg.mode.set_inherit();
                    }
                }
                (Some(out_id), None) => {
                    let out = cfg
                        .outputs
                        .iter_mut()
                        .find(|o| o.id == out_id)
                        .ok_or_else(|| format!("Output '{}' not found", out_id))?;

                    if let Some(new_id) = effect_id {
                        let started_at = if let Some(res) = &current_resolved {
                            if res.effect_id == new_id {
                                res.started_at
                            } else {
                                Instant::now()
                            }
                        } else {
                            Instant::now()
                        };

                        out.mode.set_effect(new_id, started_at)?;

                        // Force segments to inherit (per spec).
                        for seg in &mut out.segments {
                            seg.mode.set_inherit();
                        }
                    } else {
                        out.mode.set_inherit();
                    }
                }
                (Some(out_id), Some(seg_id)) => {
                    let out = cfg
                        .outputs
                        .iter_mut()
                        .find(|o| o.id == out_id)
                        .ok_or_else(|| format!("Output '{}' not found", out_id))?;
                    let seg = out
                        .segments
                        .iter_mut()
                        .find(|s| s.id == seg_id)
                        .ok_or_else(|| format!("Segment '{}' not found", seg_id))?;

                    if let Some(new_id) = effect_id {
                        let started_at = if let Some(res) = &current_resolved {
                            if res.effect_id == new_id {
                                res.started_at
                            } else {
                                Instant::now()
                            }
                        } else {
                            Instant::now()
                        };
                        seg.mode.set_effect(new_id, started_at)?;
            } else {
                        seg.mode.set_inherit();
                    }
                }
                (None, Some(_)) => {
                    return Err("Invalid scope: segment_id requires output_id".to_string())
                }
            }
        }

        self.ensure_runner_state_locked(&mut devices, port, app_handle)?;
        Ok(())
    }

    pub fn update_scope_effect_params(
        &self,
        port: &str,
        output_id: Option<&str>,
        segment_id: Option<&str>,
        params: Value,
    ) -> Result<(), String> {
        let params_obj = params
            .as_object()
            .ok_or_else(|| "Params must be a JSON object".to_string())?;

        let mut devices = self.devices.lock().unwrap();
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;

        let mut cfg = md.config.lock().unwrap();
        let resolved = self.resolve_effect_for_scope(&cfg, port, output_id, segment_id);
        let resolved = resolved.ok_or_else(|| "No active effect in this scope hierarchy".to_string())?;

        // Helper to promote a scope to explicit with continuity.
        let ensure_explicit = |mode: &mut ModeConfig| -> Result<String, String> {
            if let Some(active) = &mode.active_effect {
                return Ok(active.effect_id.clone());
            }
            mode.set_effect(&resolved.effect_id, resolved.started_at)?;
            Ok(resolved.effect_id.clone())
        };

        let target_effect_id = match (output_id, segment_id) {
            (None, None) => ensure_explicit(&mut cfg.mode)?,
            (Some(out_id), None) => {
                let out = cfg
                    .outputs
                    .iter_mut()
                    .find(|o| o.id == out_id)
                    .ok_or_else(|| format!("Output '{}' not found", out_id))?;
                ensure_explicit(&mut out.mode)?
            }
            (Some(out_id), Some(seg_id)) => {
                let out = cfg
                    .outputs
                    .iter_mut()
                    .find(|o| o.id == out_id)
                    .ok_or_else(|| format!("Output '{}' not found", out_id))?;
                let seg = out
                    .segments
                    .iter_mut()
                    .find(|s| s.id == seg_id)
                    .ok_or_else(|| format!("Segment '{}' not found", seg_id))?;
                ensure_explicit(&mut seg.mode)?
            }
            (None, Some(_)) => {
                return Err("Invalid scope: segment_id requires output_id".to_string())
            }
        };

        // Merge params into the target scope store.
        match (output_id, segment_id) {
            (None, None) => cfg.mode.merge_params(&target_effect_id, params_obj)?,
            (Some(out_id), None) => {
                let out = cfg
                    .outputs
                    .iter_mut()
                    .find(|o| o.id == out_id)
                    .ok_or_else(|| format!("Output '{}' not found", out_id))?;
                out.mode.merge_params(&target_effect_id, params_obj)?;
            }
            (Some(out_id), Some(seg_id)) => {
                let out = cfg
                    .outputs
                    .iter_mut()
                    .find(|o| o.id == out_id)
                    .ok_or_else(|| format!("Output '{}' not found", out_id))?;
                let seg = out
                    .segments
                    .iter_mut()
                    .find(|s| s.id == seg_id)
                    .ok_or_else(|| format!("Segment '{}' not found", seg_id))?;
                seg.mode.merge_params(&target_effect_id, params_obj)?;
            }
            (None, Some(_)) => unreachable!(),
        }

        Ok(())
    }

    pub fn set_brightness(&self, port: &str, brightness: u8) -> Result<(), String> {
        let mut devices = self.devices.lock().unwrap();
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;
        let mut cfg = md.config.lock().unwrap();
        cfg.brightness = brightness;
        Ok(())
    }

    pub fn set_output_segments(
        &self,
        port: &str,
        output_id: &str,
        segments: Vec<SegmentDefinition>,
    ) -> Result<(), String> {
        let mut devices = self.devices.lock().unwrap();
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;

        let mut cfg = md.config.lock().unwrap();
        let out = cfg
            .outputs
            .iter_mut()
            .find(|o| o.id == output_id)
            .ok_or_else(|| format!("Output '{}' not found", output_id))?;

        if out.output_type != SegmentType::Linear {
            return Err(format!(
                "Output '{}' is {:?}; segments are only supported for Linear outputs",
                output_id, out.output_type
            ));
        }

        if !out.capabilities.editable {
            return Err(format!("Output '{}' is not editable", output_id));
        }

        // Validate segment types and matrix payloads.
        for seg in &segments {
            if !out.capabilities.allowed_segment_types.contains(&seg.segment_type) {
                return Err(format!(
                    "Segment type {:?} is not allowed on output '{}'",
                    seg.segment_type, output_id
                ));
            }

            match seg.segment_type {
                SegmentType::Single => {
                    if seg.leds_count != 1 {
                        return Err("Single segment must have leds_count = 1".to_string());
                    }
                }
                SegmentType::Matrix => {
                    let m = seg
                        .matrix
                        .as_ref()
                        .ok_or_else(|| "Matrix segment requires matrix map".to_string())?;
                    let physical = m.map.iter().filter(|v| v.is_some()).count();
                    if physical != seg.leds_count {
                        return Err(format!(
                            "Matrix leds_count mismatch: leds_count={}, map_has_leds={}",
                            seg.leds_count, physical
                        ));
                    }
                }
                SegmentType::Linear => {}
            }
        }

        let total = segments.iter().map(|s| s.leds_count).sum::<usize>();
        if total != out.leds_count {
            return Err(format!(
                "Segment total LED count {} must equal output leds_count {}",
                total, out.leds_count
            ));
        }
        if total < out.capabilities.min_total_leds || total > out.capabilities.max_total_leds {
            return Err(format!(
                "Total LED count {} is outside allowed range {}..={}",
                total, out.capabilities.min_total_leds, out.capabilities.max_total_leds
            ));
        }
        if let Some(allowed) = &out.capabilities.allowed_total_leds {
            if !allowed.is_empty() && !allowed.contains(&total) {
                return Err(format!(
                    "Total LED count {} is not allowed (allowed: {:?})",
                    total, allowed
                ));
            }
        }

        // Preserve per-segment mode state when ids match.
        let mut old_by_id: HashMap<String, SegmentConfig> =
            out.segments.drain(..).map(|s| (s.id.clone(), s)).collect();

        out.segments = segments
            .into_iter()
            .map(|seg| {
                if let Some(mut existing) = old_by_id.remove(&seg.id) {
                    existing.name = seg.name;
                    existing.segment_type = seg.segment_type;
                    existing.leds_count = seg.leds_count;
                    existing.matrix = seg.matrix;
                    existing
                } else {
                    SegmentConfig {
                        id: seg.id,
                        name: seg.name,
                        segment_type: seg.segment_type,
                        leds_count: seg.leds_count,
                        matrix: seg.matrix,
                        mode: ModeConfig::default(),
                    }
                }
            })
            .collect();

        Ok(())
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    fn sync_all_output_defs(&self) {
        let devices = self.devices.lock().unwrap();
        for (_port, md) in devices.iter() {
            let defs = md.controller.lock().unwrap().outputs();
            let mut cfg = md.config.lock().unwrap();
            cfg.sync_with_output_defs(defs);
        }
    }

    fn build_device_dto(&self, port: &str, md: &ManagedDevice) -> Device {
        let (model, description, serial_id, device_type) = {
            let c = md.controller.lock().unwrap();
            (c.model(), c.description(), c.serial_id(), c.device_type())
        };

        let cfg = md.config.lock().unwrap();

        let device_mode = self.build_mode_state_for_device(&cfg, port);

        let outputs = cfg
            .outputs
            .iter()
            .map(|out| {
                let out_mode = self.build_mode_state_for_output(&cfg, port, &out.id);
                let segments = out
                    .segments
                    .iter()
                    .map(|seg| Segment {
                        id: seg.id.clone(),
                        name: seg.name.clone(),
                        segment_type: seg.segment_type,
                        leds_count: seg.leds_count,
                        matrix: seg.matrix.clone(),
                        mode: self.build_mode_state_for_segment(&cfg, port, &out.id, &seg.id),
                    })
                    .collect();

                OutputPort {
                    id: out.id.clone(),
                    name: out.name.clone(),
                    output_type: out.output_type,
                    leds_count: out.leds_count,
                    matrix: out.matrix.clone(),
                    capabilities: out.capabilities.clone(),
                    segments,
                    mode: out_mode,
                }
            })
            .collect();

        Device {
            port: port.to_string(),
            model,
            description,
            id: serial_id,
            device_type,
            brightness: cfg.brightness,
            outputs,
            mode: device_mode,
        }
    }

    fn build_mode_state_for_device(&self, cfg: &DeviceConfig, port: &str) -> ScopeModeState {
        let selected = cfg.mode.selected_effect_id();
        let resolved = self.resolve_effect_for_scope(cfg, port, None, None);
        ScopeModeState {
            selected_effect_id: selected,
            effective_effect_id: resolved.as_ref().map(|r| r.effect_id.clone()),
            effective_params: resolved.as_ref().map(|r| r.params.clone()),
            effective_from: resolved.as_ref().map(|r| r.from.clone()),
        }
    }

    fn build_mode_state_for_output(&self, cfg: &DeviceConfig, port: &str, output_id: &str) -> ScopeModeState {
        let out = cfg.outputs.iter().find(|o| o.id == output_id);
        let selected = out.and_then(|o| o.mode.selected_effect_id());
        let resolved = self.resolve_effect_for_scope(cfg, port, Some(output_id), None);
        ScopeModeState {
            selected_effect_id: selected,
            effective_effect_id: resolved.as_ref().map(|r| r.effect_id.clone()),
            effective_params: resolved.as_ref().map(|r| r.params.clone()),
            effective_from: resolved.as_ref().map(|r| r.from.clone()),
        }
    }

    fn build_mode_state_for_segment(
        &self,
        cfg: &DeviceConfig,
        port: &str,
        output_id: &str,
        segment_id: &str,
    ) -> ScopeModeState {
        let selected = cfg
            .outputs
            .iter()
            .find(|o| o.id == output_id)
            .and_then(|o| o.segments.iter().find(|s| s.id == segment_id))
            .and_then(|s| s.mode.selected_effect_id());

        let resolved = self.resolve_effect_for_scope(cfg, port, Some(output_id), Some(segment_id));
        ScopeModeState {
            selected_effect_id: selected,
            effective_effect_id: resolved.as_ref().map(|r| r.effect_id.clone()),
            effective_params: resolved.as_ref().map(|r| r.params.clone()),
            effective_from: resolved.as_ref().map(|r| r.from.clone()),
        }
    }

    fn resolve_effect_for_scope(
        &self,
        cfg: &DeviceConfig,
        port: &str,
        output_id: Option<&str>,
        segment_id: Option<&str>,
    ) -> Option<ResolvedEffect> {
        match (output_id, segment_id) {
            (None, None) => cfg.mode.active_effect.as_ref().and_then(|active| {
                let params = cfg.mode.params_for_effect(&active.effect_id)?;
                Some(ResolvedEffect {
                    effect_id: active.effect_id.clone(),
                    from: ScopeRef {
                        port: port.to_string(),
                        output_id: None,
                        segment_id: None,
                    },
                    started_at: active.started_at,
                    params,
                    origin_rev: cfg.mode.rev,
                })
            }),
            (Some(out_id), None) => {
                let out = cfg.outputs.iter().find(|o| o.id == out_id)?;
                if let Some(active) = &out.mode.active_effect {
                    let params = out.mode.params_for_effect(&active.effect_id)?;
                    Some(ResolvedEffect {
                        effect_id: active.effect_id.clone(),
                        from: ScopeRef {
                            port: port.to_string(),
                            output_id: Some(out.id.clone()),
                            segment_id: None,
                        },
                        started_at: active.started_at,
                        params,
                        origin_rev: out.mode.rev,
                    })
                } else {
                    self.resolve_effect_for_scope(cfg, port, None, None)
                }
            }
            (Some(out_id), Some(seg_id)) => {
                let out = cfg.outputs.iter().find(|o| o.id == out_id)?;
                let seg = out.segments.iter().find(|s| s.id == seg_id)?;

                if let Some(active) = &seg.mode.active_effect {
                    let params = seg.mode.params_for_effect(&active.effect_id)?;
                    Some(ResolvedEffect {
                        effect_id: active.effect_id.clone(),
                        from: ScopeRef {
                            port: port.to_string(),
                            output_id: Some(out.id.clone()),
                            segment_id: Some(seg.id.clone()),
                        },
                        started_at: active.started_at,
                        params,
                        origin_rev: seg.mode.rev,
                    })
                } else {
                    self.resolve_effect_for_scope(cfg, port, Some(out_id), None)
                }
            }
            (None, Some(_)) => None,
        }
    }

    fn device_has_any_effect(&self, cfg: &DeviceConfig, _port: &str) -> bool {
        if cfg.mode.active_effect.is_some() {
            return true;
        }

        for out in &cfg.outputs {
            if out.mode.active_effect.is_some() {
                return true;
            }
            for seg in &out.segments {
                if seg.mode.active_effect.is_some() {
                    return true;
                }
            }
        }

        false
    }

    fn ensure_runner_state_locked(
        &self,
        devices: &mut HashMap<String, ManagedDevice>,
        port: &str,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;
        let cfg = md.config.lock().unwrap();
        let should_run = self.device_has_any_effect(&cfg, port);
        drop(cfg);

        match (should_run, md.runner.is_some()) {
            (true, false) => {
                md.runner = Some(DeviceRunner::start(
                    port.to_string(),
                    md.controller.clone(),
                    md.config.clone(),
                    app_handle,
                )?);
            }
            (false, true) => {
                if let Some(runner) = md.runner.take() {
            runner.stop();
                }
            }
            _ => {}
        }

        Ok(())
    }
}


