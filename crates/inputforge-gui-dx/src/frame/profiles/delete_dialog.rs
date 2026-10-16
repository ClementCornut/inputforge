//! Prepared destructive profile confirmation.
use crate::{
    components::{Button, ButtonVariant, DialogBody, DialogFooter, DialogRoot, DialogTitle},
    profiles::{Context, Intent},
};
use dioxus::prelude::*;
#[component]
pub(crate) fn ProfileDeleteDialog() -> Element {
    let c = use_context::<Context>().capture();
    let view = c.view.read().clone();
    let mut open = use_signal(|| false);
    use_effect(move || {
        let wanted = c.view.read().delete.is_some();
        if *open.peek() != wanted {
            open.set(wanted);
        }
    });
    let name = view.delete.map(|t| t.name).unwrap_or_default();
    rsx! {
        DialogRoot {
            open,
            onclose: move |()| {
                let _ = c.send(Intent::CancelDelete);
            },
            DialogTitle {
                "Delete profile"
            }
            DialogBody {
                "Delete '{name}'? This removes the .toml file from disk."
                if view.deleting_active {
                    div {
                        class: "profiles-panel__delete-confirm-active",
                        "This is the active profile. Deleting it will leave no profile loaded."
                    }
                }
                if let Some(error) = view.error.or(view.name_error) {
                    div {
                        role: "alert",
                        "{error}"
                    }
                }
            }
            DialogFooter {
                Button {
                    variant: ButtonVariant::Ghost,
                    onmounted: move |e| c.bindings.focus.call((c.owner, c.workflow, e)),
                    onclick: move |_| {
                        let _ = c.send(Intent::CancelDelete);
                    },
                    "Cancel"
                }
                Button {
                    variant: ButtonVariant::Danger,
                    onclick: move |_| {
                        let _ = c.send(Intent::Delete);
                    },
                    "Delete"
                }
            }
        }
    }
}
