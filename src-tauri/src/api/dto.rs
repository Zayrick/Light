use serde::Serialize;
use crate::interface::effect::{
    DependencyBehavior, EffectParam, EffectParamDependency, EffectParamKind,
};

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
                        eprintln!(
                            "[effects] Failed to resolve select options for '{}': {}",
                            param.key, err
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

#[cfg(not(target_os = "windows"))]
#[derive(Serialize)]
pub struct DisplayInfoResponse {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

