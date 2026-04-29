mod logic;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::{Button, ButtonVariant};
use crate::context::AppContext;
use crate::frame::view_state::ViewState;

use logic::{BannerState, derive_banner_state};

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: BANNER_CSS }"
)]
const BANNER_CSS: Asset = asset!("/assets/frame/banner.css");

#[component]
pub(crate) fn Banner() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let commands_ref = ctx.commands.clone();

    let state = use_memo(move || {
        let m = ctx.meta.read();
        let e = view.editing_mode.read().clone();
        derive_banner_state(&e, &m.current_mode, m.mode_force.as_ref())
    });

    let s = state.read().clone();
    rsx! {
        Stylesheet { href: BANNER_CSS }
        match s {
            BannerState::Hidden => rsx! {},
            // Diverged: informational. role=status + polite + atomic so AT
            // re-announces the full message on transition.
            BannerState::Diverged { editing, current } => {
                let edit_for_btn = editing.clone();
                let cmd = commands_ref.clone();
                rsx! {
                    div { class: "if-banner if-banner--diverged",
                        role: "status", "aria-live": "polite", "aria-atomic": "true",
                        "aria-label": "Mode banner",
                        span { class: "if-banner__copy",
                            "Editing "
                            strong { "{editing}" }
                            " — engine is in "
                            strong { "{current}" }
                            "."
                        }
                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| {
                                let _ = cmd.send(EngineCommand::ForceMode { mode: edit_for_btn.clone() });
                            },
                            "Activate {editing}"
                        }
                    }
                }
            }
            // Forced (alone): steady override; polite is fine — the user
            // already triggered the force, so AT doesn't need to interrupt.
            BannerState::Forced { forced } => {
                let cmd = commands_ref.clone();
                rsx! {
                    div { class: "if-banner if-banner--forced",
                        role: "status", "aria-live": "polite", "aria-atomic": "true",
                        "aria-label": "Mode banner",
                        span { class: "if-banner__copy",
                            "Engine override: "
                            strong { "{forced}" }
                            ". Mode-change rules paused."
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| {
                                let _ = cmd.send(EngineCommand::ReleaseMode);
                            },
                            "Release"
                        }
                    }
                }
            }
            // ForcedAndDiverged: the user is editing one mode while the
            // engine is forced into another — escalate to role=alert +
            // assertive so AT interrupts and announces immediately. The
            // user must understand the divergence before further edits.
            BannerState::ForcedAndDiverged { editing, forced } => {
                let edit_for_btn = editing.clone();
                let cmd_a = commands_ref.clone();
                let cmd_r = commands_ref.clone();
                rsx! {
                    div { class: "if-banner if-banner--forced",
                        role: "alert", "aria-live": "assertive", "aria-atomic": "true",
                        "aria-label": "Mode banner",
                        span { class: "if-banner__copy",
                            "Editing "
                            strong { "{editing}" }
                            " — engine is in "
                            strong { "{forced}" }
                            " (forced). Mode-change rules paused."
                        }
                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| {
                                let _ = cmd_a.send(EngineCommand::ForceMode { mode: edit_for_btn.clone() });
                            },
                            "Activate {editing}"
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| {
                                let _ = cmd_r.send(EngineCommand::ReleaseMode);
                            },
                            "Release"
                        }
                    }
                }
            }
        }
    }
}
