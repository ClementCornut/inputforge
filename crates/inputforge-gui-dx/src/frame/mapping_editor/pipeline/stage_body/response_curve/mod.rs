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
use inputforge_core::pipeline::evaluate_actions_through;
use inputforge_core::processing::curves::{ResponseCurve, sample_curve_path};
use inputforge_core::types::InputValue;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::at_path;
use crate::frame::mapping_editor::pipeline::stage::stage_summary_for;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

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

/// Build a stable DOM id for a `ResponseCurveBody`'s plot wrapper.
///
/// `MountedData::get_client_rect()` does not return a working rect on the
/// Dioxus 0.7 desktop target (the same limitation noted in
/// `top_bar/mode_tabs/mod.rs:179`). `on_mounted` instead calls
/// `document::eval` with `getBoundingClientRect()`, so the wrapper div needs
/// a unique id selectable from JS. The id is derived from `stage_id` so each
/// curve body on screen queries its own rect even when multiple stages of
/// the same kind are mounted (e.g. a top-level curve plus one inside a
/// Conditional branch).
fn stage_id_dom_id(stage_id: &StageId) -> String {
    use std::fmt::Write as _;
    let mut s = String::from("if-curve-plot");
    for seg in &stage_id.0 {
        match seg {
            StageIdSegment::Index(n) => {
                let _ = write!(s, "-i{n}");
            }
            StageIdSegment::IfTrue => s.push_str("-t"),
            StageIdSegment::IfFalse => s.push_str("-f"),
        }
    }
    s
}

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

