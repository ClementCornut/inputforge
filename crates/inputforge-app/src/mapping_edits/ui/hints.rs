//! Stage validation uses core values and publishes the existing diagnostic text.
use dioxus::prelude::*;
use inputforge_core::{
    action::{Condition, ModeChangeStrategy},
    types::OutputId,
};
use inputforge_gui_dx::mapping_edits::hints::{self, Input, Request, View};
pub(super) fn publish(request: Request) -> View {
    let mut view = View::default();
    let (message, tag) = match request.input {
        Input::Vjoy(output, devices) => {
            let device = devices.iter().find(|d| d.device_id == output.device);
            let valid = device.is_some_and(|device| valid_output(device, &output.output));
            (
                (!valid).then(|| {
                    if device.is_none() {
                        format!("Device {} not configured", output.device)
                    } else {
                        "Output not available on this device".into()
                    }
                }),
                None,
            )
        }
        Input::Merge(secondary, primary) => (
            if secondary.is_unbound() {
                Some(hints::MERGE_UNBOUND.into())
            } else if secondary == primary {
                Some("Secondary input must differ from primary".into())
            } else {
                None
            },
            None,
        ),
        Input::Predicate(condition, (min, max)) => {
            let message = match condition {
                Condition::ButtonPressed { input } | Condition::ButtonReleased { input } => {
                    input.is_unbound().then(|| hints::PREDICATE_UNBOUND.into())
                }
                Condition::AxisInRange { input, .. } => {
                    if input.is_unbound() {
                        Some(hints::PREDICATE_UNBOUND.into())
                    } else {
                        (min > max).then(|| "min must not exceed max".into())
                    }
                }
                Condition::HatDirection { input, directions } => {
                    if input.is_unbound() {
                        Some(hints::PREDICATE_UNBOUND.into())
                    } else {
                        directions
                            .is_empty()
                            .then(|| "at least one direction must be selected".into())
                    }
                }
                Condition::All { conditions } | Condition::Any { conditions } => conditions
                    .is_empty()
                    .then(|| "at least one condition is required".into()),
                Condition::Not { .. } => return view,
            };
            (message, None)
        }
        Input::Mode(strategy, primary, modes) => {
            let hold = matches!(strategy, ModeChangeStrategy::Temporary { .. });
            let (ModeChangeStrategy::Temporary { mode } | ModeChangeStrategy::SwitchTo { mode }) =
                strategy;
            view.hold_disabled = !primary.is_button_shaped();
            view.target_orphaned = !mode.is_empty() && !modes.contains(&mode);
            if mode.is_empty() {
                (
                    Some(hints::TARGET_EMPTY.into()),
                    Some("Set target mode".into()),
                )
            } else if view.target_orphaned && hold && view.hold_disabled {
                (
                    Some(format!(
                        r#"Mode "{mode}" no longer exists. Hold also requires a button input. Pick a button on your device, then a current mode."#
                    )),
                    Some("Mode missing".into()),
                )
            } else if view.target_orphaned {
                (
                    Some(format!(
                        r#"Mode "{mode}" no longer exists. Pick a current mode."#
                    )),
                    Some("Mode missing".into()),
                )
            } else if hold && view.hold_disabled {
                (
                    Some(hints::HOLD_NOT_BUTTON.into()),
                    Some("Needs button".into()),
                )
            } else {
                (None, None)
            }
        }
    };
    publish_text(request.hints, &request.path, message);
    if let Some(tags) = request.tags {
        publish_text(tags, &request.path, tag);
    }
    view
}
fn publish_text(
    mut signal: Signal<
        std::collections::HashMap<inputforge_gui_dx::mapping_edits::StageId, String>,
    >,
    path: &inputforge_gui_dx::mapping_edits::StageId,
    text: Option<String>,
) {
    if signal.peek().get(path) == text.as_ref() {
        return;
    }
    if let Some(text) = text {
        signal.write().insert(path.clone(), text);
    } else {
        signal.write().remove(path);
    }
}

fn valid_output(config: &inputforge_core::types::VirtualDeviceConfig, output: &OutputId) -> bool {
    match output {
        OutputId::Axis { id } => config.axes.contains(id),
        OutputId::Button { id } => *id > 0 && *id <= config.button_count,
        OutputId::Hat { id } => *id > 0 && *id <= config.hat_count,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_and_last_one_based_controls_are_valid_and_zero_is_rejected() {
        let config = inputforge_core::types::VirtualDeviceConfig {
            device_id: 1,
            axes: vec![],
            button_count: 2,
            hat_count: 1,
        };
        assert!(!valid_output(&config, &OutputId::Button { id: 0 }));
        assert!(valid_output(&config, &OutputId::Button { id: 2 }));
        assert!(!valid_output(&config, &OutputId::Hat { id: 0 }));
        assert!(valid_output(&config, &OutputId::Hat { id: 1 }));
    }
}
