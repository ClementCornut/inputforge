//! Live-capture publication signals and app callback wiring.
use crate::capture::{Bindings, Capture, CaptureFilter, Publication, Request, Target};
use dioxus::prelude::*;
use inputforge_core::types::InputAddress;

pub(crate) const CAPTURE_PROMPT: &str = "Press an input\u{2026}";

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

/// Prepared root state used by other rendered keyboard surfaces.
#[derive(Debug, Clone, Copy)]
pub struct LiveCapture {
    pub active: Signal<bool>,
}

/// Allocate the renderer's single root listening signal.
#[expect(
    clippy::must_use_candidate,
    reason = "installs context and lifecycle even when the returned handle is unused"
)]
pub fn use_live_capture_provider() -> LiveCapture {
    #[cfg(not(test))]
    let bindings = use_context::<Bindings>();
    #[cfg(test)]
    let bindings = try_use_context::<Bindings>().unwrap_or_else(|| {
        let ctx = use_context::<crate::context::AppContext>();
        inputforge_app::live_capture::Controller::new(ctx.state, ctx.commands).bindings()
    });
    use_context_provider(|| bindings);
    let active = use_signal(|| false);
    #[cfg(not(test))]
    let view = use_context::<crate::frame::ViewState>();
    #[cfg(test)]
    let view = try_use_context::<crate::frame::ViewState>().unwrap_or_else(|| {
        let ctx = use_context::<crate::context::AppContext>();
        crate::frame::use_view_state_provider(ctx.meta)
    });
    use_hook(|| {
        bindings.install.call(Publication {
            active,
            mode: view.editing_mode,
            selection: view.selected_mapping,
        });
    });
    use_drop(move || bindings.dispose.call(()));
    let live = LiveCapture { active };
    use_context_provider(|| live);
    live
}

/// Wire one capture surface to app-owned start, cancel and completion callbacks.
pub fn use_capture(
    target: Target,
    filter: CaptureFilter,
    on_capture: impl FnMut(InputAddress) + 'static,
    on_cancel: impl FnMut(()) + 'static,
) -> Capture {
    let bindings = use_context::<Bindings>();
    let mut target_signal = use_signal(|| target.clone());
    if *target_signal.peek() != target {
        target_signal.set(target);
    }
    let listening = use_signal(|| false);
    let request = Request {
        owner: dioxus::core::current_scope_id(),
        filter,
        target: target_signal,
        listening,
        on_capture: use_callback(on_capture),
        on_cancel: use_callback(on_cancel),
    };
    let start = use_callback(move |()| bindings.start.call(request));
    let cancel = use_callback(move |()| bindings.cancel.call(request.owner));
    use_drop(move || bindings.cancel.call(request.owner));
    Capture {
        listening,
        start,
        cancel,
    }
}

#[cfg(test)]
mod tests;
