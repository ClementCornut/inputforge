// Rust guideline compliant 2026-05-02

//! F10 response-curve body. See spec
//! `docs/superpowers/specs/2026-05-01-f10-curve-editor-design.md`.

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
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::at_path;
use crate::frame::mapping_editor::pipeline::stage::stage_summary_for;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::bridge::{
    BridgeEvent, mount_mouse_bridge, stage_id_dom_id,
};
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::{
    dispatch_stage_edit, dispatch_stage_edit_no_undo,
};
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

/// Single-message dispatcher invoked by the eval loop in `on_mounted`. Routes
/// each event kind through the existing pure handlers in `interaction.rs` and
/// updates the body component's signals in place. All Signal<T> arguments are
/// captured by `Copy`; the heavier values (`mapping_key`, `stage_id`, etc.) are
/// borrowed because they live in the spawn task's stack across iterations.
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Body component shape; one big match arm per event kind keeps each handler local."
)]
fn dispatch_bridge_event(
    payload: &BridgeEvent,
    mut body: Signal<BodyState>,
    mut working_curve: Signal<Option<ResponseCurve>>,
    config_signal: Signal<crate::context::ConfigSnapshot>,
    mut undo_log: Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
    mut malformed_hints: Signal<std::collections::HashMap<StageId, String>>,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    curve_seed: &ResponseCurve,
    cmd_tx: &std::sync::mpsc::Sender<inputforge_core::engine::EngineCommand>,
) {
    // Build a fresh PlotRect from the live rect carried by every event payload.
    // The rect is captured by the JS bridge per-event via getBoundingClientRect,
    // so it stays accurate across page scroll, toolbar layout shifts, and any
    // other reflow that would invalidate a mount-time cache.
    if payload.rs <= 0.0 {
        return;
    }
    let rect = interaction::PlotRect {
        x: payload.rl,
        y: payload.rt,
        size: payload.rs,
    };

    match payload.kind.as_str() {
        "down" => {
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_curve(actions, stage_id, curve_seed);
            drop(cfg);
            let prev = body.peek().clone();
            let (next, _, _) =
                interaction::handle_pointer_down(prev, &live, (payload.x, payload.y), &rect);
            if next.dragging.is_some() {
                working_curve.set(Some(live));
            }
            body.set(next);
        }
        "move" => {
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_curve(actions, stage_id, curve_seed);
            drop(cfg);
            let prev = body.peek().clone();
            let (mut next, new_curve_opt, _) =
                interaction::handle_pointer_move(prev, &live, (payload.x, payload.y), &rect);
            if let Some(new_curve) = new_curve_opt {
                next.cached_path = sample_curve_path(&new_curve, CURVE_SAMPLE_COUNT);
                next.cached_anchors = extract_anchors(&new_curve);
                next.cache_dirty = false;
                working_curve.set(Some(new_curve));
            }
            body.set(next);
        }
        "up" => {
            let prev = body.peek().clone();
            if prev.dragging.is_none() {
                return;
            }
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_curve(actions, stage_id, curve_seed);
            let actions_snap = actions.to_vec();
            drop(cfg);
            let dragged = working_curve.peek().clone().unwrap_or_else(|| live.clone());
            working_curve.set(None);
            let (mut next, result, _) = interaction::handle_pointer_up(prev, &dragged);
            // Phantom-undo guard: a mousedown + mouseup with no intervening move
            // means `working_curve` was never updated, so `dragged == live`. Without
            // this guard the body would dispatch a no-op `SetMapping` and record a
            // `"curve: drag"` undo entry for a user gesture that did not change
            // anything. F11 has the same guard in `deadzone/mod.rs` (Task 14).
            if dragged == live {
                body.set(next);
                return;
            }
            match result {
                Ok(valid) => {
                    let cfg2 = config_signal.read();
                    let name = cfg2.mapping_names.get(&mapping_key.1).cloned();
                    drop(cfg2);
                    dispatch_stage_edit(
                        &actions_snap,
                        stage_id,
                        Action::ResponseCurve { curve: valid },
                        mapping_key,
                        name,
                        cmd_tx,
                        &mut undo_log,
                        "curve: drag".to_owned(),
                    );
                    malformed_hints.write().remove(stage_id);
                }
                Err(err) if err.is_empty() => {}
                Err(err) => {
                    let _revert = next.pre_drag_curve.take();
                    malformed_hints.write().insert(stage_id.clone(), err);
                }
            }
            body.set(next);
        }
        "dbl" => {
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_curve(actions, stage_id, curve_seed);
            let actions_snap = actions.to_vec();
            drop(cfg);
            let prev = body.peek().clone();
            let (mut next, new_curve_opt, changed) =
                interaction::handle_double_click(prev, &live, (payload.x, payload.y), &rect);
            if changed {
                if let Some(new_curve) = new_curve_opt {
                    next.cached_path = sample_curve_path(&new_curve, CURVE_SAMPLE_COUNT);
                    next.cached_anchors = extract_anchors(&new_curve);
                    next.cache_dirty = false;
                    match mutation::reconstruct_curve(&new_curve) {
                        Ok(valid) => {
                            // `rect` was already gated on `payload.rs > 0.0` at
                            // the top of dispatch_bridge_event, so this branch
                            // is reached only with a measured rect; fall back
                            // to a coordless label if that ever changes.
                            let label =
                                match interaction::screen_to_viewbox((payload.x, payload.y), &rect)
                                {
                                    Some(vb) => {
                                        format!("curve: add point at ({:.2}, {:.2})", vb.0, vb.1)
                                    }
                                    None => "curve: add point".to_owned(),
                                };
                            let cfg2 = config_signal.read();
                            let name = cfg2.mapping_names.get(&mapping_key.1).cloned();
                            drop(cfg2);
                            dispatch_stage_edit(
                                &actions_snap,
                                stage_id,
                                Action::ResponseCurve { curve: valid },
                                mapping_key,
                                name,
                                cmd_tx,
                                &mut undo_log,
                                label,
                            );
                            malformed_hints.write().remove(stage_id);
                        }
                        Err(err) => {
                            malformed_hints.write().insert(stage_id.clone(), err);
                        }
                    }
                }
            }
            body.set(next);
        }
        "ctx" => {
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_curve(actions, stage_id, curve_seed);
            let actions_snap = actions.to_vec();
            drop(cfg);
            let prev = body.peek().clone();
            let (mut next, new_curve_opt, changed) = interaction::handle_context_menu(prev, &live);
            if changed {
                if let Some(new_curve) = new_curve_opt {
                    next.cached_path = sample_curve_path(&new_curve, CURVE_SAMPLE_COUNT);
                    next.cached_anchors = extract_anchors(&new_curve);
                    next.cache_dirty = false;
                    match mutation::reconstruct_curve(&new_curve) {
                        Ok(valid) => {
                            let cfg2 = config_signal.read();
                            let name = cfg2.mapping_names.get(&mapping_key.1).cloned();
                            drop(cfg2);
                            dispatch_stage_edit(
                                &actions_snap,
                                stage_id,
                                Action::ResponseCurve { curve: valid },
                                mapping_key,
                                name,
                                cmd_tx,
                                &mut undo_log,
                                "curve: remove point".to_owned(),
                            );
                            malformed_hints.write().remove(stage_id);
                        }
                        Err(err) => {
                            malformed_hints.write().insert(stage_id.clone(), err);
                        }
                    }
                }
            }
            body.set(next);
        }
        _ => {}
    }
}

