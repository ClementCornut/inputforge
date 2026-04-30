//! GUI-only chrome state.
//!
//! Provided in `app_root` alongside `AppContext`, `LaunchParams`, and
//! `ToastQueue`. Three Signals carry chrome-only state — `editing_mode`
//! (the user's authoring focus), `panel_slot` (which right panel is open),
//! and `via_calibration` (sticky while `panel_slot == Devices` so the
//! Calibration tool button can re-open Devices in calibration mode).

use dioxus::prelude::*;

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
}

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
/// - `editing_mode`    = `meta.startup_mode` (or "Default" if no profile)
/// - `panel_slot`      = `None`
/// - `via_calibration` = `false`
///
/// Reconciliation `use_effect`:
/// - When `meta.profile_name` flips, reset `editing_mode` to the new
///   `startup_mode` (or `"Default"` if `startup_mode` is None).
/// - Else if `editing_mode` is no longer present in `meta.modes`, reset to
///   `startup_mode`. If `startup_mode` is also missing from `modes`, fall
///   back to `modes[0]` (DFS-pre-order root). If `modes` is empty, leave
///   `editing_mode` alone — `mode_tabs` is robust to a non-existent value.
#[allow(dead_code, reason = "Called from app_root in Task 18")]
pub(crate) fn use_view_state_provider(meta: Signal<MetaSnapshot>) -> ViewState {
    let initial_editing = meta
        .peek()
        .startup_mode
        .clone()
        .unwrap_or_else(|| "Default".to_owned());
    let editing_mode = use_signal(|| initial_editing);
    let panel_slot = use_signal(PanelSlot::default);
    let via_calibration = use_signal(|| false);

    let mut last_profile_name: Signal<Option<String>> =
        use_signal(|| meta.peek().profile_name.clone());

    let mut em = editing_mode;
    use_effect(move || {
        let m = meta.read();
        let profile_changed = *last_profile_name.peek() != m.profile_name;
        if profile_changed {
            last_profile_name.write().clone_from(&m.profile_name);
            let next = m
                .startup_mode
                .clone()
                .unwrap_or_else(|| "Default".to_owned());
            *em.write() = next;
            return;
        }
        let editing_now = em.peek().clone();
        if !m.modes.iter().any(|n| n == &editing_now) {
            // Editing mode disappeared mid-session.
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
        }
    });

    ViewState {
        editing_mode,
        panel_slot,
        via_calibration,
    }
}
