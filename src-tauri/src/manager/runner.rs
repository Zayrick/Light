use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::interface::controller::{Color, MatrixMap, SegmentType};
use crate::interface::effect::Effect;

use super::inventory::create_effect;
use super::{resolve_effect_for_scope, DeviceConfig, ResolvedEffect, Scope, EFFECT_READY_TIMEOUT};

type ControllerRef = Arc<Mutex<Box<dyn crate::interface::controller::Controller>>>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TargetKey {
    output_id: String,
    segment_id: Option<String>, // None = whole output
}

struct TargetRuntime {
    effect_id: String,
    origin_started_at: Instant,
    origin_rev: u64,
    width: usize,
    height: usize,
    effect: Box<dyn Effect>,
    /// Final rendered buffer (after optional transitions), in virtual order.   
    buffer: Vec<Color>,
    /// Scratch buffer for the currently active effect output (used during transitions).
    effect_buffer: Vec<Color>,
    transition: Option<EffectTransition>,
    pending: Option<PendingEffect>,
    ready_wait: Option<ReadyWait>,
    blocked: Option<BlockedSpec>,
}

struct PendingEffect {
    effect_id: String,
    origin_started_at: Instant,
    origin_rev: u64,
    width: usize,
    height: usize,
    effect: Box<dyn Effect>,
    buffer: Vec<Color>,
    started_at: Instant,
    ready_to_commit: bool,
}

struct ReadyWait {
    started_at: Instant,
}

struct BlockedSpec {
    effect_id: String,
    origin_started_at: Instant,
    width: usize,
    height: usize,
    origin_rev: u64,
}

struct EffectTransition {
    started_at: Instant,
    duration: Duration,
    from: Vec<Color>,
}

struct TargetSpec<'a> {
    effect_id: &'a str,
    width: usize,
    height: usize,
    origin_started_at: Instant,
    origin_rev: u64,
    params: &'a serde_json::Map<String, Value>,
}

const EFFECT_SWITCH_FADE_DURATION: Duration = Duration::from_millis(120);

impl TargetRuntime {
    fn create_configured_effect(
        effect_id: &str,
        width: usize,
        height: usize,
        params: &serde_json::Map<String, Value>,
    ) -> Result<Box<dyn Effect>, String> {
        let mut effect = create_effect(effect_id)
            .ok_or_else(|| format!("Effect '{}' not found", effect_id))?;
        effect.resize(width, height);
        effect.update_params(Value::Object(params.clone()));
        Ok(effect)
    }

    fn new(
        effect_id: &str,
        width: usize,
        height: usize,
        origin_started_at: Instant,
        origin_rev: u64,
        params: &serde_json::Map<String, Value>,
        now: Instant,
    ) -> Result<Self, String> {
        let effect = Self::create_configured_effect(effect_id, width, height, params)?;

        let len = width.checked_mul(height).unwrap_or(0).max(1);
        let fade_from_black = EffectTransition {
            started_at: now,
            duration: EFFECT_SWITCH_FADE_DURATION,
            from: vec![Color::default(); len],
        };

        Ok(Self {
            effect_id: effect_id.to_string(),
            origin_started_at,
            origin_rev,
            width,
            height,
            effect,
            buffer: vec![Color::default(); len],
            effect_buffer: Vec::new(),
            transition: Some(fade_from_black),
            pending: None,
            ready_wait: None,
            blocked: None,
        })
    }

