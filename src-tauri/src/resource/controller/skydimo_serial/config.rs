use crate::interface::controller::{MatrixMap, SegmentType};

#[derive(Clone, Copy, Debug)]
pub enum SkydimoLayoutType {
    Strip1,
    Sides2,
    Perimeter3,
    Perimeter4,
}

#[derive(Clone, Copy, Debug)]
pub struct SkydimoZoneConfig {
    #[allow(dead_code)]
    pub name: &'static str,
    pub led_count: usize,
}

#[derive(Clone, Debug)]
pub struct SkydimoModelConfig {
    pub layout: SkydimoLayoutType,
    pub zones: Vec<SkydimoZoneConfig>,
    pub total_leds: usize,
}

/// Port of GetSkydimoModelConfig from SkydimoDeviceConfig.h
pub fn get_skydimo_model_config(model_id: &str) -> Option<SkydimoModelConfig> {
    use SkydimoLayoutType::*;

    let cfg = match model_id {
        // 2-zone models
        "SK0201" => SkydimoModelConfig {
            layout: Sides2,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 20,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 20,
                },
            ],
            total_leds: 40,
        },
        "SK0202" => SkydimoModelConfig {
            layout: Sides2,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 30,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 30,
                },
            ],
            total_leds: 60,
        },
        "SK0204" => SkydimoModelConfig {
            layout: Sides2,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 25,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 25,
                },
            ],
            total_leds: 50,
        },
        "SK0F01" => SkydimoModelConfig {
            layout: Sides2,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 29,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 29,
                },
            ],
            total_leds: 58,
        },
        "SK0F02" => SkydimoModelConfig {
            layout: Sides2,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 25,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 25,
                },
            ],
            total_leds: 50,
        },

        // 3-zone models
        "SK0121" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 13,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 25,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 13,
                },
            ],
            total_leds: 51,
        },
        "SK0124" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 14,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 26,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 14,
                },
            ],
            total_leds: 54,
        },
        "SK0127" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 17,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 31,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 17,
                },
            ],
            total_leds: 65,
        },
        "SK0132" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 20,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 37,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 20,
                },
            ],
            total_leds: 77,
        },
        "SK0134" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 15,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 41,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 15,
                },
            ],
            total_leds: 71,
        },
        "SK0149" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 19,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 69,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 19,
                },
            ],
            total_leds: 107,
        },

        // 4-zone models
        "SK0L21" => SkydimoModelConfig {
            layout: Perimeter4,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 13,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 25,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 13,
                },
                SkydimoZoneConfig {
                    name: "Zone 4",
                    led_count: 25,
                },
            ],
            total_leds: 76,
        },
        "SK0L24" => SkydimoModelConfig {
            layout: Perimeter4,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 14,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 26,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 14,
                },
                SkydimoZoneConfig {
                    name: "Zone 4",
                    led_count: 26,
                },
            ],
            total_leds: 80,
        },
        "SK0L27" => SkydimoModelConfig {
            layout: Perimeter4,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 17,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 31,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 17,
                },
                SkydimoZoneConfig {
                    name: "Zone 4",
                    led_count: 31,
                },
            ],
            total_leds: 96,
        },
        "SK0L32" => SkydimoModelConfig {
            layout: Perimeter4,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 20,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 37,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 20,
                },
                SkydimoZoneConfig {
                    name: "Zone 4",
                    led_count: 37,
                },
            ],
            total_leds: 114,
        },
        "SK0L34" => SkydimoModelConfig {
            layout: Perimeter4,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 15,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 41,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 15,
                },
                SkydimoZoneConfig {
                    name: "Zone 4",
                    led_count: 41,
                },
            ],
            total_leds: 112,
        },

        // SKA series (3-zone)
        "SKA124" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 18,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 34,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 18,
                },
            ],
            total_leds: 70,
        },
        "SKA127" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 20,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 41,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 20,
                },
            ],
            total_leds: 81,
        },
        "SKA132" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 25,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 45,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 25,
                },
            ],
            total_leds: 95,
        },
        "SKA134" => SkydimoModelConfig {
            layout: Perimeter3,
            zones: vec![
                SkydimoZoneConfig {
                    name: "Zone 1",
                    led_count: 21,
                },
                SkydimoZoneConfig {
                    name: "Zone 2",
                    led_count: 51,
                },
                SkydimoZoneConfig {
                    name: "Zone 3",
                    led_count: 21,
                },
            ],
            total_leds: 93,
        },

        // Single-zone LED strips (keep linear)
        "SK0402" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 72,
            }],
            total_leds: 72,
        },
        "SK0403" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 96,
            }],
            total_leds: 96,
        },
        "SK0404" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 144,
            }],
            total_leds: 144,
        },
        "SK0901" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 14,
            }],
            total_leds: 14,
        },
        "SK0801" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 2,
            }],
            total_leds: 2,
        },
        "SK0803" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 10,
            }],
            total_leds: 10,
        },
        "SK0E01" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 16,
            }],
            total_leds: 16,
        },
        "SK0H01" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 2,
            }],
            total_leds: 2,
        },
        "SK0H02" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 4,
            }],
            total_leds: 4,
        },
        "SK0S01" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 32,
            }],
            total_leds: 32,
        },
        "SK0K01" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 120,
            }],
            total_leds: 120,
        },
        "SK0K02" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 15,
            }],
            total_leds: 15,
        },
        "SK0M01" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 24,
            }],
            total_leds: 24,
        },
        "SK0N01" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 256,
            }],
            total_leds: 256,
        },
        "SK0N02" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 1024,
            }],
            total_leds: 1024,
        },
        "SK0N03" => SkydimoModelConfig {
            layout: Strip1,
            zones: vec![SkydimoZoneConfig {
                name: "LED Strip",
                led_count: 253,
            }],
            total_leds: 253,
        },

        _ => return None,
    };

    Some(cfg)
}

