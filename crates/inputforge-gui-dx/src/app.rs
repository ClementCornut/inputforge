use dioxus::prelude::*;

use crate::LaunchParams;
use crate::bridge::spawn_polling_task;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::frame;
use crate::lifecycle;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::theme::ThemeProvider;
use crate::toast::{ToastQueue, ToastState, ToastViewport, install_warnings_bridge};
use crate::tray;
use crate::tray::action::TrayAction;

/// Root Dioxus component, assembles `AppContext`, installs it for descendants,
/// installs `ViewState` for frame chrome, spawns the polling task, wires the
/// tray bridge (channel + handler + listener), applies `--start-minimized`,
/// and renders the F7 frame layout.
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

    let view = frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);

    // F4: ToastQueue context, Signal lives in app_root's scope, mirroring the
    // F1 AppContext pattern. Calling Signal::new() outside a hook leaks per
    // dioxus-signals/src/signal.rs:30-52, so use_signal is mandatory here.
    let toast_state = use_signal(ToastState::default);
    let toast_queue = ToastQueue { state: toast_state };
    use_context_provider(|| toast_queue);

    // F8: live-capture primitive. Single instance, sibling of ToastQueue.
    // Provider self-installs the context.
    use_live_capture_provider();

    // F4: warnings bridge, reads ctx.meta, pushes new tail entries as
    // Warning toasts. last_seen initializes from peek() so first run is a
    // no-op even if warnings accumulated before mount.
    let last_seen = use_signal(|| ctx.meta.peek().warnings.len());
    use_effect(install_warnings_bridge(ctx.clone(), toast_queue, last_seen));

    // Polling task, bridges AppState into Dioxus signals. One-shot per scope
    // mount; auto-cancelled when the runtime tears down.
    use_hook(|| spawn_polling_task(ctx.clone()));

    // Tray bridge, channel + listener task created once; the handler is
    // installed at the top level (it's itself a hook).
    //
    // The channel construction lives inside `use_hook` so it runs exactly
    // once per scope mount. `spawn_listener_task` consumes `rx` there.
    // `install_event_handler` MUST be top-level because it wraps
    // `use_muda_event_handler` (a hook), calling it from inside another
    // hook's initializer would only register on first render.
    let tx = use_hook(|| {
        let (tx, rx) = tokio::sync::mpsc::channel::<TrayAction>(tray::CHANNEL_CAPACITY);
        tray::spawn_listener_task(rx, ctx.clone());
        tx
    });
    tray::install_event_handler(params.tray_menu_ids.clone(), tx);

    // --start-minimized, applied once on first mount.
    use_hook(|| lifecycle::apply_start_minimized(params.start_minimized));

    app_root_view()
}

/// Pure render fn: assumes `AppContext`, `ViewState`, and `ToastQueue` are
/// already in context (provided by `app_root` at runtime, by the test
/// harness in unit tests). Holds the *single* source of truth for what
/// the application root mounts, both `app_root` and the mount-regression
/// test in `tests` below render through this function, so a regression
/// that swaps `frame::Layout` for any other component lives here and gets
/// caught by `app_root_mounts_frame_layout_not_placeholder_shell`.
fn app_root_view() -> Element {
    rsx! {
        ThemeProvider {
            ToastViewport {}
            frame::Layout {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, mpsc};

    use dioxus::prelude::*;
    use dioxus_ssr::render;
    use parking_lot::RwLock;

    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;

    use crate::LaunchParams;
    use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
    use crate::frame;
    use crate::toast::{ToastQueue, ToastState};
    use crate::tray::action::TrayMenuIds;

    /// Test harness: provides every context that `app_root_view` reads
    /// (`AppContext`, `ViewState`, `ToastQueue`, plus the upstream
    /// `RawHandles` and `LaunchParams` for symmetry with the runtime
    /// path), then renders the **same** `app_root_view()` that `app_root`
    /// renders at runtime. This makes the rsx tree at the application
    /// root single-source-of-truth: a regression that changes
    /// `frame::Layout {}` to anything else lives in `app_root_view` and
    /// fails the assertions below.
    ///
    /// Side-effect hooks (`warnings_bridge`, polling task, tray bridge,
    /// `start-minimized`) are intentionally not wired here, they require
    /// a live Dioxus desktop runtime and are exercised by integration
    /// tests / the smoke run, not by this SSR mount test.
    fn app_root_view_with_stub_contexts() -> Element {
        // --- minimal stubs for context providers ---
        let (cmd_tx, _cmd_rx) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        use_context_provider(|| LaunchParams {
            start_minimized: false,
            tray_menu_ids: TrayMenuIds {
                show: muda::MenuId::new("show"),
                toggle: muda::MenuId::new("toggle"),
                quit: muda::MenuId::new("quit"),
            },
        });

        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx.clone());

        let view = frame::use_view_state_provider(ctx.meta);
        use_context_provider(|| view);

        // ToastViewport (rendered by app_root_view) reads ToastQueue;
        // provide it with an empty toast state.
        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });

        // F8: mirror the runtime path's live-capture provider install so
        // any consumer that lands inside `app_root_view` and reads the
        // context resolves successfully under the test harness too.
        crate::patterns::live_capture::use_live_capture_provider();

        super::app_root_view()
    }

    /// Verifies that the same `app_root_view()` used by `app_root` mounts
    /// `frame::Layout` and does NOT mount `PlaceholderShell`. Because
    /// `app_root_view` is the single source of truth for what the
    /// application root renders, a regression that swaps `frame::Layout`
    /// for any other component is caught here.
    #[test]
    fn app_root_mounts_frame_layout_not_placeholder_shell() {
        let mut vdom = VirtualDom::new(app_root_view_with_stub_contexts);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("if-layout"),
            "frame::Layout should mount: expected `if-layout` class in rendered HTML, got: {html}"
        );
        assert!(
            !html.contains("if-placeholder-shell"),
            "PlaceholderShell must not render: found `if-placeholder-shell` class"
        );
    }
}
