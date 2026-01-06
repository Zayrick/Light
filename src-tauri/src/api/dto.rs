use serde::{Deserialize, Serialize};
use crate::interface::effect::{
    DependencyBehavior, EffectParam, EffectParamDependency, EffectParamKind,
};

// ============================================================================
// App config DTOs (persisted via tauri-plugin-store)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenCaptureConfigDto {
    pub scale_percent: u8,
    pub fps: u8,
    /// Capture backend/method identifier (e.g. "dxgi", "gdi", "graphics", "xcap").
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigDto {
    pub schema_version: u32,
    pub window_effect: String,
    pub minimize_to_tray: bool,
    pub screen_capture: ScreenCaptureConfigDto,
}

impl AppConfigDto {
    pub fn default_for_platform() -> Self {
        let default_method = if cfg!(target_os = "windows") {
            "dxgi"
        } else if cfg!(target_os = "macos") {
            "screencapturekit"
        } else {
            "xcap"
        };

        // Keep defaults aligned with current frontend expectations.
        AppConfigDto {
            schema_version: 1,
            window_effect: "".to_string(),
            minimize_to_tray: false,
            screen_capture: ScreenCaptureConfigDto {
                scale_percent: 5,
                fps: 30,
                method: default_method.to_string(),
            },
        }
    }
}

#[derive(Serialize)]
pub struct ParamDependencyInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    equals: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    not_equals: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    behavior: Option<&'static str>,
}

impl From<&EffectParamDependency> for ParamDependencyInfo {
    fn from(dep: &EffectParamDependency) -> Self {
        match dep {
            EffectParamDependency::Dependency {
                key,
                equals,
                not_equals,
                behavior,
            } => {
                let behavior_str = match behavior {
                    DependencyBehavior::Hide => Some("hide"),
                    DependencyBehavior::Disable => Some("disable"),
                };

                ParamDependencyInfo {
                    key: Some(key),
                    equals: *equals,
                    not_equals: *not_equals,
                    behavior: behavior_str,
                }
            }
            EffectParamDependency::Always(behavior) => {
                let behavior_str = match behavior {
                    DependencyBehavior::Hide => Some("hide"),
                    DependencyBehavior::Disable => Some("disable"),
                };
                ParamDependencyInfo {
                    key: None,
                    equals: None,
                    not_equals: None,
                    behavior: behavior_str,
                }
            }
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum EffectParamInfo {
    #[serde(rename = "slider")]
    Slider {
        key: &'static str,
        label: &'static str,
        min: f64,
        max: f64,
        step: f64,
        default: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        dependency: Option<ParamDependencyInfo>,
    },
    #[serde(rename = "select")]
    Select {
        key: &'static str,
        label: &'static str,
        default: f64,
        options: Vec<SelectOptionInfo>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dependency: Option<ParamDependencyInfo>,
    },
    #[serde(rename = "toggle")]
    Toggle {
        key: &'static str,
        label: &'static str,
        default: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        dependency: Option<ParamDependencyInfo>,
    },
    #[serde(rename = "color")]
    Color {
        key: &'static str,
        label: &'static str,
        default: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        dependency: Option<ParamDependencyInfo>,
    },
}

#[derive(Serialize)]
pub struct SelectOptionInfo {
    label: String,
    value: f64,
}

impl From<&'static EffectParam> for EffectParamInfo {
    fn from(param: &'static EffectParam) -> Self {
        let dependency = param.dependency.as_ref().map(ParamDependencyInfo::from);

        match &param.kind {
            EffectParamKind::Slider {
                min,
                max,
                step,
                default,
            } => EffectParamInfo::Slider {
                key: param.key,
                label: param.label,
                min: *min,
                max: *max,
                step: *step,
                default: *default,
                dependency,
            },
            EffectParamKind::Select { default, options } => {
                let resolved = match options.resolve() {
                    Ok(list) => list,
                    Err(err) => {
                        log::warn!(
                            param = param.key,
                            err:display = err;
                            "[effects] Failed to resolve select options"
                        );
                        Vec::new()
                    }
                };

                let mut default_value = *default;
                if !resolved.is_empty()
                    && !resolved
                        .iter()
                        .any(|option| (option.value - default_value).abs() < f64::EPSILON)
                {
                    default_value = resolved[0].value;
                }

                let options = resolved
                    .into_iter()
                    .map(|option| SelectOptionInfo {
                        label: option.label,
                        value: option.value,
                    })
                    .collect();

                EffectParamInfo::Select {
                    key: param.key,
                    label: param.label,
                    default: default_value,
                    options,
                    dependency,
                }
            }
            EffectParamKind::Toggle { default } => EffectParamInfo::Toggle {
                key: param.key,
                label: param.label,
                default: *default,
                dependency,
            },
            EffectParamKind::Color { default } => EffectParamInfo::Color {
                key: param.key,
                label: param.label,
                default,
                dependency,
            },
        }
    }
}

#[derive(Serialize)]
pub struct EffectInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub group: Option<&'static str>,
    pub icon: Option<&'static str>,
    pub params: Vec<EffectParamInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfoResponse {
    pub os_platform: String,
    pub os_version: String,
    pub os_build: String,
    pub arch: String,
}