    fn ensure_updated(
        &mut self,
        spec: TargetSpec<'_>,
        now: Instant,
        target: &TargetKey,
        switch_tx: &flume::Sender<super::SwitchEvent>,
    ) -> Result<(), String> {
        let current_matches = self.effect_id == spec.effect_id
            && self.origin_started_at == spec.origin_started_at
            && self.width == spec.width
            && self.height == spec.height;

        if current_matches {
            self.pending = None;
            self.ready_wait = None;
            self.blocked = None;

            if self.origin_rev != spec.origin_rev {
                self.origin_rev = spec.origin_rev;
                self.effect
                    .update_params(Value::Object(spec.params.clone()));
            }

            return Ok(());
        }

        // Desired effect differs from currently active effect. Keep rendering the current
        // effect while we initialize the next one in `pending`.
        self.ready_wait = None;

        if let Some(blocked) = &self.blocked {
            let is_same = blocked.effect_id == spec.effect_id
                && blocked.origin_started_at == spec.origin_started_at
                && blocked.width == spec.width
                && blocked.height == spec.height
                && blocked.origin_rev == spec.origin_rev;
            if is_same {
                return Ok(());
            }
        }
        self.blocked = None;

        let pending_matches = self.pending.as_ref().is_some_and(|p| {
            p.effect_id == spec.effect_id
                && p.origin_started_at == spec.origin_started_at
                && p.width == spec.width
                && p.height == spec.height
        });

        if !pending_matches {
            match Self::create_configured_effect(spec.effect_id, spec.width, spec.height, spec.params)
            {
                Ok(effect) => {
                    let len = spec.width.checked_mul(spec.height).unwrap_or(0).max(1);
                    self.pending = Some(PendingEffect {
                        effect_id: spec.effect_id.to_string(),
                        origin_started_at: spec.origin_started_at,
                        origin_rev: spec.origin_rev,
                        width: spec.width,
                        height: spec.height,
                        effect,
                        buffer: vec![Color::default(); len],
                        started_at: now,
                        ready_to_commit: false,
                    });
                }
                Err(err) => {
                    let _ = switch_tx.send(super::SwitchEvent::Failed {
                        output_id: target.output_id.clone(),
                        segment_id: target.segment_id.clone(),
                        effect_id: spec.effect_id.to_string(),
                        origin_rev: spec.origin_rev,
                        reason: err.clone(),
                    });
                    self.pending = None;
                    self.blocked = Some(BlockedSpec {
                        effect_id: spec.effect_id.to_string(),
                        origin_started_at: spec.origin_started_at,
                        width: spec.width,
                        height: spec.height,
                        origin_rev: spec.origin_rev,
                    });
                    return Ok(());
                }
            }
        }

        if let Some(pending) = &mut self.pending {
            if pending.origin_rev != spec.origin_rev {
                pending.origin_rev = spec.origin_rev;
                pending
                    .effect
                    .update_params(Value::Object(spec.params.clone()));
            }
        }

        Ok(())
    }

    fn tick(
        &mut self,
        now: Instant,
        target: &TargetKey,
        switch_tx: &flume::Sender<super::SwitchEvent>,
    ) {
        if matches!(
            self.pending.as_ref().map(|pending| pending.ready_to_commit),
            Some(true)
        ) {
            let pending = self.pending.take().unwrap();
            let mut from = std::mem::take(&mut self.buffer);

            let commit_len = pending.width.checked_mul(pending.height).unwrap_or(0).max(1);
            if from.len() != commit_len {
                from.resize(commit_len, Color::default());
            }

            self.effect_id = pending.effect_id;
            self.origin_started_at = pending.origin_started_at;
            self.origin_rev = pending.origin_rev;
            self.width = pending.width;
            self.height = pending.height;
            self.effect = pending.effect;
            self.transition = Some(EffectTransition {
                started_at: now,
                duration: EFFECT_SWITCH_FADE_DURATION,
                from,
            });
        }

        let len = self.width.checked_mul(self.height).unwrap_or(0).max(1);
        if self.buffer.len() != len {
            self.buffer.resize(len, Color::default());
        }

        let elapsed = now.duration_since(self.origin_started_at);

        let Some(transition) = &mut self.transition else {
            self.effect.tick(elapsed, &mut self.buffer);

            self.process_ready_events(now, target, switch_tx);
            self.tick_pending(now, target, switch_tx);
            return;
        };

        if self.effect_buffer.len() != len {
            self.effect_buffer.resize(len, Color::default());
        }

        // Produce the new effect frame.
        self.effect.tick(elapsed, &mut self.effect_buffer);

        // Blend with the previous frame.
        let t = if transition.duration.is_zero() {
            1.0
        } else {
            (now.duration_since(transition.started_at).as_secs_f32() / transition.duration.as_secs_f32())
                .clamp(0.0, 1.0)
        };

        // Keep lengths consistent even if something resized unexpectedly.
        if transition.from.len() != len {
            transition.from.resize(len, Color::default());
        }

        for i in 0..len {
            self.buffer[i] = lerp_color(transition.from[i], self.effect_buffer[i], t);
        }

        // Finish transition.
        if t >= 1.0 {
            self.transition = None;
            std::mem::swap(&mut self.buffer, &mut self.effect_buffer);
        }

        self.process_ready_events(now, target, switch_tx);
        self.tick_pending(now, target, switch_tx);
    }

