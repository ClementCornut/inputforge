// Rust guideline compliant 2026-04-30

//! F4 destructive-confirm dialog for Delete-mode. Hoisted out of the
//! `ModeTabs` component so it doesn't render as a child of the mode
//! strip or the top-bar flex line. `Layout` mounts it as shell-level
//! chrome, keeping navigation children lists pure.
//!
//! State is shared with `ModeTabs` via the [`ModeDeleteSignal`]
//! context: `ModeTabs` writes the target mode name on the Delete
//! keybind and the context-menu Delete item; this component reads the
//! target, drives `dialog_open`, derives the blast-radius counts, and
//! posts `EngineCommand::DeleteMode` on Confirm.

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;

/// Newtype wrapper so multiple `Signal<Option<String>>` providers can
/// coexist in the context tree without collisions.
#[derive(Clone, Copy)]
pub(crate) struct ModeDeleteSignal(pub Signal<Option<String>>);

#[component]
pub(crate) fn ModeDeleteDialog() -> Element {
    let ctx = use_context::<AppContext>();
    let mut delete_target = use_context::<ModeDeleteSignal>().0;

    // Mirror `delete_target` into a boolean `dialog_open` so the
    // DialogRoot's `Signal<bool>` contract is satisfied. The reverse
    // direction (close → clear target) is covered by Cancel/Confirm
    // onclicks plus the DialogRoot's onclose handler.
    let mut dialog_open: Signal<bool> = use_signal(|| false);
    use_effect(move || {
        let want = delete_target.read().is_some();
        if *dialog_open.peek() != want {
            dialog_open.set(want);
        }
    });

    // Pre-compute the blast-radius counts every render, cheap walk.
    // The numeric magnitudes are the dialog body's "what does this
    // affect" readout; mode count includes the target itself plus any
    // descendants in the active profile's mode tree.
    let (display_name, modes_count, mappings_count, restore_idx) =
        match delete_target.read().as_ref() {
            Some(name) => {
                let s = ctx.state.read();
                let counts = s.active_profile.as_ref().map_or((1, 0), |p| {
                    let descendants = p.modes().descendants_of(name).unwrap_or_default();
                    let modes_count = 1 + descendants.len();
                    let mut deleted: Vec<String> = descendants;
                    deleted.push(name.clone());
                    let mappings_count = p
                        .mappings()
                        .iter()
                        .filter(|m| deleted.iter().any(|d| d == &m.mode))
                        .count();
                    (modes_count, mappings_count)
                });
                let restore_idx = ctx.meta.read().modes.iter().position(|m| m == name);
                (name.clone(), counts.0, counts.1, restore_idx)
            }
            None => (String::new(), 0, 0, None),
        };

    let cmd_for_delete = ctx.commands.clone();
    let confirm_name = display_name.clone();

    // Focus-restore callback. After the modes list updates (delete
    // confirmed) or the user cancels, return focus to the tab at the
    // remembered index, clamped to the new list length. Uses a DOM
    // selector keyed off the integer-derived `mode-tab-N` id pattern
    // emitted by `ModeTabs`, this decouples the dialog from
    // `ModeTabs`'s local `tab_refs` storage now that the two
    // components are siblings rather than a parent-child pair.
    let onclose = move |()| {
        if let Some(idx) = restore_idx {
            spawn(async move {
                let _ = document::eval(&format!(
                    "var tabs = document.querySelectorAll('[role=\"tab\"]');\n\
                     if (tabs.length > 0) {{\n\
                         var i = Math.min({idx}, tabs.length - 1);\n\
                         tabs[i].focus();\n\
                     }}"
                ));
            });
        }
        delete_target.set(None);
    };

    rsx! {
        crate::components::DialogRoot {
            open: dialog_open,
            onclose,
            crate::components::DialogTitle { "Delete mode" }
            // Body splits the prose question from the numeric blast-
            // radius readout. The lead carries the action; the mono
            // count strip below carries the consequence. Cockpit
            // vocabulary: a system-status caption ("MODES" / "MAPPINGS")
            // beside its tabular-nums value reads as a real instrument-
            // panel readout, which fits a destructive-confirm better
            // than a single prose sentence.
            crate::components::DialogBody {
                "Delete '{display_name}'?"
                div { class: "if-modetab-delete-confirm__counts",
                    span { strong { "{modes_count}" } " modes" }
                    span { strong { "{mappings_count}" } " mappings" }
                }
            }
            crate::components::DialogFooter {
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Ghost,
                    onmounted: move |evt: MountedEvent| {
                        spawn(async move {
                            let _ = evt.data().set_focus(true).await;
                        });
                    },
                    onclick: move |_| { delete_target.set(None); },
                    "Cancel"
                }
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Secondary,
                    onclick: move |_| {
                        let _ = cmd_for_delete.send(EngineCommand::DeleteMode {
                            name: confirm_name.clone(),
                        });
                        delete_target.set(None);
                    },
                    "Confirm"
                }
            }
        }
    }
}
