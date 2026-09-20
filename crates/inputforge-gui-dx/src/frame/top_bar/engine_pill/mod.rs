// Rust guideline compliant 2026-04-29

mod logic;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::{
    context::AppContext,
    frame::{ViewState, view_state::PanelSlot},
};

use logic::{Variant, engine_pill_state};

#[component]
pub(crate) fn EnginePill() -> Element {
    tracing::trace!(target: "frame::render", region = "engine_pill");
    let ctx = use_context::<AppContext>();
    let status = use_memo(move || ctx.meta.read().engine_status);
    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());
    let commands = ctx.commands.clone();

    let s = *status.read();
    let p = *has_profile.read();
    let (variant, label, command) = engine_pill_state(s);
    let offline = ctx.meta.read().session.offline;
    let pending = ctx.controller_changes_pending();
    let view = try_use_context::<ViewState>();
    let class = format!("if-engine-pill if-engine-pill--{}", variant.class_suffix());

    // `aria-pressed` exposes the pill as a toggle button, Running
    // reads as the "on" state. Stopped read as "off". The
    // visible dot+label are aria-hidden so AT users hear the action
    // verb (button accessible name) followed by the live region
    // announcement, not the raw label twice.
    let aria_pressed = match variant {
        Variant::Live => "true",
        Variant::Warning | Variant::Error => "false",
    };
    // Action verb is derived from the dispatch command, not the
    // status: clicking always toggles, and the verb has to match the
    // outcome of the click for the button name to be honest.
    let action_verb = if pending {
        "Review controller changes in Devices"
    } else {
        match &command {
            EngineCommand::Activate => "Start routing",
            EngineCommand::Deactivate => "Stop routing",
            _ => "Engine",
        }
    };
    // sr-only live region carries the state announcement (separate
    // from the button so the button keeps its native role). Phrasing
    // is "Engine running / stopped", full sentence so AT
    // users get the subject, not a bare adjective.
    let live_text = format!("Engine {}", label.to_lowercase());

    // `EngineCommand` is not `Clone`/`Copy` (some variants carry
    // `Action`s and `PathBuf`s), so we discriminate by reference
    // and reconstruct the unit variant for sending. The fall-through
    // arm is a structural guard against `engine_pill_state` ever
    // returning a third variant, we log and bail rather than
    // silently rewriting to `Activate`.
    let onclick = move |_| {
        if pending {
            if let Some(mut view) = view {
                view.via_calibration.set(false);
                view.panel_slot.set(PanelSlot::Devices);
            }
            return;
        }
        let cmd = match &command {
            EngineCommand::Activate => EngineCommand::Activate,
            EngineCommand::Deactivate => EngineCommand::Deactivate,
            other => {
                tracing::error!(
                    target: "gui",
                    ?other,
                    "engine_pill_state returned unexpected variant"
                );
                return;
            }
        };
        let _ = commands.send(cmd);
    };

    rsx! {
        button {
            r#type: "button",
            class: "{class}",
            disabled: !p || offline || (pending && view.is_none()),
            title: action_verb,
            "aria-label": "{action_verb}",
            "aria-pressed": "{aria_pressed}",
            onclick,
            span { class: "if-engine-pill__dot", "aria-hidden": "true" }
            span { class: "if-engine-pill__label", "aria-hidden": "true", "{label}" }
        }
        span {
            class: "if-sr-only",
            role: "status",
            "aria-live": "polite",
            "{live_text}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot};
    use inputforge_core::{
        profile::controllers::ControllerConfig, state::AppState, types::VirtualDeviceConfig,
    };
    use parking_lot::RwLock;
    use std::sync::{Arc, mpsc};

    fn pending_harness() -> Element {
        let mut state = AppState::new();
        state.session.output_active = true;
        state.session.output_layout = vec![VirtualDeviceConfig {
            device_id: 1,
            axes: vec![],
            button_count: 2,
            hat_count: 0,
        }];
        let mut meta = MetaSnapshot::from_state(&state);
        meta.profile_name = Some("test".into());
        let meta = use_signal(|| meta);
        let config = use_signal(|| ConfigSnapshot {
            controllers: Some(ControllerConfig {
                virtual_devices: vec![VirtualDeviceConfig {
                    device_id: 1,
                    axes: vec![],
                    button_count: 3,
                    hat_count: 0,
                }],
                ..Default::default()
            }),
            ..Default::default()
        });
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(SettingsSnapshot::default);
        let (commands, _) = mpsc::channel();
        use_context_provider(|| AppContext {
            state: Arc::new(RwLock::new(state)),
            commands,
            meta,
            config,
            live,
            settings,
        });
        let view = crate::frame::use_view_state_provider(meta);
        use_context_provider(|| view);
        rsx! { EnginePill {} }
    }

    #[test]
    fn retained_layout_change_offers_review_instead_of_start() {
        let mut dom = VirtualDom::new(pending_harness);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("aria-label=\"Review controller changes in Devices\""));
        assert!(!html.contains("aria-label=\"Start routing\""));
        assert!(!html.contains(" disabled"));
    }
}
