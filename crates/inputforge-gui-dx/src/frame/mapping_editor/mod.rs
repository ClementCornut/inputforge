// Rust guideline compliant 2026-05-01

//! F9 mapping editor (center column). See
//! `docs/superpowers/specs/2026-04-30-f9-mapping-editor-design.md`.

#![allow(
    dead_code,
    reason = "Sub-modules expose APIs that orchestrator + Tasks 12+ consume; \
              clippy's reachability check loses some pub(crate) items here."
)]

mod empty_state;
mod engine_offline_banner;
mod header;
mod inactive_hint;
pub(crate) mod keyboard;
pub(crate) mod live_readout;
pub(crate) mod pipeline;
pub(crate) mod undo_log;
mod undo_recap;

pub(crate) use empty_state::EmptyState;
use engine_offline_banner::EngineOfflineBanner;
use header::Header;
use inactive_hint::InactiveHint;
use live_readout::LiveReadout;
use pipeline::{Pipeline, StageActionsMenu};
use undo_recap::UndoRecap;

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

use crate::components::sortable::{SortableLiveRegion, SortableState, use_sortable_state};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::{StageId, UndoLog};
use crate::frame::view_state::ViewState;

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: MAPPING_EDITOR_CSS }"
)]
const MAPPING_EDITOR_CSS: Asset = asset!("/assets/frame/mapping_editor.css");

/// Top-level mapping editor orchestrator mounted in `if-layout__center`.
///
/// Renders a shared shell (`Stylesheet` + `EngineOfflineBanner`) and then
/// either the selected-mapping sections (header, name field, and future
/// sections) or the empty-state CTA when no mapping is selected.
#[component]
pub(crate) fn MappingEditor() -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_editor");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    // `editor` must be `mut` because the Task 32 use_effect below calls
    // `editor.undo_log.write()` to clear the log on profile flip. Dioxus
    // Signals are Copy and the mutability lives on the binding, not the type.
    let mut editor = use_context::<EditorState>();

    // One shared sortable state for the entire editor. Mounted unconditionally
    // (Dioxus hook rules) so the signals exist regardless of whether a mapping
    // is selected. Provided as context so Stage components can read it without
    // prop-drilling through Pipeline. The live region is rendered once inside
    // the editor shell for AT a11y.
    let sortable: SortableState<StageId> = use_sortable_state::<StageId>();
    // SortableState is `Copy` regardless of `G` (every field is a
    // `Signal`, which is `Copy` unconditionally), so the bundle is
    // freely shared with the live-region mount and the context.
    let sortable_for_live = sortable;
    use_context_provider(|| sortable);

    // Mount the editor-scoped keyboard listener unconditionally (Dioxus hook
    // rules require all hooks to run every render). The listener internally
    // guards against re-installation on subsequent renders.
    keyboard::use_kb_listener();

    // Task 32 (AC #26): clear the undo log on profile flip.
    //
    // The proper intercept point (a DirtyConfirmDialog wired into a real
    // profile-picker component) cannot be implemented yet because the Profiles
    // side panel is an F13 placeholder with no dispatch surface. Instead, we
    // watch `ctx.meta.profile_name` for changes and clear on every transition.
    //
    // Race-condition note: a profile flip also rebuilds `ConfigSnapshot`
    // (via the polling task in bridge.rs), so by the time any mapping-editor
    // sub-component reads the new snapshot the undo log is already empty.
    // The only observable gap is between the `profile_name` signal updating
    // and this effect running, which is sub-frame and therefore safe.
    //
    // When F13 ships a real picker, replace this with a DirtyConfirmDialog
    // onsave callback that calls `editor.undo_log.write().clear_all()` before
    // dispatching the load-profile command, then remove this effect.
    let profile_name_memo = use_memo(move || ctx.meta.read().profile_name.clone());
    // `prev_profile` holds the last-seen name so we can detect the None to Some,
    // Some to None, and Some(a) to Some(b) transitions without clearing on mount.
    // Outer `Option` distinguishes "not yet observed" (initial mount) from
    // "observed and stored". Inner `Option<String>` mirrors the meta field.
    let mut prev_profile: Signal<Option<Option<String>>> = use_signal(|| None);
    use_effect(move || {
        let current = profile_name_memo.read().clone();
        // Clone out of the read guard so we can call .set() without overlap.
        let prev_snapshot = prev_profile.read().clone();
        match prev_snapshot {
            // First render: record the baseline without clearing.
            None => prev_profile.set(Some(current)),
            // Subsequent renders: clear only when the name actually changed.
            Some(last) if last != current => {
                editor.undo_log.write().clear_all();
                prev_profile.set(Some(current));
            }
            // Same name: no-op.
            Some(_) => {}
        }
    });

    // Task 34 (AC #27): revert to empty state when the selected mapping has
    // been deleted externally (e.g. from the rail or via an external edit).
    //
    // This MUST run inside a `use_effect` and NOT during render. Mutating a
    // signal during render causes Dioxus to schedule an immediate re-render,
    // creating an infinite loop. The effect runs after the DOM commit, so the
    // write is batched into the next frame rather than into the current one.
    //
    // The key used here is `ctx.config` (reads `mappings`), which changes
    // whenever the external snapshot is updated. Dioxus effects re-fire when
    // any signal they read inside changes; `sel.peek()` is used so the effect
    // does NOT re-fire merely because we cleared the selection ourselves.
    let cfg_for_stale = ctx.config;
    let mut sel = view.selected_mapping;
    use_effect(move || {
        // Reading `cfg_for_stale` subscribes this effect to config changes.
        let snap = cfg_for_stale.read();
        // `peek` does NOT subscribe; prevents a self-triggered re-run after
        // we call `sel.set(None)` below.
        let current = sel.peek().clone();
        if let Some((mode, input)) = current {
            let resolved = snap
                .mappings
                .iter()
                .any(|m| m.input == input && m.mode == mode);
            if !resolved {
                sel.set(None);
            }
        }
    });

    let view_state_for_render: Option<MappingKey> = view.selected_mapping.read().clone();

    // Hoist the stylesheet and offline banner above the if/else split so
    // both branches render under a shared shell. This prevents the
    // duplication from compounding as Tasks 16-19 add more sections.
    rsx! {
        Stylesheet { href: MAPPING_EDITOR_CSS }
        div { class: "if-editor",
            EngineOfflineBanner {}
            SortableLiveRegion { state: sortable_for_live }
            if let Some((mode, input)) = view_state_for_render {
                {
                    let mapping_name = ctx
                        .config
                        .read()
                        .mappings
                        .iter()
                        .find(|m| m.input == input && m.mode == mode)
                        .and_then(|m| m.name.clone())
                        .unwrap_or_else(|| "Untitled mapping".to_owned());
                    let actions_clone = ctx
                        .config
                        .read()
                        .selected_mapping_actions
                        .clone()
                        .unwrap_or_default();
                    rsx! {
                        Header {
                            name: mapping_name,
                            input: input.clone(),
                            mapping_key: (mode.clone(), input.clone()),
                            actions: actions_clone.clone(),
                        }
                        LiveReadout {
                            primary: input.clone(),
                            actions: actions_clone.clone(),
                        }
                        Pipeline {
                            mapping_key: (mode.clone(), input.clone()),
                            actions: actions_clone.clone(),
                            root_actions: actions_clone.clone(),
                            path_prefix: vec![],
                            depth: 0,
                        }
                        StageActionsMenu {
                            mapping_key: (mode.clone(), input.clone()),
                            root_actions: actions_clone.clone(),
                        }
                        InactiveHint {}
                        UndoRecap { mapping_key: (mode, input) }
                    }
                }
            } else {
                EmptyState {}
            }
        }
    }
}

