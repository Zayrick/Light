pub mod inventory;
pub mod runner;

use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::AppHandle;

use crate::interface::controller::{
    Controller, DeviceType, MatrixMap, OutputCapabilities, OutputPortDefinition, SegmentDefinition,
    SegmentType,
};

use self::inventory::{default_params_for_effect, scan_controllers};
use self::runner::DeviceRunner;

type ControllerRef = Arc<Mutex<Box<dyn Controller>>>;

fn default_brightness() -> u8 {
    100
}

// ============================================================================
// Scope helpers (internal)
// ============================================================================

/// Internal representation of a configuration scope.
///
/// This replaces scattered `(output_id, segment_id)` branching and makes it harder
/// to accidentally accept invalid combinations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope<'a> {
    Device,
    Output { output_id: &'a str },
    Segment { output_id: &'a str, segment_id: &'a str },
}

impl<'a> Scope<'a> {
    fn from_options(output_id: Option<&'a str>, segment_id: Option<&'a str>) -> Result<Self, String> {
        match (output_id, segment_id) {
            (None, None) => Ok(Scope::Device),
            (Some(out_id), None) => Ok(Scope::Output { output_id: out_id }),
            (Some(out_id), Some(seg_id)) => Ok(Scope::Segment {
                output_id: out_id,
                segment_id: seg_id,
            }),
            (None, Some(_)) => Err("Invalid scope: segment_id requires output_id".to_string()),
        }
    }
}

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
pub struct ScopeBrightnessState {
    /// Stored brightness at this scope (0..=100). Always present even if currently following.
    pub value: u8,
    /// Resolved brightness after applying inheritance rules.
    pub effective_value: u8,
    /// Where `effective_value` is coming from.
    pub effective_from: Option<ScopeRef>,
    /// Whether this scope is currently following its parent brightness.
    ///
    /// Rule: non-device scopes follow when their mode is inheriting
    /// (`active_effect` is None at this scope).
    pub is_following: bool,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct Segment {
    pub id: String,
    pub name: String,
    pub segment_type: SegmentType,
    pub leds_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix: Option<MatrixMap>,
    pub brightness: ScopeBrightnessState,
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
    pub brightness: ScopeBrightnessState,
    pub mode: ScopeModeState,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct Device {
    pub port: String,
    pub model: String,
    pub description: String,
    pub id: String,
    pub device_type: DeviceType,
    pub brightness: ScopeBrightnessState,
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

// ============================================================================
// Persisted config DTOs (stored under config/devices/*.json)
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedModeConfig {
    pub selected: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, Map<String, Value>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedDeviceConfig {
    pub device: PersistedDeviceSection,
    pub effects: PersistedEffectsSection,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedDeviceSection {
    /// Segment layout configuration (e.g. linear strip segmentation).
    ///
    /// Keyed by `output_id`.
    #[serde(default)]
    pub layout: HashMap<String, PersistedOutputLayout>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedOutputLayout {
    /// Ordered segment list for this output.
    ///
    /// Order matters for linear outputs because we derive physical offsets by accumulation.
    #[serde(default)]
    pub segments: Vec<SegmentDefinition>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedEffectsSection {
    /// Device-scope mode config.
    pub selected: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, Map<String, Value>>,
    /// Device-scope brightness (0..=100).
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    /// Output / segment scoped mode configs.
    #[serde(default)]
    pub outputs: Vec<PersistedOutputEffectsConfig>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedOutputEffectsConfig {
    pub id: String,
    /// Output-scope brightness. If omitted, runtime falls back to 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    pub selected: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, Map<String, Value>>,
    #[serde(default)]
    pub segments: Vec<PersistedSegmentEffectsConfig>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSegmentEffectsConfig {
    pub id: String,
    /// Segment-scope brightness. If omitted, runtime falls back to 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    pub selected: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, Map<String, Value>>,
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

impl From<&ModeConfig> for PersistedModeConfig {
    fn from(value: &ModeConfig) -> Self {
        PersistedModeConfig {
            selected: value.selected_effect_id(),
            params: value.params_by_effect.clone(),
        }
    }
}

fn apply_persisted_mode(mode: &mut ModeConfig, persisted: &PersistedModeConfig) -> Result<(), String> {
    mode.params_by_effect = persisted.params.clone();

    if let Some(effect_id) = &persisted.selected {
        mode.ensure_params_entry(effect_id)?;
        mode.active_effect = Some(ActiveEffect {
            effect_id: effect_id.clone(),
            started_at: Instant::now(),
        });
    } else {
        mode.active_effect = None;
    }

    mode.rev = mode.rev.wrapping_add(1);
    Ok(())
}

#[derive(Clone, Debug)]
struct SegmentConfig {
    id: String,
    name: String,
    segment_type: SegmentType,
    leds_count: usize,
    matrix: Option<MatrixMap>,
    brightness: u8,
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
    brightness: u8,
    mode: ModeConfig,
    segments: Vec<SegmentConfig>,
}

#[derive(Clone, Debug)]
struct DeviceConfig {
    brightness: u8,
    mode: ModeConfig,
    outputs: Vec<OutputConfig>,
    /// Fast lookup table for outputs by id. `outputs` remains the source of truth.
    output_index: HashMap<String, usize>,
}

#[derive(Clone, Debug)]
struct ResolvedBrightness {
    value: u8,
    from: ScopeRef,
}

fn scope_ref_for(port: &str, scope: Scope<'_>) -> ScopeRef {
    match scope {
        Scope::Device => ScopeRef {
            port: port.to_string(),
            output_id: None,
            segment_id: None,
        },
        Scope::Output { output_id } => ScopeRef {
            port: port.to_string(),
            output_id: Some(output_id.to_string()),
            segment_id: None,
        },
        Scope::Segment {
            output_id,
            segment_id,
        } => ScopeRef {
            port: port.to_string(),
            output_id: Some(output_id.to_string()),
            segment_id: Some(segment_id.to_string()),
        },
    }
}

fn brightness_for_scope(cfg: &DeviceConfig, scope: Scope<'_>) -> Option<u8> {
    match scope {
        Scope::Device => Some(cfg.brightness),
        Scope::Output { output_id } => cfg.output(output_id).map(|o| o.brightness),
        Scope::Segment {
            output_id,
            segment_id,
        } => cfg
            .output(output_id)
            .and_then(|o| o.segments.iter().find(|s| s.id == segment_id))
            .map(|s| s.brightness),
    }
}

fn brightness_for_scope_mut<'a>(
    cfg: &'a mut DeviceConfig,
    scope: Scope<'_>,
) -> Result<&'a mut u8, String> {
    match scope {
        Scope::Device => Ok(&mut cfg.brightness),
        Scope::Output { output_id } => cfg
            .output_mut(output_id)
            .map(|o| &mut o.brightness)
            .ok_or_else(|| format!("Output '{}' not found", output_id)),
        Scope::Segment {
            output_id,
            segment_id,
        } => {
            let out = cfg
                .output_mut(output_id)
                .ok_or_else(|| format!("Output '{}' not found", output_id))?;
            let seg = out
                .segments
                .iter_mut()
                .find(|s| s.id == segment_id)
                .ok_or_else(|| format!("Segment '{}' not found", segment_id))?;
            Ok(&mut seg.brightness)
        }
    }
}

fn scope_is_following_mode(cfg: &DeviceConfig, scope: Scope<'_>) -> bool {
    if scope == Scope::Device {
        return false;
    }

    // Follow rule: if this scope doesn't explicitly pick a mode, it is following.
    // This is intentionally independent from whether the resolved effective effect exists.
    mode_for_scope(cfg, scope)
        .map(|m| m.active_effect.is_none())
        .unwrap_or(false)
}

/// Resolve effective brightness for a scope.
///
/// Rule: brightness follows the mode selection hierarchy.
/// - If a scope explicitly selects a mode, it uses its stored brightness.
/// - Otherwise (non-device scopes), it follows parent scope effective brightness.
fn resolve_brightness_for_scope(
    cfg: &DeviceConfig,
    port: &str,
    scope: Scope<'_>,
) -> Option<ResolvedBrightness> {
    match scope {
        Scope::Device => Some(ResolvedBrightness {
            value: cfg.brightness,
            from: scope_ref_for(port, Scope::Device),
        }),
        Scope::Output { output_id } => {
            let out = cfg.output(output_id)?;
            if out.mode.active_effect.is_some() {
                Some(ResolvedBrightness {
                    value: out.brightness,
                    from: scope_ref_for(port, Scope::Output { output_id }),
                })
            } else {
                resolve_brightness_for_scope(cfg, port, Scope::Device)
            }
        }
        Scope::Segment {
            output_id,
            segment_id,
        } => {
            let out = cfg.output(output_id)?;
            let seg = out.segments.iter().find(|s| s.id == segment_id)?;

            if seg.mode.active_effect.is_some() {
                Some(ResolvedBrightness {
                    value: seg.brightness,
                    from: scope_ref_for(
                        port,
                        Scope::Segment {
                            output_id,
                            segment_id,
                        },
                    ),
                })
            } else {
                resolve_brightness_for_scope(cfg, port, Scope::Output { output_id })
            }
        }
    }
}

#[derive(Clone, Debug)]
struct ResolvedEffect {
    effect_id: String,
    from: ScopeRef,
    started_at: Instant,
    params: Map<String, Value>,
    origin_rev: u64,
}

const EFFECT_READY_TIMEOUT: Duration = Duration::from_secs(5);
const EFFECT_READY_TIMEOUT_GRACE: Duration = Duration::from_millis(250);

#[derive(Clone, Debug)]
enum SwitchEvent {
    Ready {
        output_id: String,
        segment_id: Option<String>,
        effect_id: String,
        origin_rev: u64,
    },
    Failed {
        output_id: String,
        segment_id: Option<String>,
        effect_id: String,
        origin_rev: u64,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SwitchTarget {
    output_id: String,
    segment_id: Option<String>,
}

fn mode_for_scope<'a>(cfg: &'a DeviceConfig, scope: Scope<'_>) -> Option<&'a ModeConfig> {
    match scope {
        Scope::Device => Some(&cfg.mode),
        Scope::Output { output_id } => cfg.output(output_id).map(|o| &o.mode),
        Scope::Segment {
            output_id,
            segment_id,
        } => cfg
            .output(output_id)
            .and_then(|o| o.segments.iter().find(|s| s.id == segment_id))
            .map(|s| &s.mode),
    }
}

fn mode_for_scope_mut<'a>(
    cfg: &'a mut DeviceConfig,
    scope: Scope<'_>,
) -> Result<&'a mut ModeConfig, String> {
    match scope {
        Scope::Device => Ok(&mut cfg.mode),
        Scope::Output { output_id } => cfg
            .output_mut(output_id)
            .map(|o| &mut o.mode)
            .ok_or_else(|| format!("Output '{}' not found", output_id)),
        Scope::Segment {
            output_id,
            segment_id,
        } => {
            let out = cfg
                .output_mut(output_id)
                .ok_or_else(|| format!("Output '{}' not found", output_id))?;
            let seg = out
                .segments
                .iter_mut()
                .find(|s| s.id == segment_id)
                .ok_or_else(|| format!("Segment '{}' not found", segment_id))?;
            Ok(&mut seg.mode)
        }
    }
}

fn replace_segments_for_output(
    out: &mut OutputConfig,
    output_id: &str,
    segments: Vec<SegmentDefinition>,
) -> Result<(), String> {
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
                    brightness: 100,
                    mode: ModeConfig::default(),
                }
            }
        })
        .collect();

    Ok(())
}

fn force_children_inherit(cfg: &mut DeviceConfig, scope: Scope<'_>) {
    match scope {
        Scope::Device => {
            for out in &mut cfg.outputs {
                out.mode.set_inherit();
                for seg in &mut out.segments {
                    seg.mode.set_inherit();
                }
            }
        }
        Scope::Output { output_id } => {
            if let Some(out) = cfg.output_mut(output_id) {
                for seg in &mut out.segments {
                    seg.mode.set_inherit();
                }
            }
        }
        Scope::Segment { .. } => {}
    }
}

/// Resolve the effective effect for a scope by applying inheritance:
/// segment -> output -> device.
fn resolve_effect_for_scope(cfg: &DeviceConfig, port: &str, scope: Scope<'_>) -> Option<ResolvedEffect> {
    match scope {
        Scope::Device => cfg.mode.active_effect.as_ref().and_then(|active| {
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
        Scope::Output { output_id } => {
            let out = cfg.output(output_id)?;
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
                resolve_effect_for_scope(cfg, port, Scope::Device)
            }
        }
        Scope::Segment {
            output_id,
            segment_id,
        } => {
            let out = cfg.output(output_id)?;
            let seg = out.segments.iter().find(|s| s.id == segment_id)?;

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
                resolve_effect_for_scope(cfg, port, Scope::Output { output_id })
            }
        }
    }
}

impl DeviceConfig {
    fn rebuild_output_index(&mut self) {
        self.output_index.clear();
        for (idx, out) in self.outputs.iter().enumerate() {
            self.output_index.insert(out.id.clone(), idx);
        }
    }

    fn output(&self, output_id: &str) -> Option<&OutputConfig> {
        let idx = self.output_index.get(output_id).copied()?;
        self.outputs.get(idx)
    }

    fn output_mut(&mut self, output_id: &str) -> Option<&mut OutputConfig> {
        let idx = self.output_index.get(output_id).copied()?;
        self.outputs.get_mut(idx)
    }

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
                brightness: 100,
                mode: ModeConfig::default(),
                // Segments are user-defined and only meaningful for linear outputs (future).
                segments: Vec::new(),
            })
            .collect();

        let mut cfg = Self {
            brightness: 100,
            mode: ModeConfig::default(),
            outputs,
            output_index: HashMap::new(),
        };
        cfg.rebuild_output_index();
        cfg
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
                    brightness: 100,
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
        self.rebuild_output_index();
    }
}

struct ManagedDevice {
    controller: ControllerRef,
    config: Arc<Mutex<DeviceConfig>>,
    runner: Option<DeviceRunner>,
    switch_tx: flume::Sender<SwitchEvent>,
    switch_rx: Option<flume::Receiver<SwitchEvent>>,
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
                    let (switch_tx, switch_rx) = flume::unbounded();

