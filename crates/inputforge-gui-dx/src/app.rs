use dioxus::prelude::*;

use crate::bridge::Publisher;
use crate::context::{
    AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles, SettingsSnapshot,
};
use crate::frame;
use crate::frame::use_editor_state_provider;
use crate::patterns::keyboard_capture::use_keyboard_capture_provider;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::theme::ThemeProvider;
use crate::toast::{ToastQueue, ToastState, ToastViewport, install_warnings_bridge};

/// Assemble editor presentation contexts and render the frame layout.
pub(crate) fn app_root() -> Element {
    let raw = use_context::<RawHandles>();
    let bindings = use_context::<crate::routing::Bindings>();
    let routing_view = use_signal(crate::routing::View::default);
    let routing = crate::routing::Context {
        view: routing_view,
        dispatch: bindings.dispatch,
    };
    use_context_provider(|| routing);
    let controller_bindings = use_context::<crate::controllers::Bindings>();
    let mut controller_error = use_signal(|| None);
    let controllers = crate::controllers::Context {
        view: use_signal(crate::controllers::View::default),
        error: controller_error,
        dispatch: Callback::new(move |action| {
            controller_error.set(controller_bindings.dispatch.call(action));
        }),
    };
    use_context_provider(|| controllers);

    let meta = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);

    let ctx = AppContext {
        state: std::sync::Arc::clone(&raw.state),
        commands: raw.commands.clone(),
        settings,
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

    // Keyboard capture primitive. Single root listener bridge, armed by callers
    // only while they need global shortcut capture.
    use_keyboard_capture_provider();

    // F9: editor-internal state. Single instance, sibling of LiveCapture.
    use_editor_state_provider();

    // F4: warnings bridge, reads ctx.meta, pushes new tail entries as
    // Warning toasts. last_seen initializes from peek() so first run is a
    // no-op even if warnings accumulated before mount.
    let last_seen = use_signal(|| ctx.meta.peek().warnings.len());
    use_effect(install_warnings_bridge(ctx.clone(), toast_queue, last_seen));

    // Start the app-owned bridge once after its GUI signals are initialized.
    let start_polling = use_context::<Callback<Publisher>>();
    use_hook(|| start_polling.call(Publisher::new(ctx.clone(), view, routing, controllers)));

    // F13: window-level Ctrl+S handler that dispatches the manual snapshot
    // command when focus is outside editable or modal UI. Installed at the
    // app root so the shortcut is live on every screen, gated server-side
    // (Rust) on a focus-scope tuple computed in JS from
    // `document.activeElement`. See
    // `frame::profiles::snapshot_drawer::install_snapshot_shortcut_listener`.
    frame::install_snapshot_shortcut_listener(ctx.commands.clone());

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

    use inputforge_core::state::AppState;

    use crate::context::{
        AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles, SettingsSnapshot,
    };
    use crate::frame;
    use crate::toast::{ToastQueue, ToastState};

    /// Test harness: provides every context that `app_root_view` reads
    /// (`AppContext`, `ViewState`, `ToastQueue`, plus the upstream
    /// `RawHandles` for symmetry with the runtime
    /// path), then renders the **same** `app_root_view()` that `app_root`
    /// renders at runtime. This makes the rsx tree at the application
    /// root single-source-of-truth: a regression that changes
    /// `frame::Layout {}` to anything else lives in `app_root_view` and
    /// fails the assertions below.
    ///
    /// Side-effect hooks (`warnings_bridge`, polling task) are intentionally not wired here, they require
    /// a live Dioxus desktop runtime and are exercised by integration
    /// tests / the smoke run, not by this SSR mount test.
    fn app_root_view_with_stub_contexts() -> Element {
        crate::routing::provide_fixture(crate::routing::View::default());
        crate::controllers::provide_fixture(crate::controllers::View::default());
        // --- minimal stubs for context providers ---
        let (cmd_tx, _cmd_rx) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(SettingsSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings,
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

        // Mirror the runtime path's keyboard-capture provider so consumers can
        // read the root capture context under SSR tests.
        crate::patterns::keyboard_capture::use_keyboard_capture_provider();

        // F9: mirror the runtime path's editor-state provider install.
        frame::use_editor_state_provider();

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
