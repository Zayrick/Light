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
use super::{resolve_effect_for_scope, DeviceConfig, ResolvedEffect, Scope};

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
    buffer: Vec<Color>,
}

impl TargetRuntime {
    fn new(
        effect_id: &str,
        width: usize,
        height: usize,
        origin_started_at: Instant,
        origin_rev: u64,
        params: &serde_json::Map<String, Value>,
    ) -> Result<Self, String> {
        let mut effect = create_effect(effect_id)
            .ok_or_else(|| format!("Effect '{}' not found", effect_id))?;
        effect.resize(width, height);
        effect.update_params(Value::Object(params.clone()));

        let len = width.checked_mul(height).unwrap_or(0).max(1);

        Ok(Self {
            effect_id: effect_id.to_string(),
            origin_started_at,
            origin_rev,
            width,
            height,
            effect,
            buffer: vec![Color::default(); len],
        })
    }

    fn ensure_updated(
        &mut self,
        effect_id: &str,
        width: usize,
        height: usize,
        origin_started_at: Instant,
        origin_rev: u64,
        params: &serde_json::Map<String, Value>,
    ) -> Result<(), String> {
        let needs_recreate = self.effect_id != effect_id
            || self.origin_started_at != origin_started_at
            || self.width != width
            || self.height != height;

        if needs_recreate {
            *self = Self::new(effect_id, width, height, origin_started_at, origin_rev, params)?;
            return Ok(());
        }

        if self.origin_rev != origin_rev {
            self.origin_rev = origin_rev;
            self.effect.update_params(Value::Object(params.clone()));
        }

        Ok(())
    }

    fn tick(&mut self, elapsed: Duration) {
        let len = self.width.checked_mul(self.height).unwrap_or(0).max(1);
        if self.buffer.len() != len {
            self.buffer.resize(len, Color::default());
        }
        self.effect.tick(elapsed, &mut self.buffer);
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
                            if let Err(err) = e.get_mut().ensure_updated(
                                &resolved.effect_id,
                                width,
                                height,
                                resolved.started_at,
                                resolved.origin_rev,
                                &params,
                            ) {
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
                        std::collections::hash_map::Entry::Vacant(v) => match TargetRuntime::new(
                            &resolved.effect_id,
                            width,
                            height,
                            resolved.started_at,
                            resolved.origin_rev,
                            &params,
                        ) {
                            Ok(rt) => v.insert(rt),
                            Err(err) => {
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
                        },
                    };

                    let elapsed = now.duration_since(runtime.origin_started_at);
                    runtime.tick(elapsed);

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