    fn tick_pending(
        &mut self,
        now: Instant,
        target: &TargetKey,
        switch_tx: &flume::Sender<super::SwitchEvent>,
    ) {
        let Some(pending) = &mut self.pending else {
            return;
        };

        if pending.ready_to_commit {
            return;
        }

        if now.duration_since(pending.started_at) > EFFECT_READY_TIMEOUT {
            let reason = format!(
                "Effect switch timeout ({}s)",
                EFFECT_READY_TIMEOUT.as_secs()
            );
            let _ = switch_tx.send(super::SwitchEvent::Failed {
                output_id: target.output_id.clone(),
                segment_id: target.segment_id.clone(),
                effect_id: pending.effect_id.clone(),
                origin_rev: pending.origin_rev,
                reason: reason.clone(),
            });
            self.blocked = Some(BlockedSpec {
                effect_id: pending.effect_id.clone(),
                origin_started_at: pending.origin_started_at,
                width: pending.width,
                height: pending.height,
                origin_rev: pending.origin_rev,
            });
            self.pending = None;
            return;
        }

        let len = pending.width.checked_mul(pending.height).unwrap_or(0).max(1);
        if pending.buffer.len() != len {
            pending.buffer.resize(len, Color::default());
        }

        let elapsed = now.duration_since(pending.origin_started_at);
        pending.effect.tick(elapsed, &mut pending.buffer);

        if pending.effect.is_ready() {
            pending.ready_to_commit = true;
            let _ = switch_tx.send(super::SwitchEvent::Ready {
                output_id: target.output_id.clone(),
                segment_id: target.segment_id.clone(),
                effect_id: pending.effect_id.clone(),
                origin_rev: pending.origin_rev,
            });
        }
    }

    fn process_ready_events(
        &mut self,
        now: Instant,
        target: &TargetKey,
        switch_tx: &flume::Sender<super::SwitchEvent>,
    ) {
        let Some(wait) = &self.ready_wait else {
            return;
        };

        if self.effect.is_ready() {
            let _ = switch_tx.send(super::SwitchEvent::Ready {
                output_id: target.output_id.clone(),
                segment_id: target.segment_id.clone(),
                effect_id: self.effect_id.clone(),
                origin_rev: self.origin_rev,
            });
            self.ready_wait = None;
            return;
        }

        if now.duration_since(wait.started_at) > EFFECT_READY_TIMEOUT {
            let reason = format!(
                "Effect switch timeout ({}s)",
                EFFECT_READY_TIMEOUT.as_secs()
            );
            let _ = switch_tx.send(super::SwitchEvent::Failed {
                output_id: target.output_id.clone(),
                segment_id: target.segment_id.clone(),
                effect_id: self.effect_id.clone(),
                origin_rev: self.origin_rev,
                reason,
            });
            self.ready_wait = None;
        }
    }
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
        let a = a as f32;
        let b = b as f32;
        (a + (b - a) * t).round().clamp(0.0, 255.0) as u8
    }

    Color {
        r: lerp_u8(from.r, to.r, t),
        g: lerp_u8(from.g, to.g, t),
        b: lerp_u8(from.b, to.b, t),
    }
}

