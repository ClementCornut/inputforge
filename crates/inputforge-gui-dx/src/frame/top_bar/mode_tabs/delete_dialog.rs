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
use inputforge_core::profile::Profile;

use crate::context::AppContext;

/// Newtype wrapper so multiple `Signal<Option<String>>` providers can
/// coexist in the context tree without collisions.
#[derive(Clone, Copy)]
pub(crate) struct ModeDeleteSignal(pub Signal<Option<String>>);

/// Imperative tab-focus channel shared between `ModeTabs` (which feeds
/// it into `TabsRoot::focus_request`) and the dialog (which sets it on
/// close to land focus back on a tab name). Lifted to context so the
/// dialog and the tabs are siblings under `Layout`.
#[derive(Clone, Copy)]
pub(crate) struct ModeFocusSignal(pub Signal<Option<String>>);

fn affected_counts(name: &str, profile: Option<&Profile>) -> (usize, usize) {
    let modes_count = 1usize;
    let mappings_count = profile.map_or(0usize, |profile| {
        profile
            .mappings()
            .iter()
            .filter(|mapping| mapping.mode == *name)
            .count()
    });
    (modes_count, mappings_count)
}

fn count_label(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}

#[component]
pub(crate) fn ModeDeleteDialog() -> Element {
    let ctx = use_context::<AppContext>();
    let mut delete_target = use_context::<ModeDeleteSignal>().0;
    let mut mode_focus = use_context::<ModeFocusSignal>().0;

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

    // Pre-compute the affected counts every render, cheap walk.
    // The numeric magnitudes are the dialog body's "what does this
    // affect" readout; mode count is the target itself. `survivor_name`
    // is the tab the user should land on after a Confirm: take the current
    // modes list, drop the deleted mode, clamp the deleted tab's old index
    // against the survivors length. For Cancel / Escape / click-outside,
    // focus simply returns to the tab that owned the menu (`display_name`).
    let (display_name, modes_count, mappings_count, survivor_name) =
        match delete_target.read().as_ref() {
            Some(name) => {
                let counts = {
                    let s = ctx.state.read();
                    affected_counts(name, s.active_profile.as_ref())
                };
                let modes_now = ctx.meta.read().modes.clone();
                let restore_idx = modes_now.iter().position(|m| m == name);
                let survivors: Vec<String> = modes_now.into_iter().filter(|m| m != name).collect();
                let survivor = restore_idx.and_then(|idx| {
                    if survivors.is_empty() {
                        None
                    } else {
                        let clamped = idx.min(survivors.len() - 1);
                        Some(survivors[clamped].clone())
                    }
                });
                (name.clone(), counts.0, counts.1, survivor)
            }
            None => (String::new(), 0, 0, None),
        };

    let cmd_for_delete = ctx.commands.clone();
    let confirm_name = display_name.clone();
    let cancel_focus_name = display_name.clone();
    let confirm_focus_name = survivor_name.clone();
    let modes_label = count_label(modes_count, "mode", "modes");
    let mappings_label = count_label(mappings_count, "mapping", "mappings");

    // Focus-restore callback. on_close fires for Escape and
    // click-outside; both are Cancel-shaped (the modes list is
    // unchanged), so route focus back to the tab that owned the
    // dialog via the shared `ModeFocusSignal`. The Cancel and
    // Confirm button onclick handlers below set the signal explicitly
    // before clearing `delete_target`, so this callback only matters
    // when neither button was clicked. Single focus channel,
    // `TabsList` is the only thing that calls `set_focus(true)`.
    let onclose = move |()| {
        mode_focus.set(Some(cancel_focus_name.clone()));
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
                    span { strong { "{modes_count}" } " {modes_label}" }
                    span { strong { "{mappings_count}" } " {mappings_label}" }
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
                    onclick: move |_| {
                        // Cancel: modes list is unchanged, return
                        // focus to the tab that owned the dialog.
                        mode_focus.set(Some(display_name.clone()));
                        delete_target.set(None);
                    },
                    "Cancel"
                }
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Secondary,
                    onclick: move |_| {
                        // Confirm: route focus to the survivor at the
                        // deleted tab's old index (clamped). Computed
                        // above from the modes list at render time;
                        // by the time TabsList consumes the signal
                        // the modes list has updated and the survivor
                        // name is in the registry.
                        let _ = cmd_for_delete.send(EngineCommand::DeleteMode {
                            name: confirm_name.clone(),
                        });
                        if let Some(target) = confirm_focus_name.clone() {
                            mode_focus.set(Some(target));
                        }
                        delete_target.set(None);
                    },
                    "Confirm"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    use super::{affected_counts, count_label};

    fn input(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index },
        }
    }

    fn mapping(mode: &str, index: u8) -> Mapping {
        Mapping {
            input: input(index),
            mode: mode.to_owned(),
            name: None,
            actions: vec![],
        }
    }

    fn profile_with_mappings(mappings: Vec<Mapping>) -> Profile {
        let modes = Modes::new(vec![
            "Default".to_owned(),
            "Combat".to_owned(),
            "Landing".to_owned(),
        ])
        .unwrap();
        Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        )
    }

    #[test]
    fn affected_counts_include_selected_mode_and_direct_mappings_only() {
        let profile = profile_with_mappings(vec![
            mapping("Combat", 0),
            mapping("Combat", 1),
            mapping("Landing", 2),
            mapping("Default", 3),
        ]);

        assert_eq!(affected_counts("Combat", Some(&profile)), (1, 2));
    }

    #[test]
    fn count_label_uses_singular_for_one() {
        assert_eq!(count_label(1, "mode", "modes"), "mode");
    }
}
