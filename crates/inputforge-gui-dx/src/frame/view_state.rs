//! GUI-only chrome state.
//!
//! Provided in `app_root` alongside `AppContext`, `LaunchParams`, and
//! `ToastQueue`. Four Signals carry chrome-only state — `editing_mode`
//! (the user's authoring focus), `panel_slot` (which right panel is open),
//! `via_calibration` (sticky while `panel_slot == Devices` so the
//! Calibration tool button can re-open Devices in calibration mode), and
//! `selected_mapping` (the currently highlighted mapping row, cleared on
//! profile flip or editing-mode flip).

use dioxus::prelude::*;
use inputforge_core::types::InputAddress;

use crate::context::MetaSnapshot;

/// Which right-side panel is currently mounted.
///
/// `None` collapses the panel column; `Devices` and `Profiles` mount the
/// respective panel content (F12 / F13 own contents — F7 ships placeholders).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code, reason = "Used by regions in Task 18+")]
pub(crate) enum PanelSlot {
    #[default]
    None,
    Devices,
    Profiles,
}

/// GUI-only chrome state — provided once in `app_root`.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code, reason = "Used in app_root context provider (Task 18)")]
pub(crate) struct ViewState {
    pub editing_mode: Signal<String>,
    pub panel_slot: Signal<PanelSlot>,
    pub via_calibration: Signal<bool>,
    /// The currently selected mapping row `(action_name, InputAddress)`.
    ///
    /// Reset to `None` on profile flip and on editing-mode flip so that
    /// stale selection state never leaks across context boundaries.
    pub selected_mapping: Signal<Option<(String, InputAddress)>>,
}

// ---------------------------------------------------------------------------
// Reconciliation helpers
// ---------------------------------------------------------------------------

/// Decision produced by the pure reconciliation logic.
///
/// The hook adapter reads this value and applies the corresponding side
/// effects (signal writes). Keeping the decision logic pure lets unit tests
/// cover it without a Dioxus runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "Consumed by unit tests and future mapping-list tasks"
)]
pub(crate) enum ReconcileOutcome {
    /// Profile and modes list are unchanged — no action required.
    NoChange,
    /// `meta.profile_name` differs from the previously seen profile name.
    ProfileFlipped,
    /// The profile name is the same but `prev_mode` is no longer present in
    /// `meta.modes` (mode was deleted mid-session).
    ModesListDrifted,
}

