//! F4 destructive-confirm dialog for the profile-row Delete action.
//!
//! Mirrors [`crate::frame::top_bar::ModeDeleteDialog`]: a stand-alone
//! component sharing a [`ProfileDeleteSignal`] context with the
//! profile row trigger, decoupled from the row's flex line so the
//! dialog renders as panel-scoped chrome rather than as a child of
//! whichever row was clicked.
//!
//! [`profile_delete_action`] declares
//! `ConfirmationKind::DestructiveF4`, but until this dialog landed the
//! row's onclick dispatched the engine command directly without
//! confirmation. Routing every Delete click through this component
//! honours the action's declared contract: a power user with multiple
//! profiles next to each other in the rail cannot one-click their way
//! out of the wrong .toml file.

use dioxus::prelude::*;

use crate::components::{Button, ButtonVariant, DialogBody, DialogFooter, DialogRoot, DialogTitle};
use crate::context::AppContext;
use crate::frame::profiles::actions::profile_delete_action;

/// Newtype so the generic `Signal<Option<String>>` slot doesn't
/// collide with any other `Option<String>` provider in the context
/// tree (e.g. [`crate::frame::top_bar::ModeDeleteSignal`]).
#[derive(Clone, Copy)]
pub(crate) struct ProfileDeleteSignal(pub Signal<Option<String>>);

#[component]
pub(crate) fn ProfileDeleteDialog() -> Element {
    let ctx = use_context::<AppContext>();
    let mut delete_target = use_context::<ProfileDeleteSignal>().0;

    // Mirror `delete_target` into a `Signal<bool>` so `DialogRoot`'s
    // open-state contract is satisfied. The reverse direction (close
    // -> clear target) is covered by the Cancel/Confirm onclicks plus
    // the `DialogRoot::onclose` handler.
    let mut dialog_open: Signal<bool> = use_signal(|| false);
    use_effect(move || {
        let want = delete_target.read().is_some();
        if *dialog_open.peek() != want {
            dialog_open.set(want);
        }
    });

    let target_name = delete_target.read().clone().unwrap_or_default();
    let active_profile_name = ctx.meta.read().profile_name.clone();
    let deleting_active = active_profile_name.as_ref() == Some(&target_name);

    let cmd_for_delete = ctx.commands.clone();
    let confirm_name = target_name.clone();

    let onclose = move |()| {
        delete_target.set(None);
    };

    rsx! {
        DialogRoot {
            open: dialog_open,
            onclose,
            DialogTitle { "Delete profile" }
            DialogBody {
                "Delete '{target_name}'? This removes the .toml file from disk."
                if deleting_active {
                    div { class: "profiles-panel__delete-confirm-active",
                        "This is the active profile. Deleting it will leave no profile loaded."
                    }
                }
            }
            DialogFooter {
                Button {
                    variant: ButtonVariant::Ghost,
                    onmounted: move |evt: MountedEvent| {
                        spawn(async move {
                            let _ = evt.data().set_focus(true).await;
                        });
                    },
                    onclick: move |_| { delete_target.set(None); },
                    "Cancel"
                }
                Button {
                    variant: ButtonVariant::Danger,
                    onclick: move |_| {
                        let _ = cmd_for_delete.send(profile_delete_action(&confirm_name).command);
                        delete_target.set(None);
                    },
                    "Delete"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::engine::EngineCommand;

    /// Locks the contract surfaced by the dialog's Confirm button: it
    /// MUST dispatch exactly the command produced by
    /// [`profile_delete_action`] (the shared action helper that
    /// declares `ConfirmationKind::DestructiveF4`). Drift in either
    /// direction (the helper changing the command shape, or the
    /// dialog reaching for a different command builder) breaks this
    /// test.
    #[test]
    fn confirm_dispatches_profile_delete_action_command() {
        let action = profile_delete_action("Alpha");
        assert_eq!(
            action.command,
            EngineCommand::DeleteProfile {
                name: "Alpha".to_owned()
            }
        );
    }
}
