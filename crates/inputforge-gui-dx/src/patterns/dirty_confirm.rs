//! Presentational dirty-state confirmation dialog. Cancel/Discard/Save in
//! fixed document order so `showModal()`'s default-focus rule lands on
//! Cancel (the safe default, destructive-confirmation a11y guidance).
//!
//! ESC routes to `oncancel` (matches Cancel button). `close_on_backdrop_click`
//! is hard-coded to `false`, destructive dialogs should not close on a stray
//! click outside the panel.

use dioxus::prelude::*;

use crate::components::{
    Button, ButtonVariant, DialogBody, DialogDescription, DialogFooter, DialogRoot, DialogTitle,
};

// `Signal<bool>` and `EventHandler<T>` from Dioxus do not implement `Debug`,
// so we cannot derive it on this Props struct. The workspace lints enable
// `missing_debug_implementations = "warn"` for non-prop public types; opt
// out here at the Props level. Other components in this crate sidestep this
// by using inline `#[component]` parameters, but the pattern composer here
// keeps an explicit Props struct to mirror Dioxus 0.7 docs and to give
// callers a typed handle for builder-style construction.
#[allow(
    missing_debug_implementations,
    reason = "dioxus Signal<T>/EventHandler<T> do not implement Debug"
)]
#[derive(Clone, PartialEq, Props)]
pub struct DirtyConfirmDialogProps {
    /// Controlled open state. Closes only after the chosen callback accepts the resolution.
    pub open: Signal<bool>,

    /// Title, defaults to "Unsaved Changes".
    #[props(default)]
    pub title: Option<String>,
    /// Description, defaults to
    /// "You have unsaved changes. What would you like to do?".
    #[props(default)]
    pub message: Option<String>,
    /// Save button label, defaults to "Save". Future consumers may pass
    /// "Save & Switch", "Save & Close", etc.
    #[props(default)]
    pub save_label: Option<String>,

    pub oncancel: Callback<(), bool>,
    pub ondiscard: Callback<(), bool>,
    pub onsave: Callback<(), bool>,

    #[props(default)]
    pub class: Option<String>,
}

#[component]
pub fn DirtyConfirmDialog(props: DirtyConfirmDialogProps) -> Element {
    let title = props.title.as_deref().unwrap_or("Unsaved Changes");
    let message = props
        .message
        .as_deref()
        .unwrap_or("You have unsaved changes. What would you like to do?");
    let save_label = props.save_label.as_deref().unwrap_or("Save");

    let open = props.open;
    let resolve = crate::components::behavior().overlays.resolve;
    let cancel = props.oncancel;
    let discard = props.ondiscard;
    let save = props.onsave;

    let onclose = cancel;
    let on_cancel_click = move |_| {
        resolve.call((open, cancel));
    };
    let on_discard_click = move |_| {
        resolve.call((open, discard));
    };
    let on_save_click = move |_| {
        resolve.call((open, save));
    };

    rsx! {
        DialogRoot {
            open: open,
            // ESC routes to Cancel, matches default-focus and safe-default
            // semantics. The dialog's own onclose handler fires after the
            // browser closes the <dialog>.
            onclose: onclose,
            dismissible: true,
            close_on_backdrop_click: false,
            class: props.class,

            DialogTitle { "{title}" }
            DialogDescription { "{message}" }
            DialogBody {} // empty, Description carries the body content
            DialogFooter {
                // Cancel first → receives showModal()'s default focus.
                Button {
                    variant: ButtonVariant::Secondary,
                    onclick: on_cancel_click,
                    "Cancel"
                }
                Button {
                    variant: ButtonVariant::Danger,
                    onclick: on_discard_click,
                    "Discard"
                }
                Button {
                    variant: ButtonVariant::Primary,
                    onclick: on_save_click,
                    "{save_label}"
                }
            }
        }
    }
}
