//! GUI-only chrome state.
//!
//! Provided in `app_root` alongside `AppContext`, `LaunchParams`, and
//! `ToastQueue`. Four Signals carry chrome-only state, `editing_mode`
//! (the user's authoring focus), `panel_slot` (which right panel is open),
//! `via_calibration` (sticky while `panel_slot == Devices` so the
//! Calibration tool button can re-open Devices in calibration mode), and
//! `selected_mapping` (the currently highlighted mapping row, cleared on
//! profile flip or editing-mode flip).

use dioxus::prelude::*;
use inputforge_core::types::InputAddress;

use crate::context::MetaSnapshot;

/// Identifier for a mapping in the editor: `(mode, input)`.
///
/// Used by `view.selected_mapping`, `ConfigSnapshot.selected_mapping_key`,
/// the `UndoLog` map key, and every editor key passing.
pub(crate) type MappingKey = (String, InputAddress);

/// Which right-side panel is currently mounted.
///
/// `None` collapses the panel column; `Devices` and `Profiles` mount the
/// respective panel content (F12 / F13 own contents, F7 ships placeholders).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code, reason = "Used by regions in Task 18+")]
pub(crate) enum PanelSlot {
    #[default]
    None,
    Devices,
    Profiles,
}

/// Which primary workspace owns the main surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MainSurface {
    #[default]
    Mappings,
    BulkMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(dead_code, reason = "New profile sub-mode is wired in Task 7")]
