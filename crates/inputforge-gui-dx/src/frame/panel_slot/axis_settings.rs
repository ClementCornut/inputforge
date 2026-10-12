//! Render prepared global axis settings and wire app-owned callbacks.
use crate::{
    components::{Button, ButtonSize, ButtonVariant, Select, SelectOption},
    context::AppContext,
    controllers::{Action, Context},
    frame::mapping_list::source_label,
};
use dioxus::prelude::*;
use inputforge_core::types::{AxisPolarity, InputAddress, InputId};

#[component]
pub(super) fn AxisSettings() -> Element {
    let ctx = use_context::<AppContext>();
    let controllers = use_context::<Context>();
    let rows = controllers.view.read().axes.clone();
    let config = ctx.config.read();
    rsx! {
        details {
            summary { "Advanced axis settings" }
            p { "Leave each axis at rest for automatic detection. These settings apply across profiles." }
            for row in rows {
                { let label = source_label::format(&InputAddress::Bound { device: row.device.clone(), input: InputId::Axis { index: row.index } }, &config);
                  let value = match row.override_polarity {
                      Some(AxisPolarity::Bipolar) => "centered",
                      Some(AxisPolarity::Unipolar) => "minimum",
                      None => "auto",
                  };
                  let detected = match row.detected {
                      Some(AxisPolarity::Bipolar) => "Detected: centered",
                      Some(AxisPolarity::Unipolar) => "Detected: rests at minimum",
                      None => "Not detected yet",
                  };
                  let detect_device = row.device.clone();
                  let dispatch = controllers.dispatch;
                  rsx! {
                    div { class: "if-controller-session__slot",
                        label { "{label}"
                            Select {
                                id: format!("axis-polarity-{}-{}-{}", row.device.0, row.index, row.code),
                                value: value.to_owned(), disabled: !row.can_override,
                                options: [("auto", "Auto"), ("centered", "Centered"), ("minimum", "Rests at minimum")].into_iter().map(|(value, label)| SelectOption { value: value.to_owned(), label: label.to_owned(), disabled: false, class: None }).collect(),
                                onchange: move |event: FormEvent| dispatch.call(Action::Polarity { device: row.device.clone(), axis: row.index, code: row.code, value: event.value() }),
                            }
                        }
                        p { "{detected}" }
                        Button { size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: !row.can_detect,
                            aria_label: format!("Detect {label} again"),
                            onclick: move |_| dispatch.call(Action::Detect { device: detect_device.clone(), axis: row.index, code: row.code }),
                            "Detect again"
                        }
                    }
                  }
                }
            }
        }
    }
}
