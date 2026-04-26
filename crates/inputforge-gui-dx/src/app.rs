use dioxus::prelude::*;

use crate::LaunchParams;
use crate::bridge::spawn_polling_task;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::lifecycle;
use crate::shell::PlaceholderShell;
use crate::theme::ThemeProvider;
use crate::toast::{ToastQueue, ToastState, ToastViewport, install_warnings_bridge};
use crate::tray;
use crate::tray::action::TrayAction;

/// Root Dioxus component — assembles `AppContext`, installs it for descendants,
/// spawns the polling task, wires the tray bridge (channel + handler + listener),
/// applies `--start-minimized`, and renders the placeholder shell.
pub(crate) fn app_root() -> Element {
    let raw = use_context::<RawHandles>();
    let params = use_context::<LaunchParams>();

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

    // F4: ToastQueue context — Signal lives in app_root's scope, mirroring the
    // F1 AppContext pattern. Calling Signal::new() outside a hook leaks per
    // dioxus-signals/src/signal.rs:30-52, so use_signal is mandatory here.
    let toast_state = use_signal(ToastState::default);
    let toast_queue = ToastQueue { state: toast_state };
    use_context_provider(|| toast_queue);

    // F4: warnings bridge — reads ctx.meta, pushes new tail entries as
    // Warning toasts. last_seen initializes from peek() so first run is a
    // no-op even if warnings accumulated before mount.
    let last_seen = use_signal(|| ctx.meta.peek().warnings.len());
    use_effect(install_warnings_bridge(ctx.clone(), toast_queue, last_seen));

    // Polling task — bridges AppState into Dioxus signals. One-shot per scope
    // mount; auto-cancelled when the runtime tears down.
    use_hook(|| spawn_polling_task(ctx.clone()));

    // Tray bridge — channel + listener task created once; the handler is
    // installed at the top level (it's itself a hook).
    //
    // The channel construction lives inside `use_hook` so it runs exactly
    // once per scope mount. `spawn_listener_task` consumes `rx` there.
    // `install_event_handler` MUST be top-level because it wraps
    // `use_muda_event_handler` (a hook) — calling it from inside another
    // hook's initializer would only register on first render.
    let tx = use_hook(|| {
        let (tx, rx) = tokio::sync::mpsc::channel::<TrayAction>(tray::CHANNEL_CAPACITY);
        tray::spawn_listener_task(rx, ctx.clone());
        tx
    });
    tray::install_event_handler(params.tray_menu_ids.clone(), tx);

    // --start-minimized — applied once on first mount.
    use_hook(|| lifecycle::apply_start_minimized(params.start_minimized));

    rsx! {
        ThemeProvider {
            ToastViewport {}
            PlaceholderShell {}
        }
    }
}