                    ManagedDevice {
                        controller: controller_ref,
                        config: Arc::new(Mutex::new(config)),
                        runner: None,
                        switch_tx,
                        switch_rx: Some(switch_rx),
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

    /// Return a single device snapshot without probing.
    pub fn get_device(&self, port: &str) -> Result<Device, String> {
        let devices = self.devices.lock().unwrap();
        let md = devices
            .get(port)
            .ok_or_else(|| "Device not found".to_string())?;
        Ok(self.build_device_dto(port, md))
    }

    /// Set effect selection for a scope.
    ///
    /// - `Scope::Device` targets the device scope
    /// - `Scope::Output` targets an output scope
    /// - `Scope::Segment` targets a segment scope
    pub fn set_scope_effect(
        &self,
        port: &str,
        output_id: Option<&str>,
        segment_id: Option<&str>,
        effect_id: Option<&str>,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        let scope = Scope::from_options(output_id, segment_id)?;

        let mut devices = self.devices.lock().unwrap();
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;

        {
            let mut cfg = md.config.lock().unwrap();

            // Resolve current effective effect for continuity (before mutation).
            let current_resolved = resolve_effect_for_scope(&cfg, port, scope);

            let started_at = |new_id: &str| {
                if let Some(res) = &current_resolved {
                    if res.effect_id == new_id {
                        res.started_at
                    } else {
                        Instant::now()
                    }
                } else {
                    Instant::now()
                }
            };

            let mode = mode_for_scope_mut(&mut cfg, scope)?;

            if let Some(new_id) = effect_id {
                mode.set_effect(new_id, started_at(new_id))?;
                // Per spec: when parent becomes explicit, force children to inherit.
                force_children_inherit(&mut cfg, scope);
            } else {
                mode.set_inherit();
            }
        }

        self.ensure_runner_state_locked(&mut devices, port, app_handle)?;       
        Ok(())
    }

    pub fn set_scope_effect_wait_ready(
        &self,
        port: &str,
        output_id: Option<&str>,
        segment_id: Option<&str>,
        effect_id: Option<&str>,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        let scope = Scope::from_options(output_id, segment_id)?;
        let Some(effect_id) = effect_id else {
            return self.set_scope_effect(port, output_id, segment_id, None, app_handle);
        };

        fn use_segments_for_output(out: &OutputConfig) -> bool {
            if out.output_type != SegmentType::Linear || out.segments.is_empty() {
                return false;
            }

            let out_len = out.leds_count.max(1);
            let seg_total = out.segments.iter().map(|s| s.leds_count).sum::<usize>();
            seg_total == out_len
        }

        fn scope_targets(cfg: &DeviceConfig, scope: Scope<'_>) -> Result<Vec<SwitchTarget>, String> {
            let mut targets = Vec::new();

            match scope {
                Scope::Device => {
                    for out in &cfg.outputs {
                        if use_segments_for_output(out) {
                            for seg in &out.segments {
                                targets.push(SwitchTarget {
                                    output_id: out.id.clone(),
                                    segment_id: Some(seg.id.clone()),
                                });
                            }
                        } else {
                            targets.push(SwitchTarget {
                                output_id: out.id.clone(),
                                segment_id: None,
                            });
                        }
                    }
                }
                Scope::Output { output_id } => {
                    let out = cfg
                        .output(output_id)
                        .ok_or_else(|| format!("Output '{}' not found", output_id))?;
                    if use_segments_for_output(out) {
                        for seg in &out.segments {
                            targets.push(SwitchTarget {
                                output_id: out.id.clone(),
                                segment_id: Some(seg.id.clone()),
                            });
                        }
                    } else {
                        targets.push(SwitchTarget {
                            output_id: out.id.clone(),
                            segment_id: None,
                        });
                    }
                }
                Scope::Segment {
                    output_id,
                    segment_id,
                } => {
                    let out = cfg
                        .output(output_id)
                        .ok_or_else(|| format!("Output '{}' not found", output_id))?;
                    if !out.segments.iter().any(|s| s.id == segment_id) {
                        return Err(format!("Segment '{}' not found", segment_id));
                    }
                    targets.push(SwitchTarget {
                        output_id: out.id.clone(),
                        segment_id: Some(segment_id.to_string()),
                    });
                }
            }

            Ok(targets)
        }

        fn all_target_effects(
            cfg: &DeviceConfig,
            port: &str,
        ) -> HashMap<SwitchTarget, Option<(String, u64)>> {
            let mut map: HashMap<SwitchTarget, Option<(String, u64)>> = HashMap::new();

            for out in &cfg.outputs {
                if use_segments_for_output(out) {
                    for seg in &out.segments {
                        let resolved = resolve_effect_for_scope(
                            cfg,
                            port,
                            Scope::Segment {
                                output_id: out.id.as_str(),
                                segment_id: seg.id.as_str(),
                            },
                        );
                        map.insert(
                            SwitchTarget {
                                output_id: out.id.clone(),
                                segment_id: Some(seg.id.clone()),
                            },
                            resolved.map(|r| (r.effect_id, r.origin_rev)),
                        );
                    }
                } else {
                    let resolved = resolve_effect_for_scope(
                        cfg,
                        port,
                        Scope::Output {
                            output_id: out.id.as_str(),
                        },
                    );
                    map.insert(
                        SwitchTarget {
                            output_id: out.id.clone(),
                            segment_id: None,
                        },
                        resolved.map(|r| (r.effect_id, r.origin_rev)),
                    );
                }
            }

            map
        }

        // Setup switch + start pending in runner (old effect keeps running).
        let (switch_rx, backup_cfg, expected) = {
            let mut devices = self.devices.lock().unwrap();
            let md = devices
                .get_mut(port)
                .ok_or_else(|| "Device not found".to_string())?;

            let switch_rx = md
                .switch_rx
                .take()
                .ok_or_else(|| "Effect switch already in progress".to_string())?;
            while switch_rx.try_recv().is_ok() {}

            let mut cfg = md.config.lock().unwrap();
            let backup_cfg = cfg.clone();

            let before = all_target_effects(&cfg, port);
            let scope_targets = match scope_targets(&cfg, scope) {
                Ok(targets) => targets,
                Err(err) => {
                    md.switch_rx = Some(switch_rx);
                    return Err(err);
                }
            };

            // Apply config mutation (same semantics as `set_scope_effect`).
            let set_result = (|| -> Result<(), String> {
                let current_resolved = resolve_effect_for_scope(&cfg, port, scope);
                let started_at = |new_id: &str| {
                    if let Some(res) = &current_resolved {
                        if res.effect_id == new_id {
                            res.started_at
                        } else {
                            Instant::now()
                        }
                    } else {
                        Instant::now()
                    }
                };

                let mode = mode_for_scope_mut(&mut cfg, scope)?;
                mode.set_effect(effect_id, started_at(effect_id))?;
                // Per spec: when parent becomes explicit, force children to inherit.
                force_children_inherit(&mut cfg, scope);
                Ok(())
            })();

            if let Err(err) = set_result {
                *cfg = backup_cfg.clone();
                md.switch_rx = Some(switch_rx);
                return Err(err);
            }

            let after = all_target_effects(&cfg, port);
            let mut expected = HashMap::<SwitchTarget, (String, u64)>::new();
            for target in &scope_targets {
                let before_id = before
                    .get(target)
                    .and_then(|meta| meta.as_ref().map(|(id, _rev)| id.as_str()));
                let after_meta = after.get(target).cloned().unwrap_or(None);
                let after_id = after_meta.as_ref().map(|(id, _rev)| id.as_str());

                if before_id != after_id {
                    if let Some(after_meta) = after_meta {
                        expected.insert(target.clone(), after_meta);
                    }
                }
            }

            drop(cfg);
            if let Err(err) = self.ensure_runner_state_for_device(md, port, app_handle.clone()) {
                let mut cfg = md.config.lock().unwrap();
                *cfg = backup_cfg.clone();
                drop(cfg);
                md.switch_rx = Some(switch_rx);
                let _ = self.ensure_runner_state_for_device(md, port, app_handle.clone());
                return Err(err);
            }

            (switch_rx, backup_cfg, expected)
        };

        // No visible change => no runner switch => nothing to wait for.
        let wait_result = if expected.is_empty() {
            Ok(())
        } else {
            let mut remaining: HashSet<SwitchTarget> = expected.keys().cloned().collect();
            let deadline = Instant::now() + EFFECT_READY_TIMEOUT + EFFECT_READY_TIMEOUT_GRACE;

            let mut result = Ok(());
            while !remaining.is_empty() {
                let now = Instant::now();
                if now >= deadline {
                    result = Err(format!(
                        "Effect switch timeout ({}s)",
                        EFFECT_READY_TIMEOUT.as_secs()
                    ));
                    break;
                }

                let wait_for = deadline.saturating_duration_since(now);
                match switch_rx.recv_timeout(wait_for) {
                    Ok(SwitchEvent::Ready {
                        output_id,
                        segment_id,
                        effect_id: ready_id,
                        origin_rev,
                    }) => {
                        let key = SwitchTarget { output_id, segment_id };
                        if let Some((expected_id, expected_rev)) = expected.get(&key) {
                            if expected_id == &ready_id && *expected_rev == origin_rev {
                                remaining.remove(&key);
                            }
                        }
                    }
                    Ok(SwitchEvent::Failed {
                        output_id,
                        segment_id,
                        effect_id: failed_id,
                        origin_rev,
                        reason,
                    }) => {
                        let key = SwitchTarget { output_id, segment_id };
                        if let Some((expected_id, expected_rev)) = expected.get(&key) {
                            if expected_id == &failed_id && *expected_rev == origin_rev {
                                result = Err(reason);
                                break;
                            }
                        }
                    }
                    Err(flume::RecvTimeoutError::Timeout) => {
                        result = Err(format!(
                            "Effect switch timeout ({}s)",
                            EFFECT_READY_TIMEOUT.as_secs()
                        ));
                        break;
                    }
                    Err(flume::RecvTimeoutError::Disconnected) => {
                        result = Err("Effect switch channel disconnected".to_string());
                        break;
                    }
                }
            }

            result
        };

        // Put receiver back + rollback on failure.
        let mut devices = self.devices.lock().unwrap();
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;

        if let Err(err) = &wait_result {
            let mut cfg = md.config.lock().unwrap();
            *cfg = backup_cfg;
            drop(cfg);
            let _ = self.ensure_runner_state_for_device(md, port, app_handle);

            md.switch_rx = Some(switch_rx);
            return Err(err.clone());
        }

        md.switch_rx = Some(switch_rx);
        wait_result
    }

    pub fn update_scope_effect_params(
        &self,
        port: &str,
        output_id: Option<&str>,
        segment_id: Option<&str>,
        params: Value,
    ) -> Result<(), String> {
        let scope = Scope::from_options(output_id, segment_id)?;

        let params_obj = params
            .as_object()
            .ok_or_else(|| "Params must be a JSON object".to_string())?;

        let mut devices = self.devices.lock().unwrap();
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;

        let mut cfg = md.config.lock().unwrap();
        let resolved = resolve_effect_for_scope(&cfg, port, scope);
        let resolved = resolved.ok_or_else(|| "No active effect in this scope hierarchy".to_string())?;

        // Helper to promote a scope to explicit with continuity.
        let ensure_explicit = |mode: &mut ModeConfig| -> Result<String, String> {
            if let Some(active) = &mode.active_effect {
                return Ok(active.effect_id.clone());
            }
            mode.set_effect(&resolved.effect_id, resolved.started_at)?;
            Ok(resolved.effect_id.clone())
        };

        let target_effect_id = {
            let mode = mode_for_scope_mut(&mut cfg, scope)?;
            ensure_explicit(mode)?
        };

        // Merge params into the target scope store.
        {
            let mode = mode_for_scope_mut(&mut cfg, scope)?;
            mode.merge_params(&target_effect_id, params_obj)?;
        }

        Ok(())
    }

    pub fn set_scope_brightness(
        &self,
        port: &str,
        output_id: Option<&str>,
        segment_id: Option<&str>,
        brightness: u8,
    ) -> Result<(), String> {
        let scope = Scope::from_options(output_id, segment_id)?;

        let mut devices = self.devices.lock().unwrap();
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;

        let mut cfg = md.config.lock().unwrap();

        // Brightness follows mode inheritance: when a scope is inheriting mode,
        // its brightness is locked (but still stored).
        if scope_is_following_mode(&cfg, scope) {
            return Err("Brightness is following parent mode at this scope".to_string());
        }

        let target = brightness_for_scope_mut(&mut cfg, scope)?;
        *target = brightness;
        Ok(())
    }

    pub fn set_brightness(&self, port: &str, brightness: u8) -> Result<(), String> {
        // Legacy device-level entrypoint.
        self.set_scope_brightness(port, None, None, brightness)
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
            .output_mut(output_id)
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

        replace_segments_for_output(out, output_id, segments)?;

        Ok(())
    }

    /// Export a device config snapshot for persistence.
    /// Returns `(device_id, config)` where `device_id` is the controller serial id.
    pub fn export_persisted_device_config(
        &self,
        port: &str,
    ) -> Result<(String, PersistedDeviceConfig), String> {
        let devices = self.devices.lock().unwrap();
        let md = devices
            .get(port)
            .ok_or_else(|| "Device not found".to_string())?;

        let device_id = md.controller.lock().unwrap().serial_id();
        let cfg = md.config.lock().unwrap();

        let mut layout: HashMap<String, PersistedOutputLayout> = HashMap::new();
        let mut outputs: Vec<PersistedOutputEffectsConfig> = Vec::with_capacity(cfg.outputs.len());

        for out in &cfg.outputs {
            // Layout: persist only if user-defined segments exist.
            if !out.segments.is_empty() {
                let segments = out
                    .segments
                    .iter()
                    .map(|s| SegmentDefinition {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        segment_type: s.segment_type,
                        leds_count: s.leds_count,
                        matrix: s.matrix.clone(),
                    })
                    .collect::<Vec<_>>();

                layout.insert(out.id.clone(), PersistedOutputLayout { segments });
            }

            // Effects: persist mode state for each scope.
            let segments = out
                .segments
                .iter()
                .map(|s| PersistedSegmentEffectsConfig {
                    id: s.id.clone(),
                    brightness: {
                        let explicit = s.mode.selected_effect_id().is_some();
                        if explicit || s.brightness != 100 {
                            Some(s.brightness)
                        } else {
                            None
                        }
                    },
                    selected: s.mode.selected_effect_id(),
                    params: s.mode.params_by_effect.clone(),
                })
                .collect::<Vec<_>>();

            outputs.push(PersistedOutputEffectsConfig {
                id: out.id.clone(),
                brightness: {
                    let explicit = out.mode.selected_effect_id().is_some();
                    if explicit || out.brightness != 100 {
                        Some(out.brightness)
                    } else {
                        None
                    }
                },
                selected: out.mode.selected_effect_id(),
                params: out.mode.params_by_effect.clone(),
                segments,
            });
        }

        Ok((
            device_id,
            PersistedDeviceConfig {
                device: PersistedDeviceSection {
                    layout,
                },
                effects: PersistedEffectsSection {
                    selected: cfg.mode.selected_effect_id(),
                    params: cfg.mode.params_by_effect.clone(),
                    brightness: cfg.brightness,
                    outputs,
                },
            },
        ))
    }

    /// Apply a persisted device config to a live device instance.
    ///
    /// Best-effort: unknown outputs/segments are ignored; invalid segments are skipped.
    pub fn apply_persisted_device_config(
        &self,
        port: &str,
        persisted: &PersistedDeviceConfig,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        let mut devices = self.devices.lock().unwrap();
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;

        {
            let mut cfg = md.config.lock().unwrap();

            cfg.brightness = persisted.effects.brightness;

            // 1) Apply layout first so segments exist before applying segment modes.
            for (output_id, layout) in &persisted.device.layout {
                let Some(out) = cfg.output_mut(output_id) else {
                    continue;
                };

                // Segments: only meaningful for editable linear outputs.
                if out.output_type == SegmentType::Linear
                    && out.capabilities.editable
                    && !layout.segments.is_empty()
                {
                    if let Err(err) =
                        replace_segments_for_output(out, output_id, layout.segments.clone())
                    {
                        log::warn!(
                            port,
                            output = output_id.as_str(),
                            err:display = err;
                            "[config] Skip invalid persisted layout"
                        );
                    }
                }
            }

            // 2) Apply device-scope effects.
            let device_mode = PersistedModeConfig {
                selected: persisted.effects.selected.clone(),
                params: persisted.effects.params.clone(),
            };
            apply_persisted_mode(&mut cfg.mode, &device_mode)?;

            // 3) Apply output/segment effects.
            for out_persisted in &persisted.effects.outputs {
                let Some(out) = cfg.output_mut(&out_persisted.id) else {
                    continue;
                };

                // Brightness (optional per-scope).
                out.brightness = out_persisted.brightness.unwrap_or(100);

                let out_mode = PersistedModeConfig {
                    selected: out_persisted.selected.clone(),
                    params: out_persisted.params.clone(),
                };
                apply_persisted_mode(&mut out.mode, &out_mode)?;

                for seg_persisted in &out_persisted.segments {
                    if let Some(seg) = out
                        .segments
                        .iter_mut()
                        .find(|s| s.id == seg_persisted.id)
                    {
                        seg.brightness = seg_persisted.brightness.unwrap_or(100);
                        let seg_mode = PersistedModeConfig {
                            selected: seg_persisted.selected.clone(),
                            params: seg_persisted.params.clone(),
                        };
                        let _ = apply_persisted_mode(&mut seg.mode, &seg_mode);
                    }
                }
            }
        }

        // Ensure runner state matches restored modes.
        self.ensure_runner_state_for_device(md, port, app_handle)?;
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
                        brightness: self.build_brightness_state_for_segment(
                            &cfg,
                            port,
                            &out.id,
                            &seg.id,
                        ),
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
                    brightness: self.build_brightness_state_for_output(&cfg, port, &out.id),
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
            brightness: self.build_brightness_state_for_device(&cfg, port),
            outputs,
            mode: device_mode,
        }
    }

