#![allow(
    unused_qualifications,
    reason = "Dioxus event attributes produce false-positive qualification spans"
)]
use super::controller_session::update_config;
use crate::{
    components::{Button, ButtonSize, ButtonVariant},
    context::AppContext,
};
use dioxus::prelude::*;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{VJoyAxis, VirtualDeviceConfig};
const AXES: [VJoyAxis; 8] = [
    VJoyAxis::X,
    VJoyAxis::Y,
    VJoyAxis::Z,
    VJoyAxis::Rx,
    VJoyAxis::Ry,
    VJoyAxis::Rz,
    VJoyAxis::Slider0,
    VJoyAxis::Slider1,
];

#[component]
pub(super) fn VirtualConfig(locked: bool) -> Element {
    let ctx = use_context::<AppContext>();
    let capabilities = ctx.meta.read().session.controller_capabilities;
    let requested_configs = ctx
        .config
        .read()
        .controllers
        .as_ref()
        .map_or_else(Vec::new, |p| p.virtual_devices.clone());
    let configs = if capabilities.configurable {
        requested_configs.clone()
    } else {
        ctx.config.read().virtual_devices.clone()
    };
    let pending = ctx
        .meta
        .read()
        .session
        .requires_output_reconfiguration(&requested_configs);
    let apply_commands = ctx.commands.clone();
    let add_ctx = ctx.clone();
    rsx! {
        fieldset { disabled: locked,
            legend { "Virtual controllers" }
            if !capabilities.configurable { p { "Virtual controller layouts are provided by the output driver." } }
            if pending {
                p { role: "status", "Apply controller changes before starting. This reconnects the virtual controllers." }
                Button { size: ButtonSize::Sm, onclick: move |_| { let _ = apply_commands.send(EngineCommand::ApplyControllerChanges); }, "Apply controller changes" }
            }
            for config in configs.iter().cloned() {
                VirtualSlot { config, locked: !capabilities.configurable }
            }
            if capabilities.configurable { Button { size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: configs.len() >= 16,
                onclick: move |_| update_config(&add_ctx, |p| {
                    if let Some(slot) = (1..=16).find(|slot| !p.virtual_devices.iter().any(|c| c.device_id == *slot)) {
                        p.virtual_devices.push(VirtualDeviceConfig {device_id: slot, axes: AXES.to_vec(), button_count: 32_u8.clamp(capabilities.min_buttons, capabilities.max_buttons), hat_count: capabilities.max_hats});
                    }
                }), "Add virtual controller"
            } }
        }
    }
}
#[component]
fn VirtualSlot(config: VirtualDeviceConfig, locked: bool) -> Element {
    let ctx = use_context::<AppContext>();
    let capabilities = ctx.meta.read().session.controller_capabilities;
    let slot = config.device_id;
    let buttons_ctx = ctx.clone();
    let hats_ctx = ctx.clone();
    let remove_ctx = ctx.clone();
    rsx! {
        div { class: "if-controller-session__slot",
            h4 { "Virtual controller {slot}" }
            label { "Buttons"
                input { r#type: "number", disabled: locked, min: "{capabilities.min_buttons}", max: "{capabilities.max_buttons}", value: "{config.button_count}",
                    onchange: move |event: FormEvent| { if let Ok(count) = event.value().parse::<u8>() { update_config(&buttons_ctx, |p| { if let Some(c) = p.virtual_devices.iter_mut().find(|c| c.device_id == slot) { c.button_count = count; } }); } }
                }
            }
            label { "Hats"
                input { r#type: "number", disabled: locked, min: "0", max: "{capabilities.max_hats}", value: "{config.hat_count}",
                    onchange: move |event: FormEvent| { if let Ok(count) = event.value().parse::<u8>() { update_config(&hats_ctx, |p| { if let Some(c) = p.virtual_devices.iter_mut().find(|c| c.device_id == slot) { c.hat_count = count; } }); } }
                }
            }
            div { class: "if-controller-session__axes",
                for axis in AXES {
                    { let ctx = ctx.clone();
                        rsx! { label { input { r#type: "checkbox", disabled: locked, checked: config.axes.contains(&axis),
                            onchange: move |event: FormEvent| update_config(&ctx, |p| {
                                if let Some(c) = p.virtual_devices.iter_mut().find(|c| c.device_id == slot) {
                                    c.axes.retain(|a| a != &axis); if event.checked() { c.axes.push(axis); }
                                }
                            })
                        } "{axis:?}" } }
                    }
                }
            }
            if capabilities.configurable { Button { size: ButtonSize::Sm, variant: ButtonVariant::Danger, onclick: move |_| update_config(&remove_ctx, |p| p.virtual_devices.retain(|c| c.device_id != slot)), "Remove controller {slot}" } }
        }
    }
}
