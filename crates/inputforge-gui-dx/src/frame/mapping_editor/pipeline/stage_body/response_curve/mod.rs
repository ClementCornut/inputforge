// Rust guideline compliant 2026-05-02

//! F10 response-curve body. See spec
//! `docs/superpowers/specs/2026-05-01-f10-curve-editor-design.md`.

#![allow(
    dead_code,
    reason = "submodules expose APIs consumed across F10 tasks; clippy's \
              reachability check loses some pub(crate) items here."
)]

pub(crate) mod interaction;
pub(crate) mod keyboard;
pub(crate) mod mutation;
pub(crate) mod rendering;
pub(crate) mod state;
pub(crate) mod thumbnail;
pub(crate) mod toolbar;

#[cfg(test)]
mod tests;

/// Curve interpolation variant. Mirrors the engine's `ResponseCurve` discriminant
/// but is owned by the GUI layer so the toolbar can operate independently of the
/// engine type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurveType {
    /// Piecewise-linear interpolation between control points.
    PiecewiseLinear,
    /// Catmull-Rom cubic-spline interpolation through control points.
    CubicSpline,
    /// Cubic Bezier segments with explicit handle points.
    CubicBezier,
}

impl CurveType {
    /// Short human-readable label used in the type-selector toolbar.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::PiecewiseLinear => "Linear",
            Self::CubicSpline => "Spline",
            Self::CubicBezier => "Bezier",
        }
    }
}

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::curves::{ResponseCurve, sample_curve_path};

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::at_path;
use crate::frame::mapping_editor::pipeline::stage::stage_summary_for;
use crate::frame::mapping_editor::undo_log::StageId;

use self::state::{BodyState, extract_anchors};

/// Number of polyline vertices sampled from the curve for the SVG plot.
///
/// 200 points gives sub-pixel fidelity at up to 4K display densities
/// for a 240 px plot. Raising this increases SSR output size linearly;
/// lowering it introduces visible jaggedness on cubic variants.
const CURVE_SAMPLE_COUNT: usize = 200;

// `RESPONSE_CURVE_CSS` is registered centrally in
// `crates/inputforge-gui-dx/src/theme/mod.rs` alongside the other frame
// stylesheets. Do NOT declare a per-component `Asset` here, and do NOT
// mount `Stylesheet { ... }` in this body's `rsx!`. The theme module is
// the single owner of `<link rel="stylesheet">` mounts.

/// Project the curve stored at `stage_id` from the current root `actions`.
///
/// Falls back to `fallback` when projection fails (e.g., transient mid-edit
/// state before the dispatcher writes the new action tree). Extracted so that
/// Tasks 12, 13, and 14 can share the same projection logic.
fn project_stage_curve(
    actions: &[Action],
    stage_id: &StageId,
    fallback: &ResponseCurve,
) -> ResponseCurve {
    match at_path(actions, stage_id) {
        Some(Action::ResponseCurve { curve }) => curve.clone(),
        _ => fallback.clone(),
    }
}

/// Body component for a `ResponseCurve` pipeline stage.
///
/// Renders the type-selector toolbar and the SVG plot. Pointer and keyboard
/// interaction is wired in Tasks 12 and 13 respectively; live-input tracking
/// in Task 14. This scaffolding task (Task 11) renders a static, correct plot.
///
/// The `curve` and `root_actions` props are first-render seeds only. The live
/// source of truth is `ConfigSnapshot.selected_mapping_actions` read from the
/// `AppContext` signal so that undo replay, external edits, and sibling-stage
/// mutations all propagate to this component without a prop change.
#[component]
pub(crate) fn ResponseCurveBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    curve: ResponseCurve,
    /// Outermost actions vec for the mapping, threaded by F9's `StageBody`.
    /// Used as the initial-render seed; the live source is
    /// `ConfigSnapshot.selected_mapping_actions` from context.
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let config_signal = ctx.config;
    // Seed the cache immediately from the prop so the first SSR render
    // already contains the correct path and anchor data. The `use_effect`
    // below will overwrite these with the live-projection values on the
    // first reactive tick (after mount), keeping everything in sync.
    let curve_for_seed = curve.clone();
    let mut body: Signal<BodyState> = use_signal(move || BodyState {
        cached_path: sample_curve_path(&curve_for_seed, CURVE_SAMPLE_COUNT),
        cached_anchors: extract_anchors(&curve_for_seed),
        cache_dirty: false,
        ..BodyState::default()
    });

    // Reactivity: read the config signal inside the effect closure so any
    // change to `selected_mapping_actions` (own dispatch, undo replay, or
    // external edit) re-fires this effect and keeps the cached path and
    // anchors in sync with the live action tree.
    //
    // `selected_mapping_actions` is `Option<Vec<Action>>`; unwrap to `&[]`
    // when absent (transient window between mapping selection and config push).
    let curve_seed = curve.clone();
    let stage_id_for_effect = stage_id.clone();
    use_effect(move || {
        let cfg = config_signal.read();
        let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
        let live_curve = project_stage_curve(actions, &stage_id_for_effect, &curve_seed);
        let path = sample_curve_path(&live_curve, CURVE_SAMPLE_COUNT);
        let anchors = extract_anchors(&live_curve);
        body.with_mut(|b| {
            b.cached_path = path;
            b.cached_anchors = anchors;
            b.cache_dirty = false;
            // Clamp focused index to the new anchor count so stale focus
            // from a previous curve does not index out of bounds.
            if let Some(idx) = b.focused_point {
                if idx >= b.cached_anchors.len() {
                    b.focused_point = if b.cached_anchors.is_empty() {
                        None
                    } else {
                        Some(b.cached_anchors.len() - 1)
                    };
                }
            }
        });
    });

    // Re-project on each render so the toolbar and plot see the freshest
    // live data. Clone the snapshot to drop the read guard before the
    // second read that feeds `stage_summary_for`.
    let cfg = config_signal.read().clone();
    let live_actions = cfg
        .selected_mapping_actions
        .clone()
        .unwrap_or_else(|| root_actions.clone());
    let live_curve = project_stage_curve(&live_actions, &stage_id, &curve);

    let body_read = body.read();
    // Reuse F9's existing summary formatter ("Linear · 5 pts · sym" style).
    let summary = stage_summary_for(
        &Action::ResponseCurve {
            curve: live_curve.clone(),
        },
        &cfg,
    );

    rsx! {
        div { class: "if-curve",
            "data-summary": "{summary}",
            toolbar::Toolbar {
                curve: live_curve.clone(),
                stage_id: stage_id.clone(),
                root_actions: live_actions.clone(),
                mapping_key: mapping_key.clone(),
            }
            // `live_value` is `None` at this scaffolding step; Task 14
            // wires the live tracking dot.
            { rendering::render_plot(&live_curve, &body_read, None, 240.0) }
        }
    }
}
