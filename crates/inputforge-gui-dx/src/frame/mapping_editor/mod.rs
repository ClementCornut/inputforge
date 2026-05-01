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
mod name_field;
pub(crate) mod pipeline;
pub(crate) mod undo_log;

pub(crate) use empty_state::EmptyState;
use engine_offline_banner::EngineOfflineBanner;
use header::Header;
use name_field::NameField;

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

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
    let _editor = use_context::<EditorState>();

    let view_state_for_render: Option<MappingKey> = view.selected_mapping.read().clone();

    // Hoist the stylesheet and offline banner above the if/else split so
    // both branches render under a shared shell. This prevents the
    // duplication from compounding as Tasks 16-19 add more sections.
    rsx! {
        Stylesheet { href: MAPPING_EDITOR_CSS }
        div { class: "if-editor",
            EngineOfflineBanner {}
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
                        Header { name: mapping_name.clone(), input: input.clone() }
                        NameField {
                            initial: mapping_name,
                            mapping_key: (mode, input),
                            actions: actions_clone,
                        }
                        // Remaining sections (live readout, pipeline, footer)
                        // land in subsequent tasks.
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
    /// External-edit reconciliation token. Incremented by the polling
    /// task (bridge.rs) on every external snapshot change. Bodies
    /// subscribe via `use_effect` and reset their local Signals when the
    /// token advances. See Task 33.
    pub external_edit_reset: Signal<u64>,
}

/// Allocate signals and install `EditorState` in context. Call exactly
/// once from `app_root`, the provider self-installs.
pub(crate) fn use_editor_state_provider() -> EditorState {
    let undo_log: Signal<UndoLog> = use_signal(UndoLog::default);
    let expanded_stages: Signal<HashSet<StageId>> = use_signal(HashSet::new);
    let stage_menu: Signal<Option<StageMenuState>> = use_signal(|| None);
    let malformed_hints: Signal<HashMap<StageId, String>> = use_signal(HashMap::new);
    let external_edit_reset: Signal<u64> = use_signal(|| 0_u64);

    let state = EditorState {
        undo_log,
        expanded_stages,
        stage_menu,
        malformed_hints,
        external_edit_reset,
    };
    use_context_provider(|| state);
    state
}

#[cfg(test)]
mod tests;
