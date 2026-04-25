use dioxus::prelude::*;

use crate::bridge::spawn_polling_task;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};

/// Root Dioxus component — assembles `AppContext`, installs it for descendants,
/// spawns the polling task once, and renders the F1 readout.
#[expect(dead_code, reason = "called by launch_gui in Task 10")]
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

    rsx! { F1Readout {} }
}

#[component]
fn F1Readout() -> Element {
    let ctx = use_context::<AppContext>();

    let status = use_memo(move || format!("{:?}", ctx.meta.read().engine_status));
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

    rsx! {
        main {
            style: "font-family: system-ui; padding: 24px; color: #ddd; \
                    background: #1A1A2E; min-height: 100vh;",
            h1 { "InputForge — Dioxus (F1 bridge smoke test)" }
            p { "Engine status: "     strong { "{status}" } }
            p { "Current mode: "      strong { "{mode}" } }
            p { "Active profile: "    strong { "{profile}" } }
            p { "Connected devices: " strong { "{devices}" } }
            p { "Virtual devices: "   strong { "{vdevices}" } }
            p { "Warnings: "          strong { "{warnings}" } }
            hr {}
            small { "Tray wiring: stubbed (F3). Theme: F2. Layout: F3." }
        }
    }
}