/// Extract model ID from a full device name like "Skydimo SK0121".
pub fn extract_model_from_device_name(device_name: &str) -> Option<&str> {
    let prefix = "Skydimo ";
    if let Some(pos) = device_name.find(prefix) {
        let start = pos + prefix.len();
        if start < device_name.len() {
            return Some(&device_name[start..]);
        }
    }
    None
}

pub struct SkydimoDefaultLayout {
    pub total_leds: usize,
    pub segment_type: SegmentType,
    pub matrix: Option<MatrixMap>,
}

fn build_matrix_for_config(config: &SkydimoModelConfig) -> Option<SkydimoDefaultLayout> {
    use SkydimoLayoutType::*;

    let zone_count = config.zones.len();
    let z1 = if zone_count >= 1 {
        config.zones[0].led_count
    } else {
        0
    };
    let z2 = if zone_count >= 2 {
        config.zones[1].led_count
    } else {
        0
    };
    let z3 = if zone_count >= 3 {
        config.zones[2].led_count
    } else {
        0
    };
    let z4 = if zone_count >= 4 {
        config.zones[3].led_count
    } else {
        0
    };

    let total_leds = config.total_leds;

    // For pure strips, keep linear layout instead of a sparse matrix.
    if let Strip1 = config.layout {
        return Some(SkydimoDefaultLayout {
            total_leds,
            segment_type: SegmentType::Linear,
            matrix: None,
        });
    }

    let (height, width) = match config.layout {
        Perimeter4 => {
            let h = z1.max(z3) + 2;
            let w = z2.max(z4) + 2;
            (h, w)
        }
        Perimeter3 => {
            let h = z1.max(z3) + 1;
            let w = z2 + 2;
            (h, w)
        }
        Sides2 => {
            let h = z1.max(z2) + 2;
            let mut w_f = ((16.0 / 9.0) * (h as f64)).round() as isize;
            if w_f < 3 {
                w_f = 3;
            }
            (h, w_f as usize)
        }
        Strip1 => unreachable!(),
    };

    let cell_count = height * width;
    let mut map = vec![None; cell_count];

    let mut idx: usize = 0;

    // Helper to index into the flattened map safely.
    let mut set_cell = |y: isize, x: isize| {
        if y < 0 || x < 0 {
            return;
        }
        let y_us = y as usize;
        let x_us = x as usize;
        if y_us >= height || x_us >= width {
            return;
        }
        let p = y_us * width + x_us;
        if p < cell_count {
            map[p] = Some(idx);
            idx += 1;
        }
    };

    // Z1
    match config.layout {
        Sides2 => {
            // Left side, bottom -> top (skip corners)
            let mut placed = 0;
            let mut y = height as isize - 2;
            while placed < z1 && y >= 1 {
                set_cell(y, 0);
                placed += 1;
                y -= 1;
            }
        }
        Perimeter3 | Perimeter4 => {
            // Right side, bottom -> top (skip corners)
            let start_y = if let Perimeter3 = config.layout {
                height as isize - 1
            } else {
                height as isize - 2
            };
            let mut placed = 0;
            let mut y = start_y;
            while placed < z1 && y >= 1 {
                set_cell(y, width as isize - 1);
                placed += 1;
                y -= 1;
            }
        }
        Strip1 => {}
    }

    // Z2
    match config.layout {
        Perimeter3 | Perimeter4 => {
            // Top row, right -> left (skip corners)
            let mut placed = 0;
            let mut x = width as isize - 2;
            while placed < z2 && x >= 1 {
                set_cell(0, x);
                placed += 1;
                x -= 1;
            }
        }
        Sides2 => {
            // Right side, top -> bottom (skip corners)
            let mut placed = 0;
            let mut y = 1;
            while placed < z2 && y <= (height as isize - 2) {
                set_cell(y, width as isize - 1);
                placed += 1;
                y += 1;
            }
        }
        Strip1 => {}
    }

    // Z3: left side, top -> bottom (skip corners)
    if let Perimeter3 | Perimeter4 | Sides2 = config.layout {
        let end_y = if let Perimeter3 = config.layout {
            height as isize - 1
        } else {
            height as isize - 2
        };
        let mut placed = 0;
        let mut y = 1;
        while placed < z3 && y <= end_y {
            set_cell(y, 0);
            placed += 1;
            y += 1;
        }
    }

    // Z4: bottom row, left -> right (skip corners)
    if let Perimeter4 = config.layout {
        let mut placed = 0;
        let mut x = 1;
        while placed < z4 && x <= (width as isize - 2) {
            set_cell(height as isize - 1, x);
            placed += 1;
            x += 1;
        }
    }

    let matrix = MatrixMap { width, height, map };

    Some(SkydimoDefaultLayout {
        total_leds,
        segment_type: SegmentType::Matrix,
        matrix: Some(matrix),
    })
}

/// Build the best-guess layout from a full device name string.
pub fn build_layout_from_device_name(device_name: &str) -> Option<SkydimoDefaultLayout> {
    // First try "Skydimo XXX" form.
    if let Some(model_id) = extract_model_from_device_name(device_name) {
        if let Some(config) = get_skydimo_model_config(model_id) {
            return build_matrix_for_config(&config);
        }
    }

    // Then try treating the whole string as the model ID.
    if let Some(config) = get_skydimo_model_config(device_name.trim()) {
        return build_matrix_for_config(&config);
    }

    None
}