pub(crate) enum ProfilesPanelMode {
    #[default]
    Library,
    NewProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InlineRenameDraft {
    pub id: String,
    pub value: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfilesPanelState {
    pub filter: String,
    pub mode: ProfilesPanelMode,
    pub open_row_menu_id: Option<String>,
    pub profile_rename: Option<InlineRenameDraft>,
    pub snapshot_drawer_open: bool,
    pub snapshot_rename: Option<InlineRenameDraft>,
    pub pending_snapshot_delete_toast_id: Option<String>,
}

impl Default for ProfilesPanelState {
    fn default() -> Self {
        Self {
            filter: String::new(),
            mode: ProfilesPanelMode::Library,
            open_row_menu_id: None,
            profile_rename: None,
            snapshot_drawer_open: false,
            snapshot_rename: None,
            pending_snapshot_delete_toast_id: None,
        }
    }
}

/// GUI-only chrome state, provided once in `app_root`.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code, reason = "Used in app_root context provider (Task 18)")]
pub(crate) struct ViewState {
    pub main_surface: Signal<MainSurface>,
    pub editing_mode: Signal<String>,
    pub panel_slot: Signal<PanelSlot>,
    pub via_calibration: Signal<bool>,
    /// The currently selected mapping row `(action_name, InputAddress)`.
    ///
    /// Reset to `None` on profile flip and on editing-mode flip so that
    /// stale selection state never leaks across context boundaries.
    pub selected_mapping: Signal<Option<MappingKey>>,
    pub profiles_panel: Signal<ProfilesPanelState>,
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
pub(crate) enum ReconcileOutcome {
    /// Profile and modes list are unchanged, no action required.
    NoChange,
    /// `meta.profile_name` differs from the previously seen profile name.
    ProfileFlipped,
    /// `cur_mode` (the current `editing_mode` signal value) differs from
    /// `prev_mode` (the `last_editing_mode` shadow), an external write
    /// flipped the editing mode without going through reconciliation.
    /// Callers MUST clear `selected_mapping`.
    ModeFlipped,
    /// `cur_mode` (or `prev_mode`, equivalently after a fresh tick) is no
    /// longer present in `meta.modes`. Caller falls back to `startup_mode`
    /// or the first available mode.
    ModesListDrifted,
}

/// Pure reconciliation decision, unit-testable without a Dioxus runtime.
///
/// Given the previously-seen profile name, the previously-seen editing mode,
/// the current live editing mode, and the latest `MetaSnapshot`, returns the
/// appropriate [`ReconcileOutcome`]. The hook adapter applies the resulting
/// side effects (signal writes).
///
/// ## Branch ordering
///
/// Branches are evaluated in priority order:
///
/// 1. **Profile flip**, `meta.profile_name` differs from `prev_profile`.
///    Resets everything downstream; wins over all other conditions.
/// 2. **Mode flip**, `cur_mode` differs from `prev_mode`.
///    External write changed `editing_mode`; clears `selected_mapping`.
/// 3. **Modes-list drift**, `cur_mode` is absent from `meta.modes`.
///    Mode was deleted mid-session; caller falls back to `startup_mode`
///    or `modes[0]`.
/// 4. **No change**, none of the above.
pub(crate) fn reconcile_pure(
    prev_profile: &str,
    prev_mode: &str,
    cur_mode: &str,
    meta: &MetaSnapshot,
) -> ReconcileOutcome {
    let cur_profile = meta.profile_name.as_deref().unwrap_or("");
    if prev_profile != cur_profile {
        return ReconcileOutcome::ProfileFlipped;
    }
    if cur_mode != prev_mode {
        return ReconcileOutcome::ModeFlipped;
    }
    if !meta.modes.iter().any(|m| m == cur_mode) {
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
/// `MetaSnapshot::default()` populated by `app.rs`, `startup_mode`
/// is `None` at that point, so the initial `peek()` falls back to
/// `"Default"`. The first effect run happens *after* the polling task
/// has populated `meta` with a real profile (or kept the empty default
/// if no profile is loaded). `last_profile_name` is peek-initialized
/// to whatever `profile_name` is at mount, which in current `app_root`
/// is `None` because `meta` is initialized to `MetaSnapshot::default`
/// before this hook runs, so the first effect run sees
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
/// Reconciliation `use_effect` delegates to [`reconcile_pure`] and matches
/// on the returned [`ReconcileOutcome`]:
///
/// **`ProfileFlipped`:** `meta.profile_name` changed.
///   Reset `editing_mode` to the new `startup_mode` (or `"Default"`),
///   mirror the new editing mode into `last_editing_mode`, and clear
///   `selected_mapping`. The mirror prevents `ModeFlipped` from firing
///   spuriously on the same tick.
///
/// **`ModeFlipped`:** `editing_mode` changed since the last tick
///   (detected via `last_editing_mode` shadow). Write the new value into
///   `last_editing_mode` and clear `selected_mapping`.
///
/// **`ModesListDrifted`:** `editing_mode` is no longer in `meta.modes`
///   (mode deleted mid-session). Reset to `startup_mode`; if that is also
///   missing, fall back to `modes[0]`; if `modes` is empty, leave
///   `editing_mode` unchanged. `ModeFlipped` will clear `selected_mapping`
///   on the subsequent effect tick when it detects the value change.
///
/// **`NoChange`:** no action required.
#[allow(dead_code, reason = "Called from app_root in Task 18")]
pub(crate) fn use_view_state_provider(meta: Signal<MetaSnapshot>) -> ViewState {
    let initial_editing = meta
        .peek()
        .startup_mode
        .clone()
        .unwrap_or_else(|| "Default".to_owned());

    let editing_mode = use_signal(|| initial_editing.clone());
    let main_surface = use_signal(MainSurface::default);
    let panel_slot = use_signal(PanelSlot::default);
    let via_calibration = use_signal(|| false);
    let selected_mapping: Signal<Option<MappingKey>> = use_signal(|| None);
    let profiles_panel = use_signal(ProfilesPanelState::default);

    let mut last_profile_name: Signal<Option<String>> =
        use_signal(|| meta.peek().profile_name.clone());
    let mut last_editing_mode: Signal<String> = use_signal(|| initial_editing);

    let mut em = editing_mode;
    let mut sel = selected_mapping;
    use_effect(move || {
        let m = meta.read();
        let prev_profile = last_profile_name.peek().clone().unwrap_or_default();
        let prev_mode = last_editing_mode.peek().clone();
        let cur_mode = em.peek().clone();
        let outcome = reconcile_pure(&prev_profile, &prev_mode, &cur_mode, &m);

        match outcome {
            ReconcileOutcome::NoChange => {}
            ReconcileOutcome::ProfileFlipped => {
                last_profile_name.write().clone_from(&m.profile_name);
                let next = m
                    .startup_mode
                    .clone()
                    .unwrap_or_else(|| "Default".to_owned());
                // Mirror into the shadow first so ModeFlipped does not fire on
                // the same tick and clear selection a second time unnecessarily.
                (*last_editing_mode.write()).clone_from(&next);
                *em.write() = next;
                sel.set(None);
            }
            ReconcileOutcome::ModeFlipped => {
                *last_editing_mode.write() = cur_mode;
                sel.set(None);
            }
            ReconcileOutcome::ModesListDrifted => {
                let fallback = if let Some(s) = m.startup_mode.as_ref() {
                    if m.modes.iter().any(|n| n == s) {
                        s.clone()
                    } else {
                        m.modes.first().cloned().unwrap_or(cur_mode)
                    }
                } else {
                    m.modes.first().cloned().unwrap_or(cur_mode)
                };
                *em.write() = fallback;
                // ModeFlipped clears selected_mapping on the next effect tick
                // when it detects that editing_mode has changed.
            }
        }
    });

    ViewState {
        main_surface,
        editing_mode,
        panel_slot,
        via_calibration,
        selected_mapping,
        profiles_panel,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, InputId};

    /// Compile-time gate, proves `selected_mapping` lives on `ViewState`.
    #[test]
    fn selected_mapping_field_type() {
        fn _assert(view: ViewState) {
            let _: Signal<Option<MappingKey>> = view.selected_mapping;
        }
    }

    /// Compile-time gate, proves the primary workspace state lives on `ViewState`.
    #[test]
    fn main_surface_field_type() {
        fn _assert(view: ViewState) {
            let _: Signal<MainSurface> = view.main_surface;
        }
    }

    #[test]
    fn main_surface_defaults_to_mappings() {
        assert_eq!(MainSurface::default(), MainSurface::Mappings);
    }

    fn _synthetic_addr() -> InputAddress {
        InputAddress::Bound {
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
        // cur_mode == prev_mode == "Default", profile unchanged, NoChange.
        let outcome = reconcile_pure("P", "Default", "Default", &meta);
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
        // prev_profile "P" vs cur_profile "Q", ProfileFlipped.
        let outcome = reconcile_pure("P", "Default", "Default", &meta);
        assert_eq!(outcome, ReconcileOutcome::ProfileFlipped);
    }

    #[test]
    fn reconcile_modes_list_drift() {
        // cur_mode == prev_mode == "Combat", but "Combat" is not in meta.modes.
        let meta = MetaSnapshot {
            profile_name: Some("P".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned()],
            ..MetaSnapshot::default()
        };
        let outcome = reconcile_pure("P", "Combat", "Combat", &meta);
        assert_eq!(outcome, ReconcileOutcome::ModesListDrifted);
    }

    #[test]
    fn reconcile_mode_flipped_when_cur_differs_from_prev() {
        let meta = MetaSnapshot {
            profile_name: Some("P".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned(), "Combat".to_owned()],
            ..MetaSnapshot::default()
        };
        // prev_mode = "Default" (shadow), cur_mode = "Combat" (live signal).
        // External write flipped editing_mode → ModeFlipped.
        let outcome = reconcile_pure("P", "Default", "Combat", &meta);
        assert_eq!(outcome, ReconcileOutcome::ModeFlipped);
    }

    #[test]
    fn reconcile_profile_flip_takes_precedence_over_mode_flip() {
        let meta = MetaSnapshot {
            profile_name: Some("Q".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned()],
            ..MetaSnapshot::default()
        };
        // Both prev_profile != cur_profile AND prev_mode != cur_mode.
        // Profile flip wins over mode flip.
        let outcome = reconcile_pure("P", "Default", "Combat", &meta);
        assert_eq!(outcome, ReconcileOutcome::ProfileFlipped);
    }

    #[test]
    #[allow(
        clippy::used_underscore_items,
        reason = "_synthetic_addr is a test-only helper intentionally prefixed."
    )]
    fn mapping_key_alias_compiles() {
        let _: MappingKey = ("Default".to_owned(), _synthetic_addr());
    }
}
// Rust guideline compliant 2026-04-30
