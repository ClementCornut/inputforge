//! Snapshots section: the only F15 section.
//!
//! Two field rows (Snapshot buffer size, Skip startup snapshot if unchanged)
//! plus the prune-confirm dialog. Owns the local in-flight `Signal<String>`
//! for `max_count`, the validate-and-dispatch handler, and the would-prune
//! computation (`unpinned - candidate`, saturating).

// Rust guideline compliant 2026-05-09

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::snapshot::SnapshotConfig;

use crate::components::{IntegerInput, IntegerInputError, Switch};
use crate::context::AppContext;
use crate::frame::settings_panel::field_row::SettingsFieldRow;
use crate::frame::settings_panel::prune_confirm::PruneConfirmDialog;
use crate::frame::settings_panel::section::SettingsSection;
use crate::toast::{ToastLevel, ToastQueue};

/// Inclusive lower bound of the snapshot buffer size. A buffer of zero would
/// disable the rolling history entirely; one is the smallest meaningful value.
const MAX_COUNT_MIN: usize = 1;

/// Inclusive upper bound of the snapshot buffer size. Mirrors the engine-side
/// `SnapshotConfig` policy and prevents accidental disk pressure from a typo.
const MAX_COUNT_MAX: usize = 100;

/// HTML id linking the buffer-size label to the IntegerInput control.
const MAX_COUNT_ID: &str = "if-settings-snapshot-max-count";

/// HTML id linking the skip-startup label to the Switch control.
const SKIP_UNCHANGED_ID: &str = "if-settings-snapshot-skip-unchanged";

