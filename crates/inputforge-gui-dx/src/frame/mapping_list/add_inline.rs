//! Add pad rendering; app callbacks prepare every workflow transition.
use super::add_inline_rendering as rendering;
use crate::{
    capture::{CaptureFilter, Target},
    context::AppContext,
    frame::view_state::ViewState,
    mapping_edits::Bindings,
    mapping_rail::{AddIntent, AddRequest, AddView, Phase, Scope, Target as RailTarget},
    patterns::live_capture::{LiveCapture, use_capture},
};
use dioxus::prelude::*;

pub(super) fn submit(
    mut state: Signal<AddView>,
    mut name: Signal<String>,
    bindings: &Bindings,
    intent: AddIntent,
) -> bool {
    let next = bindings.add.call(AddRequest {
        view: state.peek().clone(),
        intent,
    });
    let capturing = next.phase == Phase::Capturing;
    if next.phase == Phase::Resting {
        name.set(String::new());
    }
    if *state.peek() != next {
        state.set(next);
    }
    capturing
}
pub(super) fn commit(state: Signal<AddView>, name: Signal<String>, bindings: &Bindings) {
    let raw = name.peek().clone();
    submit(state, name, bindings, AddIntent::Commit(raw));
}
#[component]
pub(crate) fn AddInline(
    force_expanded: Signal<bool>,
    pending_duplicate: Signal<Option<RailTarget>>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let cap = use_context::<LiveCapture>();
    let bindings = use_context::<Bindings>();
    let state = use_signal(AddView::default);
    let mut name = use_signal(String::new);
    let capture = use_capture(
        Target {
            profile: ctx.meta.read().profile_id.clone(),
            mode: view.editing_mode.read().clone(),
            mapping: None,
            name: None,
            actions: None,
            field: "add".into(),
        },
        CaptureFilter::Any,
        move |addr| {
            submit(state, name, &bindings, AddIntent::Captured(addr));
        },
        move |()| {
            submit(state, name, &bindings, AddIntent::CaptureCancelled);
        },
    );
    let ctx_start = ctx.clone();
    let begin = use_callback(move |source: Option<RailTarget>| {
        let scope = Scope {
            profile: ctx_start.meta.peek().profile_id.clone(),
            mode: view.editing_mode.peek().clone(),
        };
        if let Some(source) = &source {
            name.set(format!(
                "{} (copy)",
                source.before.name.as_deref().unwrap_or("(unnamed)")
            ));
        }
        if submit(state, name, &bindings, AddIntent::Start { scope, source }) {
            capture.start.call(());
        }
    });
    let observe = move || {
        let mut pending = pending_duplicate;
        let source = pending.peek().clone();
        if source.is_some() {
            pending.set(None);
            begin.call(source);
        }
        let mut force = force_expanded;
        if *force.peek() {
            force.set(false);
            begin.call(None);
        }
    };
    use_hook(observe);
    use_effect(move || {
        let _ = pending_duplicate.read();
        let _ = force_expanded.read();
        observe();
    });
    use_effect(move || {
        let _ = ctx.config.read();
        let _ = ctx.meta.read();
        let _ = view.editing_mode.read();
        let _ = state.read();
        submit(state, name, &bindings, AddIntent::Reconcile);
        if state.peek().phase == Phase::Resting && *capture.listening.peek() {
            capture.cancel.call(());
        }
    });
    let dismiss = use_callback(move |()| {
        submit(state, name, &bindings, AddIntent::Cancel);
    });
    (crate::components::behavior().overlays.idle_escape)(cap.active.into(), dismiss);
    let error = state.read().error.clone();
    let phase = state.read().phase.clone();
    let body = rendering::body(
        phase,
        &ctx.config.read(),
        begin,
        state,
        name,
        &bindings,
        capture,
    );
    rsx! {
        if let Some(error) = error { div { role: "alert", "{error}" } }
        {body}
    }
}
