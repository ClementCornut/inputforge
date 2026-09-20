//! Shared routing controls; inspecting a device never selects it for routing.
#![allow(
    unused_qualifications,
    reason = "Dioxus event attributes produce false-positive qualification spans"
)]
use super::{axis_settings::AxisSettings, virtual_config::VirtualConfig};
use crate::{
    components::{Button, ButtonSize, ButtonVariant},
    context::AppContext,
};
use dioxus::prelude::*;
use inputforge_core::{
    engine::EngineCommand, profile::controllers::ControllerConfig, state::EngineStatus,
};

pub(super) fn update_config(ctx: &AppContext, change: impl FnOnce(&mut ControllerConfig)) {
    let mut config = ctx
        .state
        .read()
        .active_profile
        .as_ref()
        .and_then(|p| p.controllers())
        .cloned()
        .unwrap_or_default();
    change(&mut config);
    let _ = ctx
        .commands
        .send(EngineCommand::SetControllerConfig(config));
}

#[component]
pub(super) fn ControllerSession() -> Element {
    let ctx = use_context::<AppContext>();
    let meta = ctx.meta.read().clone();
    let config = ctx.config.read().clone();
    let controllers = config.controllers.clone().unwrap_or_default();
    let pending_layout = meta
        .session
        .requires_output_reconfiguration(&controllers.virtual_devices);
    let locked = matches!(
        meta.engine_status,
        EngineStatus::Running | EngineStatus::Starting
    ) || !meta.session.captured.is_empty()
        || meta.session.offline
        || meta.profile_name.is_none();
    let can_start = meta.profile_name.is_some()
        && !meta.session.offline
        && !pending_layout
        && !matches!(
            meta.engine_status,
            EngineStatus::Running | EngineStatus::Starting
        );
    let ready = if meta.session.ready {
        "Inputs ready"
    } else {
        "Waiting for inputs"
    };
    let monitored = meta.session.monitored.len();
    let captured = meta.session.captured.len();
    let tx_run = ctx.commands.clone();
    let tx_stop = ctx.commands.clone();
    let tx_refresh = ctx.commands.clone();
    rsx! {
        section { class: "if-controller-session", "aria-label": "Controller session",
            h3 { "Controller session" }
            p { role: "status", "{ready} · {monitored} monitored · {captured} captured" }
            div { class: "if-controller-session__actions",
                Button { size: ButtonSize::Sm, disabled: !can_start, onclick: move |_| { let _ = tx_run.send(EngineCommand::Activate); },
                    if meta.engine_status == EngineStatus::Faulted { "Retry" } else { "Start" }
                }
                Button { size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: meta.session.offline || meta.engine_status == EngineStatus::Stopped,
                    onclick: move |_| { let _ = tx_stop.send(EngineCommand::Deactivate); }, "Stop"
                }
            }
            if meta.engine_status == EngineStatus::Stopped && meta.session.output_active {
                p { "Routing stopped. Virtual controllers stay connected and neutral. Inputs remain available." }
            }
            if let Some(error) = meta.session.error { p { role: "alert", "{error}" } }
            if !meta.session.keyboard_supported { p { "Keyboard output is unavailable." } }
            if !meta.session.mouse_supported { p { "Mouse output is unavailable." } }
            fieldset { disabled: locked,
                legend { "Route these controllers" }
                for row in config.device_panel_rows.iter().cloned() {
                    { let id = row.device_id.clone(); let commands = ctx.commands.clone(); let state = std::sync::Arc::clone(&ctx.state);
                        rsx! { label {
                            input { r#type: "checkbox", checked: controllers.selected.contains(&id), disabled: !row.connected && !controllers.selected.contains(&id),
                                onchange: move |event: FormEvent| {
                                    let mut selected = state.read().active_profile.as_ref().and_then(|p| p.controllers()).map_or_else(Vec::new, |p| p.selected.clone());
                                    selected.retain(|d| d != &id);
                                    if event.checked() { selected.push(id.clone()); }
                                    let _ = commands.send(EngineCommand::SelectControllers(selected));
                                }
                            }
                            "{row.display_name}"
                            if !row.connected { " (disconnected)" }
                        } }
                    }
                }
                p { "Stop routing to change the controller selection or virtual output." }
            }
            Button { size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: meta.session.offline,
                onclick: move |_| { let _ = tx_refresh.send(EngineCommand::RefreshInput); }, "Refresh devices"
            }
            AxisSettings {}
            VirtualConfig { locked }
        }
    }
}
