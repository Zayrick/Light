use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::sync::mpsc::{self, Sender};
use serde_json::Value;

use crate::interface::controller::{Controller, Color};
use crate::manager::inventory::create_effect;

pub struct EffectRunner {
    running: Arc<AtomicBool>,
    ticker_thread: Option<JoinHandle<()>>,
    writer_thread: Option<JoinHandle<()>>,
    shared_state: Arc<(Mutex<Option<Vec<Color>>>, Condvar)>,
    param_tx: Sender<Value>,
}

impl EffectRunner {
    pub fn start(
        effect_id: &str,
        controller_arc: Arc<Mutex<Box<dyn Controller>>>,
    ) -> Result<Self, String> {
        // Check if effect exists before spawning
        if create_effect(effect_id).is_none() {
            return Err(format!("Effect '{}' not found", effect_id));
        }

        let running = Arc::new(AtomicBool::new(true));
        let shared_state = Arc::new((Mutex::new(None::<Vec<Color>>), Condvar::new()));
        let (param_tx, param_rx) = mpsc::channel();
        
        // Channel for recycling buffers to avoid allocation
        let (recycle_tx, recycle_rx) = mpsc::channel();

        // --- Writer Thread ---
        let writer_running = running.clone();
        let writer_state = shared_state.clone();
        let writer_controller = controller_arc.clone();
        let writer_recycle_tx = recycle_tx.clone();

        let writer_thread = thread::spawn(move || {
            let (lock, cvar) = &*writer_state;
            loop {
                let mut frame_guard = lock.lock().unwrap();
                
                // Wait for data or stop signal
                while frame_guard.is_none() && writer_running.load(Ordering::Relaxed) {
                    frame_guard = cvar.wait(frame_guard).unwrap();
                }

                // Check exit condition
                if !writer_running.load(Ordering::Relaxed) && frame_guard.is_none() {
                    break;
                }

                // Take latest frame
                let frame = frame_guard.take();
                drop(frame_guard); // Unlock to allow Ticker to produce next frame

                if let Some(colors) = frame {
                    let mut c = writer_controller.lock().unwrap();
                    if let Err(_) = c.update(&colors) {
                        break; // Stop on hardware error
                    }
                    // Recycle the buffer
                    let _ = writer_recycle_tx.send(colors);
                }
            }
        });

        // --- Ticker Thread ---
        let ticker_running = running.clone();
        let ticker_state = shared_state.clone();
        let effect_id = effect_id.to_string();
        let ticker_controller = controller_arc.clone(); // For getting length
        let ticker_recycle_tx = recycle_tx; // Move original tx here (or clone if needed later)

        let ticker_thread = thread::spawn(move || {
            let mut effect = match create_effect(&effect_id) {
                Some(e) => e,
                None => return,
            };

            let led_count = {
                let c = ticker_controller.lock().unwrap();
                c.length()
            };

            let (lock, cvar) = &*ticker_state;
            let start_time = Instant::now();
            let frame_duration = Duration::from_micros(16666); // ~60 FPS
            let mut next_frame_time = start_time;

            while ticker_running.load(Ordering::Relaxed) {
                // 0. Check for param updates
                while let Ok(params) = param_rx.try_recv() {
                    effect.update_params(params);
                }

                // 1. Get buffer (recycle or create)
                let mut buffer = recycle_rx.try_recv().unwrap_or_else(|_| {
                    vec![Color::default(); led_count]
                });
                
                // Ensure size is correct (in case led_count changed or new buffer)
                if buffer.len() != led_count {
                    buffer.resize(led_count, Color::default());
                }

                // 2. Tick Effect
                let now = Instant::now();
                effect.tick(now.duration_since(start_time), &mut buffer);

                // 3. Send to Writer (Overwrite existing)
                {
                    let mut frame_guard = lock.lock().unwrap();
                    
                    // If there was an unconsumed frame, recycle it
                    if let Some(dropped_frame) = frame_guard.take() {
                        let _ = ticker_recycle_tx.send(dropped_frame);
                    }
                    
                    *frame_guard = Some(buffer);
                    cvar.notify_one();
                }

                // 4. Precise Timing
                next_frame_time += frame_duration;
                let now_after = Instant::now();

                if next_frame_time > now_after {
                    thread::sleep(next_frame_time - now_after);
                } else {
                    // Running behind: reset schedule to prevent catch-up bursts
                    next_frame_time = now_after; 
                    thread::yield_now();
                }
            }
            
            // Ensure Writer wakes up to see running=false
            let (_lock, cvar) = &*ticker_state;
            cvar.notify_all();
        });

        Ok(Self { 
            running,
            ticker_thread: Some(ticker_thread),
            writer_thread: Some(writer_thread),
            shared_state,
            param_tx,
        })
    }

    pub fn update_params(&self, params: Value) {
        let _ = self.param_tx.send(params);
    }

    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        
        // Wake up writer in case it's waiting
        {
            let (_lock, cvar) = &*self.shared_state;
            cvar.notify_all();
        }

        if let Some(handle) = self.ticker_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.writer_thread.take() {
            let _ = handle.join();
        }
    }
}
