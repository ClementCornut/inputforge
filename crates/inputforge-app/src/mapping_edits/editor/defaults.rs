//! Existing palette defaults, with current app-owned gesture settings.
use inputforge_core::{
    action::{Action, Condition, ModeChangeStrategy, MouseTarget, OutputBehavior},
    processing::{DeadzoneConfig, ResponseCurve},
    state::AppState,
    types::{InputAddress, MergeOp, OutputAddress, OutputId, VJoyAxis},
};
use inputforge_gui_dx::mapping_edits::StageKind;
pub(super) fn action(kind: &StageKind, state: &AppState) -> Action {
    match kind {
        StageKind::Invert => Action::Invert,
        StageKind::Deadzone => Action::Deadzone {
            config: DeadzoneConfig::default(),
        },
        StageKind::ResponseCurve => Action::ResponseCurve {
            curve: ResponseCurve::PiecewiseLinear {
                points: vec![(-1.0, -1.0), (1.0, 1.0)],
                symmetric: false,
            },
        },
        StageKind::VirtualDevice => Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
        StageKind::Keyboard(key) => Action::MapToKeyboard {
            key: key.clone(),
            behavior: OutputBehavior::Hold,
        },
        StageKind::Mouse => Action::MapToMouse {
            target: MouseTarget::LeftButton,
            behavior: OutputBehavior::Hold,
        },
        StageKind::MergeAxis => Action::MergeAxis {
            second_input: InputAddress::Unbound,
            operation: MergeOp::Average,
        },
        StageKind::Conditional => Action::Conditional {
            condition: Condition::ButtonPressed {
                input: InputAddress::Unbound,
            },
            if_true: vec![],
            if_false: vec![],
        },
        StageKind::ChangeMode => Action::ChangeMode {
            strategy: ModeChangeStrategy::SwitchTo {
                mode: String::new(),
            },
        },
        StageKind::Tap => Action::TapGesture {
            threshold_ms: state.default_double_tap_threshold_ms,
            fire_single_immediately: false,
            single_tap: vec![],
            double_tap: vec![],
        },
        StageKind::Press => Action::PressGesture {
            threshold_ms: state.default_long_press_threshold_ms,
            fire_long_when_threshold_crossed: false,
            short_press: vec![],
            long_press: vec![],
        },
    }
}