    fn build_mode_state(&self, cfg: &DeviceConfig, port: &str, scope: Scope<'_>) -> ScopeModeState {
        let selected = mode_for_scope(cfg, scope).and_then(|m| m.selected_effect_id());
        let resolved = resolve_effect_for_scope(cfg, port, scope);
        ScopeModeState {
            selected_effect_id: selected,
            effective_effect_id: resolved.as_ref().map(|r| r.effect_id.clone()),
            effective_params: resolved.as_ref().map(|r| r.params.clone()),
            effective_from: resolved.as_ref().map(|r| r.from.clone()),
        }
    }

    fn build_mode_state_for_device(&self, cfg: &DeviceConfig, port: &str) -> ScopeModeState {
        self.build_mode_state(cfg, port, Scope::Device)
    }

    fn build_mode_state_for_output(
        &self,
        cfg: &DeviceConfig,
        port: &str,
        output_id: &str,
    ) -> ScopeModeState {
        self.build_mode_state(cfg, port, Scope::Output { output_id })
    }

    fn build_mode_state_for_segment(
        &self,
        cfg: &DeviceConfig,
        port: &str,
        output_id: &str,
        segment_id: &str,
    ) -> ScopeModeState {
        self.build_mode_state(cfg, port, Scope::Segment { output_id, segment_id })
    }

