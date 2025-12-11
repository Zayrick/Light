use crate::interface::effect::{
    DependencyBehavior, EffectParam, EffectParamDependency, EffectParamKind, SelectOption,
    SelectOptions, StaticSelectOption,
};

const BLACK_BORDER_MODE_OPTIONS: [StaticSelectOption; 4] = [
    StaticSelectOption {
        label: "默认模式",
        value: 0.0,
    },
    StaticSelectOption {
        label: "经典模式",
        value: 1.0,
    },
    StaticSelectOption {
        label: "OSD 模式",
        value: 2.0,
    },
    StaticSelectOption {
        label: "信箱模式",
        value: 3.0,
    },
];

fn screen_source_options() -> Result<Vec<SelectOption>, String> {
    use crate::resource::screen::list_displays;

    list_displays()
        .map(|displays| {
            displays
                .into_iter()
                .map(|display| SelectOption {
                    label: format!("{} ({}x{})", display.name, display.width, display.height),
                    value: display.index as f64,
                })
                .collect()
        })
        .map_err(|err| err.to_string())
}

pub const SCREEN_PARAMS: [EffectParam; 12] = [
    EffectParam {
        key: "displayIndex",
        label: "屏幕来源",
        kind: EffectParamKind::Select {
            default: 0.0,
            options: SelectOptions::Dynamic(screen_source_options),
        },
        dependency: None,
    },
    EffectParam {
        key: "smoothness",
        label: "平滑度",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            default: 80.0,
        },
        dependency: None,
    },
    EffectParam {
        key: "brightness",
        label: "亮度增益",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 3.0,
            step: 0.1,
            default: 1.0,
        },
        dependency: None,
    },
    EffectParam {
        key: "saturation",
        label: "饱和度增益",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 3.0,
            step: 0.1,
            default: 1.0,
        },
        dependency: None,
    },
    EffectParam {
        key: "gamma",
        label: "Gamma 校正",
        kind: EffectParamKind::Slider {
            min: 0.1,
            max: 4.0,
            step: 0.1,
            default: 1.0,
        },
        dependency: None,
    },
    EffectParam {
        key: "autoCrop",
        label: "黑边裁剪",
        kind: EffectParamKind::Toggle {
            default: true,
        },
        dependency: None,
    },
    EffectParam {
        key: "bbThreshold",
        label: "黑边判定阈值 (%)",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            default: 5.0,
        },
        dependency: Some(EffectParamDependency::Dependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbUnknownFrameCnt",
        label: "未知边框切换帧数",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 2000.0,
            step: 50.0,
            default: 600.0,
        },
        dependency: Some(EffectParamDependency::Dependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbBorderFrameCnt",
        label: "稳定边框切换帧数",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 200.0,
            step: 1.0,
            default: 50.0,
        },
        dependency: Some(EffectParamDependency::Dependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbMaxInconsistentCnt",
        label: "最大允许不一致帧数",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 50.0,
            step: 1.0,
            default: 10.0,
        },
        dependency: Some(EffectParamDependency::Dependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbBlurRemoveCnt",
        label: "模糊安全边界 (像素)",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 10.0,
            step: 1.0,
            default: 1.0,
        },
        dependency: Some(EffectParamDependency::Dependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbMode",
        label: "黑边检测模式",
        kind: EffectParamKind::Select {
            default: 0.0,
            options: SelectOptions::Static(&BLACK_BORDER_MODE_OPTIONS),
        },
        dependency: Some(EffectParamDependency::Dependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
];

