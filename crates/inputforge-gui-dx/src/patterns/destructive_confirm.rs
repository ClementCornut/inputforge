//! Presentational destructive-confirmation dialog.
//!
//! Cancel + Danger in fixed document order so `showModal()`'s default-focus
//! rule lands on Cancel (the safe default, destructive-confirm a11y guidance).
//! ESC routes to `oncancel`. `close_on_backdrop_click` is hard-coded to
//! `false`, destructive dialogs should not close on a stray click outside
//! the panel.
//!
//! F4's destructive-shape primitive in concrete form, parallel to
//! [`DirtyConfirmDialog`](super::dirty_confirm::DirtyConfirmDialog).
//! Consumers: F15 prune-confirm; future destructive flows (profile delete,
//! snapshot delete, mapping bulk-delete) MAY adopt it.

// Rust guideline compliant 2026-05-09

use dioxus::prelude::*;

use crate::components::{
    Button, ButtonVariant, DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle,
};

#[expect(
    missing_debug_implementations,
    reason = "dioxus Signal<T>/EventHandler<T> do not implement Debug"
)]
#[derive(Clone, PartialEq, Props)]
pub struct DestructiveConfirmDialogProps {
    /// Controlled open state. The component flips this to `false` on every
    /// resolution path (Cancel/Confirm) and fires the matching callback.
    pub open: Signal<bool>,

    /// Title, defaults to "Confirm".
    #[props(default)]
    pub title: Option<String>,

    /// Rich body for emphasis. Caller passes a `rsx!`-built element, so the
    /// description can carry counts, profile names, formatted text without
    /// passing pre-rendered strings.
    ///
    /// Avoid wrapping the content in a block element such as `<p>`;
    /// `DialogDescription` already supplies the outer `<p>`. Inline content,
    /// `<span>`, or text-only nodes are correct.
    pub description: Element,

    /// Cancel button label, defaults to "Cancel".
    #[props(default)]
    pub cancel_label: Option<String>,

    /// Confirm-action verb. No default; caller must supply (e.g. "Reduce",
    /// "Delete") so the affirmative button names the action.
    pub confirm_label: String,

    pub oncancel: EventHandler<()>,
    pub onconfirm: EventHandler<()>,

    #[props(default)]
    pub class: Option<String>,
}

#[component]
pub fn DestructiveConfirmDialog(props: DestructiveConfirmDialogProps) -> Element {
    let title = props.title.as_deref().unwrap_or("Confirm");
    let cancel_label = props.cancel_label.as_deref().unwrap_or("Cancel");
    let confirm_label = props.confirm_label;

    let mut open = props.open;
    let cancel = props.oncancel;
    let confirm = props.onconfirm;

    let onclose = move |()| {
        open.set(false);
        cancel.call(());
    };
    let on_cancel_click = move |_| {
        open.set(false);
        cancel.call(());
    };
    let on_confirm_click = move |_| {
        open.set(false);
        confirm.call(());
    };

    rsx! {
        DialogRoot {
            open: open,
            onclose: onclose,
            dismissible: true,
            close_on_backdrop_click: false,
            class: props.class,

            DialogTitle { "{title}" }
            DialogDescription { {props.description} }
            DialogBody {}
            DialogFooter {
                Button {
                    variant: ButtonVariant::Secondary,
                    onclick: on_cancel_click,
                    "{cancel_label}"
                }
                Button {
                    variant: ButtonVariant::Danger,
                    onclick: on_confirm_click,
                    "{confirm_label}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    #[allow(non_snake_case, reason = "Dioxus components are PascalCase")]
    fn Harness() -> Element {
        let open = use_signal(|| true);
        rsx! {
            DestructiveConfirmDialog {
                open: open,
                title: Some("Test title".to_owned()),
                description: rsx! { "Test body" },
                confirm_label: "Reduce".to_owned(),
                oncancel: move |()| {},
                onconfirm: move |()| {},
            }
        }
    }

    #[test]
    fn renders_title_description_and_action_labels() {
        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("Test title"), "title missing: {html}");
        assert!(html.contains("Test body"), "description missing: {html}");
        assert!(html.contains("Reduce"), "confirm label missing: {html}");
        assert!(
            html.contains("Cancel"),
            "default cancel label missing: {html}"
        );
    }

    #[test]
    fn cancel_button_precedes_confirm_in_dom_order() {
        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        let cancel_pos = html.find("Cancel").expect("Cancel label missing");
        let confirm_pos = html.find("Reduce").expect("Reduce label missing");
        assert!(
            cancel_pos < confirm_pos,
            "Cancel must precede Confirm in DOM order to land default focus on the safe action"
        );
    }
}
