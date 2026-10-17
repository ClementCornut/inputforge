// Rust guideline compliant 2026-05-01

//! F9 mapping editor (center column).

#![allow(
    dead_code,
    reason = "Sub-modules expose APIs that orchestrator + Tasks 12+ consume; \
              clippy's reachability check loses some pub(crate) items here."
)]

#[cfg(test)]
mod capture_events_tests;
mod content;
mod empty_state;
mod engine_offline_banner;
mod header;
mod inactive_hint;
pub(crate) mod keyboard;
pub(crate) mod live_readout;
mod mapping_issue_copy;
mod mapping_issues;
pub(crate) mod pipeline;
pub(crate) mod undo_log;
mod undo_recap;

pub(crate) use empty_state::EmptyState;
use engine_offline_banner::EngineOfflineBanner;

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
    let editor = use_context::<EditorState>();
    let view = use_context::<ViewState>();
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
            if let Some(message) = editor.error.read().as_ref() {
                div { role: "alert", "{message}" }
            }
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
                    let scope_key = format!("{:?}:{mode}:{input:?}", ctx.meta.read().profile_id);
                    rsx! { content::MappingContent { key: "{scope_key}", name: mapping_name, mapping_key: (mode, input), actions: actions_clone } }
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
    pub error: Signal<Option<String>>,
    /// Prepared per-mapping history labels published by the app controller.
    pub undo_log: Signal<UndoLog>,
    /// Stage IDs that are currently expanded. Resets on selection change
    /// AND on every structural mutation (insert/remove): see Task 11.
    pub expanded_stages: Signal<HashSet<StageId>>,
    /// Right-click menu state (anchor + target stage).
    pub stage_menu: Signal<Option<StageMenuState>>,
    /// Per-stage validation prose surfaced inside the stage body banner
    /// when the user expands the stage. Bodies write on render; the body
    /// renderer reads. Cleared on every structural mutation: see Task 11.
    pub malformed_hints: Signal<HashMap<StageId, String>>,
    /// Per-stage compact tag (<= 24 chars) surfaced in the collapsed-header
    /// summary slot. Distinct from `malformed_hints` so the right-aligned
    /// 12px mono summary stays glanceable while the body banner carries
    /// the full prose. Bodies write a tag whenever they write a hint;
    /// readers fall back to the prose when a tag is absent (transitional
    /// safety net for any stage body that has not adopted the tag yet).
    pub malformed_summary_tags: Signal<HashMap<StageId, String>>,
}

/// Allocate signals and install `EditorState` in context. Call exactly
/// once from `app_root`, the provider self-installs.
pub(crate) fn use_editor_state_provider() -> EditorState {
    let undo_log: Signal<UndoLog> = use_signal(UndoLog::default);
    #[cfg(test)]
    {
        pipeline::stage_body::instruments::stage_dispatch::use_origin();
        let ctx = use_context::<AppContext>();
        let fixture = use_hook(|| {
            try_consume_context::<crate::mapping_edits::Bindings>().unwrap_or_else(|| {
                inputforge_app::mapping_edits::Controller::new(
                    std::sync::Arc::clone(&ctx.state),
                    ctx.commands.clone(),
                )
                .bindings()
            })
        });
        use_context_provider(|| fixture);
        let fallback = crate::frame::use_view_state_provider(ctx.meta);
        let view = use_hook(|| try_consume_context::<ViewState>().unwrap_or(fallback));
        use_context_provider(|| view);
    };
    let bindings = use_context::<crate::mapping_edits::Bindings>();
    let view = use_context::<ViewState>();
    use_hook(|| {
        bindings.install.call(crate::mapping_edits::Publication {
            history: undo_log,
            mode: view.editing_mode,
            selection: view.selected_mapping,
        });
    });
    use_drop(move || bindings.dispose.call(()));
    use_effect(move || {
        let _ = view.editing_mode.read();
        let _ = view.selected_mapping.read();
        bindings.reconcile.call(());
    });
    let expanded_stages: Signal<HashSet<StageId>> = use_signal(HashSet::new);
    let stage_menu: Signal<Option<StageMenuState>> = use_signal(|| None);
    let malformed_hints: Signal<HashMap<StageId, String>> = use_signal(HashMap::new);
    let malformed_summary_tags: Signal<HashMap<StageId, String>> = use_signal(HashMap::new);

    let error = use_signal(|| None);
    let state = EditorState {
        error,
        undo_log,
        expanded_stages,
        stage_menu,
        malformed_hints,
        malformed_summary_tags,
    };
    use_context_provider(|| state);
    state
}

#[cfg(test)]
pub(crate) mod test_frame;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_helpers;