    fn build_brightness_state(
        &self,
        cfg: &DeviceConfig,
        port: &str,
        scope: Scope<'_>,
    ) -> ScopeBrightnessState {
        let stored = brightness_for_scope(cfg, scope).unwrap_or(100);
        let resolved = resolve_brightness_for_scope(cfg, port, scope);

        ScopeBrightnessState {
            value: stored,
            effective_value: resolved.as_ref().map(|r| r.value).unwrap_or(stored),
            effective_from: resolved.as_ref().map(|r| r.from.clone()),
            is_following: scope_is_following_mode(cfg, scope),
        }
    }

    fn build_brightness_state_for_device(
        &self,
        cfg: &DeviceConfig,
        port: &str,
    ) -> ScopeBrightnessState {
        self.build_brightness_state(cfg, port, Scope::Device)
    }

    fn build_brightness_state_for_output(
        &self,
        cfg: &DeviceConfig,
        port: &str,
        output_id: &str,
    ) -> ScopeBrightnessState {
        self.build_brightness_state(cfg, port, Scope::Output { output_id })
    }

    fn build_brightness_state_for_segment(
        &self,
        cfg: &DeviceConfig,
        port: &str,
        output_id: &str,
        segment_id: &str,
    ) -> ScopeBrightnessState {
        self.build_brightness_state(cfg, port, Scope::Segment { output_id, segment_id })
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

    fn ensure_runner_state_for_device(
        &self,
        md: &mut ManagedDevice,
        port: &str,
        app_handle: AppHandle,
    ) -> Result<(), String> {
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
                    md.switch_tx.clone(),
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

    fn ensure_runner_state_locked(
        &self,
        devices: &mut HashMap<String, ManagedDevice>,
        port: &str,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        let md = devices
            .get_mut(port)
            .ok_or_else(|| "Device not found".to_string())?;
        self.ensure_runner_state_for_device(md, port, app_handle)
    }
}


