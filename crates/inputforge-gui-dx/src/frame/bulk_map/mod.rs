//! Batch mapping rendering; wizard policy and delivery belong to the app.
mod empty_state;
#[cfg(test)]
mod events_tests;
mod labels;
mod metadata;
mod row_readout;
mod rows;
#[cfg(test)]
mod tests;
use crate::{
    batch_mapping::{Intent, Publication, Request, View},
    components::{Button, ButtonVariant},
    frame::view_state::{MainSurface, ViewState},
    toast::{ToastLevel, ToastQueue},
};
use dioxus::prelude::*;
use empty_state::NoVjoyEmptyState;
#[component]
pub(crate) fn BulkMapPanel() -> Element {
    let navigation = use_context::<ViewState>();
    let mappings = use_context::<crate::mapping_edits::Bindings>();
    let callbacks = mappings.batch;
    let model = use_signal(View::default);
    let toast = use_context::<ToastQueue>();
    let applied = Callback::new(move |count| {
        toast.push(ToastLevel::Success, format!("Requested {count} mappings"));
        mappings
            .navigation
            .dispatch
            .call(crate::navigation::Intent::Surface(MainSurface::Mappings));
    });
    let owner = use_hook(|| {
        callbacks.install.call(Publication {
            view: model,
            mode: navigation.editing_mode,
            applied,
        })
    });
    use_drop(move || callbacks.dispose.call(owner));
    use_effect(move || {
        let _ = navigation.editing_mode.read();
        mappings.reconcile.call(());
    });
    let view = model.read().clone();
    let generation = view.generation;
    let on_intent = Callback::new(move |intent| {
        callbacks.dispatch.call(Request {
            owner,
            generation,
            intent,
        });
    });
    let count = view.counts.create + view.counts.replace;
    let ready = view.profile.is_some() && !view.targets.is_empty();
    rsx! {
        section { class: "if-bulk-map", "aria-label": "Batch map device inputs",
            if let Some(error) = &view.error { div { role: "alert", "{error}" } }
            if view.profile.is_none() {
                NoVjoyEmptyState { title: "No profile loaded".to_owned(), caption: "Load or create a profile, then reopen.".to_owned() }
            } else if view.targets.is_empty() { NoVjoyEmptyState {} }
            if ready {
                metadata::Metadata { model: view.clone(), on_intent }
                div { role: "grid", class: "if-bulk-map__table",
                    for group in &view.groups {
                        if view.rows.iter().any(|r| r.kind == group.kind) {
                            rows::Group { model: group.clone(),
                                rows: view.rows.iter().filter(|r| r.kind == group.kind).cloned().collect(),
                                conflicting: view.rows.iter().zip(&view.conflicting).filter(|(r,_)| r.kind == group.kind).map(|(_,c)| *c).collect(), on_intent }
                        }
                    }
                    if view.rows.is_empty() { div { role: "row", class: "if-bulk-map__table-empty", "No inputs available for this source device." } }
                }
            }
            footer { class: "if-bulk-map__footer",
                if ready {
                    div { class: "if-bulk-map__footer-info",
                        div { class: "if-bulk-map__summary",
                            span { class: "if-bulk-map__summary-create", "+{view.counts.create} create" }
                            if view.all_modes { span { class: "if-bulk-map__summary-modes", " across {view.modes.len()} modes" } }
                            span { class: "if-bulk-map__summary-sep", " / " }
                            span { class: "if-bulk-map__summary-replace", "{view.counts.replace} replace" }
                            span { class: "if-bulk-map__summary-sep", " / " }
                            span { class: "if-bulk-map__summary-skip", "{view.counts.skip} skip" }
                            span { class: "if-bulk-map__summary-sep", " / " }
                            span { class: "if-bulk-map__summary-excluded", "{view.counts.excluded} excluded" }
                        }
                        span { class: "if-bulk-map__caption", "Recovery snapshot requested before changes." }
                    }
                }
                div { class: "if-bulk-map__footer-actions",
                    if view.dirty { Button { variant: ButtonVariant::Secondary, onclick: move |_| on_intent.call(Intent::Reset), "Reset" } }
                    Button { disabled: !view.can_apply, onclick: move |_| on_intent.call(Intent::Apply),
                        if ready { "Apply {count} mappings" } else { "Apply" }
                    }
                }
            }
        }
    }
}
