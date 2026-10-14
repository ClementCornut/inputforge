// Rust guideline compliant 2026-05-11

//! `MapToMouse` body: target picker plus button behavior picker.

use dioxus::prelude::*;

use inputforge_core::action::{Action, MouseTarget, OutputBehavior};

use crate::components::{SegmentedControl, SegmentedControlOption, Select, SelectOption};
use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

const MOUSE_TARGETS: &[MouseTarget] = &[
    MouseTarget::LeftButton,
    MouseTarget::RightButton,
    MouseTarget::MiddleButton,
    MouseTarget::BackButton,
    MouseTarget::ForwardButton,
    MouseTarget::WheelUp,
    MouseTarget::WheelDown,
];

#[component]
pub(crate) fn MapToMouseBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    target: MouseTarget,
    behavior: OutputBehavior,
    root_actions: Vec<Action>,
) -> Element {
    let target_value = mouse_target_value(target).to_owned();
    let mut target_signal: Signal<String> = use_signal(|| target_value.clone());
    if *target_signal.peek() != target_value {
        target_signal.set(target_value.clone());
    }
    let target_options: Vec<SelectOption> = MOUSE_TARGETS
        .iter()
        .map(|candidate| SelectOption {
            value: mouse_target_value(*candidate).to_owned(),
            label: candidate.label().to_owned(),
            disabled: false,
            class: None,
        })
        .collect();

    let mapping_key_hold = mapping_key.clone();
    let stage_id_hold = stage_id.clone();
    let root_actions_hold = root_actions.clone();
    let on_hold = move |_| {
        if is_output_behavior_click_noop(behavior, OutputBehavior::Hold) {
            return;
        }
        dispatch_mouse_change(
            target,
            OutputBehavior::Hold,
            "behavior",
            &mapping_key_hold,
            &stage_id_hold,
            &root_actions_hold,
        );
    };

    let mapping_key_pulse = mapping_key.clone();
    let stage_id_pulse = stage_id.clone();
    let root_actions_pulse = root_actions.clone();
    let on_pulse = move |_| {
        if is_output_behavior_click_noop(behavior, OutputBehavior::Pulse) {
            return;
        }
        dispatch_mouse_change(
            target,
            OutputBehavior::Pulse,
            "behavior",
            &mapping_key_pulse,
            &stage_id_pulse,
            &root_actions_pulse,
        );
    };

    let mapping_key_target = mapping_key.clone();
    let stage_id_target = stage_id.clone();
    let root_actions_target = root_actions.clone();
    let on_target_change = move |evt: FormEvent| {
        let Some(candidate) = mouse_target_from_value(&evt.value()) else {
            tracing::warn!(
                target: "f9::mapping_editor",
                value = evt.value(),
                "mouse output target change ignored: unknown target"
            );
            return;
        };
        if is_mouse_target_click_noop(target, candidate) {
            return;
        }
        dispatch_mouse_change(
            candidate,
            behavior,
            "target",
            &mapping_key_target,
            &stage_id_target,
            &root_actions_target,
        );
    };

    rsx! {
        div { class: "if-stage__body-mouse",
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Target" }
                Select {
                    value: target_signal,
                    options: target_options,
                    onchange: on_target_change,
                }
            }
            if !target.is_wheel() {
                div { class: "if-stage__body-field",
                    label { class: "if-stage__body-label", "Behavior" }
                    SegmentedControl { aria_label: "Mouse output behavior".to_owned(),
                        SegmentedControlOption {
                            value: "hold".to_owned(),
                            selected: behavior == OutputBehavior::Hold,
                            onclick: on_hold,
                            "Hold"
                        }
                        SegmentedControlOption {
                            value: "pulse".to_owned(),
                            selected: behavior == OutputBehavior::Pulse,
                            onclick: on_pulse,
                            "Pulse"
                        }
                    }
                }
            }
        }
    }
}

fn mouse_target_value(target: MouseTarget) -> &'static str {
    match target {
        MouseTarget::LeftButton => "left_button",
        MouseTarget::RightButton => "right_button",
        MouseTarget::MiddleButton => "middle_button",
        MouseTarget::BackButton => "back_button",
        MouseTarget::ForwardButton => "forward_button",
        MouseTarget::WheelUp => "wheel_up",
        MouseTarget::WheelDown => "wheel_down",
    }
}

fn mouse_target_from_value(value: &str) -> Option<MouseTarget> {
    match value {
        "left_button" => Some(MouseTarget::LeftButton),
        "right_button" => Some(MouseTarget::RightButton),
        "middle_button" => Some(MouseTarget::MiddleButton),
        "back_button" => Some(MouseTarget::BackButton),
        "forward_button" => Some(MouseTarget::ForwardButton),
        "wheel_up" => Some(MouseTarget::WheelUp),
        "wheel_down" => Some(MouseTarget::WheelDown),
        _ => None,
    }
}

fn is_mouse_target_click_noop(current_target: MouseTarget, requested_target: MouseTarget) -> bool {
    current_target == requested_target
}

fn is_output_behavior_click_noop(
    current_behavior: OutputBehavior,
    requested_behavior: OutputBehavior,
) -> bool {
    current_behavior == requested_behavior
}

fn dispatch_mouse_change(
    new_target: MouseTarget,
    new_behavior: OutputBehavior,
    field: &'static str,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
) {
    let effective_behavior = if new_target.is_wheel() {
        OutputBehavior::Pulse
    } else {
        new_behavior
    };
    let new_action = Action::MapToMouse {
        target: new_target,
        behavior: effective_behavior,
    };
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Map to mouse"),
            field: Some(field),
            ..LabelArgs::default()
        },
    );
    super::instruments::stage_dispatch::dispatch_stage_edit(
        root_actions,
        stage_id,
        new_action,
        mapping_key,
        label,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_to_mouse_target_click_noop_only_when_target_is_unchanged() {
        assert!(is_mouse_target_click_noop(
            MouseTarget::LeftButton,
            MouseTarget::LeftButton
        ));
        assert!(!is_mouse_target_click_noop(
            MouseTarget::LeftButton,
            MouseTarget::RightButton
        ));
    }

    #[test]
    fn map_to_mouse_behavior_click_noop_only_when_behavior_is_unchanged() {
        assert!(is_output_behavior_click_noop(
            OutputBehavior::Hold,
            OutputBehavior::Hold
        ));
        assert!(!is_output_behavior_click_noop(
            OutputBehavior::Hold,
            OutputBehavior::Pulse
        ));
    }
}
