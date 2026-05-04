//! Live-capture primitive, see Task 7's `machine.rs` for the pure
//! state-transition logic.
//!
//! Single-instance pattern: provided once via context in `app_root`.
//! Each consumer reads via `use_context::<LiveCapture>()`. Starting a
//! new capture cancels any in-flight one, there is exactly one
//! capture at a time across the entire GUI.

mod machine;
#[cfg(test)]
mod tests;

use std::time::Instant;

use dioxus::prelude::*;

use inputforge_core::types::InputAddress;

use crate::context::AppContext;

pub(crate) use machine::{CoreState, LiveCaptureCore};

/// Filter governing which input kinds the primitive accepts. F9-F12
/// will use `AxesOnly` / `ButtonsOnly` to discriminate range-record vs.
/// button-bind flows. F8's `+ Add mapping` always uses `Any`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(
    dead_code,
    reason = "AxesOnly / ButtonsOnly are exercised by tests; lib consumers land in F9-F12"
)]
pub(crate) enum CaptureFilter {
    #[default]
    Any,
    AxesOnly,
    ButtonsOnly,
}

pub(crate) const CAPTURE_PROMPT: &str = "Press an input\u{2026}";

pub(crate) fn is_current_capture_session(owned_session: Option<u64>, current_session: u64) -> bool {
    owned_session == Some(current_session)
}

pub(crate) fn rebind_composite_class(input: &InputAddress, is_listening: bool) -> &'static str {
    match (is_listening, input.is_unbound()) {
        (true, true) => {
            "if-rebind-composite if-rebind-composite--listening if-rebind-composite--unbound"
        }
        (true, false) => "if-rebind-composite if-rebind-composite--listening",
        (false, true) => "if-rebind-composite if-rebind-composite--unbound",
        (false, false) => "if-rebind-composite",
    }
}

/// Public handle exposed via context. `Copy` (every field is `Signal`
/// or `Callback`, both `Copy` in Dioxus 0.7+) so consumers do
/// `use_context::<LiveCapture>()` without an explicit clone.
#[derive(Clone, Copy)]
#[allow(
    dead_code,
    reason = "captured/cancel exposed for F8 row consumers and Esc handling"
)]
pub(crate) struct LiveCapture {
    pub active: Signal<bool>,
    pub session: Signal<u64>,
    pub captured: Signal<Option<InputAddress>>,
    pub start: Callback<CaptureFilter>,
    pub cancel: Callback<()>,
}

/// Allocate the signals and callbacks, install the polling effect and
/// the document-level Esc-priority listener, AND register the resulting
/// `LiveCapture` with the Dioxus context system. Call exactly once from
/// `app_root`, the provider self-installs.
#[allow(
    clippy::too_many_lines,
    reason = "single-instance hook braids 5 signals + 2 callbacks + polling effect + Esc \
              listener; splitting forces the closures to capture indirected handles instead \
              of values, which obscures the single-source-of-truth wiring"
)]
pub(crate) fn use_live_capture_provider() -> LiveCapture {
    let active: Signal<bool> = use_signal(|| false);
    let session: Signal<u64> = use_signal(|| 0);
    let captured: Signal<Option<InputAddress>> = use_signal(|| None);
    let core_state: Signal<CoreState> = use_signal(CoreState::default);

    let armed_listener_mounted: Signal<bool> = use_signal(|| false);
    let shutdown_signal: Signal<bool> = use_signal(|| false);

    let start = use_callback(move |filter: CaptureFilter| {
        let mut s = core_state;
        s.set(CoreState {
            baseline: None,
            pending: None,
            filter,
        });
        let mut cap = captured;
        cap.set(None);
        let mut session_signal = session;
        let next_session = session_signal.peek().wrapping_add(1);
        session_signal.set(next_session);
        let mut a = active;
        a.set(true);
        tracing::debug!(target: "f8::live_capture", ?filter, "capture armed");
    });

    let cancel = use_callback(move |()| {
        let mut s = core_state;
        let prev_filter = s.read().filter;
        s.set(CoreState {
            baseline: None,
            pending: None,
            filter: prev_filter,
        });
        let mut a = active;
        a.set(false);
        let mut cap = captured;
        cap.set(None);
        let mut sd = shutdown_signal;
        sd.set(true);
        tracing::debug!(target: "f8::live_capture", "capture cancelled");
    });

    let ctx = use_context::<AppContext>();

    // Polling effect, subscribes to ctx.live as wake gate.
    use_effect(move || {
        let _live = ctx.live.read();

        if !*active.read() {
            return;
        }

        let snapshot = {
            let Some(guard) = ctx.state.try_read() else {
                return;
            };
            let snap = guard.input_cache.clone_compact();
            drop(guard);
            snap
        };

        let prev = core_state.peek().clone();
        let (next, fired) = LiveCaptureCore::step(prev, &snapshot, Instant::now());
        let mut s = core_state;
        if *s.peek() != next {
            s.set(next);
        }
        if let Some(addr) = fired {
            let mut cap = captured;
            cap.set(Some(addr.clone()));
            let mut a = active;
            a.set(false);
            let mut sd = shutdown_signal;
            sd.set(true);
            tracing::debug!(target: "f8::live_capture", ?addr, "capture fired");
        }
    });

    // Document-level Esc listener, mirrors the document::eval / dioxus.send
    // pattern used in mode_tabs::context_menu (see frame/top_bar/mode_tabs/mod.rs).
    let cancel_for_esc = cancel;
    use_effect(move || {
        if !*active.read() {
            return;
        }
        let mut mounted = armed_listener_mounted;
        if *mounted.peek() {
            return;
        }
        mounted.set(true);

        let mut sd = shutdown_signal;
        sd.set(false);

        spawn(async move {
            let mut handle = document::eval(
                "const h = (ev) => {\n\
                   if (ev.key === 'Escape') {\n\
                     ev.stopPropagation();\n\
                     dioxus.send('esc');\n\
                   }\n\
                 };\n\
                 window.addEventListener('keydown', h, true);\n\
                 (async () => {\n\
                   while (true) {\n\
                     const msg = await dioxus.recv();\n\
                     if (msg === '__shutdown__') {\n\
                       window.removeEventListener('keydown', h, true);\n\
                       dioxus.send('shutdown_ack');\n\
                       return;\n\
                     }\n\
                   }\n\
                 })();\n\
                 ",
            );

            loop {
                if *shutdown_signal.peek() {
                    let _ = handle.send("__shutdown__".to_owned());
                    let _ = handle.recv::<String>().await;
                    break;
                }
                match handle.recv::<String>().await {
                    Ok(s) if s == "esc" => {
                        cancel_for_esc.call(());
                    }
                    _ => break,
                }
            }
            let mut mounted = armed_listener_mounted;
            mounted.set(false);
        });
    });

    let live = LiveCapture {
        active,
        session,
        captured,
        start,
        cancel,
    };
    use_context_provider(|| live);
    live
}