/// Pure reconciliation decision — unit-testable without a Dioxus runtime.
///
/// Given the previously-seen profile name, the previously-seen editing mode,
/// and the latest `MetaSnapshot`, returns the appropriate `ReconcileOutcome`.
/// The hook adapter applies the resulting side effects (signal writes).
///
/// Note: editing-mode *flip* (user switching tabs) is detected on the hook
/// side via the `last_editing_mode` shadow signal because it requires
/// comparing two runtime signals, which is not representable in this pure
/// (`prev_profile`, `prev_mode`, meta) signature.
#[allow(
    dead_code,
    reason = "Consumed by unit tests and future mapping-list tasks"
)]
pub(crate) fn reconcile_pure(
    prev_profile: &str,
    prev_mode: &str,
    meta: &MetaSnapshot,
) -> ReconcileOutcome {
    let cur_profile = meta.profile_name.as_deref().unwrap_or("");
    if prev_profile != cur_profile {
        return ReconcileOutcome::ProfileFlipped;
    }
    if !meta.modes.iter().any(|m| m == prev_mode) {
        return ReconcileOutcome::ModesListDrifted;
    }
    ReconcileOutcome::NoChange
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

/// Hook that creates `ViewState` and reconciles `editing_mode` against
/// `meta` on profile load/switch and after mode CRUD that removes the
/// editing mode.
///
/// **Two-phase init.** The hook fires once at mount when `meta` is the
/// `MetaSnapshot::default()` populated by `app.rs` — `startup_mode`
/// is `None` at that point, so the initial `peek()` falls back to
/// `"Default"`. The first effect run happens *after* the polling task
/// has populated `meta` with a real profile (or kept the empty default
/// if no profile is loaded). `last_profile_name` is peek-initialized
/// to whatever `profile_name` is at mount, which in current `app_root`
/// is `None` because `meta` is initialized to `MetaSnapshot::default`
/// before this hook runs — so the first effect run sees
/// `profile_changed == false` (None → None) when no profile is loaded
/// and `profile_changed == true` when one is loaded; both branches do
/// the right thing.
///
/// Initial values:
/// - `editing_mode`     = `meta.startup_mode` (or `"Default"` if no profile)
/// - `panel_slot`       = `None`
/// - `via_calibration`  = `false`
/// - `selected_mapping` = `None`
///
/// Reconciliation `use_effect` — three branches, evaluated in order:
///
/// **Branch 1 (profile flip):** `meta.profile_name` changed.
///   Reset `editing_mode` to the new `startup_mode` (or `"Default"`),
///   mirror the new editing mode into `last_editing_mode`, and clear
///   `selected_mapping`. The mirror prevents Branch 2 from firing
///   spuriously on the same tick.
///
/// **Branch 2 (editing-mode flip):** `editing_mode` changed since the last
///   tick (detected via `last_editing_mode` shadow). Write the new value
///   into `last_editing_mode` and clear `selected_mapping`.
///
/// **Branch 3 (modes-list drift):** `editing_mode` is no longer in
///   `meta.modes` (mode deleted mid-session). Reset to `startup_mode`;
///   if that is also missing, fall back to `modes[0]`; if `modes` is
///   empty, leave `editing_mode` unchanged. Branch 2 will clear
///   `selected_mapping` on the subsequent effect tick when it detects
///   the value change.
#[allow(dead_code, reason = "Called from app_root in Task 18")]
pub(crate) fn use_view_state_provider(meta: Signal<MetaSnapshot>) -> ViewState {
    let initial_editing = meta
        .peek()
        .startup_mode
        .clone()
        .unwrap_or_else(|| "Default".to_owned());

    let editing_mode = use_signal(|| initial_editing.clone());
    let panel_slot = use_signal(PanelSlot::default);
    let via_calibration = use_signal(|| false);
    let selected_mapping: Signal<Option<(String, InputAddress)>> = use_signal(|| None);

    let mut last_profile_name: Signal<Option<String>> =
        use_signal(|| meta.peek().profile_name.clone());
    let mut last_editing_mode: Signal<String> = use_signal(|| initial_editing);

    let mut em = editing_mode;
    let mut sel = selected_mapping;
    use_effect(move || {
        let m = meta.read();

        // Branch 1: profile flip.
        let profile_changed = *last_profile_name.peek() != m.profile_name;
        if profile_changed {
            last_profile_name.write().clone_from(&m.profile_name);
            let next = m
                .startup_mode
                .clone()
                .unwrap_or_else(|| "Default".to_owned());
            // Mirror into the shadow first so Branch 2 does not fire on the
            // same tick and clear selection a second time unnecessarily.
            last_editing_mode.write().clone_from(&next);
            *em.write() = next;
            sel.set(None);
            return;
        }

        // Branch 2: editing-mode flip (user switched tabs).
        let editing_now = em.peek().clone();
        if *last_editing_mode.peek() != editing_now {
            *last_editing_mode.write() = editing_now;
            sel.set(None);
            return;
        }

        // Branch 3: modes-list drift — editing mode was deleted mid-session.
        if !m.modes.iter().any(|n| n == &*em.peek()) {
            let editing_now = em.peek().clone();
            let fallback = if let Some(s) = m.startup_mode.as_ref() {
                if m.modes.iter().any(|n| n == s) {
                    s.clone()
                } else {
                    m.modes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| editing_now.clone())
                }
            } else {
                m.modes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| editing_now.clone())
            };
            *em.write() = fallback;
            // Branch 2 clears selected_mapping on the next effect tick when
            // it detects the editing_mode value has changed.
        }
    });

    ViewState {
        editing_mode,
        panel_slot,
        via_calibration,
        selected_mapping,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, InputId};

    /// Compile-time gate — proves `selected_mapping` lives on `ViewState`.
    #[test]
    fn selected_mapping_field_type() {
        fn _assert(view: ViewState) {
            let _: Signal<Option<(String, InputAddress)>> = view.selected_mapping;
        }
    }

    fn _synthetic_addr() -> InputAddress {
        InputAddress {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: 0 },
        }
    }

    #[test]
    fn reconcile_no_change_returns_nochange() {
        let meta = MetaSnapshot {
            profile_name: Some("P".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned(), "Combat".to_owned()],
            ..MetaSnapshot::default()
        };
        let outcome = reconcile_pure("P", "Default", &meta);
        assert_eq!(outcome, ReconcileOutcome::NoChange);
    }

    #[test]
    fn reconcile_profile_flip() {
        let meta = MetaSnapshot {
            profile_name: Some("Q".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned()],
            ..MetaSnapshot::default()
        };
        let outcome = reconcile_pure("P", "Default", &meta);
        assert_eq!(outcome, ReconcileOutcome::ProfileFlipped);
    }

    #[test]
    fn reconcile_modes_list_drift() {
        // prev_mode points at a mode no longer in meta.modes.
        let meta = MetaSnapshot {
            profile_name: Some("P".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned()],
            ..MetaSnapshot::default()
        };
        let outcome = reconcile_pure("P", "Combat", &meta);
        assert_eq!(outcome, ReconcileOutcome::ModesListDrifted);
    }
}
