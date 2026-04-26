use dioxus::prelude::*;

use crate::bridge::spawn_polling_task;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::theme::ThemeProvider;

/// Root Dioxus component — assembles `AppContext`, installs it for descendants,
/// spawns the polling task once, and renders the F1 readout.
pub(crate) fn app_root() -> Element {
    let raw = use_context::<RawHandles>();

    let meta = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);

    let ctx = AppContext {
        state: std::sync::Arc::clone(&raw.state),
        commands: raw.commands.clone(),
        settings: std::sync::Arc::clone(&raw.settings),
        meta,
        config,
        live,
    };
    use_context_provider(|| ctx.clone());

    // One-shot per scope mount; auto-cancelled when the runtime tears down.
    use_hook(|| spawn_polling_task(ctx.clone()));

    rsx! {
        ThemeProvider {
            F1Readout {}
        }
    }
}

#[component]
fn F1Readout() -> Element {
    let ctx = use_context::<AppContext>();

    let status_text = use_memo(move || format!("{:?}", ctx.meta.read().engine_status));
    let status_variant = use_memo(move || match ctx.meta.read().engine_status {
        inputforge_core::state::EngineStatus::Running => crate::components::BadgeVariant::Success,
        inputforge_core::state::EngineStatus::Paused => crate::components::BadgeVariant::Warning,
        inputforge_core::state::EngineStatus::Stopped => crate::components::BadgeVariant::Neutral,
    });
    let mode = use_memo(move || ctx.meta.read().current_mode.clone());
    let profile = use_memo(move || {
        ctx.meta
            .read()
            .profile_name
            .clone()
            .unwrap_or_else(|| "<none>".into())
    });
    let devices = use_memo(move || ctx.config.read().devices.len());
    let vdevices = use_memo(move || ctx.config.read().virtual_devices.len());
    let warnings = use_memo(move || ctx.meta.read().warnings.len());
    let warnings_variant = use_memo(move || {
        if *warnings.read() == 0 {
            crate::components::BadgeVariant::Neutral
        } else {
            crate::components::BadgeVariant::Warning
        }
    });

    rsx! {
        main {
            crate::components::Stack { gap: "--space-4".to_owned(), padding: "--space-6".to_owned(),
                h1 { "InputForge — Dioxus (F1 bridge smoke test)" }
                crate::components::Card { padding: crate::components::CardPadding::Md,
                    // Two-column key/value grid; not a Stack/Cluster fit (asymmetric grid).
                    div { style: "display: grid; grid-template-columns: max-content 1fr; gap: var(--space-2) var(--space-4);",
                        crate::components::Label { for_id: None::<String>, "Engine status:" }
                        div { crate::components::Badge { variant: *status_variant.read(), "{status_text}" } }

                        crate::components::Label { for_id: None::<String>, "Current mode:" }
                        div { strong { "{mode}" } }

                        crate::components::Label { for_id: None::<String>, "Active profile:" }
                        div { "{profile}" }

                        crate::components::Label { for_id: None::<String>, "Connected devices:" }
                        div { "{devices}" }

                        crate::components::Label { for_id: None::<String>, "Virtual devices:" }
                        div { "{vdevices}" }

                        crate::components::Label { for_id: None::<String>, "Warnings:" }
                        div { crate::components::Badge { variant: *warnings_variant.read(), "{warnings}" } }
                    }
                }
                small { style: "color: var(--color-text-muted);", "Tray wiring: stubbed (F3). Theme: F2 ✓. Layout: F2 ✓." }
            }
        }
    }
}