/// Project the live axis reading for a top-level stage through the pipeline
/// and return the input value as `Some(f64)`, or `None` when any gate fails.
///
/// # Gates
///
/// 1. `stage_id` must be exactly `[Index(n)]`. Nested stages are gated out
///    because `evaluate_actions_through` walks only the root action list; a
///    nested stage would require a sub-slice that is not yet threaded here.
/// 2. `addr` must be `InputAddress::Bound` (has a real device). Unbound
///    mappings have no device to read from.
/// 3. The device must be in `state.devices` with `connected: true`.
/// 4. The evaluated `InputValue` must be `Axis`. Button and Hat inputs are
///    not projected because `ResponseCurve` stages operate on scalar values.
///
/// `ctx.live` is read (not peeked) to subscribe the calling component to the
/// engine's ~60 Hz polling tick, ensuring the dot re-renders on every poll.
fn compute_live_value(
    stage_id: &StageId,
    addr: &inputforge_core::types::InputAddress,
    ctx: &AppContext,
    actions: &[Action],
) -> Option<f64> {
    // Gate 1: top-level only.
    let stop_at = match stage_id.0.as_slice() {
        [StageIdSegment::Index(n)] => *n,
        _ => return None,
    };
    // Gate 2: bound input only.
    let device_id = addr.device()?;
    // Subscribe to the ~60 Hz polling tick. The actual values come from
    // `ctx.state`, not from `ctx.live`; this read is solely for reactivity.
    let _ = ctx.live.read();
    let state_guard = ctx.state.try_read()?;
    // Gate 3: device must be connected.
    let device_present = state_guard
        .devices
        .iter()
        .any(|d| &d.info.id == device_id && d.connected);
    if !device_present {
        return None;
    }
    let value = evaluate_actions_through(actions, &state_guard, addr, stop_at);
    drop(state_guard);
    // Gate 4: axis inputs only.
    match value {
        InputValue::Axis { value, .. } => Some(value.value()),
        _ => None,
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
    let mut malformed_hints = editor.malformed_hints;

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
    let mut working_curve: Signal<Option<ResponseCurve>> = use_signal(|| None);

    // Captured once per mount so that `now_ms` inside `on_key` can be computed
    // as `Instant::now().saturating_duration_since(*time_baseline.peek())`. The
    // `web_time` crate is NOT used; `Instant::EPOCH` does not exist on either
    // `std::time::Instant` or `web_time::Instant`.
    let time_baseline = use_signal(std::time::Instant::now);

    // Cached bounding rect of the plot wrapper div, populated asynchronously
    // after mount. The first pointer event that fires before the rect is ready
    // is a silent no-op; subsequent events use the cached value.
    let mut plot_rect: Signal<Option<interaction::PlotRect>> = use_signal(|| None);

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
    let live_value: Option<f64> =
        compute_live_value(&stage_id, &mapping_key.1, &ctx, &live_actions);

    // Pre-clone captures needed by event-handler closures. Each closure is a
    // `move` closure invoked on every event; captures must be cloned once here
    // so the component body retains ownership of the originals.
    let mapping_key_for_evt = mapping_key.clone();
    let stage_id_for_evt = stage_id.clone();
    let cmd_tx = ctx.commands.clone();

    // `onmounted` fires after the wrapper div is first inserted into the DOM.
    // `MountedData::get_client_rect()` returns Err on the Dioxus 0.7 desktop
    // target (see `top_bar/mode_tabs/mod.rs:179` for the established pattern).
    // Use `document::eval` with `getBoundingClientRect()` instead. The wrapper
    // div is given a stage-id-derived DOM id so the JS query targets the
    // right element when multiple curve bodies are mounted simultaneously.
    let plot_dom_id = stage_id_dom_id(&stage_id);
    let plot_dom_id_for_mount = plot_dom_id.clone();
    let on_mounted = move |_evt: MountedEvent| {
        let id = plot_dom_id_for_mount.clone();
        spawn(async move {
            let mut handle = document::eval(&format!(
                "var el = document.getElementById('{id}');\n\
                 if (!el) {{ dioxus.send([-1.0, -1.0, -1.0, -1.0]); return; }}\n\
                 var r = el.getBoundingClientRect();\n\
                 el.setAttribute('data-eval-rect', r.left+','+r.top+','+r.width+','+r.height);\n\
                 dioxus.send([r.left, r.top, r.width, r.height]);"
            ));
            if let Ok([x, y, w, h]) = handle.recv::<[f64; 4]>().await {
                if w > 0.0 && h > 0.0 {
                    plot_rect.set(Some(interaction::PlotRect {
                        x,
                        y,
                        // Use the smaller dimension so anchor hit-zones are not
                        // stretched if the wrapper is momentarily non-square
                        // during a resize event.
                        size: w.min(h),
                    }));
                }
            }
        });
    };

    // Helper: project a Dioxus MouseEvent to `(cursor, PlotRect)`.
    //
    // Switched from `PointerData` to `MouseData`: Dioxus 0.7 desktop does not
    // route `pointerdown`/`pointermove`/`pointerup` synthetic events through
    // its delegated dispatcher (the wrapper div's `data-dioxus-id` is
    // registered, but the JS bridge only forwards mouse-family events on the
    // desktop target). Native `pointerdown` still fires at the document level
    // (verified via DOM `addEventListener` probe), but Dioxus's Rust handler
    // never runs. Mouse events work for the desktop / WebView2 target; touch
    // and pen are out of scope for the instrument.
    //
    // Returns `None` while the rect cache is unpopulated (before first mount
    // completes). The closure is a `move` closure; `plot_rect` is captured by
    // copy (Signal is Copy).
    let project_event =
        move |evt: &Event<MouseData>| -> Option<((f64, f64), interaction::PlotRect)> {
            let rect = (*plot_rect.peek())?;
            let cur = evt.client_coordinates();
            Some(((cur.x, cur.y), rect))
        };

    // Helper: project a mouse cursor position to `(cursor, PlotRect)`.
    // Used by on_double_click and on_context_menu which receive MouseEvent.
    let project_mouse =
        move |cur_x: f64, cur_y: f64| -> Option<((f64, f64), interaction::PlotRect)> {
            let rect = (*plot_rect.peek())?;
            Some(((cur_x, cur_y), rect))
        };

    // --- on_pointer_down ---
    // Starts a drag when the cursor is within HIT_RADIUS_PX of an anchor.
    //
    // API note: `set_pointer_capture` is NOT exposed on Dioxus 0.7's
    // `PointerData`. The web-only impl reaches it via `try_as_web_event()` +
    // `web_sys::PointerEvent::set_pointer_capture`, which is unavailable on
    // the desktop target. The wrapper div therefore continues to receive
    // pointermove/pointerup only while the cursor stays inside it, which
    // covers the common drag case.
    let curve_for_down = curve.clone();
    let mapping_key_for_down = mapping_key_for_evt.clone();
    let stage_id_for_down = stage_id_for_evt.clone();
    let config_for_down = config_signal;
    let on_pointer_down = move |evt: MouseEvent| {
        let Some((cursor, rect)) = project_event(&evt) else {
            return;
        };
        let cfg = config_for_down.read();
        let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
        let live = project_stage_curve(actions, &stage_id_for_down, &curve_for_down);
        drop(cfg);
        let prev = body.peek().clone();
        let (next, _new_curve, _changed) =
            interaction::handle_pointer_down(prev, &live, cursor, &rect);
        if next.dragging.is_some() {
            // Initialize the working curve to the current live curve so
            // pointer-up always has a valid value even if the first
            // pointer-move event is missed.
            working_curve.set(Some(live));
        }
        body.set(next);
        // Both variables must be referenced so the closure captures them.
        let _ = &mapping_key_for_down;
    };

    // --- on_pointer_move ---
    // During a drag: applies the drag to a local curve clone, refreshes
    // `cached_path` and `cached_anchors` so the plot redraws immediately with
    // the in-flight geometry, and stores the new curve in `working_curve` for
    // pointer-up to commit. No dispatch occurs during the drag.
    // Outside a drag: updates `hovered_point` for cursor-change CSS.
    let curve_for_move = curve.clone();
    let stage_id_for_move = stage_id_for_evt.clone();
    let config_for_move = config_signal;
    let on_pointer_move = move |evt: MouseEvent| {
        let Some((cursor, rect)) = project_event(&evt) else {
            return;
        };
        let cfg = config_for_move.read();
        let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
        let live = project_stage_curve(actions, &stage_id_for_move, &curve_for_move);
        drop(cfg);
        let prev = body.peek().clone();
        let (mut next, new_curve_opt, _changed) =
            interaction::handle_pointer_move(prev, &live, cursor, &rect);
        if let Some(new_curve) = new_curve_opt {
            // Drag branch: refresh cached geometry so the SVG repaints
            // with the dragged anchor position before the next commit.
            let new_path = sample_curve_path(&new_curve, CURVE_SAMPLE_COUNT);
            let new_anchors = extract_anchors(&new_curve);
            next.cached_path = new_path;
            next.cached_anchors = new_anchors;
            next.cache_dirty = false;
            working_curve.set(Some(new_curve));
        }
        body.set(next);
    };

    // --- on_pointer_up ---
    // Commits the drag: validates the working curve, dispatches `SetMapping`
    // on success, or writes the validator error to `malformed_hints` on
    // failure. Always resets `working_curve` to `None` and clears the drag
    // state on `body`.
    let curve_for_up = curve.clone();
    let mapping_key_for_up = mapping_key_for_evt.clone();
    let stage_id_for_up = stage_id_for_evt.clone();
    let config_for_up = config_signal;
    let cmd_tx_for_up = cmd_tx.clone();
    let on_pointer_up = move |_evt: MouseEvent| {
        let prev = body.peek().clone();
        // If no drag was active, early-exit (stray pointer-up from outside a drag).
        if prev.dragging.is_none() {
            return;
        }
        // The committed geometry is whatever the last pointer-move stored.
        // Fall back to the live curve if working_curve was never updated
        // (pointer-down then immediate pointer-up without any move).
        let cfg = config_for_up.read();
        let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
        let live = project_stage_curve(actions, &stage_id_for_up, &curve_for_up);
        let actions_snap = actions.to_vec();
        drop(cfg);
        let dragged = working_curve.peek().clone().unwrap_or_else(|| live.clone());
        working_curve.set(None);

        let (mut next, result, _changed) = interaction::handle_pointer_up(prev, &dragged);
        match result {
            Ok(valid_curve) => {
                // Successful commit: dispatch and clear any stale hint.
                let cfg2 = config_for_up.read();
                let name = cfg2.mapping_names.get(&mapping_key_for_up.1).cloned();
                drop(cfg2);
                toolbar::dispatch_curve_edit(
                    &actions_snap,
                    &stage_id_for_up,
                    valid_curve,
                    &mapping_key_for_up,
                    name,
                    &cmd_tx_for_up,
                    &mut undo_log,
                    "curve: drag".to_owned(),
                );
                malformed_hints.write().remove(&stage_id_for_up);
            }
            Err(err) if err.is_empty() => {
                // Sentinel from `handle_pointer_up` when no drag was active
                // (belt-and-suspenders guard; the early-return above already
                // handles this case before we call the handler).
            }
            Err(err) => {
                // Validation failed. Write the error and skip dispatch.
                // `next.pre_drag_curve` is still populated per Task 6's
                // contract; take it to prevent it from leaking into the next
                // drag cycle. The component re-renders from `config` (which
                // still holds the pre-drag value because no dispatch landed),
                // so no explicit curve-signal restoration is needed.
                let _revert = next.pre_drag_curve.take();
                malformed_hints.write().insert(stage_id_for_up.clone(), err);
            }
        }
        body.set(next);
    };

    // --- on_double_click ---
    // Adds a new control point at the clicked viewBox coordinate and dispatches.
    let curve_for_dc = curve.clone();
    let mapping_key_for_dc = mapping_key_for_evt.clone();
    let stage_id_for_dc = stage_id_for_evt.clone();
    let config_for_dc = config_signal;
    let cmd_tx_for_dc = cmd_tx.clone();
    let on_double_click = move |evt: MouseEvent| {
        let cur = evt.client_coordinates();
        let Some((cursor, rect)) = project_mouse(cur.x, cur.y) else {
            return;
        };
        let cfg = config_for_dc.read();
        let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
        let live = project_stage_curve(actions, &stage_id_for_dc, &curve_for_dc);
        let actions_snap = actions.to_vec();
        drop(cfg);
        let prev = body.peek().clone();
        let (mut next, new_curve_opt, changed) =
            interaction::handle_double_click(prev, &live, cursor, &rect);
        if changed {
            if let Some(new_curve) = new_curve_opt {
                // Refresh cached geometry eagerly so the plot repaints with
                // the new anchor before the config round-trip completes.
                next.cached_path = sample_curve_path(&new_curve, CURVE_SAMPLE_COUNT);
                next.cached_anchors = extract_anchors(&new_curve);
                next.cache_dirty = false;
                match mutation::reconstruct_curve(&new_curve) {
                    Ok(valid_curve) => {
                        let vb = interaction::screen_to_viewbox(cursor, &rect);
                        let label = format!("curve: add point at ({:.2}, {:.2})", vb.0, vb.1);
                        let cfg2 = config_for_dc.read();
                        let name = cfg2.mapping_names.get(&mapping_key_for_dc.1).cloned();
                        drop(cfg2);
                        toolbar::dispatch_curve_edit(
                            &actions_snap,
                            &stage_id_for_dc,
                            valid_curve,
                            &mapping_key_for_dc,
                            name,
                            &cmd_tx_for_dc,
                            &mut undo_log,
                            label,
                        );
                        malformed_hints.write().remove(&stage_id_for_dc);
                    }
                    Err(err) => {
                        malformed_hints.write().insert(stage_id_for_dc.clone(), err);
                    }
                }
            }
        }
        body.set(next);
    };

    // --- on_context_menu ---
    // Removes the currently hovered control point and dispatches.
    //
    // `evt.prevent_default()` suppresses the OS/browser context menu.
    // On Dioxus 0.7 `prevent_default()` lives directly on `Event<T>`
    // (dioxus-core-0.7.6/src/events.rs:172), so it works on any event type
    // without an inner data accessor.
    let curve_for_cm = curve.clone();
    let mapping_key_for_cm = mapping_key_for_evt.clone();
    let stage_id_for_cm = stage_id_for_evt.clone();
    let config_for_cm = config_signal;
    let cmd_tx_for_cm = cmd_tx.clone();
    let on_context_menu = move |evt: MouseEvent| {
        evt.prevent_default();
        let cfg = config_for_cm.read();
        let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
        let live = project_stage_curve(actions, &stage_id_for_cm, &curve_for_cm);
        let actions_snap = actions.to_vec();
        drop(cfg);
        let prev = body.peek().clone();
        let (mut next, new_curve_opt, changed) = interaction::handle_context_menu(prev, &live);
        if changed {
            if let Some(new_curve) = new_curve_opt {
                // Refresh cached geometry.
                next.cached_path = sample_curve_path(&new_curve, CURVE_SAMPLE_COUNT);
                next.cached_anchors = extract_anchors(&new_curve);
                next.cache_dirty = false;
                match mutation::reconstruct_curve(&new_curve) {
                    Ok(valid_curve) => {
                        let cfg2 = config_for_cm.read();
                        let name = cfg2.mapping_names.get(&mapping_key_for_cm.1).cloned();
                        drop(cfg2);
                        toolbar::dispatch_curve_edit(
                            &actions_snap,
                            &stage_id_for_cm,
                            valid_curve,
                            &mapping_key_for_cm,
                            name,
                            &cmd_tx_for_cm,
                            &mut undo_log,
                            "curve: remove point".to_owned(),
                        );
                        malformed_hints.write().remove(&stage_id_for_cm);
                    }
                    Err(err) => {
                        malformed_hints.write().insert(stage_id_for_cm.clone(), err);
                    }
                }
            }
        }
        body.set(next);
    };

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
        // `as_millis()` returns `u128`; convert to `u64` with saturation.
        // An overflow (component alive for >584 million years) saturates to
        // `u64::MAX`, which won't match the 250 ms coalesce window, so it
        // correctly pushes a new undo entry rather than merging.
        let now_ms: u64 = u64::try_from(
            std::time::Instant::now()
                .saturating_duration_since(*time_baseline.peek())
                .as_millis(),
        )
        .unwrap_or(u64::MAX);

        // Re-project curve and root actions from the live config so the handler
        // sees the freshest state (no stale prop closures). Drop the read guard
        // before dispatch_curve_edit acquires its own write on undo_log.
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
                toolbar::dispatch_curve_edit(
                    &actions,
                    &stage_id_for_key,
                    new,
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
                toolbar::dispatch_curve_edit_no_undo(
                    &actions,
                    &stage_id_for_key,
                    new,
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
            s.last_nudge_at_ms = None;
            s.last_nudge_key = None;
        });
    };

    // Derive data-attribute values from the current body snapshot. These drive
    // CSS cursor rules (grab cursor during drag, pointer cursor on hover).
    let body_snapshot = body.read();
    let dragging_attr = body_snapshot.dragging.is_some().to_string();
    let hovered_attr = body_snapshot.hovered_point.is_some().to_string();
    drop(body_snapshot);
    let rect_attr = plot_rect.read().is_some().to_string();

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
            // Focusable wrapper div owns the aria-label and all pointer
            // events. The inner <svg> emits a <title> for screen readers that
            // descend into SVG by default; the outer div's aria-label is the
            // primary announcement when the user tabs in.
            div {
                class: "if-curve__plot-frame",
                id: "{plot_dom_id}",
                tabindex: "0",
                "aria-label": "response curve",
                "data-hovered": "{hovered_attr}",
                "data-dragging": "{dragging_attr}",
                "data-rect-set": "{rect_attr}",
                onmousedown: on_pointer_down,
                onmousemove: on_pointer_move,
                onmouseup: on_pointer_up,
                ondoubleclick: on_double_click,
                oncontextmenu: on_context_menu,
                onmounted: on_mounted,
                onkeydown: on_key,
                onfocusout: on_focus_out,
                { rendering::render_plot(&live_curve, &body.read(), live_value, 240.0) }
            }
        }
    }
}
