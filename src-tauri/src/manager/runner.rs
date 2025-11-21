use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::interface::controller::Controller;
use crate::manager::inventory::create_effect;

pub struct EffectRunner {
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl EffectRunner {
    pub fn start(
        effect_name: &str,
        controller_arc: Arc<Mutex<Box<dyn Controller>>>,
    ) -> Result<Self, String> {
        // Check if effect exists before spawning
        if create_effect(effect_name).is_none() {
            return Err(format!("Effect '{}' not found", effect_name));
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let c_clone = controller_arc.clone();
        let effect_name = effect_name.to_string();

        let handle = thread::spawn(move || {
            let mut effect = match create_effect(&effect_name) {
                Some(e) => e,
                None => return, // Should not happen given check above
            };

            let start = std::time::Instant::now();
            
            // Get LED count from controller
            let led_count = {
                let c = c_clone.lock().unwrap();
                c.length()
            };

            while running_clone.load(Ordering::Relaxed) {
                let colors = effect.tick(start.elapsed(), led_count);
                
                {
                    let mut c = c_clone.lock().unwrap();
                    if let Err(_) = c.update(&colors) {
                        break; // Stop if update fails
                    }
                }
                thread::sleep(Duration::from_millis(16));
            }
        });

        Ok(Self { 
            running,
            thread_handle: Some(handle),
        })
    }

    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

