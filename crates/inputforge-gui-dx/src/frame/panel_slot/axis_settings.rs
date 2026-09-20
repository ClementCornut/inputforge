//! Global axis interpretation, resolved through stable native binding codes.
#![allow(
    unused_qualifications,
    reason = "Dioxus event attributes produce false-positive qualification spans"
)]
use crate::{
    components::{Button, ButtonSize, ButtonVariant, Select, SelectOption},
    context::AppContext,
};
use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, settings::AxisSetting, types::AxisPolarity};

fn polarity_value(setting: Option<&AxisSetting>) -> &'static str {
    match setting.and_then(|setting| setting.override_polarity) {
        Some(AxisPolarity::Bipolar) => "centered",
        Some(AxisPolarity::Unipolar) => "minimum",
        None => "auto",
    }
}

#[component]
pub(super) fn AxisSettings() -> Element {
    let ctx = use_context::<AppContext>();
    let config = ctx.config.read().clone();
    let meta = ctx.meta.read().clone();
    let tables: Vec<_> = meta
        .session
        .bindings
        .iter()
        .chain(
            config
                .controllers
                .iter()
                .flat_map(|c| &c.bindings)
                .filter(|saved| {
                    !meta
                        .session
                        .bindings
                        .iter()
                        .any(|live| live.device == saved.device)
                }),
        )
        .collect();
    rsx! {
        details {
            summary { "Advanced axis settings" }
            p { "Leave each axis at rest for automatic detection. These settings apply across profiles." }
            for table in tables {
                for (index, axis) in table.axes.iter().enumerate() {
                    if let Ok(index) = u8::try_from(index) {
                        { let device = table.device.clone();
                          let setting = config.axis_settings.get(&device).and_then(|settings| settings.iter().find(|s| s.code == axis.code));
                          let value = polarity_value(setting);
                          let detected = match setting.and_then(|s| s.detected) {
                              Some(AxisPolarity::Bipolar) => "Detected: centered",
                              Some(AxisPolarity::Unipolar) => "Detected: rests at minimum",
                              None => "Not detected yet",
                          };
                          let label = format!("{} · Axis {}", config.device_display_name(&device), index);
                          let available = meta.session.monitored.contains(&device) && table.resolves(&inputforge_core::types::InputId::Axis { index });
                          let detect_device = device.clone();
                          let commands = ctx.commands.clone(); let detect_commands = ctx.commands.clone();
                          rsx! {
                            div { class: "if-controller-session__slot",
                                label { "{label}"
                                    Select { value: value.to_owned(), disabled: meta.session.offline,
                                        options: [("auto", "Auto"), ("centered", "Centered"), ("minimum", "Rests at minimum")].into_iter().map(|(value, label)| SelectOption { value: value.to_owned(), label: label.to_owned(), disabled: false, class: None }).collect(),
                                        onchange: move |event: FormEvent| {
                                            let polarity = match event.value().as_str() {
                                                "centered" => Some(AxisPolarity::Bipolar),
                                                "minimum" => Some(AxisPolarity::Unipolar),
                                                _ => None,
                                            };
                                            let _ = commands.send(EngineCommand::SetAxisPolarity { device: device.clone(), axis: index, polarity });
                                        },
                                    }
                                }
                                p { "{detected}" }
                                Button { size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: meta.session.offline || !available,
                                    aria_label: format!("Detect {label} again"),
                                    onclick: move |_| { let _ = detect_commands.send(EngineCommand::DetectAxis { device: detect_device.clone(), axis: index }); },
                                    "Detect again"
                                }
                            }
                          }
                        }
                    }
                }
            }
        }
    }
}