/// Right-click stage menu state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StageMenuState {
    pub stage: StageId,
    /// page-space anchor coordinates
    pub x: f64,
    pub y: f64,
}

/// Editor-internal context, parallel to `LiveCapture` and `ToastQueue`.
///
/// Installed once via `use_editor_state_provider` from `app_root`.
/// Components read via `use_context::<EditorState>()`.
#[derive(Clone, Copy)]
pub(crate) struct EditorState {
    /// Per-mapping undo stacks. Cleared on profile flip via Task 32's
    /// `DirtyConfirmDialog::onsave` callback.
    pub undo_log: Signal<UndoLog>,
    /// Stage IDs that are currently expanded. Resets on selection change
    /// AND on every structural mutation (insert/remove): see Task 11.
    pub expanded_stages: Signal<HashSet<StageId>>,
    /// Right-click menu state (anchor + target stage).
    pub stage_menu: Signal<Option<StageMenuState>>,
    /// Per-stage validation hints surfaced in the stage header summary
    /// slot per spec lines 587-589. Bodies write on render; the stage
    /// header reads. Cleared on every structural mutation: see Task 11.
    pub malformed_hints: Signal<HashMap<StageId, String>>,
}

/// Allocate signals and install `EditorState` in context. Call exactly
/// once from `app_root`, the provider self-installs.
pub(crate) fn use_editor_state_provider() -> EditorState {
    let undo_log: Signal<UndoLog> = use_signal(UndoLog::default);
    let expanded_stages: Signal<HashSet<StageId>> = use_signal(HashSet::new);
    let stage_menu: Signal<Option<StageMenuState>> = use_signal(|| None);
    let malformed_hints: Signal<HashMap<StageId, String>> = use_signal(HashMap::new);

    let state = EditorState {
        undo_log,
        expanded_stages,
        stage_menu,
        malformed_hints,
    };
    use_context_provider(|| state);
    state
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_helpers;