#[component]
pub(crate) fn SnapshotsSection() -> Element {
    let ctx = use_context::<AppContext>();
    let settings = ctx.settings;
    let commands = ctx.commands.clone();

    // Local error state for the max_count input.
    let mut max_count_error = use_signal(|| Option::<String>::None);

    // Pending commit, used to defer dispatch behind the prune-confirm dialog
    // when reducing max_count below the unpinned count. None when no
    // confirmation is in flight.
    let mut pending_prune = use_signal(|| Option::<PendingPrune>::None);
    let mut prune_dialog_open = use_signal(|| false);

    let polled_snapshot = settings.read().snapshot.clone();
    let polled_max_count: usize = polled_snapshot.max_count;
    let polled_skip = polled_snapshot.skip_if_unchanged;
    let unpinned_count = settings.read().unpinned_snapshot_count;

    let active_profile_name = ctx
        .meta
        .read()
        .profile_name
        .clone()
        .unwrap_or_else(|| "this profile".to_owned());

    // Mirror polled values into a Signal that IntegerInput / Switch accept
    // as `ReadSignal<T>`. The Signal is created once and resynced via
    // `use_effect` whenever the polled value changes; this is the same
    // mirror pattern as `components/number_input.rs:60-71`.
    let mut max_count_signal = use_signal(|| polled_max_count);
    use_effect(use_reactive!(|polled_max_count| {
        max_count_signal.set(polled_max_count);
    }));

    // Local in-flight Signal for the switch. Mirrors the polled value when
    // no user gesture is pending; click handlers update it locally before
    // dispatching, so two clicks within one polling tick read distinct
    // values and dispatch distinct commits (no double-click race).
    let mut skip_local = use_signal(|| polled_skip);
    use_effect(use_reactive!(|polled_skip| {
        skip_local.set(polled_skip);
    }));

    // Toast queue for the optimistic prune-success notification.
    let toast_queue = use_context::<ToastQueue>();

    let commands_for_max = commands.clone();
    let on_max_count_commit = move |candidate: usize| {
        // No-op when the value matches what the engine already holds
        // (re-blur after no edit, polling-resync, etc).
        max_count_error.set(None);
        if candidate == polled_max_count {
            return;
        }
        let would_prune = unpinned_count.saturating_sub(candidate);
        if would_prune > 0 {
            *pending_prune.write() = Some(PendingPrune {
                candidate_max: candidate,
                will_remove: would_prune,
            });
            prune_dialog_open.set(true);
        } else {
            let cfg = SnapshotConfig {
                max_count: candidate,
                skip_if_unchanged: skip_local(),
            };
            let _ = commands_for_max.send(EngineCommand::SetSnapshotConfig { config: cfg });
        }
    };

    // IntegerInput.oninvalid: translate the typed error into a
    // user-facing helper-text replacement. The Choice-9 spec wording
    // covers Empty/NotANumber/OutOfRange variants distinctly so the
    // user knows which path triggered the message.
    let on_max_count_invalid = move |err: IntegerInputError| {
        let msg = match err {
            IntegerInputError::Empty => "Enter a value between 1 and 100".to_owned(),
            IntegerInputError::NotANumber => "Must be a whole number between 1 and 100".to_owned(),
            IntegerInputError::OutOfRange { min, max } => {
                format!("Must be between {min} and {max}")
            }
        };
        max_count_error.set(Some(msg));
    };

    let commands_for_switch = commands.clone();
    let on_skip_change = move |_evt: FormEvent| {
        // Toggle the local Signal first so two clicks in one polling tick
        // read distinct values; then dispatch the new value. The polled
        // signal will catch up on the next tick and `use_reactive!` above
        // re-syncs `skip_local` to it once the engine acknowledges.
        let new_value = !skip_local();
        skip_local.set(new_value);
        let cfg = SnapshotConfig {
            max_count: polled_max_count,
            skip_if_unchanged: new_value,
        };
        let _ = commands_for_switch.send(EngineCommand::SetSnapshotConfig { config: cfg });
    };

    // Prune-confirm callbacks.
    let commands_for_confirm = commands.clone();
    let active_profile_name_for_toast = active_profile_name.clone();
    let on_prune_confirm = move |()| {
        let Some(pending) = pending_prune.write().take() else {
            return;
        };
        let cfg = SnapshotConfig {
            max_count: pending.candidate_max,
            skip_if_unchanged: skip_local(),
        };
        let _ = commands_for_confirm.send(EngineCommand::SetSnapshotConfig { config: cfg });
        // Optimistic prune-success toast (Choice 15). The engine's actual
        // prune count may diverge under fs error; in that case the engine
        // pushes a separate warning toast via the warnings channel. The
        // optimistic toast here matches the count the user just confirmed
        // in the dialog. The canonical API is
        // `ToastQueue::push(level: ToastLevel, message: impl Into<String>)`
        // (verified at `crates/inputforge-gui-dx/src/toast/queue.rs:27-33`),
        // matching call sites such as `frame/profiles/snapshot_drawer.rs`.
        toast_queue.push(
            ToastLevel::Success,
            format!(
                "Snapshot buffer set to {}. {} removed from {}.",
                pending.candidate_max, pending.will_remove, active_profile_name_for_toast,
            ),
        );
    };
    let on_prune_cancel = move |()| {
        pending_prune.write().take();
    };

    let pending_for_dialog = *pending_prune.read();

    rsx! {
        SettingsSection {
            children: rsx! {
                SettingsFieldRow {
                    label: "Snapshot buffer size".to_owned(),
                    helper: "Maximum number of unpinned snapshots kept per profile. \
                             The oldest are auto-evicted. Pinned snapshots are kept regardless.".to_owned(),
                    control_id: MAX_COUNT_ID.to_owned(),
                    error: max_count_error.read().clone(),
                    control: rsx! {
                        IntegerInput {
                            id: Some(MAX_COUNT_ID.to_owned()),
                            value: max_count_signal,
                            min: MAX_COUNT_MIN,
                            max: MAX_COUNT_MAX,
                            class: "if-integer-input--inset".to_owned(),
                            oncommit: on_max_count_commit,
                            oninvalid: on_max_count_invalid,
                        }
                    },
                }

                SettingsFieldRow {
                    label: "Skip startup snapshot if unchanged".to_owned(),
                    helper: "Don't take a snapshot at app start when the active profile is \
                             identical to the most recent snapshot.".to_owned(),
                    control_id: SKIP_UNCHANGED_ID.to_owned(),
                    control: rsx! {
                        Switch {
                            checked: skip_local,
                            onchange: on_skip_change,
                        }
                    },
                }
            },
        }

        if let Some(pending) = pending_for_dialog {
            PruneConfirmDialog {
                open: prune_dialog_open,
                candidate_max: pending.candidate_max,
                will_remove: pending.will_remove,
                profile_name: active_profile_name,
                oncancel: on_prune_cancel,
                onconfirm: on_prune_confirm,
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PendingPrune {
    candidate_max: usize,
    will_remove: usize,
}
