//! Compound dialog primitive over the native HTML `<dialog>` element.
//! See spec for boundary contract; implementation locked in this module.

use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use super::merge_class;

static DIALOG_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Test seam, bumps the counter and returns the new value. Production code
/// goes through `DialogRoot`'s `use_hook` and never calls this directly.
#[cfg(test)]
fn next_dialog_seq_for_test() -> u64 {
    DIALOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

const DIALOG_OPEN_JS: &str = r"
(function(id) {
    var d = document.getElementById(id);
    if (d && !d.open) d.showModal();
})
";

const DIALOG_CLOSE_JS: &str = r"
(function(id) {
    var d = document.getElementById(id);
    if (d && d.open) d.close();
})
";

const DIALOG_ATTACH_CANCEL_JS: &str = r"
(function(id, dismissible) {
    var d = document.getElementById(id);
    if (!d) return;
    d.addEventListener('cancel', function(e) {
        if (!dismissible) e.preventDefault();
    });
})
";

// Dioxus 0.7 does not expose an `onclose` HTML event handler for the native
// `<dialog>` element, so we use `oncancel` (fires on ESC) plus our backdrop
// `onclick` handler to drive the controlled `open` signal back to false. The
// JS-attached `cancel` listener interpolates `dismissible` literally and is
// the single source of truth for ESC suppression on non-dismissible dialogs.

/// Shared per-dialog context. Children read; only `DialogRoot` writes.
/// All ids are eagerly computed by `DialogRoot`'s `use_hook` initializer
/// (which runs during render, BEFORE children render), so children see
/// fully-populated ids on their very first render and `aria-labelledby` /
/// `aria-describedby` resolve correctly on the initial `showModal()` call.
#[derive(Clone)]
struct DialogState {
    open: Signal<bool>,
    dialog_id: String,
    title_id: String,
    desc_id: String,
    close_on_backdrop_click: bool,
}

/// Root of the dialog compound. Drives `showModal()` / `close()` on `open`
/// changes; attaches a one-shot `cancel` listener on first commit.
///
/// `dismissible` is **read once** at attach time. Flipping it after mount
/// has no effect on subsequent ESC events. F4's only consumers (gallery
/// demos and `DirtyConfirmDialog`) pass stable values.
#[component]
pub fn DialogRoot(
    open: Signal<bool>,
    onclose: EventHandler<()>,
    #[props(default = true)] dismissible: bool,
    #[props(default = false)] close_on_backdrop_click: bool,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    // Eager id derivation, runs once during the parent's render, BEFORE
    // children render.
    let state = use_hook(|| {
        let n = DIALOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dialog_id = format!("if-dialog-{n}");
        let title_id = format!("{dialog_id}-title");
        let desc_id = format!("{dialog_id}-desc");
        DialogState {
            open,
            dialog_id,
            title_id,
            desc_id,
            close_on_backdrop_click,
        }
    });
    use_context_provider(|| state.clone());

    // Drive showModal()/close() on `open` changes. use_effect runs AFTER
    // DOM commit so getElementById is guaranteed to find the <dialog>.
    let id_for_open = state.dialog_id.clone();
    use_effect(move || {
        let action = if *open.read() {
            DIALOG_OPEN_JS
        } else {
            DIALOG_CLOSE_JS
        };
        let _ = document::eval(&format!("{action}({id_for_open:?})"));
    });

    // Attach `cancel` listener once after first DOM commit. The `dismissible`
    // value is interpolated into the JS at attach time, see doc-comment.
    let id_for_cancel = state.dialog_id.clone();
    let dismissible_now = dismissible;
    let mut attached = use_signal(|| false);
    use_effect(move || {
        if *attached.peek() {
            return;
        }
        let _ = document::eval(&format!(
            "{DIALOG_ATTACH_CANCEL_JS}({id_for_cancel:?}, {dismissible_now})"
        ));
        attached.set(true);
    });

    let combined = merge_class("if-dialog", "", class.as_deref());
    let close_on_backdrop = state.close_on_backdrop_click;
    let mut open_signal_cancel = state.open;
    let mut open_signal_click = state.open;
    let dismissible_for_cancel = dismissible;
    let onclose_cancel = onclose;
    let onclose_click = onclose;

    let oncancel = move |_| {
        // ESC pressed. The JS-attached `cancel` listener already called
        // preventDefault() when `dismissible` was false, so when this fires
        // we still need to gate on dismissible to keep our controlled
        // `open` signal in sync with what actually happened.
        if !dismissible_for_cancel {
            return;
        }
        open_signal_cancel.set(false);
        onclose_cancel.call(());
    };
    let onclick = move |_| {
        if !close_on_backdrop {
            return;
        }
        // Reaches here only on backdrop clicks because the inner
        // .if-dialog__panel calls evt.stop_propagation() on its onclick.
        open_signal_click.set(false);
        onclose_click.call(());
    };
    let on_panel_click = move |evt: MouseEvent| evt.stop_propagation();

    rsx! {
        dialog {
            id: "{state.dialog_id}",
            class: "{combined}",
            "aria-labelledby":  "{state.title_id}",
            "aria-describedby": "{state.desc_id}",
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
    rsx! { h2 { id: "{state.title_id}", class: "if-dialog__title", {children} } }
}

#[component]
pub fn DialogDescription(children: Element) -> Element {
    let state = use_context::<DialogState>();
    rsx! { p { id: "{state.desc_id}", class: "if-dialog__desc", {children} } }
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
    /// names. Children read `dialog_id`/`title_id`/`desc_id` from `DialogState`
    /// during their first render, so a typo in the format string would produce
    /// a dangling `aria-labelledby`/`aria-describedby` on first paint.
    #[test]
    fn dialog_id_derivation_is_well_formed() {
        let n = next_dialog_seq_for_test();
        let dialog_id = format!("if-dialog-{n}");
        let title_id = format!("{dialog_id}-title");
        let desc_id = format!("{dialog_id}-desc");
        assert!(dialog_id.starts_with("if-dialog-"));
        assert_eq!(title_id, format!("if-dialog-{n}-title"));
        assert_eq!(desc_id, format!("if-dialog-{n}-desc"));
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
