//! Compound dialog primitive over the native HTML `<dialog>` element.
//! See spec for boundary contract; implementation locked in this module.

use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use crate::components::merge_class;

static DIALOG_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Test seam, bumps the counter and returns the new value. Production code
/// goes through `DialogRoot`'s `use_hook` and never calls this directly.
#[cfg(test)]
fn next_dialog_seq_for_test() -> u64 {
    DIALOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Shared per-dialog context. Children read; only `DialogRoot` writes.
/// All ids are eagerly computed by `DialogRoot`'s `use_hook` initializer
/// (which runs during render, BEFORE children render), so children see
/// fully-populated ids on their very first render and `aria-labelledby` /
/// `aria-describedby` resolve correctly on the initial `showModal()` call.
#[derive(Clone)]
struct DialogState {
    dialog: String,
    title: String,
    description: String,
}

/// Render the native dialog and forward visibility/dismissal to app bindings.
///
/// Dismissal uses the current props and waits for the app callback to accept.
#[component]
pub fn DialogRoot(
    open: Signal<bool>,
    onclose: Callback<(), bool>,
    #[props(default = true)] dismissible: bool,
    #[props(default = false)] close_on_backdrop_click: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    // Eager id derivation, runs once during the parent's render, BEFORE
    // children render.
    let state = use_hook(|| {
        let n = DIALOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dialog = format!("if-dialog-{n}");
        let title = format!("{dialog}-title");
        let description = format!("{dialog}-desc");
        DialogState {
            dialog,
            title,
            description,
        }
    });
    use_context_provider(|| state.clone());

    let behavior = (crate::components::behavior().overlays.dialog)(crate::overlays::DialogProps {
        open,
        id: state.dialog.clone(),
        dismissible,
        backdrop: close_on_backdrop_click,
        onclose,
    });
    let combined = merge_class("if-dialog", "", class.as_deref());
    let oncancel = behavior.cancel;
    let onclick = behavior.backdrop;
    let on_panel_click = behavior.panel;
    rsx! {
        dialog {
            id: "{state.dialog}",
            class: "{combined}",
            "aria-labelledby":  "{state.title}",
            "aria-describedby": "{state.description}",
            oncancel,
            onclick,
            div {
                class: "if-dialog__panel",
                onclick: on_panel_click,
                {children}
            }
        }
    }
}

#[component]
pub fn DialogTitle(children: Element) -> Element {
    let state = use_context::<DialogState>();
    rsx! { h2 { id: "{state.title}", class: "if-dialog__title", {children} } }
}

#[component]
pub fn DialogDescription(children: Element) -> Element {
    let state = use_context::<DialogState>();
    rsx! { p { id: "{state.description}", class: "if-dialog__desc", {children} } }
}

#[component]
pub fn DialogBody(children: Element) -> Element {
    rsx! { div { class: "if-dialog__body", {children} } }
}

#[component]
pub fn DialogFooter(children: Element) -> Element {
    rsx! { div { class: "if-dialog__footer", {children} } }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: id derivation must produce stable, monotonic, well-formed
    /// names. Children read `dialog`/`title`/`description` from `DialogState`
    /// during their first render, so a typo in the format string would produce
    /// a dangling `aria-labelledby`/`aria-describedby` on first paint.
    #[test]
    fn dialog_derivation_is_well_formed() {
        let n = next_dialog_seq_for_test();
        let dialog = format!("if-dialog-{n}");
        let title = format!("{dialog}-title");
        let description = format!("{dialog}-desc");
        assert!(dialog.starts_with("if-dialog-"));
        assert_eq!(title, format!("if-dialog-{n}-title"));
        assert_eq!(description, format!("if-dialog-{n}-desc"));
    }

    #[test]
    fn dialog_seq_is_monotonic() {
        let a = next_dialog_seq_for_test();
        let b = next_dialog_seq_for_test();
        let c = next_dialog_seq_for_test();
        assert!(b > a);
        assert!(c > b);
    }
}