/// Body component for a `ResponseCurve` pipeline stage.
///
/// Renders the type-selector toolbar and the SVG plot with full pointer
/// interaction: drag anchors, double-click to add a point, right-click to
/// remove the hovered point. Dispatches `SetMapping` and pushes undo entries
/// on commit-points (drag end, double-click add, right-click remove).
///
/// The `curve` and `root_actions` props are first-render seeds only. The live
/// source of truth is `ConfigSnapshot.selected_mapping_actions` read from the
/// `AppContext` signal so that undo replay, external edits, and sibling-stage
/// mutations all propagate to this component without a prop change.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
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
    let editor = use_context::<EditorState>();
    let mut undo_log = editor.undo_log;
    let malformed_hints = editor.malformed_hints;

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

    // `working_curve` tracks the in-flight drag curve so `on_pointer_up` can
    // commit the final dragged position. It is `None` when no drag is active
    // and `Some(c)` during a drag (updated on every `on_pointer_move` that
    // returns a new local curve). Reset to `None` on pointer-up (both success
    // and failure paths).
    let working_curve: Signal<Option<ResponseCurve>> = use_signal(|| None);

    // Captured once per mount so that `now_ms` inside `on_key` can be computed
    // as `Instant::now().saturating_duration_since(*time_baseline.peek())`. The
    // `web_time` crate is NOT used; `Instant::EPOCH` does not exist on either
    // `std::time::Instant` or `web_time::Instant`.
    let time_baseline = use_signal(std::time::Instant::now);

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

    // --- Live tracking dot projection ---
    //
    // Two reads, two roles: `ctx.live` is a `Signal<LiveSnapshot>` updated
    // at the engine's polling tick (~60 Hz); reading it subscribes the body
    // to that tick so the live dot re-renders on every poll. The actual input
    // and output values come from `ctx.state` (the engine's authoritative
    // `AppState`), evaluated through the same actions chain the engine uses,
    // so the dot tracks the curve exactly.
    //
    // Gates (all must pass to produce `Some(f64)`):
    //   1. `stage_id` must be exactly `[Index(n)]` (top-level only).
    //   2. `mapping_key.1` must be `InputAddress::Bound` (has a real device).
    //   3. The device must be present in `state.devices` with `connected: true`.
    //   4. The evaluated `InputValue` must be `Axis` (non-axis inputs yield `None`).
    let live_value: Option<f64> = crate::frame::mapping_editor::pipeline::stage_body::instruments::live_axis::compute_live_axis_value(
        &stage_id, &mapping_key.1, &ctx, &live_actions,
    );

    // Pre-clone captures needed by event-handler closures. Each closure is a
    // `move` closure invoked on every event; captures must be cloned once here
    // so the component body retains ownership of the originals.
    let mapping_key_for_evt = mapping_key.clone();
    let stage_id_for_evt = stage_id.clone();
    let cmd_tx = ctx.commands.clone();

    // ----- JS-bridge mouse-event dispatch -----
    //
    // Background: Dioxus 0.7.6 desktop's delegated event dispatcher does NOT
    // route mousedown / mousemove / mouseup / dblclick / contextmenu to handlers
    // on a non-button `<div>`, even when the wrapper has `data-dioxus-id`
    // registered. Native browser events fire (verified with document-level JS
    // probes) but the Rust handler never runs. The most plausible upstream
    // cause is the float-vs-integer payload deserialization bug tracked in
    // Dioxus issue #4706 (Windows / WebView2 emits floating-point coords where
    // the deserializer expects integers, silently dropping the event). `onclick`
    // on `<button>` works because the button's payload format is simpler and
    // round-trips correctly; complex mouse payloads do not.
    //
    // Workaround: install raw JS event listeners at the document level via
    // `document::eval`, capture coords as integers (`| 0`), and stream them
    // back through the eval `Channel` to a Rust dispatcher closure that calls
    // the existing pure handlers in `interaction.rs`. Bypasses the broken
    // delegator entirely. `onkeydown` / `onfocusout` continue to work via the
    // normal Dioxus path and remain on the wrapper div. The shared
    // `mount_mouse_bridge` infrastructure (JS install, listener cleanup, event
    // parse, rect projection) lives in `instruments::bridge`; the per-editor
    // dispatch closure passed below is the F10-shaped portion that owns the
    // signals and calls `dispatch_bridge_event`.
    let plot_dom_id = stage_id_dom_id("if-curve-plot", &stage_id);
    let curve_for_bridge = curve.clone();
    let mapping_key_for_bridge = mapping_key_for_evt.clone();
    let stage_id_for_bridge = stage_id_for_evt.clone();
    let cmd_tx_for_bridge = cmd_tx.clone();
    let dispatch = move |payload: BridgeEvent| {
        dispatch_bridge_event(
            &payload,
            body,
            working_curve,
            config_signal,
            undo_log,
            malformed_hints,
            &mapping_key_for_bridge,
            &stage_id_for_bridge,
            &curve_for_bridge,
            &cmd_tx_for_bridge,
        );
    };
    let on_mounted = mount_mouse_bridge(plot_dom_id.clone(), dispatch);

    // --- on_key ---
    // Routes a normalized `KeyInput` through `keyboard::handle_key`, updates
    // body state, and dispatches the resulting curve edit (if any) to the
    // engine. Tab/ShiftTab do NOT call `prevent_default()` so the browser can
    // advance focus past the plot at the list boundary; all other handled keys
    // consume the event.
    let mapping_key_for_key = mapping_key.clone();
    let stage_id_for_key = stage_id.clone();
    let cmd_tx_for_key = cmd_tx.clone();
    let on_key = move |evt: KeyboardEvent| {
        let key = match (evt.key(), evt.modifiers().shift()) {
            (Key::Tab, true) => keyboard::KeyInput::ShiftTab,
            (Key::Tab, false) => keyboard::KeyInput::Tab,
            (Key::ArrowLeft, shift) => keyboard::KeyInput::ArrowLeft { shift },
            (Key::ArrowRight, shift) => keyboard::KeyInput::ArrowRight { shift },
            (Key::ArrowUp, shift) => keyboard::KeyInput::ArrowUp { shift },
            (Key::ArrowDown, shift) => keyboard::KeyInput::ArrowDown { shift },
            (Key::Home, _) => keyboard::KeyInput::Home,
            (Key::End, _) => keyboard::KeyInput::End,
            (Key::Enter, _) => keyboard::KeyInput::Enter,
            (Key::Delete | Key::Backspace, _) => keyboard::KeyInput::Delete,
            (Key::Escape, _) => keyboard::KeyInput::Escape,
            _ => return,
        };

        // Tab/ShiftTab: do NOT prevent default. The browser handles focus
        // wrap when the user reaches the end of the anchor list (the outer
        // page should advance focus past the plot). All other keys are
        // consumed locally.
        if !matches!(key, keyboard::KeyInput::Tab | keyboard::KeyInput::ShiftTab) {
            evt.prevent_default();
        }

        // `now_ms` is the milliseconds since component mount. Using
        // `std::time::Instant` directly mirrors `live_capture`. There is no
        // `Instant::EPOCH` on either std or web_time, so we use a baseline
        // captured once at mount and compute elapsed time from it.
        //
        // `as_millis()` is u128; cast to u64 saturates the unreachable
        // >584-million-year arm. Even on wraparound the value would not
        // collide with the 250 ms coalesce window for any real session.
        #[allow(clippy::cast_possible_truncation, reason = "see comment")]
        let now_ms = std::time::Instant::now()
            .saturating_duration_since(*time_baseline.peek())
            .as_millis() as u64;

        // Re-project curve and root actions from the live config so the handler
        // sees the freshest state (no stale prop closures). Drop the read guard
        // before dispatch_stage_edit acquires its own write on undo_log.
        let cfg = config_signal.read();
        let actions: Vec<Action> = cfg.selected_mapping_actions.clone().unwrap_or_default();
        let live_curve = project_stage_curve(&actions, &stage_id_for_key, &curve);
        let name = cfg.mapping_names.get(&mapping_key_for_key.1).cloned();
        drop(cfg);

        let (next_state, new_curve, outcome, _changed) =
            keyboard::handle_key(body.peek().clone(), &live_curve, key, now_ms);
        body.set(next_state);
        let Some(new) = new_curve else { return };
        match outcome {
            Some(keyboard::KeyOutcome::PushUndo { label }) => {
                dispatch_stage_edit(
                    &actions,
                    &stage_id_for_key,
                    Action::ResponseCurve { curve: new },
                    &mapping_key_for_key,
                    name,
                    &cmd_tx_for_key,
                    &mut undo_log,
                    label,
                );
            }
            Some(keyboard::KeyOutcome::MergeUndo) => {
                // Same-key burst within 250 ms: dispatch the new curve to the
                // engine but do NOT push a new undo entry. The first nudge of
                // the burst already pushed an entry whose `mapping_before`
                // captures the pre-burst state, so undo restores correctly.
                // Redo replays the first nudge's SetMapping only; accepted as
                // a deliberate UX simplification.
                dispatch_stage_edit_no_undo(
                    &actions,
                    &stage_id_for_key,
                    Action::ResponseCurve { curve: new },
                    &mapping_key_for_key,
                    name,
                    &cmd_tx_for_key,
                );
            }
            None => {
                // Escape revert: body-local only. The drag never dispatched,
                // so the engine state is already correct; no dispatch is
                // needed. (Pointer-up's revert path is analogous.)
            }
        }
    };

    // --- on_focus_out ---
    // Reset the coalesce state when the plot wrapper loses focus so that the
    // next nudge after refocus pushes a fresh undo entry rather than merging
    // into a stale prior burst.
    let mut body_for_focusout = body;
    let on_focus_out = move |_| {
        body_for_focusout.with_mut(|s| {
            s.nudge_coalesce.reset();
        });
    };

    // Derive data-attribute values from the current body snapshot. These drive
    // CSS cursor rules (grab cursor during drag, pointer cursor on hover).
    let body_snapshot = body.read();
    let dragging_attr = body_snapshot.dragging.is_some().to_string();
    let hovered_attr = body_snapshot.hovered_point.is_some().to_string();
    drop(body_snapshot);

    // Reuse F9's existing summary formatter ("Linear 5 pts sym" style).
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
            // Focusable wrapper div owns the aria-label, the keyboard handlers,
            // and the JS-bridge mount point. Mouse-class events
            // (mousedown / mousemove / mouseup / dblclick / contextmenu) are
            // captured at the document level by JS listeners installed inside
            // `on_mounted`'s `document::eval` and routed back to Rust through
            // an event-channel; see the BRIDGE_JS_TEMPLATE / dispatch_bridge_event
            // pair above. The wrapper does NOT register Dioxus rsx attributes
            // for those events because the Dioxus 0.7 desktop dispatcher silently
            // drops them on non-button elements.
            div {
                class: "if-curve__plot-frame",
                id: "{plot_dom_id}",
                tabindex: "0",
                "aria-label": "response curve",
                "data-hovered": "{hovered_attr}",
                "data-dragging": "{dragging_attr}",
                onmounted: on_mounted,
                onkeydown: on_key,
                onfocusout: on_focus_out,
                { rendering::render_plot(&live_curve, &body.read(), live_value, 240.0) }
            }
        }
    }
}
