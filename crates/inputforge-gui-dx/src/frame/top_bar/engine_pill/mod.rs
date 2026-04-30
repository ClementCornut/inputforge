// Rust guideline compliant 2026-04-29

mod logic;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;

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
    let class = format!("if-engine-pill if-engine-pill--{}", variant.class_suffix());

    // `aria-pressed` exposes the pill as a toggle button — Running
    // reads as the "on" state. Paused/Stopped read as "off". The
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
    let action_verb = match &command {
        EngineCommand::Activate => "Activate engine",
        EngineCommand::Deactivate => "Deactivate engine",
        _ => "Engine",
    };
    // sr-only live region carries the state announcement (separate
    // from the button so the button keeps its native role). Phrasing
    // is "Engine running / paused / stopped" — full sentence so AT
    // users get the subject, not a bare adjective.
    let live_text = format!("Engine {}", label.to_lowercase());

    // `EngineCommand` is not `Clone`/`Copy` (some variants carry
    // `Action`s and `PathBuf`s), so we discriminate by reference
    // and reconstruct the unit variant for sending. The fall-through
    // arm is a structural guard against `engine_pill_state` ever
    // returning a third variant — we log and bail rather than
    // silently rewriting to `Activate`.
    let onclick = move |_| {
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
            disabled: !p,
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