pub struct DeviceRunner {
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DeviceRunner {
    pub(super) fn start(
        port: String,
        controller: ControllerRef,
        config: Arc<Mutex<DeviceConfig>>,
        app_handle: AppHandle,
        switch_tx: flume::Sender<super::SwitchEvent>,
    ) -> Result<Self, String> {
        let running = Arc::new(AtomicBool::new(true));
        let running_thread = running.clone();

        let thread = thread::spawn(move || {
            let frame_duration = Duration::from_micros(16666); // ~60 FPS
            let mut next_frame = Instant::now();

            let mut target_runtimes: HashMap<TargetKey, TargetRuntime> = HashMap::new();
            let mut device_buffer: Vec<Color> = Vec::new();

            while running_thread.load(Ordering::Relaxed) {
                let now = Instant::now();

                // Snapshot config for this tick.
                let (brightness, tasks, total_len) = {
                    let cfg = config.lock().unwrap();
                    let brightness = cfg.brightness;
                    let mut tasks = Vec::new();

                    let mut offset: usize = 0;
                    for out in &cfg.outputs {
                        let out_len = out.leds_count.max(1);

                        // Segments are user-defined and only meaningful for linear outputs.
                        // If there are no segments, render the output as a whole.
                        let use_segments =
                            out.output_type == SegmentType::Linear && !out.segments.is_empty();

                        if use_segments {
                            let seg_total =
                                out.segments.iter().map(|s| s.leds_count).sum::<usize>();

                            // Safety fallback: if segments don't cover the output, ignore them.
                            if seg_total != out_len {
                                let resolved = resolve_effect_for_scope(
                                    &cfg,
                                    &port,
                                    Scope::Output {
                                        output_id: out.id.as_str(),
                                    },
                                );
                                tasks.push(TargetTask {
                                    key: TargetKey {
                                        output_id: out.id.clone(),
                                        segment_id: None,
                                    },
                                    layout_type: out.output_type,
                                    leds_count: out_len,
                                    matrix: out.matrix.clone(),
                                    physical_offset: offset,
                                    resolved,
                                });
                                offset = offset.saturating_add(out_len);
                            } else {
                                for seg in &out.segments {
                                    let resolved = resolve_effect_for_scope(
                                        &cfg,
                                        &port,
                                        Scope::Segment {
                                            output_id: out.id.as_str(),
                                            segment_id: seg.id.as_str(),
                                        },
                                    );

                                    tasks.push(TargetTask {
                                        key: TargetKey {
                                            output_id: out.id.clone(),
                                            segment_id: Some(seg.id.clone()),
                                        },
                                        layout_type: seg.segment_type,
                                        leds_count: seg.leds_count.max(1),
                                        matrix: seg.matrix.clone(),
                                        physical_offset: offset,
                                        resolved,
                                    });

                                    offset = offset.saturating_add(seg.leds_count.max(1));
                                }
                            }
                        } else {
                            let resolved = resolve_effect_for_scope(
                                &cfg,
                                &port,
                                Scope::Output {
                                    output_id: out.id.as_str(),
                                },
                            );
                            tasks.push(TargetTask {
                                key: TargetKey {
                                    output_id: out.id.clone(),
                                    segment_id: None,
                                },
                                layout_type: out.output_type,
                                leds_count: out_len,
                                matrix: out.matrix.clone(),
                                physical_offset: offset,
                                resolved,
                            });

                            offset = offset.saturating_add(out_len);
                        }
                    }

                    (brightness, tasks, offset)
                };

                // Prune runtimes for removed targets (config edits).
                let task_keys: HashSet<TargetKey> =
                    tasks.iter().map(|t| t.key.clone()).collect();
                target_runtimes.retain(|k, _| task_keys.contains(k));

                // Prepare device buffer in physical order.
                if device_buffer.len() != total_len {
                    device_buffer.resize(total_len, Color::default());
                }
                device_buffer.fill(Color::default());

                // Render all targets.
                for task in tasks {
                    let Some(resolved) = task.resolved else {
                        target_runtimes.remove(&task.key);
                        continue;
                    };

                    let (width, height) =
                        virtual_layout_for_segment(task.layout_type, task.leds_count, &task.matrix);
                    if width == 0 || height == 0 {
                        continue;
                    }

                    let params = resolved.params.clone();
                    let entry = target_runtimes.entry(task.key.clone());
                    let runtime = match entry {
                        std::collections::hash_map::Entry::Occupied(mut e) => {
                            let spec = TargetSpec {
                                effect_id: &resolved.effect_id,
                                width,
                                height,
                                origin_started_at: resolved.started_at,
                                origin_rev: resolved.origin_rev,
                                params: &params,
                            };
                            if let Err(err) = e
                                .get_mut()
                                .ensure_updated(spec, now, &task.key, &switch_tx)
                            {
                                let seg = task
                                    .key
                                    .segment_id
                                    .as_deref()
                                    .unwrap_or("<output>");
                                log::warn!(
                                    port = port.as_str(),
                                    output_id = task.key.output_id.as_str(),
                                    segment_id = seg,
                                    err:display = err;
                                    "[runner] Failed to update segment runtime"
                                );
                                e.remove();
                                continue;
                            }
                            e.into_mut()
                        }
                        std::collections::hash_map::Entry::Vacant(v) => {
                            match TargetRuntime::new(
                                &resolved.effect_id,
                                width,
                                height,
                                resolved.started_at,
                                resolved.origin_rev,
                                &params,
                                now,
                            ) {
                                Ok(mut rt) => {
                                    if rt.effect.is_ready() {
                                        let _ = switch_tx.send(super::SwitchEvent::Ready {
                                            output_id: task.key.output_id.clone(),
                                            segment_id: task.key.segment_id.clone(),
                                            effect_id: rt.effect_id.clone(),
                                            origin_rev: rt.origin_rev,
                                        });
                                    } else {
                                        rt.ready_wait = Some(ReadyWait { started_at: now });
                                    }
                                    v.insert(rt)
                                }
                                Err(err) => {
                                    let _ = switch_tx.send(super::SwitchEvent::Failed {
                                        output_id: task.key.output_id.clone(),
                                        segment_id: task.key.segment_id.clone(),
                                        effect_id: resolved.effect_id.clone(),
                                        origin_rev: resolved.origin_rev,
                                        reason: err.clone(),
                                    });

                                    let seg = task
                                        .key
                                        .segment_id
                                        .as_deref()
                                        .unwrap_or("<output>");
                                    log::warn!(
                                        port = port.as_str(),
                                        output_id = task.key.output_id.as_str(),
                                        segment_id = seg,
                                        err:display = err;
                                        "[runner] Failed to create segment runtime"
                                    );
                                    continue;
                                }
                            }
                        }
                    };

                    runtime.tick(now, &task.key, &switch_tx);

                    map_segment_into_physical(
                        &runtime.buffer,
                        task.layout_type,
                        task.leds_count,
                        &task.matrix,
                        task.physical_offset,
                        &mut device_buffer,
                    );
                }

                // Apply brightness (0..=100).
                if brightness < 100 {
                    let factor = (brightness as f32 / 100.0).clamp(0.0, 1.0);
                    for c in &mut device_buffer {
                        c.r = (c.r as f32 * factor).round() as u8;
                        c.g = (c.g as f32 * factor).round() as u8;
                        c.b = (c.b as f32 * factor).round() as u8;
                    }
                }

                // Write to hardware.
                {
                    let mut c = controller.lock().unwrap();
                    if let Err(err) = c.update(&device_buffer) {
                        log::warn!(
                            port = port.as_str(),
                            err:display = err;
                            "[runner] Controller update failed"
                        );
                        break;
                    }
                }

                // Emit preview event (flattened physical order for now).
                let _ = app_handle.emit(
                    "device-led-update",
                    serde_json::json!({
                        "port": port.as_str(),
                        "colors": device_buffer.clone(),
                    }),
                );

                // Timing.
                next_frame += frame_duration;
                let after = Instant::now();
                if next_frame > after {
                    thread::sleep(next_frame - after);
                } else {
                    next_frame = after;
                    thread::yield_now();
                }
            }
        });

        Ok(Self {
            running,
            thread: Some(thread),
        })
    }

    pub(super) fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

// ============================================================================
// Segment rendering helpers
// ============================================================================

#[derive(Clone)]
struct TargetTask {
    key: TargetKey,
    layout_type: SegmentType,
    leds_count: usize,
    matrix: Option<MatrixMap>,
    physical_offset: usize,
    resolved: Option<ResolvedEffect>,
}

fn virtual_layout_for_segment(
    segment_type: SegmentType,
    leds_count: usize,
    matrix: &Option<MatrixMap>,
) -> (usize, usize) {
    match segment_type {
        SegmentType::Single => (1, 1),
        SegmentType::Linear => (leds_count.max(1), 1),
        SegmentType::Matrix => {
            if let Some(m) = matrix {
                (m.width.max(1), m.height.max(1))
            } else {
                // Fallback: treat as 1D.
                (leds_count.max(1), 1)
            }
        }
    }
}

fn map_segment_into_physical(
    virtual_buffer: &[Color],
    segment_type: SegmentType,
    leds_count: usize,
    matrix: &Option<MatrixMap>,
    physical_offset: usize,
    physical_out: &mut [Color],
) {
    match segment_type {
        SegmentType::Single => {
            if physical_offset < physical_out.len() && !virtual_buffer.is_empty() {
                physical_out[physical_offset] = virtual_buffer[0];
            }
        }
        SegmentType::Linear => {
            let len = leds_count.min(virtual_buffer.len());
            let end = (physical_offset + len).min(physical_out.len());
            let write_len = end.saturating_sub(physical_offset);
            if write_len > 0 {
                physical_out[physical_offset..physical_offset + write_len]
                    .copy_from_slice(&virtual_buffer[..write_len]);
            }
        }
        SegmentType::Matrix => {
            let Some(m) = matrix else {
                // No map: fall back to linear mapping.
                let len = leds_count.min(virtual_buffer.len());
                let end = (physical_offset + len).min(physical_out.len());
                let write_len = end.saturating_sub(physical_offset);
                if write_len > 0 {
                    physical_out[physical_offset..physical_offset + write_len]
                        .copy_from_slice(&virtual_buffer[..write_len]);
                }
                return;
            };

            // Map virtual indices to physical indices within this segment.
            for (virtual_idx, opt_phys_idx) in m.map.iter().enumerate() {
                let Some(local_phys) = opt_phys_idx else { continue };
                if virtual_idx >= virtual_buffer.len() {
                    break;
                }
                if *local_phys >= leds_count {
                    continue;
                }
                let dest = physical_offset.saturating_add(*local_phys);
                if dest < physical_out.len() {
                    physical_out[dest] = virtual_buffer[virtual_idx];
                }
            }
        }
    }
}


