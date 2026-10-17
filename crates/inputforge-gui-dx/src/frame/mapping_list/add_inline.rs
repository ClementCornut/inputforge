//! Add pad rendering; app callbacks prepare every workflow transition.
use super::add_inline_rendering as rendering;
use crate::{
    capture::{CaptureFilter, Target},
    context::AppContext,
    frame::view_state::ViewState,
    mapping_edits::Bindings,
    mapping_rail::{AddIntent, AddView, Scope, Target as RailTarget},
    patterns::live_capture::{LiveCapture, use_capture},
};
use dioxus::prelude::*;

pub(super) fn submit(
    state: Signal<AddView>,
    name: Signal<String>,
    bindings: &Bindings,
    intent: AddIntent,
) -> bool {
    bindings.ui.add_state.call((state, name, intent))
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
    let name = use_signal(String::new);
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
    let publication = crate::mapping_edits::ui::Add {
        state,
        name,
        capture,
    };
    let ctx_start = ctx.clone();
    let begin = use_callback(move |source| {
        bindings.ui.add.call((
            publication,
            crate::mapping_edits::ui::AddEvent::Begin {
                scope: Scope {
                    profile: ctx_start.meta.peek().profile_id.clone(),
                    mode: view.editing_mode.peek().clone(),
                },
                source,
            },
        ));
    });
    let ctx_observe = ctx.clone();
    let observe = move || {
        bindings.ui.add.call((
            publication,
            crate::mapping_edits::ui::AddEvent::Observe {
                scope: Scope {
                    profile: ctx_observe.meta.peek().profile_id.clone(),
                    mode: view.editing_mode.peek().clone(),
                },
                pending: pending_duplicate,
                expanded: force_expanded,
            },
        ));
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
        bindings
            .ui
            .add
            .call((publication, crate::mapping_edits::ui::AddEvent::Reconcile));
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
