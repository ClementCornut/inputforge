// Rust guideline compliant 2026-04-29

mod logic;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;

use logic::engine_pill_state;

#[component]
pub(crate) fn EnginePill() -> Element {
    tracing::trace!(target: "frame::render", region = "engine_pill");
    let ctx = use_context::<AppContext>();
    let status = use_memo(move || ctx.meta.read().engine_status);
    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());
    let commands = ctx.commands.clone();

    let s = *status.read();
    let p = *has_profile.read();
    let (variant, label, command) = engine_pill_state(s, p);
    let class = format!("if-engine-pill if-engine-pill--{}", variant.class_suffix());

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
            "aria-live": "polite",
            role: "status",
            onclick,
            span { class: "if-engine-pill__dot" }
            span { class: "if-engine-pill__label", "{label}" }
        }
    }
}
