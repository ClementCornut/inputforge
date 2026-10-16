//! Replace only the requested field, preserving effective sibling fields and branches.
use inputforge_core::{
    action::{Action, ModeChangeStrategy, MouseTarget, OutputBehavior},
    types::{MergeOp, OutputId, VJoyAxis},
};
use inputforge_gui_dx::mapping_edits::Field;
pub(super) fn replace(current: &Action, field: &Field) -> Option<Action> {
    let mut action = current.clone();
    match (&mut action, field) {
        (Action::ChangeMode { strategy }, Field::ModeHold(hold)) => {
            let (ModeChangeStrategy::SwitchTo { mode } | ModeChangeStrategy::Temporary { mode }) =
                strategy;
            *strategy = if *hold {
                ModeChangeStrategy::Temporary { mode: mode.clone() }
            } else {
                ModeChangeStrategy::SwitchTo { mode: mode.clone() }
            };
        }
        (Action::ChangeMode { strategy }, Field::ModeTarget(value)) => {
            let (ModeChangeStrategy::SwitchTo { mode } | ModeChangeStrategy::Temporary { mode }) =
                strategy;
            mode.clone_from(value);
        }
        (Action::MapToKeyboard { key, .. }, Field::Keyboard(value)) => key.clone_from(value),
        (Action::MapToKeyboard { behavior, .. }, Field::Behavior(value)) => *behavior = *value,
        (Action::MapToMouse { target, behavior }, Field::Behavior(value)) => {
            *behavior = if target.is_wheel() {
                OutputBehavior::Pulse
            } else {
                *value
            };
        }
        (Action::MapToMouse { target, behavior }, Field::MouseTarget(value)) => {
            *target = match value.as_str() {
                "left_button" => MouseTarget::LeftButton,
                "right_button" => MouseTarget::RightButton,
                "middle_button" => MouseTarget::MiddleButton,
                "back_button" => MouseTarget::BackButton,
                "forward_button" => MouseTarget::ForwardButton,
                "wheel_up" => MouseTarget::WheelUp,
                "wheel_down" => MouseTarget::WheelDown,
                _ => return None,
            };
            if target.is_wheel() {
                *behavior = OutputBehavior::Pulse;
            }
        }
        (Action::MapToVJoy { output }, Field::OutputDevice(raw)) => {
            output.device = raw.parse().ok()?;
        }
        (Action::MapToVJoy { output }, Field::OutputControl(raw)) => {
            output.output = output_id(raw)?;
        }
        (Action::MergeAxis { second_input, .. }, Field::MergeInput(value)) => {
            second_input.clone_from(value);
        }
        (Action::MergeAxis { operation, .. }, Field::MergeOperation(value)) => {
            *operation = match value.as_str() {
                "Bidirectional" => MergeOp::Bidirectional,
                "Average" => MergeOp::Average,
                "Maximum" => MergeOp::Maximum,
                _ => return None,
            }
        }
        (Action::Conditional { condition, .. }, field) => {
            *condition = super::predicate::replace(condition, field)?;
        }
        (
            Action::TapGesture { threshold_ms, .. } | Action::PressGesture { threshold_ms, .. },
            Field::GestureThreshold(raw),
        ) => {
            let value = raw.parse().ok()?;
            inputforge_core::action::validate_gesture_threshold_ms(value).ok()?;
            *threshold_ms = value;
        }
        (
            Action::TapGesture {
                fire_single_immediately,
                ..
            },
            Field::GestureToggle(value),
        ) => *fire_single_immediately = *value,
        (
            Action::PressGesture {
                fire_long_when_threshold_crossed,
                ..
            },
            Field::GestureToggle(value),
        ) => *fire_long_when_threshold_crossed = *value,
        _ => return None,
    }
    Some(action)
}
fn output_id(raw: &str) -> Option<OutputId> {
    if let Some(axis) = raw.strip_prefix("axis:") {
        return Some(OutputId::Axis {
            id: match axis {
                "X axis" => VJoyAxis::X,
                "Y axis" => VJoyAxis::Y,
                "Z axis" => VJoyAxis::Z,
                "Rx axis" => VJoyAxis::Rx,
                "Ry axis" => VJoyAxis::Ry,
                "Rz axis" => VJoyAxis::Rz,
                "Slider 0" => VJoyAxis::Slider0,
                "Slider 1" => VJoyAxis::Slider1,
                _ => return None,
            },
        });
    }
    if let Some(id) = raw.strip_prefix("button:") {
        return Some(OutputId::Button {
            id: id.parse().ok()?,
        });
    }
    Some(OutputId::Hat {
        id: raw.strip_prefix("hat:")?.parse().ok()?,
    })
}
