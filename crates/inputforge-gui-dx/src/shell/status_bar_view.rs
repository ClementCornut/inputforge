use dioxus::prelude::*;

use inputforge_core::state::EngineStatus;

use crate::components::{Badge, BadgeVariant, Separator, SeparatorOrientation, StatusBar};
use crate::context::AppContext;

/// Signal-bound consumer of the `StatusBar` primitive.
///
/// Subscribes to `meta.engine_status` / `meta.current_mode` /
/// `meta.profile_name` / `config.devices` and composes the four readouts:
/// engine-status badge (with `role="status" aria-live="polite"` on its
/// wrapper for AT announcements), mode badge (always rendered — Neutral
/// when the value is "Default"), `connected/total devices` text, and
/// profile-name span (plain `<span>`, not clickable in F3 — F14 owns
/// profile-manager wiring).
#[component]
pub(crate) fn StatusBarView() -> Element {
    let ctx = use_context::<AppContext>();

    let status = use_memo(move || ctx.meta.read().engine_status);
    let mode = use_memo(move || ctx.meta.read().current_mode.clone());
    let profile = use_memo(move || ctx.meta.read().profile_name.clone());
    let dev_count = use_memo(move || {
        let cfg = ctx.config.read();
        let connected = cfg.devices.iter().filter(|d| d.connected).count();
        (connected, cfg.devices.len())
    });

    // Capture Memo values as locals before rsx! — Memo<T> does not implement
    // Display directly, and rsx! does not accept top-level `let` bindings
    // between elements inside a slot. Lifting both is the idiomatic 0.7 form.
    let status_value = *status.read();
    let mode_str = mode.read().clone();
    let profile_str = profile.read().clone();
    let (connected, total) = *dev_count.read();

    rsx! {
        StatusBar {
            class: "if-placeholder-shell__status".to_owned(),
            start: rsx! {
                span { role: "status", "aria-live": "polite",
                    Badge {
                        variant: status_to_variant(status_value),
                        "{status_label(status_value)}"
                    }
                }
                Separator { orientation: SeparatorOrientation::Vertical }
                Badge { variant: BadgeVariant::Neutral, "{mode_str}" }
            },
            middle: rsx! {
                span { "{connected}/{total} devices" }
            },
            end: rsx! {
                if let Some(name) = profile_str.as_ref() {
                    span { "{name}" }
                }
            },
        }
    }
}

#[allow(dead_code, reason = "shell/ stays compilable until Task 32 removes it")]
fn status_to_variant(s: EngineStatus) -> BadgeVariant {
    match s {
        EngineStatus::Running => BadgeVariant::Success,
        EngineStatus::Paused => BadgeVariant::Warning,
        EngineStatus::Stopped => BadgeVariant::Neutral,
    }
}

#[allow(dead_code, reason = "shell/ stays compilable until Task 32 removes it")]
fn status_label(s: EngineStatus) -> &'static str {
    match s {
        EngineStatus::Running => "Running",
        EngineStatus::Paused => "Paused",
        EngineStatus::Stopped => "Stopped",
    }
}
