# F10 Curve Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace F9's placeholder body and chevron for `Action::ResponseCurve` with a Dioxus curve editor that ports the existing egui widget's logic verbatim onto an SVG plot, ships a 28x14 stage-header thumbnail, and adds a toolbar (type / symmetric / reset) plus keyboard a11y, live-input tracking, and undo per F9's dispatcher contract.

**Architecture:** A self-contained submodule `pipeline/stage_body/response_curve/` with three layers: (1) pure logic ported from the archived egui curve editor (see "Reference source" below) plus new pure handlers (interaction, keyboard); (2) SVG rendering helpers reading CSS custom properties; (3) the `ResponseCurveBody` Dioxus component that threads `EditorState` + `ConfigSnapshot` + `LiveSnapshot` and dispatches `EngineCommand::SetMapping` paired with `UndoLog::push_edit`. One new engine helper (`sample_curve_path`) lives in `inputforge_core::processing::curves`. F9 ownership is narrow: F10 modifies F9's `stage_body/mod.rs` dispatcher arms (`Action::ResponseCurve` in `StageBody` and the per-arm header right slot), adds one `mod response_curve;` declaration, and adds a new `aria_label_override: Option<String>` prop on F9's `StageHeader` (`pipeline/stage_header.rs`) so the ResponseCurve arm can override the accessible name on the underlying `<button>`.

**Tech Stack:** Rust, Dioxus 0.7 (rsx!, signals, use_effect, use_context), Dioxus SSR for tests, inline SVG (no canvas, no third-party plot crate), CSS custom properties. Engine port uses `inputforge_core::processing::curves::{ResponseCurve, BezierSegment}`.

**Reference source (port):** the egui crate `crates/inputforge-gui/` was deleted in commit `2271256`. A read-only worktree has been created at `E:\Git\Perso\inputforge-egui-ref` (detached at commit `af44e57`, the parent of the deletion). The five port-source files live at `E:\Git\Perso\inputforge-egui-ref\crates\inputforge-gui\src\widgets\curve_editor\{mod,mutation,symmetry,interaction,rendering}.rs`. Read those files directly when this plan says "see egui mutation.rs:..." or similar. The egui port plotted output on the visual X axis and input on the visual Y axis (see `mod.rs::rebuild_cache`, `mod.rs::extract_control_points`, and `rendering.rs` for the `[output, input]` ordering). The SVG port reverses this convention: input on X, output on Y, with `<g transform="scale(1, -1)">` applied so positive output points up. Engine-native `(input, output)` ordering flows through every pure handler unchanged; the swap that lived in the egui interaction/rendering layer is not needed and is not reintroduced.

**Spec:** `docs/superpowers/specs/2026-05-01-f10-curve-editor-design.md` (read this before each task).

---

## File structure

| File | Responsibility |
|---|---|
| `crates/inputforge-core/src/processing/curves.rs` | Add `sample_curve_path(curve, samples) -> Vec<(f64, f64)>` (engine-native `(input, output)`). Existing functions untouched. |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs` | `ResponseCurveBody` component, `header_thumbnail(curve) -> Element`, glow `<defs>`. The accessible-name string for the header override is built from F9's existing `pipeline::stage::stage_summary_for(action, cfg)` (no new `header_summary` helper). |
| `.../response_curve/state.rs` | `BodyState`, `DragInProgress`, `extract_anchors(curve) -> Vec<(f64, f64)>`. Pure types. |
| `.../response_curve/mutation.rs` | Port of egui `mutation.rs` + `symmetry.rs::apply_symmetry`. `PlotPoint` → `(f64, f64)`. |
| `.../response_curve/interaction.rs` | Pure pointer-event handlers + screen-to-viewBox conversion + nearest-anchor lookup. |
| `.../response_curve/keyboard.rs` | Pure `handle_key`, including same-key 250ms coalesce. |
| `.../response_curve/rendering.rs` | SVG render fns: grid, identity, curve path, anchors, handles, hover/drag/focus rings, live tracking, tick labels. |
| `.../response_curve/thumbnail.rs` | 28x14 inline SVG thumbnail for stage-header right slot. |
| `.../response_curve/toolbar.rs` | F2 `Tabs` (type) + F2 `Switch` (symmetric) + F2 `Button` (reset). |
| `.../response_curve/tests.rs` | SSR mount tests; pure-fn tests live next to their module via `#[cfg(test)] mod tests`. |
| `crates/inputforge-gui-dx/assets/frame/response_curve.css` | `.if-curve*` classes + `.if-curve` token block + reduced-motion rule. |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs` | Modify (after F9 ships this file with placeholder arms): replace `Action::ResponseCurve` arms in `StageBody` and the per-arm header right slot. |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_header.rs` | Modify: F9's `StageHeader` is a plain `<button class="if-stage__header">` (not an `IconButton`) with `aria-expanded` and `aria-controls` only. Add a new `aria_label_override: Option<String>` prop and emit `aria-label="{s}"` on that button when `Some`. The ResponseCurve arm passes `Toggle stage body. Curve: {stage_summary_for(action, cfg)}`. |

**Asset registration note (Task 17):** Stylesheet assets are mounted centrally in `crates/inputforge-gui-dx/src/theme/mod.rs` (see lines 10-44 + 63-98 for the existing pattern). Register `RESPONSE_CURVE_CSS` there alongside the other frame stylesheets, NOT inside `response_curve/mod.rs`.

---

## Constants

Defined once in `response_curve/mod.rs` (or local to consumers):

- `CURVE_SAMPLE_COUNT: usize = 200`
- `THUMBNAIL_SAMPLE_COUNT: usize = 30`
- `HIT_RADIUS_PX: f64 = 10.0`
- `MIN_X_GAP: f64 = 0.001` (mirrors egui-ref `mod.rs:37`; see `E:\Git\Perso\inputforge-egui-ref\crates\inputforge-gui\src\widgets\curve_editor\mod.rs:37`)
- `KEY_NUDGE_STEP: f64 = 0.01`
- `KEY_NUDGE_STEP_LARGE: f64 = 0.10`
- `KEY_COALESCE_WINDOW_MS: u64 = 250`

---

### Task 1: `sample_curve_path` engine helper

Pure helper used by both `rendering.rs` (200 samples) and `thumbnail.rs` (30 samples). Engine-native `(input, output)` tuples; the egui crate's `rebuild_cache` continues to emit `[output, input]` for `egui_plot` until F17 deletes it.

**Files:**
- Modify: `crates/inputforge-core/src/processing/curves.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(test)] mod tests { ... }` block at the bottom of `crates/inputforge-core/src/processing/curves.rs`:

```rust
#[test]
fn sample_curve_path_piecewise_round_trips_identity() {
    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
    let samples = sample_curve_path(&curve, 200);
    assert_eq!(samples.len(), 200);
    let tol = 1e-9;
    assert!((samples[0].0 - (-1.0)).abs() < tol, "first input == -1");
    assert!((samples[0].1 - (-1.0)).abs() < tol, "first output == -1");
    let last = samples[199];
    assert!((last.0 - 1.0).abs() < tol, "last input == 1");
    assert!((last.1 - 1.0).abs() < tol, "last output == 1");
    // Engine-native ordering: tuple is (input, output), NOT (output, input).
    // For identity, midpoint should be ~ (0, 0).
    let mid = samples[100];
    assert!(mid.0.abs() < 0.02 && mid.1.abs() < 0.02);
}

#[test]
fn sample_curve_path_bezier_continuity() {
    let seg_a = BezierSegment {
        start: (-1.0, -1.0),
        control1: (-2.0 / 3.0, -2.0 / 3.0),
        control2: (-1.0 / 3.0, -1.0 / 3.0),
        end: (0.0, 0.0),
    };
    let seg_b = BezierSegment {
        start: (0.0, 0.0),
        control1: (1.0 / 3.0, 1.0 / 3.0),
        control2: (2.0 / 3.0, 2.0 / 3.0),
        end: (1.0, 1.0),
    };
    let curve = ResponseCurve::cubic_bezier(vec![seg_a, seg_b], false).unwrap();
    let samples = sample_curve_path(&curve, 200);
    assert!(samples.len() >= 198 && samples.len() <= 200);
    // No discontinuities greater than the local step size.
    for w in samples.windows(2) {
        let dy = (w[1].1 - w[0].1).abs();
        assert!(dy < 0.1, "bezier sample jump too large: {dy}");
    }
}

#[test]
fn sample_curve_path_engine_native_byte_order() {
    // Regression: this helper must NOT swap to [output, input] like the
    // egui port's rebuild_cache. Output tuples are (input, output).
    let curve = ResponseCurve::piecewise_linear(vec![(-1.0, 1.0), (1.0, -1.0)], false).unwrap();
    let samples = sample_curve_path(&curve, 3);
    // For this inverted-identity curve, evaluate(-1) = 1 and evaluate(1) = -1.
    // First tuple must be (-1, 1) NOT (1, -1).
    assert!((samples[0].0 - (-1.0)).abs() < 1e-9);
    assert!((samples[0].1 - 1.0).abs() < 1e-9);
}

#[test]
fn sample_curve_path_zero_samples_returns_empty() {
    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false).unwrap();
    assert!(sample_curve_path(&curve, 0).is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-core --lib processing::curves::tests::sample_curve_path`
Expected: FAIL with `error[E0425]: cannot find function 'sample_curve_path' in this scope`.

- [ ] **Step 3: Implement `sample_curve_path`**

Append to `crates/inputforge-core/src/processing/curves.rs` (just above the `#[cfg(test)]` block):

```rust
/// Sample a curve into `samples` evenly-spaced `(input, output)` tuples
/// in engine-native ordering.
///
/// For `PiecewiseLinear` and `CubicSpline`, samples are taken evenly by
/// input across `[-1, 1]`. For `CubicBezier`, samples are taken by the
/// parameter `t` per segment (mirroring egui's `rebuild_cache`) so that
/// non-monotonic `x(t)` regions render correctly.
///
/// Used by the F10 curve editor's polyline render and 28x14 thumbnail.
/// `samples == 0` returns an empty `Vec`.
#[must_use]
pub fn sample_curve_path(curve: &ResponseCurve, samples: usize) -> Vec<(f64, f64)> {
    if samples == 0 {
        return Vec::new();
    }
    if let ResponseCurve::CubicBezier { segments, .. } = curve {
        let per_seg = (samples / segments.len().max(1)).max(2);
        let mut out = Vec::with_capacity(per_seg * segments.len());
        for seg in segments {
            let last = (per_seg - 1).max(1);
            for i in 0..per_seg {
                let t = i as f64 / last as f64;
                out.push((bezier_x(seg, t), bezier_y(seg, t)));
            }
        }
        return out;
    }
    let mut out = Vec::with_capacity(samples);
    if samples == 1 {
        out.push((-1.0, curve.evaluate(-1.0)));
        return out;
    }
    let step = 2.0 / (samples - 1) as f64;
    for i in 0..samples {
        let x = -1.0 + i as f64 * step;
        out.push((x, curve.evaluate(x)));
    }
    out
}
```

Also re-export the new fn from `crates/inputforge-core/src/processing/mod.rs`. The current re-export is at line 10:

```rust
pub use curves::{BezierSegment, ResponseCurve, bezier_x, bezier_y};
```

Add `sample_curve_path` to the list:

```rust
pub use curves::{BezierSegment, ResponseCurve, bezier_x, bezier_y, sample_curve_path};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-core --lib processing::curves`
Expected: PASS, including the four new tests.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-core/src/processing/curves.rs crates/inputforge-core/src/processing/mod.rs
git commit -m "feat(curves): sample_curve_path engine-native helper for F10 render"
```

---

### Task 2: Scaffold `response_curve` submodule + port `mutation.rs`

Create the new submodule directory under `pipeline/stage_body/` and port the egui `mutation.rs` from the egui-ref worktree at `E:\Git\Perso\inputforge-egui-ref\crates\inputforge-gui\src\widgets\curve_editor\mutation.rs`, replacing every `egui_plot::PlotPoint` with a `(f64, f64)` tuple and changing `pub(super)` to `pub(crate)`. The egui `mutation.rs` data layout is already engine-native `(input, output)`; no swap needs unwinding. The port is structural rather than line-for-line: a few field-by-field assignments (`seg.start.0 = ...; seg.start.1 = ...`) in the egui source are rewritten as tuple assignments (`seg.start = (..., ...)`) for readability. F9's dispatcher arm for `ResponseCurve` is left at the placeholder caption until Task 16.

The egui file exposes seven `pub(super)` fns (verified at `E:\Git\Perso\inputforge-egui-ref\crates\inputforge-gui\src\widgets\curve_editor\mutation.rs:24, 82, 246, 264, 328, 383, 466`): `adjacent_x_bounds`, `update_point_in_curve`, `reconstruct_curve`, `default_identity_curve`, `convert_curve_type`, `add_control_point`, `remove_control_point`. All seven move over.

**Plan amendment:** `reconstruct_curve` is promoted from `Option<ResponseCurve>` to `Result<ResponseCurve, String>` so Task 6's invalid-drag path can write the validator's actual error to `EditorState.malformed_hints`. The `Some(curve) => Ok(curve)` / `None => Err(...)` mapping is mechanical; the call sites in this submodule must adopt the new return type.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mutation.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs` (add `mod response_curve;`)

- [ ] **Step 1: Write the failing tests**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mutation.rs` with the test module skeleton at the bottom (tests will fail to compile because the module is empty):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

    fn identity_piecewise() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
    }

    fn identity_bezier() -> ResponseCurve {
        let seg = BezierSegment {
            start: (-1.0, -1.0),
            control1: (-1.0 / 3.0, -1.0 / 3.0),
            control2: (1.0 / 3.0, 1.0 / 3.0),
            end: (1.0, 1.0),
        };
        ResponseCurve::cubic_bezier(vec![seg], false).unwrap()
    }

    #[test]
    fn adjacent_x_bounds_locks_first_and_last() {
        let curve = identity_piecewise();
        let (lo, hi) = adjacent_x_bounds(&curve, 0);
        assert!((lo - (-1.0)).abs() < f64::EPSILON);
        assert!((hi - (-1.0)).abs() < f64::EPSILON);
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 1.0).abs() < f64::EPSILON);
        assert!((hi - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adjacent_x_bounds_locks_symmetric_center() {
        let curve = ResponseCurve::piecewise_linear(
            vec![
                (-1.0, -1.0),
                (-0.5, -0.3),
                (0.0, 0.0),
                (0.5, 0.3),
                (1.0, 1.0),
            ],
            true,
        )
        .unwrap();
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 0.0).abs() < f64::EPSILON);
        assert!((hi - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_point_freezes_symmetric_center() {
        let mut curve = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            symmetric: true,
        };
        let bounds = adjacent_x_bounds(&curve, 1);
        update_point_in_curve(&mut curve, 1, (0.3, 0.5), bounds);
        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert!(points[1].0.abs() < f64::EPSILON, "center x stays at 0");
            assert!(points[1].1.abs() < f64::EPSILON, "center y stays at 0");
        } else {
            panic!("expected PiecewiseLinear");
        }
    }

    #[test]
    fn update_point_mirrors_in_symmetric() {
        let mut curve = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (-0.5, -0.5), (0.0, 0.0), (0.5, 0.5), (1.0, 1.0)],
            symmetric: true,
        };
        let bounds = adjacent_x_bounds(&curve, 3);
        update_point_in_curve(&mut curve, 3, (0.4, 0.7), bounds);
        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert!((points[3].0 - 0.4).abs() < 1e-9);
            assert!((points[3].1 - 0.7).abs() < 1e-9);
            // Mirror at index 1.
            assert!((points[1].0 - (-0.4)).abs() < 1e-9);
            assert!((points[1].1 - (-0.7)).abs() < 1e-9);
        }
    }

    #[test]
    fn convert_curve_type_preserves_symmetric_flag() {
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            true,
        )
        .unwrap();
        let bezier = convert_curve_type(&curve, CurveType::CubicBezier).unwrap();
        match bezier {
            ResponseCurve::CubicBezier { symmetric, segments } => {
                assert!(symmetric);
                assert_eq!(segments.len(), 2, "symmetric bezier has 2 segments");
            }
            _ => panic!("expected CubicBezier"),
        }
    }

    #[test]
    fn add_control_point_inserts_between_neighbors() {
        let mut curve = identity_piecewise();
        let added = add_control_point(&mut curve, (0.5, 0.7));
        assert!(added);
        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert_eq!(points.len(), 4);
            assert!(points.windows(2).all(|w| w[0].0 < w[1].0));
        }
    }

    #[test]
    fn remove_control_point_refuses_edges_and_handles() {
        let mut curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (0.5, 0.5), (1.0, 1.0)],
            false,
        )
        .unwrap();
        assert!(!remove_control_point(&mut curve, 0), "first edge cannot be removed");
        assert!(!remove_control_point(&mut curve, 3), "last edge cannot be removed");
        // Bezier handle (local 1 or 2) cannot be removed.
        let mut bz = identity_bezier();
        assert!(!remove_control_point(&mut bz, 1), "bezier handle cannot be removed");
        assert!(!remove_control_point(&mut bz, 2), "bezier handle cannot be removed");
    }

    #[test]
    fn reconstruct_curve_returns_validated() {
        let curve = identity_piecewise();
        let valid = reconstruct_curve(&curve);
        assert!(valid.is_some());
    }

    #[test]
    fn default_identity_curve_preserves_type_and_symmetric() {
        let curve = ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-0.5, -0.5),
                control2: (0.5, 0.5),
                end: (1.0, 1.0),
            }],
            true,
        )
        .unwrap();
        let reset = default_identity_curve(&curve);
        match reset {
            ResponseCurve::CubicBezier { symmetric: true, segments } => {
                assert_eq!(segments.len(), 2, "symmetric reset is 2 segments");
            }
            _ => panic!("expected symmetric CubicBezier"),
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::stage_body::response_curve::mutation::tests`
Expected: FAIL: module is not declared yet.

Add the wiring in `pipeline/stage_body/mod.rs` (above the existing `mod invert;` etc.):

```rust
mod response_curve;
```

And in the new `response_curve/mod.rs` (create with the minimum needed for module discovery):

```rust
//! F10 response-curve body. See spec
//! `docs/superpowers/specs/2026-05-01-f10-curve-editor-design.md`.

#![allow(
    dead_code,
    reason = "submodules expose APIs consumed across F10 tasks; clippy's \
              reachability check loses some pub(crate) items here."
)]

pub(crate) mod mutation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurveType {
    PiecewiseLinear,
    CubicSpline,
    CubicBezier,
}

impl CurveType {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::PiecewiseLinear => "Linear",
            Self::CubicSpline => "Spline",
            Self::CubicBezier => "Bezier",
        }
    }
}
```

Run again. Expected: FAIL because `adjacent_x_bounds` etc. still don't exist.

- [ ] **Step 3: Port `mutation.rs` verbatim with `PlotPoint -> (f64, f64)`**

Replace the placeholder `mutation.rs` with the port. Mechanical changes from `E:\Git\Perso\inputforge-egui-ref\crates\inputforge-gui\src\widgets\curve_editor\mutation.rs` (the egui-ref worktree at the parent of the egui-deletion commit):

- Drop `use egui_plot::PlotPoint;` and the `super::CurveType` / `super::MIN_X_GAP` imports.
- Use `super::CurveType` (now lives in the F10 `mod.rs`) and a local `const MIN_X_GAP: f64 = 0.001;`.
- `update_point_in_curve(curve, index, new_pos: PlotPoint, bounds)` becomes `update_point_in_curve(curve, index, new_pos: (f64, f64), bounds)`. Replace `new_pos.x` with `new_pos.0`, `new_pos.y` with `new_pos.1`.
- `add_control_point(curve, pos: PlotPoint)` becomes `add_control_point(curve, pos: (f64, f64))`. Replace `pos.x` with `pos.0`, `pos.y` with `pos.1`.
- Visibility: change `pub(super)` to `pub(crate)` on all seven top-level fns (`adjacent_x_bounds`, `update_point_in_curve`, `reconstruct_curve`, `default_identity_curve`, `convert_curve_type`, `add_control_point`, `remove_control_point`); they cross sibling-module boundaries.

The full body to write:

```rust
//! Curve mutation operations, ported verbatim from
//! `E:\Git\Perso\inputforge-egui-ref\crates\inputforge-gui\src\widgets\curve_editor\mutation.rs`.
//!
//! Mechanical surface change: `egui_plot::PlotPoint` becomes `(f64, f64)`.
//! The egui implementation is already engine-native `(input, output)`, so
//! no swap is unwound here. The SVG render path applies y-down via a
//! `<g transform="scale(1, -1)">`, never via tuple swap.

use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

use super::CurveType;

/// Minimum x separation between adjacent control points when dragging.
const MIN_X_GAP: f64 = 0.001;

// ---------------------------------------------------------------------------
// Drag application
// ---------------------------------------------------------------------------

#[must_use]
pub(crate) fn adjacent_x_bounds(curve: &ResponseCurve, index: usize) -> (f64, f64) {
    let symmetric = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. }
        | ResponseCurve::CubicSpline { symmetric, .. }
        | ResponseCurve::CubicBezier { symmetric, .. } => *symmetric,
    };
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => {
            let count = points.len();
            if index == 0 {
                return (points[0].0, points[0].0);
            }
            if index == count - 1 {
                return (points[count - 1].0, points[count - 1].0);
            }
            if symmetric && count % 2 == 1 && index == count / 2 {
                return (0.0, 0.0);
            }
            (points[index - 1].0 + MIN_X_GAP, points[index + 1].0 - MIN_X_GAP)
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            let seg_idx = index / 4;
            let local = index % 4;
            let last_seg = segments.len().saturating_sub(1);
            if seg_idx == 0 && local == 0 {
                return (-1.0, -1.0);
            }
            if seg_idx == last_seg && local == 3 {
                return (1.0, 1.0);
            }
            (-1.0, 1.0)
        }
    }
}

pub(crate) fn update_point_in_curve(
    curve: &mut ResponseCurve,
    index: usize,
    new_pos: (f64, f64),
    bounds: (f64, f64),
) {
    let new_x = new_pos.0.clamp(bounds.0, bounds.1);
    let new_y = new_pos.1.clamp(-1.0, 1.0);
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric, .. }
        | ResponseCurve::CubicSpline { points, symmetric, .. } => {
            if *symmetric && points.len() % 2 == 1 && index == points.len() / 2 {
                return;
            }
            if let Some(pt) = points.get_mut(index) {
                pt.0 = new_x;
                pt.1 = new_y;
            }
            if *symmetric {
                let count = points.len();
                let mirror_idx = count - 1 - index;
                if mirror_idx != index {
                    if let Some(mirror_pt) = points.get_mut(mirror_idx) {
                        mirror_pt.0 = -new_x;
                        mirror_pt.1 = -new_y;
                    }
                }
            }
        }
        ResponseCurve::CubicBezier { segments, symmetric } => {
            update_bezier_point(segments, *symmetric, index, new_x, new_y);
        }
    }
}

fn update_bezier_point(
    segments: &mut [BezierSegment],
    symmetric: bool,
    index: usize,
    new_x: f64,
    new_y: f64,
) {
    let seg_idx = index / 4;
    let local = index % 4;
    if symmetric && segments.len() % 2 == 0 {
        let center_seg = segments.len() / 2;
        if seg_idx == center_seg && local == 0 {
            return;
        }
        if seg_idx == center_seg - 1 && local == 3 {
            return;
        }
    }
    if let Some(seg) = segments.get_mut(seg_idx) {
        match local {
            0 => { seg.start = (new_x, new_y); }
            1 => { seg.control1 = (new_x, new_y); }
            2 => { seg.control2 = (new_x, new_y); }
            3 => { seg.end = (new_x, new_y); }
            _ => {}
        }
    }
    if local == 3 {
        if let Some(next) = segments.get_mut(seg_idx + 1) {
            next.start = (new_x, new_y);
        }
    } else if local == 0 && seg_idx > 0 {
        if let Some(prev) = segments.get_mut(seg_idx - 1) {
            prev.end = (new_x, new_y);
        }
    }
    if symmetric {
        let seg_count = segments.len();
        let mirror_seg_idx = seg_count - 1 - seg_idx;
        let mirror_local = 3 - local;
        let primary_synced_idx = match local {
            3 => Some(seg_idx + 1),
            0 if seg_idx > 0 => Some(seg_idx - 1),
            _ => None,
        };
        if mirror_seg_idx != seg_idx || mirror_local != local {
            if let Some(mirror_seg) = segments.get_mut(mirror_seg_idx) {
                match mirror_local {
                    0 => mirror_seg.start = (-new_x, -new_y),
                    1 => mirror_seg.control1 = (-new_x, -new_y),
                    2 => mirror_seg.control2 = (-new_x, -new_y),
                    3 => mirror_seg.end = (-new_x, -new_y),
                    _ => {}
                }
            }
            if mirror_local == 3 {
                let target = mirror_seg_idx + 1;
                if primary_synced_idx != Some(target) {
                    if let Some(next) = segments.get_mut(target) {
                        next.start = (-new_x, -new_y);
                    }
                }
            } else if mirror_local == 0 && mirror_seg_idx > 0 {
                let target = mirror_seg_idx - 1;
                if primary_synced_idx != Some(target) {
                    if let Some(prev) = segments.get_mut(target) {
                        prev.end = (-new_x, -new_y);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reconstruction + identity reset
// ---------------------------------------------------------------------------

#[must_use]
pub(crate) fn reconstruct_curve(curve: &ResponseCurve) -> Option<ResponseCurve> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric } => {
            ResponseCurve::piecewise_linear(points.clone(), *symmetric).ok()
        }
        ResponseCurve::CubicSpline { points, symmetric } => {
            ResponseCurve::cubic_spline(points.clone(), *symmetric).ok()
        }
        ResponseCurve::CubicBezier { segments, symmetric } => {
            ResponseCurve::cubic_bezier(segments.clone(), *symmetric).ok()
        }
    }
}

#[must_use]
pub(crate) fn default_identity_curve(curve: &ResponseCurve) -> ResponseCurve {
    match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. } => {
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], *symmetric)
                .unwrap_or_else(|_| {
                    ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                        .expect("hardcoded identity is valid")
                })
        }
        ResponseCurve::CubicSpline { symmetric, .. } => {
            ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], *symmetric)
                .unwrap_or_else(|_| {
                    ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (1.0, 1.0)], false)
                        .expect("hardcoded identity is valid")
                })
        }
        ResponseCurve::CubicBezier { symmetric, .. } => {
            let segs = symmetric_bezier_identity(*symmetric);
            ResponseCurve::cubic_bezier(segs, *symmetric).unwrap_or_else(|_| {
                ResponseCurve::cubic_bezier(symmetric_bezier_identity(false), false)
                    .expect("hardcoded bezier identity is valid")
            })
        }
    }
}

fn symmetric_bezier_identity(symmetric: bool) -> Vec<BezierSegment> {
    if symmetric {
        vec![
            BezierSegment {
                start: (-1.0, -1.0),
                control1: (-2.0 / 3.0, -2.0 / 3.0),
                control2: (-1.0 / 3.0, -1.0 / 3.0),
                end: (0.0, 0.0),
            },
            BezierSegment {
                start: (0.0, 0.0),
                control1: (1.0 / 3.0, 1.0 / 3.0),
                control2: (2.0 / 3.0, 2.0 / 3.0),
                end: (1.0, 1.0),
            },
        ]
    } else {
        vec![BezierSegment {
            start: (-1.0, -1.0),
            control1: (-1.0 / 3.0, -1.0 / 3.0),
            control2: (1.0 / 3.0, 1.0 / 3.0),
            end: (1.0, 1.0),
        }]
    }
}

// ---------------------------------------------------------------------------
// Type conversion
// ---------------------------------------------------------------------------

#[must_use]
pub(crate) fn convert_curve_type(
    curve: &ResponseCurve,
    target: CurveType,
) -> Option<ResponseCurve> {
    let symmetric = match curve {
        ResponseCurve::PiecewiseLinear { symmetric, .. }
        | ResponseCurve::CubicSpline { symmetric, .. }
        | ResponseCurve::CubicBezier { symmetric, .. } => *symmetric,
    };
    match target {
        CurveType::PiecewiseLinear => {
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], symmetric)
                .ok()
        }
        CurveType::CubicSpline => {
            ResponseCurve::cubic_spline(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], symmetric).ok()
        }
        CurveType::CubicBezier => {
            ResponseCurve::cubic_bezier(symmetric_bezier_identity(symmetric), symmetric).ok()
        }
    }
}

// ---------------------------------------------------------------------------
// Add / remove control points
// ---------------------------------------------------------------------------

pub(crate) fn add_control_point(curve: &mut ResponseCurve, pos: (f64, f64)) -> bool {
    let x = pos.0.clamp(-1.0, 1.0);
    let y = pos.1.clamp(-1.0, 1.0);
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric, .. }
        | ResponseCurve::CubicSpline { points, symmetric, .. } => {
            let original = points.clone();
            points.push((x, y));
            if *symmetric && x.abs() > 0.0 {
                points.push((-x, -y));
            }
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
            if points.windows(2).all(|w| w[0].0 < w[1].0) {
                true
            } else {
                *points = original;
                false
            }
        }
        ResponseCurve::CubicBezier { segments, symmetric } => {
            let Some(seg_idx) = segments.iter().position(|s| s.start.0 <= x && x <= s.end.0)
            else {
                return false;
            };
            let seg = &segments[seg_idx];
            let dx = seg.end.0 - seg.start.0;
            if dx.abs() < f64::EPSILON {
                return false;
            }
            let t = ((x - seg.start.0) / dx).clamp(0.05, 0.95);
            let (left, right) = split_bezier_segment(seg, t);
            segments.splice(seg_idx..=seg_idx, [left, right]);
            if *symmetric {
                let pre_splice_count = segments.len() - 1;
                let mut mirror_seg = pre_splice_count - 1 - seg_idx;
                if mirror_seg >= seg_idx {
                    mirror_seg += 1;
                }
                if mirror_seg != seg_idx && mirror_seg != seg_idx + 1 {
                    let mirror_x = -x;
                    let m_seg = &segments[mirror_seg];
                    let m_dx = m_seg.end.0 - m_seg.start.0;
                    let mirror_t = if m_dx.abs() < f64::EPSILON {
                        0.5
                    } else {
                        ((mirror_x - m_seg.start.0) / m_dx).clamp(0.05, 0.95)
                    };
                    let (ml, mr) = split_bezier_segment(&segments[mirror_seg], mirror_t);
                    segments.splice(mirror_seg..=mirror_seg, [ml, mr]);
                }
            }
            true
        }
    }
}

pub(crate) fn remove_control_point(curve: &mut ResponseCurve, index: usize) -> bool {
    match curve {
        ResponseCurve::PiecewiseLinear { points, symmetric, .. }
        | ResponseCurve::CubicSpline { points, symmetric, .. } => {
            let count = points.len();
            if index == 0 || index == count - 1 {
                return false;
            }
            if *symmetric && count % 2 == 1 && index == count / 2 {
                return false;
            }
            let removals = if *symmetric { 2 } else { 1 };
            if count <= removals + 1 {
                return false;
            }
            if *symmetric {
                let mirror_idx = count - 1 - index;
                debug_assert_ne!(index, mirror_idx);
                let (first, second) = if index > mirror_idx { (index, mirror_idx) } else { (mirror_idx, index) };
                points.remove(first);
                points.remove(second);
            } else {
                points.remove(index);
            }
            true
        }
        ResponseCurve::CubicBezier { segments, symmetric } => {
            let seg_idx = index / 4;
            let local = index % 4;
            if local == 1 || local == 2 {
                return false;
            }
            let (left_idx, right_idx) = if local == 3 {
                (seg_idx, seg_idx + 1)
            } else {
                if seg_idx == 0 {
                    return false;
                }
                (seg_idx - 1, seg_idx)
            };
            let seg_count = segments.len();
            if right_idx >= seg_count || seg_count < 2 {
                return false;
            }
            if *symmetric && seg_count % 2 == 0 {
                let center_seg = seg_count / 2;
                if (local == 3 && seg_idx == center_seg - 1)
                    || (local == 0 && seg_idx == center_seg)
                {
                    return false;
                }
            }
            let merged = BezierSegment {
                start: segments[left_idx].start,
                control1: segments[left_idx].control1,
                control2: segments[right_idx].control2,
                end: segments[right_idx].end,
            };
            segments.splice(left_idx..=right_idx, [merged]);
            if *symmetric {
                let pre_merge_count = segments.len() + 1;
                let mut mirror_left = pre_merge_count - 2 - left_idx;
                if mirror_left > left_idx {
                    mirror_left -= 1;
                }
                let new_count = segments.len();
                if mirror_left < new_count && mirror_left != left_idx {
                    let mirror_right = mirror_left + 1;
                    if mirror_right < new_count {
                        let mirror_merged = BezierSegment {
                            start: segments[mirror_left].start,
                            control1: segments[mirror_left].control1,
                            control2: segments[mirror_right].control2,
                            end: segments[mirror_right].end,
                        };
                        segments.splice(mirror_left..=mirror_right, [mirror_merged]);
                    }
                }
            }
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Bezier helpers
// ---------------------------------------------------------------------------

fn lerp_point(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn split_bezier_segment(seg: &BezierSegment, t: f64) -> (BezierSegment, BezierSegment) {
    let ab = lerp_point(seg.start, seg.control1, t);
    let bc = lerp_point(seg.control1, seg.control2, t);
    let cd = lerp_point(seg.control2, seg.end, t);
    let abc = lerp_point(ab, bc, t);
    let bcd = lerp_point(bc, cd, t);
    let mid = lerp_point(abc, bcd, t);
    (
        BezierSegment { start: seg.start, control1: ab, control2: abc, end: mid },
        BezierSegment { start: mid, control1: bcd, control2: cd, end: seg.end },
    )
}
```

(Re-append the test module exactly as in Step 1.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::stage_body::response_curve::mutation`
Expected: PASS, all 9 tests.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs
git commit -m "feat(response_curve): port mutation.rs from egui to dx (PlotPoint -> tuple)"
```

---

### Task 3: Port `apply_symmetry` from egui `symmetry.rs` into `mutation.rs`

The spec lists `apply_symmetry` as a function exported from `mutation.rs` (ported from `symmetry.rs`). Append it there rather than creating a sibling file: callers cluster around `mutation::*`, and the helper is conceptually a curve mutation.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mutation.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `mod tests { ... }` block at the bottom of `mutation.rs`:

```rust
#[test]
fn apply_symmetry_enabling_enforces_antisymmetric_points() {
    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
    let result = apply_symmetry(&curve, true).expect("enable symmetry on identity");
    if let ResponseCurve::PiecewiseLinear { points, symmetric } = result {
        assert!(symmetric);
        assert!(points.len() >= 3);
        let center = points.iter().find(|(x, _)| x.abs() < f64::EPSILON);
        assert!(center.is_some(), "origin must be present");
        assert!(center.unwrap().1.abs() < f64::EPSILON);
    } else {
        panic!("expected PiecewiseLinear");
    }
}

#[test]
fn apply_symmetry_two_point_default_curve() {
    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false).unwrap();
    let result = apply_symmetry(&curve, true).expect("enable symmetry on 2-point");
    if let ResponseCurve::PiecewiseLinear { points, symmetric } = result {
        assert!(symmetric);
        assert!(points.len() >= 3);
        assert!(points[0].0 < 0.0);
        assert!(points[points.len() - 1].0 > 0.0);
    }
}

#[test]
fn apply_symmetry_disabling_keeps_all_points() {
    let curve = ResponseCurve::piecewise_linear(
        vec![(-1.0, -1.0), (0.0, 0.0), (0.5, 0.2), (1.0, 1.0)],
        true,
    )
    .unwrap();
    let result = apply_symmetry(&curve, false).expect("disable symmetry");
    if let ResponseCurve::PiecewiseLinear { points, symmetric } = result {
        assert!(!symmetric);
        assert_eq!(points.len(), 4);
    }
}

#[test]
fn apply_symmetry_bezier_round_trip() {
    let curve = ResponseCurve::cubic_bezier(
        vec![BezierSegment {
            start: (-1.0, -1.0),
            control1: (-0.5, -0.5),
            control2: (0.5, 0.5),
            end: (1.0, 1.0),
        }],
        false,
    )
    .unwrap();
    let sym = apply_symmetry(&curve, true).expect("enable bezier symmetry");
    if let ResponseCurve::CubicBezier { segments, symmetric: true } = sym {
        assert!(segments.len() >= 2);
    } else {
        panic!("expected symmetric CubicBezier");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::mutation::tests::apply_symmetry`
Expected: FAIL: `apply_symmetry` not found.

- [ ] **Step 3: Append `apply_symmetry` to `mutation.rs`**

Append below the `split_bezier_segment` fn (port of egui `symmetry.rs`):

```rust
// ---------------------------------------------------------------------------
// Symmetry enforcement
// ---------------------------------------------------------------------------

/// Apply a symmetry change. Enabling enforces antisymmetry through the
/// origin (mirrors positive-half points to negative side); disabling just
/// clears the flag. Ported from egui `widgets/curve_editor/symmetry.rs`.
#[must_use]
pub(crate) fn apply_symmetry(curve: &ResponseCurve, symmetric: bool) -> Option<ResponseCurve> {
    if symmetric {
        enforce_symmetry(curve)
    } else {
        let mut result = curve.clone();
        result.set_symmetric(false);
        Some(result)
    }
}

fn enforce_symmetry(curve: &ResponseCurve) -> Option<ResponseCurve> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. } => {
            ResponseCurve::piecewise_linear(enforce_symmetry_points(points), true).ok()
        }
        ResponseCurve::CubicSpline { points, .. } => {
            ResponseCurve::cubic_spline(enforce_symmetry_points(points), true).ok()
        }
        ResponseCurve::CubicBezier { segments, .. } => {
            ResponseCurve::cubic_bezier(enforce_symmetry_bezier(segments), true).ok()
        }
    }
}

fn enforce_symmetry_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut positive: Vec<(f64, f64)> =
        points.iter().filter(|(x, _)| *x >= 0.0).copied().collect();
    positive.sort_by(|a, b| a.0.total_cmp(&b.0));
    if positive.is_empty() || positive[0].0 > 0.0 {
        positive.insert(0, (0.0, 0.0));
    } else {
        positive[0].1 = 0.0;
    }
    if positive.len() < 2 {
        positive.push((1.0, 1.0));
    }
    let mut result: Vec<(f64, f64)> = positive
        .iter()
        .filter(|(x, _)| *x > 0.0)
        .map(|(x, y)| (-x, -y))
        .collect();
    result.reverse();
    result.extend_from_slice(&positive);
    result
}

fn enforce_symmetry_bezier(segments: &[BezierSegment]) -> Vec<BezierSegment> {
    let positive: Vec<_> = segments.iter().filter(|s| s.start.0 >= 0.0).cloned().collect();
    let positive = if positive.is_empty() {
        vec![BezierSegment {
            start: (0.0, 0.0),
            control1: (1.0 / 3.0, 1.0 / 3.0),
            control2: (2.0 / 3.0, 2.0 / 3.0),
            end: (1.0, 1.0),
        }]
    } else {
        positive
    };
    let mut mirrored: Vec<BezierSegment> = positive
        .iter()
        .rev()
        .map(|seg| BezierSegment {
            start: (-seg.end.0, -seg.end.1),
            control1: (-seg.control2.0, -seg.control2.1),
            control2: (-seg.control1.0, -seg.control1.1),
            end: (-seg.start.0, -seg.start.1),
        })
        .collect();
    mirrored.extend_from_slice(&positive);
    mirrored
}
```

`ResponseCurve::set_symmetric` already exists on the engine type (used by egui `symmetry.rs:27`); no engine change needed.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::mutation`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mutation.rs
git commit -m "feat(response_curve): apply_symmetry ported from egui symmetry.rs"
```

---

### Task 4: `state.rs` body-state types + `extract_anchors`

Pure types and a helper that produces the flat anchor list driving hit-testing, rendering, and keyboard navigation. The list is in `mutation.rs` index space (4 points per bezier segment, anchors interleaved with handles). No deduplication: `point_index` round-trips into `update_point_in_curve`.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/state.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `state.rs` with the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

    #[test]
    fn body_state_default_is_idle_with_dirty_cache() {
        let s = BodyState::default();
        assert!(s.dragging.is_none());
        assert!(s.hovered_point.is_none());
        assert!(s.focused_point.is_none());
        assert!(s.pre_drag_curve.is_none());
        assert!(s.cache_dirty);
    }

    #[test]
    fn extract_anchors_piecewise_yields_engine_native_tuples() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let anchors = extract_anchors(&curve);
        assert_eq!(anchors.len(), 3);
        // Engine-native: tuple is (input, output), NOT (output, input).
        assert!((anchors[0].0 - (-1.0)).abs() < f64::EPSILON);
        assert!((anchors[0].1 - (-1.0)).abs() < f64::EPSILON);
        assert!((anchors[2].0 - 1.0).abs() < f64::EPSILON);
        assert!((anchors[2].1 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_anchors_bezier_interleaves_four_per_segment() {
        let curve = ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-1.0 / 3.0, -1.0 / 3.0),
                control2: (1.0 / 3.0, 1.0 / 3.0),
                end: (1.0, 1.0),
            }],
            false,
        )
        .unwrap();
        let anchors = extract_anchors(&curve);
        assert_eq!(anchors.len(), 4);
        assert!((anchors[1].0 - (-1.0 / 3.0)).abs() < 1e-9);
        assert!((anchors[2].0 - (1.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn clamp_focus_after_external_edit_clamps_down() {
        let mut s = BodyState::default();
        s.focused_point = Some(4);
        let next = clamp_focus_after_external_edit(s, 3);
        assert_eq!(next.focused_point, Some(2));
        assert!(next.pre_drag_curve.is_none());
    }

    #[test]
    fn clamp_focus_after_external_edit_clears_when_empty() {
        let mut s = BodyState::default();
        s.focused_point = Some(0);
        let next = clamp_focus_after_external_edit(s, 0);
        assert_eq!(next.focused_point, None);
    }

    #[test]
    fn clamp_focus_after_external_edit_noop_in_range() {
        let mut s = BodyState::default();
        s.focused_point = Some(1);
        let next = clamp_focus_after_external_edit(s, 5);
        assert_eq!(next.focused_point, Some(1));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::state::tests`
Expected: FAIL, module not declared, types not defined.

- [ ] **Step 3: Implement `state.rs` and wire it**

Replace `state.rs` with:

```rust
//! `BodyState` and helpers used by the F10 curve-editor body.

use inputforge_core::processing::curves::ResponseCurve;

/// Per-mounted-component state held in a `Signal<BodyState>` inside
/// `ResponseCurveBody`. Pure data; no Signals.
#[derive(Debug, Clone)]
pub(crate) struct BodyState {
    pub dragging: Option<DragInProgress>,
    pub hovered_point: Option<usize>,
    /// Keyboard-focused anchor; intentionally separate from `hovered_point`.
    pub focused_point: Option<usize>,
    /// Snapshot taken at drag start, used to revert on validation failure.
    pub pre_drag_curve: Option<ResponseCurve>,
    /// 200-sample polyline; engine-native (input, output).
    pub cached_path: Vec<(f64, f64)>,
    /// Flat list of draggable points; mutation.rs index space.
    pub cached_anchors: Vec<(f64, f64)>,
    pub cache_dirty: bool,
    /// Timestamp (ms since component mount) of the last keyboard nudge.
    /// Drives Task 7's 250 ms same-key coalesce window for undo merging.
    pub last_nudge_at_ms: Option<u64>,
    /// Key kind of the last nudge, used together with `last_nudge_at_ms`
    /// to decide whether the next nudge merges into the existing undo
    /// entry or pushes a fresh one.
    pub last_nudge_key: Option<NudgeKey>,
}

/// Discriminator for the in-flight keyboard nudge streak. See
/// `keyboard.rs::handle_key` for the merge policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NudgeKey {
    Up,
    Down,
    Left,
    Right,
    UpLarge,
    DownLarge,
    LeftLarge,
    RightLarge,
}

// Manual `Default` so `cache_dirty` defaults to `true`. The Task 4 test
// `body_state_default_is_idle_with_dirty_cache` asserts this; deriving
// `Default` would produce `cache_dirty: false` (Rust bool default) and
// the test would fail.
impl Default for BodyState {
    fn default() -> Self {
        Self {
            dragging: None,
            hovered_point: None,
            focused_point: None,
            pre_drag_curve: None,
            cached_path: Vec::new(),
            cached_anchors: Vec::new(),
            cache_dirty: true,
            last_nudge_at_ms: None,
            last_nudge_key: None,
        }
    }
}

/// Defensive clamp run by the body's main `use_effect` when the projected
/// curve from the live config has fewer anchors than `focused_point` indexed.
/// Originally Task 15 (external-edit reconciliation effect); the standalone
/// effect is gone (`c9e7853` deleted `EditorState.external_edit_reset`),
/// but the clamp survives as a safety net inside the cache rebuild path.
#[must_use]
pub(crate) fn clamp_focus_after_external_edit(state: BodyState, new_anchor_count: usize) -> BodyState {
    let mut s = state;
    s.pre_drag_curve = None;
    s.focused_point = match s.focused_point {
        Some(_) if new_anchor_count == 0 => None,
        Some(i) => Some(i.min(new_anchor_count - 1)),
        None => None,
    };
    s
}

#[derive(Debug, Clone)]
pub(crate) struct DragInProgress {
    pub point_index: usize,
    pub bounds: (f64, f64),
}

/// Flatten a curve to its draggable points in `mutation.rs` index space.
///
/// `PiecewiseLinear` / `CubicSpline`: returns `(x, y)` directly.
/// `CubicBezier`: returns `[start, c1, c2, end]` per segment, interleaved.
/// Engine-native `(input, output)` ordering throughout.
#[must_use]
pub(crate) fn extract_anchors(curve: &ResponseCurve) -> Vec<(f64, f64)> {
    match curve {
        ResponseCurve::PiecewiseLinear { points, .. }
        | ResponseCurve::CubicSpline { points, .. } => points.clone(),
        ResponseCurve::CubicBezier { segments, .. } => {
            let mut pts = Vec::with_capacity(segments.len() * 4);
            for seg in segments {
                pts.push(seg.start);
                pts.push(seg.control1);
                pts.push(seg.control2);
                pts.push(seg.end);
            }
            pts
        }
    }
}
```

Append to `response_curve/mod.rs`:

```rust
pub(crate) mod state;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::state`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve
git commit -m "feat(response_curve): BodyState + extract_anchors helper"
```

---

### Task 5: REUSE F9's existing `format_response_curve_summary`

F9 already ships a curve summary formatter at `pipeline/stage.rs:388-406`, returning `Linear · 5 pts`, `Spline · 5 pts · sym`, `Bezier · 1 seg`, `Bezier · 2 seg · sym`, etc. (capitalization landed in commit `35a3a9d`). It is wired into `stage_summary_for` at `pipeline/stage.rs:264`, which already populates `if-stage__summary` for every variant including `ResponseCurve`. F10 has no reason to invent a second formatter with different abbreviations or casing.

This task originally added a `header_summary` helper inside `response_curve/mod.rs`. That helper is removed.

**Files:** none modified at this step. `pipeline::stage::stage_summary_for(action, cfg)` is reused in Task 16's `aria_label_override` wiring (the only place the F10 plan needed a per-curve summary string).

- [ ] **Step 1: Verify F9's formatter is exported for cross-module reuse**

Read `pipeline/stage.rs:388-406` and confirm `format_response_curve_summary` is reachable through `stage_summary_for` (it is, via the `Action::ResponseCurve { curve }` arm in `stage_summary_for`). If `stage_summary_for` is `pub(crate)` (which it should be, since `StageHeader` already calls it), no change is needed. If it is private, promote to `pub(crate)`.

- [ ] **Step 2: Spot-check capitalization expectations in the spec**

Spec line 322 (or thereabouts) historically read `linear · 5pt`. Cross-check against the spec and, if the spec still uses the old lowercase style, either update the spec to match F9's `Linear · 5 pts` or note the divergence. Flag for the user if not obvious.

- [ ] **Step 3: No commit**

Nothing is added or changed by this task on its own. Tasks 11 and 16 reuse `stage_summary_for` directly.

---

### Task 6: `interaction.rs`, pure pointer handlers

Pure helpers that take `(BodyState, ResponseCurve, Event-as-data)` and return `(BodyState', HandlerOutcome)` where `HandlerOutcome` carries the proposed curve and any validator error so the host body can write `EditorState.malformed_hints` directly. No Dioxus types: handlers receive a viewport-relative cursor position and the SVG's bounding rect; the body component projects the actual `PointerEvent` to those primitives before calling. This follows F8's pure-routing convention (e.g. `mapping_list/keyboard.rs::handle_key`); F10 adds drag state and validator error reporting that have no F8 precedent.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/interaction.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `interaction.rs` with the test module. The tests drive each handler with seed values, asserting state transitions and dispatch payloads.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::processing::curves::ResponseCurve;
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::state::{
        extract_anchors, BodyState, DragInProgress,
    };

    fn seed_curve() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
    }

    fn rect() -> PlotRect {
        // Square plot, 240px, top-left at (10, 20).
        PlotRect { x: 10.0, y: 20.0, size: 240.0 }
    }

    #[test]
    fn screen_to_viewbox_maps_corners() {
        let r = rect();
        // top-left of plot maps to (-1.05, -1.05) in viewBox; in engine
        // coords (input axis horizontal) that's (input=-1.05, output=+1.05)
        // because SVG-y is flipped at render time.
        let p = screen_to_viewbox((10.0, 20.0), &r);
        assert!((p.0 - (-1.05)).abs() < 1e-6);
        assert!((p.1 - 1.05).abs() < 1e-6);
        // Center of plot maps to (0, 0).
        let p = screen_to_viewbox((10.0 + 120.0, 20.0 + 120.0), &r);
        assert!(p.0.abs() < 1e-6 && p.1.abs() < 1e-6);
    }

    #[test]
    fn nearest_anchor_within_radius() {
        let curve = seed_curve();
        let anchors = extract_anchors(&curve);
        let r = rect();
        // Center anchor (0, 0) projects to plot center.
        let cursor = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let hit = nearest_anchor(cursor, &anchors, &r, 10.0);
        assert_eq!(hit, Some(1));
    }

    #[test]
    fn nearest_anchor_outside_radius() {
        let curve = seed_curve();
        let anchors = extract_anchors(&curve);
        let r = rect();
        // Far from any anchor.
        let cursor = (10.0 + 120.0 + 60.0, 20.0 + 120.0);
        let hit = nearest_anchor(cursor, &anchors, &r, 10.0);
        assert!(hit.is_none());
    }

    #[test]
    fn nearest_anchor_at_bezier_junction_returns_lower_index() {
        // Two-segment bezier: anchors[3] (seg0.end) and anchors[4] (seg1.start)
        // coincide at (0, 0). A click at the junction must return the
        // lower index per `nearest_anchor`'s tie-breaking rule.
        let curve = inputforge_core::processing::curves::ResponseCurve::cubic_bezier(
            vec![
                inputforge_core::processing::curves::BezierSegment {
                    start: (-1.0, -1.0),
                    control1: (-0.5, -0.5),
                    control2: (-0.25, -0.25),
                    end: (0.0, 0.0),
                },
                inputforge_core::processing::curves::BezierSegment {
                    start: (0.0, 0.0),
                    control1: (0.25, 0.25),
                    control2: (0.5, 0.5),
                    end: (1.0, 1.0),
                },
            ],
            false,
        )
        .unwrap();
        let anchors = extract_anchors(&curve);
        let r = rect();
        // Junction (0, 0) projects to plot center.
        let cursor = (10.0 + 120.0, 20.0 + 120.0);
        let hit = nearest_anchor(cursor, &anchors, &r, 10.0);
        assert_eq!(hit, Some(3), "junction tie must return lower index (3, not 4)");
    }

    #[test]
    fn pointer_down_on_anchor_starts_drag_and_snapshots() {
        let curve = seed_curve();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        let cursor = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (next, _new_curve, _changed) =
            handle_pointer_down(state, &curve, cursor, &rect());
        assert!(next.dragging.is_some());
        assert_eq!(next.dragging.as_ref().unwrap().point_index, 1);
        assert!(next.pre_drag_curve.is_some());
    }

    #[test]
    fn pointer_down_miss_no_drag() {
        let curve = seed_curve();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        let cursor = (10.0 + 120.0 + 60.0, 20.0 + 120.0);
        let (next, new_curve, changed) =
            handle_pointer_down(state, &curve, cursor, &rect());
        assert!(next.dragging.is_none());
        assert!(new_curve.is_none());
        assert!(!changed);
    }

    #[test]
    fn pointer_move_during_drag_updates_curve_locally() {
        let curve = seed_curve();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        // Simulate drag start.
        let cursor_down = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (state, _, _) = handle_pointer_down(state, &curve, cursor_down, &rect());
        // Move down-and-right.
        let cursor_move = (10.0 + 120.0 + 30.0, 20.0 + 120.0 + 10.0);
        let (next, new_curve, changed) =
            handle_pointer_move(state, &curve, cursor_move, &rect());
        assert!(changed);
        let new_curve = new_curve.expect("drag-move yields a new local curve");
        if let ResponseCurve::PiecewiseLinear { points, .. } = new_curve {
            // Center moved away from origin.
            assert!(points[1].0 != 0.0 || points[1].1 != 0.0);
        }
        assert!(next.cache_dirty);
    }

    #[test]
    fn pointer_move_idle_updates_hover_only() {
        let curve = seed_curve();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        let cursor = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (next, new_curve, changed) =
            handle_pointer_move(state, &curve, cursor, &rect());
        assert!(!changed);
        assert!(new_curve.is_none());
        assert_eq!(next.hovered_point, Some(1));
    }

    #[test]
    fn pointer_up_after_drag_validates_and_commits() {
        let curve = seed_curve();
        // Pretend we've already mid-dragged the curve into a valid state.
        let dragged = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (0.1, 0.2), (1.0, 1.0)],
            symmetric: false,
        };
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        state.dragging = Some(DragInProgress { point_index: 1, bounds: (-1.0, 1.0) });
        state.pre_drag_curve = Some(curve.clone());
        let (next, committed, _) = handle_pointer_up(state, &dragged);
        assert!(next.dragging.is_none());
        assert!(next.pre_drag_curve.is_none());
        let committed = committed.expect("valid drag commits");
        if let ResponseCurve::PiecewiseLinear { points, .. } = committed {
            assert!((points[1].0 - 0.1).abs() < 1e-9);
        }
    }

    #[test]
    fn pointer_up_after_invalid_drag_reverts() {
        let curve = seed_curve();
        // An invalid mid-drag state: x values not strictly increasing.
        let dragged = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (1.0, 0.2), (1.0, 1.0)],
            symmetric: false,
        };
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        state.dragging = Some(DragInProgress { point_index: 1, bounds: (-1.0, 1.0) });
        state.pre_drag_curve = Some(curve.clone());
        let (next, committed, _) = handle_pointer_up(state, &dragged);
        assert!(next.dragging.is_none());
        // `committed` is `Result<ResponseCurve, String>`. On invalid curves
        // the handler returns Err with the validator's actual message; the
        // host body writes this to EditorState.malformed_hints[stage_id].
        // The handler does NOT carry per-body validator state.
        let err = committed.expect_err("invalid curve must not commit");
        assert!(!err.is_empty(), "validator error string surfaces");
    }

    #[test]
    fn double_click_adds_point_when_valid() {
        let curve = seed_curve();
        let cursor = (10.0 + 60.0, 20.0 + 80.0); // somewhere inside the plot
        let (_next, new_curve, changed) = handle_double_click(BodyState::default(), &curve, cursor, &rect());
        assert!(changed);
        assert!(new_curve.is_some());
    }

    #[test]
    fn context_menu_with_hover_removes_point() {
        // Multi-anchor curve so removal is allowed.
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (-0.3, -0.3), (0.3, 0.3), (1.0, 1.0)],
            false,
        )
        .unwrap();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        state.hovered_point = Some(1);
        let (next, new_curve, changed) = handle_context_menu(state, &curve);
        assert!(changed);
        let new_curve = new_curve.expect("removable hovered point yields a new curve");
        if let ResponseCurve::PiecewiseLinear { points, .. } = new_curve {
            assert_eq!(points.len(), 3);
        }
        assert!(next.hovered_point.is_none(), "hover clears after remove");
    }

    #[test]
    fn context_menu_without_hover_is_no_op() {
        let curve = seed_curve();
        let (_next, new_curve, changed) = handle_context_menu(BodyState::default(), &curve);
        assert!(!changed);
        assert!(new_curve.is_none());
    }

    #[test]
    fn interaction_uses_engine_native_coordinates() {
        // Regression: dragging the center anchor right by a known amount
        // produces a curve whose middle point's x increased, NOT y. The
        // SVG port plots input on X and output on Y, so engine-native
        // (input, output) tuples flow through unchanged. The egui code
        // plotted output on X and input on Y (see egui-ref interaction.rs:73-74,
        // 99-100 for `PlotPoint::new(visual_pos.y, visual_pos.x)`); that swap
        // was correct for the egui visual axes and is NOT a defect that this
        // test guards against. This test guards against accidentally porting
        // that visual-axis-swap logic into the SVG layer where it does not
        // belong.
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            false,
        )
        .unwrap();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        let down = (10.0 + 120.0 + 4.0, 20.0 + 120.0);
        let (state, _, _) = handle_pointer_down(state, &curve, down, &rect());
        let mv = (10.0 + 120.0 + 24.0, 20.0 + 120.0); // +20px right, same y
        let (_next, new_curve, _) = handle_pointer_move(state, &curve, mv, &rect());
        if let Some(ResponseCurve::PiecewiseLinear { points, .. }) = new_curve {
            // x should have increased; y should be ~0.
            assert!(points[1].0 > 0.05, "x must have moved right, got {}", points[1].0);
            assert!(points[1].1.abs() < 0.05, "y must stay ~0, got {}", points[1].1);
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::interaction::tests`
Expected: FAIL, module not declared.

Wire `pub(crate) mod interaction;` into `response_curve/mod.rs`.

Run again. Expected: FAIL, fns not defined.

- [ ] **Step 3: Implement `interaction.rs`**

```rust
//! Pure pointer-event handlers for the F10 curve editor body.
//!
//! These fns take and return values, never Signals. The host
//! component projects Dioxus `PointerEvent` data to the primitives
//! consumed here (cursor pos in viewport coords, plot rect),
//! invokes a handler, then writes the resulting `BodyState'` and
//! optional `ResponseCurve'` back to its signals.

use inputforge_core::processing::curves::ResponseCurve;

use super::mutation;
use super::state::{BodyState, DragInProgress};

/// Bounding box of the SVG plot in viewport pixel coordinates. The
/// plot is square; `size` covers both width and height.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlotRect {
    pub x: f64,
    pub y: f64,
    pub size: f64,
}

/// Project a viewport pixel coordinate onto the plot's `viewBox`
/// coordinate system (`-1.05..1.05` square). The returned tuple is
/// engine-native `(input, output)`: the SVG y-flip is applied here
/// (output points up) so callers downstream see engine-native data.
#[must_use]
pub(crate) fn screen_to_viewbox(cursor: (f64, f64), r: &PlotRect) -> (f64, f64) {
    if r.size <= 0.0 {
        return (0.0, 0.0);
    }
    let nx = (cursor.0 - r.x) / r.size; // 0..1 left-to-right
    let ny = (cursor.1 - r.y) / r.size; // 0..1 top-to-bottom
    let input = -1.05 + nx * 2.1;
    // Top of plot is +output (1.05); bottom is -output (-1.05).
    let output = 1.05 - ny * 2.1;
    (input, output)
}

/// Project a viewBox `(input, output)` tuple onto viewport pixel
/// coordinates. Inverse of `screen_to_viewbox`.
fn viewbox_to_screen(p: (f64, f64), r: &PlotRect) -> (f64, f64) {
    let nx = (p.0 + 1.05) / 2.1;
    let ny = (1.05 - p.1) / 2.1;
    (r.x + nx * r.size, r.y + ny * r.size)
}

/// Find the anchor in `anchors` whose projected position is within
/// `radius_px` of `cursor`. Ties broken by lowest index.
#[must_use]
pub(crate) fn nearest_anchor(
    cursor: (f64, f64),
    anchors: &[(f64, f64)],
    r: &PlotRect,
    radius_px: f64,
) -> Option<usize> {
    let radius_sq = radius_px * radius_px;
    let mut best: Option<(usize, f64)> = None;
    for (i, a) in anchors.iter().enumerate() {
        let p = viewbox_to_screen(*a, r);
        let dx = p.0 - cursor.0;
        let dy = p.1 - cursor.1;
        let d2 = dx * dx + dy * dy;
        if d2 <= radius_sq {
            match best {
                Some((_, bd)) if bd <= d2 => {}
                _ => best = Some((i, d2)),
            }
        }
    }
    best.map(|(i, _)| i)
}

pub(crate) const HIT_RADIUS_PX: f64 = 10.0;

/// Output of every handler. `next_state` always returns; `new_curve`
/// is `Some` only when this event produced a new local curve clone
/// the host should adopt as its working copy.
pub(crate) type HandlerOut = (BodyState, Option<ResponseCurve>, bool);

pub(crate) fn handle_pointer_down(
    mut state: BodyState,
    curve: &ResponseCurve,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    let Some(idx) = nearest_anchor(cursor, &state.cached_anchors, r, HIT_RADIUS_PX) else {
        return (state, None, false);
    };
    let bounds = mutation::adjacent_x_bounds(curve, idx);
    state.dragging = Some(DragInProgress { point_index: idx, bounds });
    state.pre_drag_curve = Some(curve.clone());
    (state, None, false)
}

pub(crate) fn handle_pointer_move(
    mut state: BodyState,
    curve: &ResponseCurve,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    if let Some(drag) = state.dragging.clone() {
        let p = screen_to_viewbox(cursor, r);
        let mut local = curve.clone();
        mutation::update_point_in_curve(&mut local, drag.point_index, p, drag.bounds);
        state.cache_dirty = true;
        return (state, Some(local), true);
    }
    state.hovered_point = nearest_anchor(cursor, &state.cached_anchors, r, HIT_RADIUS_PX);
    (state, None, false)
}

/// Pointer-up returns `Result<ResponseCurve, String>` so the host can
/// write the validator's actual error to `EditorState.malformed_hints`.
/// On Ok the body dispatches `SetMapping` with the new curve; on Err the
/// body writes the error string into `malformed_hints[stage_id]`, restores
/// from `pre_drag_curve`, and skips dispatch entirely.
pub(crate) fn handle_pointer_up(
    mut state: BodyState,
    working_curve: &ResponseCurve,
) -> (BodyState, Result<ResponseCurve, String>, bool) {
    if state.dragging.is_none() {
        return (state, Err(String::new()), false);
    }
    state.dragging = None;
    state.cache_dirty = true;
    match mutation::reconstruct_curve(working_curve) {
        Ok(valid) => {
            state.pre_drag_curve = None;
            (state, Ok(valid), true)
        }
        Err(err) => {
            // Revert: the host should restore from `pre_drag_curve` and
            // write `err` into `EditorState.malformed_hints[stage_id]`.
            let _revert = state.pre_drag_curve.take();
            (state, Err(err), false)
        }
    }
}

pub(crate) fn handle_double_click(
    mut state: BodyState,
    curve: &ResponseCurve,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    let p = screen_to_viewbox(cursor, r);
    // Bounds gate: a double-click outside the plot would otherwise be
    // clamped by `add_control_point` to a boundary anchor at (-1, ?) or
    // (1, ?), which is surprising UX.
    if !(-1.05..=1.05).contains(&p.0) || !(-1.05..=1.05).contains(&p.1) {
        return (state, None, false);
    }
    let mut local = curve.clone();
    if mutation::add_control_point(&mut local, p) {
        state.cache_dirty = true;
        return (state, Some(local), true);
    }
    (state, None, false)
}

pub(crate) fn handle_context_menu(
    mut state: BodyState,
    curve: &ResponseCurve,
) -> HandlerOut {
    let Some(idx) = state.hovered_point else {
        return (state, None, false);
    };
    let mut local = curve.clone();
    if mutation::remove_control_point(&mut local, idx) {
        state.hovered_point = None;
        state.cache_dirty = true;
        return (state, Some(local), true);
    }
    (state, None, false)
}
```

Do NOT add a `malformed_hint` field to `BodyState`. The validator error flows out through `handle_pointer_up`'s `Result<ResponseCurve, String>` return; the host body unwraps the `Err` and writes `editor.malformed_hints.write().insert(stage_id.clone(), err)`. Spec lines 134 and 248 mandate that hint plumbing land in `EditorState.malformed_hints` (the `Signal<HashMap<StageId, String>>` at `mapping_editor/mod.rs:233`); a parallel `BodyState` field would silently de-sync from the spec contract.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::interaction`
Expected: PASS, all 12 tests including the engine-native invariant.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve
git commit -m "feat(response_curve): pure pointer handlers + screen<->viewBox projection"
```

---

### Task 7: `keyboard.rs`, pure key handler with same-key 250ms coalesce

Pure routing-and-mutation logic for Tab / Shift-Tab / Arrow / Shift+Arrow / Home / End / Enter / Delete / Backspace / Escape. Returns the next `BodyState`, an optional new curve, an optional `KeyOutcome` describing whether to push or merge an undo entry, and a `ChangedFlag`. The 250ms coalesce window is enforced inside this fn so it stays a single pure unit.

**Tab order:** visits each draggable point (anchors AND bezier handles), skipping junctions where `segN.end` coincides with `seg(N+1).start`. The spec phrase "distinct on-screen points" is interpreted to include handles, since they are individually draggable and have their own focus ring. Tests cover both forward (`Tab`) and backward (`ShiftTab`) traversal across a junction.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/keyboard.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::state::{
        extract_anchors, BodyState,
    };

    fn seed() -> (ResponseCurve, BodyState) {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        state.focused_point = Some(1);
        (curve, state)
    }

    #[test]
    fn tab_advances_focus_no_wrap() {
        // F10 keyboard does NOT wrap at the end of the anchor list; Tab
        // returns None so the browser advances focus past the plot.
        let (curve, state) = seed();
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::Tab, 0);
        assert_eq!(next.focused_point, Some(2));
    }

    #[test]
    fn tab_skips_duplicate_bezier_endpoints() {
        // Two-segment bezier: 8 anchors, but seg2.start coincides with seg1.end.
        let curve = ResponseCurve::cubic_bezier(
            vec![
                BezierSegment {
                    start: (-1.0, -1.0),
                    control1: (-0.5, -0.5),
                    control2: (-0.25, -0.25),
                    end: (0.0, 0.0),
                },
                BezierSegment {
                    start: (0.0, 0.0),
                    control1: (0.25, 0.25),
                    control2: (0.5, 0.5),
                    end: (1.0, 1.0),
                },
            ],
            false,
        )
        .unwrap();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        state.focused_point = Some(3); // seg1.end (0,0)
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::Tab, 0);
        // Next visit must skip index 4 (seg2.start, same point) and
        // land on index 5 (seg2.control1).
        assert_eq!(next.focused_point, Some(5));
    }

    #[test]
    fn shift_tab_skips_duplicate_bezier_endpoints_backward() {
        // Same 2-segment bezier; backward navigation from seg2.control1 (idx 5)
        // must skip the duplicate junction at idx 4 and land on idx 3 (seg1.end).
        let curve = ResponseCurve::cubic_bezier(
            vec![
                BezierSegment {
                    start: (-1.0, -1.0),
                    control1: (-0.5, -0.5),
                    control2: (-0.25, -0.25),
                    end: (0.0, 0.0),
                },
                BezierSegment {
                    start: (0.0, 0.0),
                    control1: (0.25, 0.25),
                    control2: (0.5, 0.5),
                    end: (1.0, 1.0),
                },
            ],
            false,
        )
        .unwrap();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        state.focused_point = Some(5);
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::ShiftTab, 0);
        assert_eq!(
            next.focused_point,
            Some(3),
            "ShiftTab from 5 must skip junction at 4 and land on 3",
        );
    }

    #[test]
    fn enter_on_bezier_anchor_with_handle_neighbor_is_no_op() {
        // 1-segment bezier: idx 0 is anchor (seg.start), idx 1 is handle
        // (seg.control1). Enter on idx 0 should NOT insert because the
        // right neighbor (idx 1) is not an anchor.
        let curve = ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-1.0 / 3.0, -1.0 / 3.0),
                control2: (1.0 / 3.0, 1.0 / 3.0),
                end: (1.0, 1.0),
            }],
            false,
        )
        .unwrap();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        state.focused_point = Some(0);
        let (_, new_curve, _, changed) = handle_key(state, &curve, KeyInput::Enter, 0);
        assert!(!changed, "Enter must be a no-op when right neighbor is a handle");
        assert!(new_curve.is_none());
    }

    #[test]
    fn arrow_right_nudges_x_by_step() {
        let (curve, state) = seed();
        let (_next, new_curve, outcome, changed) =
            handle_key(state, &curve, KeyInput::ArrowRight { shift: false }, 1000);
        assert!(changed);
        let new_curve = new_curve.expect("nudge yields a curve");
        if let ResponseCurve::PiecewiseLinear { points, .. } = new_curve {
            assert!((points[1].0 - 0.01).abs() < 1e-9);
        }
        assert!(matches!(outcome, Some(KeyOutcome::PushUndo { .. })));
    }

    #[test]
    fn shift_arrow_uses_large_step() {
        let (curve, state) = seed();
        let (_, new_curve, _, _) =
            handle_key(state, &curve, KeyInput::ArrowRight { shift: true }, 1000);
        if let Some(ResponseCurve::PiecewiseLinear { points, .. }) = new_curve {
            assert!((points[1].0 - 0.10).abs() < 1e-9);
        }
    }

    #[test]
    fn enter_inserts_midpoint_when_focused_anchor_has_right_neighbor() {
        let (curve, state) = seed();
        let (_, new_curve, _, changed) =
            handle_key(state, &curve, KeyInput::Enter, 1000);
        assert!(changed);
        let new_curve = new_curve.expect("Enter inserts");
        if let ResponseCurve::PiecewiseLinear { points, .. } = new_curve {
            assert_eq!(points.len(), 4);
        }
    }

    #[test]
    fn enter_on_rightmost_anchor_is_no_op() {
        let (curve, mut state) = seed();
        state.focused_point = Some(2);
        let (_, new_curve, _, changed) =
            handle_key(state, &curve, KeyInput::Enter, 1000);
        assert!(!changed);
        assert!(new_curve.is_none());
    }

    #[test]
    fn delete_center_anchor_succeeds() {
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (-0.4, -0.4), (0.4, 0.4), (1.0, 1.0)],
            false,
        )
        .unwrap();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        state.focused_point = Some(1);
        let (_, new_curve, _, changed) =
            handle_key(state, &curve, KeyInput::Delete, 1000);
        assert!(changed);
        if let Some(ResponseCurve::PiecewiseLinear { points, .. }) = new_curve {
            assert_eq!(points.len(), 3);
        }
    }

    #[test]
    fn delete_edge_is_no_op() {
        let (curve, mut state) = seed();
        state.focused_point = Some(0);
        let (_, new_curve, _, changed) =
            handle_key(state, &curve, KeyInput::Delete, 1000);
        assert!(!changed);
        assert!(new_curve.is_none());
    }

    #[test]
    fn escape_during_drag_reverts() {
        let curve = ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.5, 0.5), (1.0, 1.0)], false).unwrap();
        let pre = ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
        let mut state = BodyState::default();
        state.cached_anchors = extract_anchors(&curve);
        state.dragging = Some(super::super::state::DragInProgress {
            point_index: 1,
            bounds: (-1.0, 1.0),
        });
        state.pre_drag_curve = Some(pre.clone());
        let (next, new_curve, _, _) =
            handle_key(state, &curve, KeyInput::Escape, 1000);
        assert!(next.dragging.is_none());
        let reverted = new_curve.expect("Escape during drag reverts");
        if let ResponseCurve::PiecewiseLinear { points, .. } = reverted {
            assert!(points[1].0.abs() < 1e-9);
        }
    }

    #[test]
    fn home_and_end_jump_focus() {
        let (curve, state) = seed();
        let (next, _, _, _) = handle_key(state.clone(), &curve, KeyInput::Home, 1000);
        assert_eq!(next.focused_point, Some(0));
        let (next, _, _, _) = handle_key(state, &curve, KeyInput::End, 1000);
        assert_eq!(next.focused_point, Some(2));
    }

    #[test]
    fn same_key_within_window_merges_undo() {
        let (curve, mut state) = seed();
        state.last_nudge_at_ms = Some(1000);
        state.last_nudge_key = Some(KeyKind::ArrowRight);
        let (_, _, outcome, _) =
            handle_key(state, &curve, KeyInput::ArrowRight { shift: false }, 1100);
        match outcome {
            Some(KeyOutcome::MergeUndo) => {}
            other => panic!("expected MergeUndo, got {other:?}"),
        }
    }

    #[test]
    fn same_key_after_window_pushes_new_undo() {
        let (curve, mut state) = seed();
        state.last_nudge_at_ms = Some(1000);
        state.last_nudge_key = Some(KeyKind::ArrowRight);
        let (_, _, outcome, _) =
            handle_key(state, &curve, KeyInput::ArrowRight { shift: false }, 1500);
        assert!(matches!(outcome, Some(KeyOutcome::PushUndo { .. })));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::keyboard::tests`
Expected: FAIL, module / fns not yet defined.

Wire `pub(crate) mod keyboard;` into `response_curve/mod.rs`.

Add fields to `BodyState` (modify `state.rs`):

```rust
pub last_nudge_at_ms: Option<u64>,
pub last_nudge_key: Option<crate::frame::mapping_editor::pipeline::stage_body::response_curve::keyboard::KeyKind>,
```

(or alternatively define `KeyKind` in `state.rs` and re-export from `keyboard`.)

- [ ] **Step 3: Implement `keyboard.rs`**

```rust
//! Pure keyboard handler for the F10 curve-editor body.

use inputforge_core::processing::curves::ResponseCurve;

use super::mutation;
use super::state::{BodyState, DragInProgress};

const KEY_NUDGE_STEP: f64 = 0.01;
const KEY_NUDGE_STEP_LARGE: f64 = 0.10;
const KEY_COALESCE_WINDOW_MS: u64 = 250;

/// Inputs the host normalizes from a Dioxus `KeyboardEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyInput {
    Tab,
    ShiftTab,
    ArrowLeft { shift: bool },
    ArrowRight { shift: bool },
    ArrowUp { shift: bool },
    ArrowDown { shift: bool },
    Home,
    End,
    Enter,
    Delete,
    Escape,
}

/// Coarse-grained kind used by the coalesce-window state. Two presses
/// of the same `KeyKind` within `KEY_COALESCE_WINDOW_MS` merge into a
/// single undo entry; presses of different kinds always push.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyKind {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
}

impl KeyInput {
    fn nudge_kind(self) -> Option<KeyKind> {
        Some(match self {
            Self::ArrowLeft { .. } => KeyKind::ArrowLeft,
            Self::ArrowRight { .. } => KeyKind::ArrowRight,
            Self::ArrowUp { .. } => KeyKind::ArrowUp,
            Self::ArrowDown { .. } => KeyKind::ArrowDown,
            _ => return None,
        })
    }
}

/// Tells the host how to record this key event in the undo log.
///
/// The `UndoLog` API at `mapping_editor/undo_log.rs:95-113` exposes only
/// `push_edit(key, before, kind, label)`; there is no `merge_with_top` /
/// `update_top` operation. `MergeUndo` therefore relies on the following
/// host-side contract:
///
/// 1. On the FIRST key in a coalesce streak (`PushUndo`), the host
///    captures `mapping_before = mapping_at(actions_root, mapping_key)`
///    and calls `dispatch_curve_edit(...)`, which internally calls
///    `undo_log.push_edit(key, mapping_before, StageEdit, label)`.
/// 2. On every SUBSEQUENT key in the same streak (`MergeUndo`), the host
///    calls `dispatch_curve_edit_no_undo(...)`, which dispatches
///    `EngineCommand::SetMapping` to the engine but does NOT touch the
///    undo log. The first entry's `mapping_before` already captures the
///    pre-streak state, so undo restores correctly.
/// 3. Redo replays the first nudge's `SetMapping` only (not the streak
///    total). Accepted as a deliberate UX simplification.
///
/// The 250 ms coalesce window is owned by `handle_key`; the host does not
/// need to track timing. The host MUST treat `MergeUndo` as "skip the
/// undo write but still dispatch the engine command".
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KeyOutcome {
    PushUndo { label: String },
    MergeUndo,
}

pub(crate) type KeyHandlerOut =
    (BodyState, Option<ResponseCurve>, Option<KeyOutcome>, bool);

#[must_use]
pub(crate) fn handle_key(
    mut state: BodyState,
    curve: &ResponseCurve,
    key: KeyInput,
    now_ms: u64,
) -> KeyHandlerOut {
    if let KeyInput::Escape = key {
        if state.dragging.is_some() {
            let revert = state.pre_drag_curve.take();
            state.dragging = None;
            state.cache_dirty = true;
            return (state, revert, None, false);
        }
        return (state, None, None, false);
    }

    match key {
        KeyInput::Tab | KeyInput::ShiftTab => {
            let new_focus = advance_focus(curve, &state.cached_anchors, state.focused_point, matches!(key, KeyInput::ShiftTab));
            state.focused_point = new_focus;
            return (state, None, None, false);
        }
        KeyInput::Home => {
            state.focused_point = if state.cached_anchors.is_empty() { None } else { Some(0) };
            return (state, None, None, false);
        }
        KeyInput::End => {
            state.focused_point = state.cached_anchors.len().checked_sub(1);
            return (state, None, None, false);
        }
        _ => {}
    }

    let Some(idx) = state.focused_point else {
        return (state, None, None, false);
    };

    let outcome_label_for_nudge = "curve: nudge".to_owned();

    match key {
        KeyInput::ArrowLeft { shift }
        | KeyInput::ArrowRight { shift }
        | KeyInput::ArrowUp { shift }
        | KeyInput::ArrowDown { shift } => {
            let step = if shift { KEY_NUDGE_STEP_LARGE } else { KEY_NUDGE_STEP };
            let (dx, dy) = match key {
                KeyInput::ArrowLeft { .. } => (-step, 0.0),
                KeyInput::ArrowRight { .. } => (step, 0.0),
                KeyInput::ArrowUp { .. } => (0.0, step),
                KeyInput::ArrowDown { .. } => (0.0, -step),
                _ => unreachable!(),
            };
            let cur = *state.cached_anchors.get(idx).unwrap_or(&(0.0, 0.0));
            let bounds = mutation::adjacent_x_bounds(curve, idx);
            let new_pos = (cur.0 + dx, cur.1 + dy);
            let mut local = curve.clone();
            mutation::update_point_in_curve(&mut local, idx, new_pos, bounds);
            let Some(valid) = mutation::reconstruct_curve(&local) else {
                return (state, None, None, false);
            };
            let kind = key.nudge_kind().expect("arrow key has nudge kind");
            let merge = matches!(state.last_nudge_key, Some(prev) if prev == kind)
                && state
                    .last_nudge_at_ms
                    .is_some_and(|t| now_ms.saturating_sub(t) <= KEY_COALESCE_WINDOW_MS);
            state.last_nudge_at_ms = Some(now_ms);
            state.last_nudge_key = Some(kind);
            state.cache_dirty = true;
            let outcome = if merge {
                KeyOutcome::MergeUndo
            } else {
                KeyOutcome::PushUndo { label: outcome_label_for_nudge }
            };
            (state, Some(valid), Some(outcome), true)
        }
        KeyInput::Enter => {
            // Insert at midpoint between idx and its right neighbor.
            // No-op for rightmost anchor or bezier handle.
            let local_idx = idx;
            let anchor = match state.cached_anchors.get(local_idx) {
                Some(a) => *a,
                None => return (state, None, None, false),
            };
            let next = match state.cached_anchors.get(local_idx + 1) {
                Some(n) => *n,
                None => return (state, None, None, false),
            };
            // Bezier handle filter: only insert when both points are anchors
            // (local % 4 in {0, 3} for bezier; piecewise/spline always pass).
            if !is_anchor_index(curve, local_idx) || !is_anchor_index(curve, local_idx + 1) {
                return (state, None, None, false);
            }
            let mid = ((anchor.0 + next.0) / 2.0, (anchor.1 + next.1) / 2.0);
            let mut local = curve.clone();
            if !mutation::add_control_point(&mut local, mid) {
                return (state, None, None, false);
            }
            state.cache_dirty = true;
            state.last_nudge_key = None;
            (
                state,
                Some(local),
                Some(KeyOutcome::PushUndo {
                    label: format!("curve: add point at ({:.2}, {:.2})", mid.0, mid.1),
                }),
                true,
            )
        }
        KeyInput::Delete => {
            let mut local = curve.clone();
            if !mutation::remove_control_point(&mut local, idx) {
                return (state, None, None, false);
            }
            state.cache_dirty = true;
            state.last_nudge_key = None;
            // Clamp focused index after removal.
            let new_anchors = super::state::extract_anchors(&local);
            state.focused_point = if new_anchors.is_empty() {
                None
            } else {
                Some(idx.min(new_anchors.len() - 1))
            };
            (
                state,
                Some(local),
                Some(KeyOutcome::PushUndo { label: "curve: remove point".to_owned() }),
                true,
            )
        }
        KeyInput::Tab | KeyInput::ShiftTab | KeyInput::Home | KeyInput::End | KeyInput::Escape => {
            unreachable!("handled above")
        }
    }
}

fn is_anchor_index(curve: &ResponseCurve, idx: usize) -> bool {
    match curve {
        ResponseCurve::PiecewiseLinear { .. } | ResponseCurve::CubicSpline { .. } => true,
        ResponseCurve::CubicBezier { .. } => matches!(idx % 4, 0 | 3),
    }
}

fn advance_focus(
    curve: &ResponseCurve,
    anchors: &[(f64, f64)],
    current: Option<usize>,
    backward: bool,
) -> Option<usize> {
    if anchors.is_empty() {
        return None;
    }
    let len = anchors.len();
    let start = current.map(|i| if backward { i.saturating_sub(1) } else { i + 1 }).unwrap_or(0);
    let order: Vec<usize> = if backward {
        (0..len).rev().collect()
    } else {
        (0..len).collect()
    };
    // Filter: skip duplicate-junction points (segN.end == seg(N+1).start).
    let visit_filter = |i: usize| -> bool {
        if let ResponseCurve::CubicBezier { .. } = curve {
            if i % 4 == 0 && i > 0 {
                // seg.start where i = 4*k, k > 0; coincides with prior seg.end.
                if let Some(prev) = anchors.get(i.saturating_sub(1)).copied() {
                    if let Some(here) = anchors.get(i).copied() {
                        if (prev.0 - here.0).abs() < f64::EPSILON
                            && (prev.1 - here.1).abs() < f64::EPSILON
                        {
                            return false;
                        }
                    }
                }
            }
        }
        true
    };
    // Scan in order from `start`, skipping filtered indices.
    for &i in order.iter().skip_while(|&&i| {
        if backward { i > start } else { i < start }
    }) {
        if visit_filter(i) {
            return Some(i);
        }
    }
    None
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::keyboard`
Expected: PASS, all 12 tests.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve
git commit -m "feat(response_curve): pure keyboard handler with 250ms same-key coalesce"
```

---

### Task 8: `rendering.rs`, SVG render helpers

A single module that builds the layered `<svg>` plot from `(curve, body_state, live_value, plot_size_px)`. Functions return `Element` (Dioxus `rsx!` blocks); CSS owns colors via custom properties. The body composes them top-to-bottom in the layer order from the spec table.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/rendering.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests.rs` (create the file if it does not yet exist) the SSR golden tests:

```rust
//! Integration tests for the F10 response_curve body. Pure-fn tests
//! live next to their owning module.
//!
//! Harness pattern mirrors `frame/mapping_editor/tests.rs:82-168`: a
//! `#[derive(Clone, Props, PartialEq)]` struct + a `#[component]`
//! wrapper, driven by `VirtualDom::new_with_props(Component, Props)`.
//! Free fns and tuple props are NOT supported by Dioxus 0.7's
//! `new_with_props` API.
use dioxus::prelude::*;
use dioxus_ssr::render;
use inputforge_core::processing::curves::ResponseCurve;

use crate::frame::mapping_editor::pipeline::stage_body::response_curve::{
    rendering,
    state::{BodyState, extract_anchors},
};

#[derive(Clone, Props, PartialEq)]
struct RenderHarnessProps {
    curve: ResponseCurve,
    body: BodyState,
    live: Option<f64>,
}

#[component]
fn RenderHarness(props: RenderHarnessProps) -> Element {
    rendering::render_plot(&props.curve, &props.body, props.live, 240.0)
}

fn seeded_body(curve: &ResponseCurve) -> BodyState {
    let mut body = BodyState::default();
    body.cached_path = inputforge_core::processing::curves::sample_curve_path(curve, 200);
    body.cached_anchors = extract_anchors(curve);
    body
}

#[test]
fn render_plot_emits_svg_with_grid_and_polyline() {
    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
    let body = seeded_body(&curve);
    let mut vdom = VirtualDom::new_with_props(
        RenderHarness,
        RenderHarnessProps { curve, body, live: None },
    );
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("<svg"), "must emit svg root: {html}");
    assert!(html.contains("if-curve__path"), "must include curve path class");
    assert!(html.contains("if-curve__grid-major"), "major grid class missing");
    assert!(html.contains("if-curve__identity"), "identity dashed line missing");
    // y-flip group
    assert!(
        html.contains(r#"transform="scale(1, -1)""#),
        "must apply y-flip group: {html}"
    );
    // No live dot when live_value is None.
    assert!(!html.contains("if-curve__live-dot"), "live dot must be absent");
}

#[test]
fn render_plot_with_live_value_emits_live_dot() {
    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
    let body = seeded_body(&curve);
    let mut vdom = VirtualDom::new_with_props(
        RenderHarness,
        RenderHarnessProps { curve, body, live: Some(0.42) },
    );
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("if-curve__live-dot"), "live dot must render: {html}");
    assert!(html.contains("if-curve__live-guide"), "live guide line must render");
}
```

Wire `pub(crate) mod rendering;` and `#[cfg(test)] mod tests;` into `response_curve/mod.rs`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::render_plot`
Expected: FAIL, module not implemented.

- [ ] **Step 3: Implement `rendering.rs`**

```rust
//! SVG rendering helpers for the F10 curve-editor body.
//!
//! All colors come from CSS custom properties on `.if-curve` (defined
//! in `assets/frame/response_curve.css`); render fns emit class names
//! only. The y-flip group ensures positive output points up; tick
//! labels render outside the flip so text is not mirrored.

use dioxus::prelude::*;

use inputforge_core::processing::curves::ResponseCurve;

use super::state::BodyState;

/// 0.012 viewBox units; `≈ 1.4px` at 240px rendered size.
const GLOW_STDDEV: &str = "0.012";

/// Top-level plot. Composes the layered SVG and returns it as a single
/// `Element`. Layer ordering matches the spec's stack table.
pub(crate) fn render_plot(
    curve: &ResponseCurve,
    state: &BodyState,
    live_value: Option<f64>,
    plot_size_px: f64,
) -> Element {
    let bezier_handles = matches!(curve, ResponseCurve::CubicBezier { .. });
    let live_output = live_value.map(|v| (v, curve.evaluate(v)));
    let _ = plot_size_px; // size is owned by CSS via `aspect-ratio: 1 / 1`.
    rsx! {
        svg {
            class: "if-curve__plot",
            view_box: "-1.05 -1.05 2.1 2.1",
            preserve_aspect_ratio: "xMidYMid meet",
            // aria-label lives on the focusable wrapper <div> (Task 12),
            // not on the inner <svg>. Inner <svg> exposes a <title> for
            // screen readers that descend by default.
            title { "response curve" }
            // Filter defs.
            defs {
                filter {
                    id: "if-curve-glow",
                    x: "-50%", y: "-50%", width: "200%", height: "200%",
                    feGaussianBlur { std_deviation: "{GLOW_STDDEV}" }
                }
            }
            // Background.
            rect {
                class: "if-curve__bg",
                x: "-1.05", y: "-1.05", width: "2.1", height: "2.1",
            }
            // Y-flipped layers.
            g {
                transform: "scale(1, -1)",
                {render_grid_micro()}
                {render_grid_major()}
                {render_identity()}
                if bezier_handles {
                    {render_bezier_handle_lines(curve)}
                }
                {render_curve_path(state)}
                {render_anchors(curve, state)}
                {render_handle_markers(curve, state)}
                {render_hover_ring(curve, state)}
                {render_drag_halo(curve, state)}
                {render_focus_ring(curve, state)}
                if let Some((input, output)) = live_output {
                    {render_live(input, output)}
                }
            }
            // Tick labels: outside the flip so text is upright.
            {render_tick_labels()}
        }
    }
}

fn render_grid_micro() -> Element {
    let mut nodes = Vec::with_capacity(40);
    for i in 1..20 {
        let v = -1.0 + (i as f64) * 0.1;
        if (v - v.round()).abs() < 1e-9 || (v * 4.0 - (v * 4.0).round()).abs() < 1e-9 {
            continue; // skip major positions at 0, ±0.25, ±0.5, ±1
        }
        nodes.push(rsx! {
            line {
                key: "vmicro-{i}",
                class: "if-curve__grid-micro",
                x1: "{v}", y1: "-1.0", x2: "{v}", y2: "1.0",
            }
        });
        nodes.push(rsx! {
            line {
                key: "hmicro-{i}",
                class: "if-curve__grid-micro",
                x1: "-1.0", y1: "{v}", x2: "1.0", y2: "{v}",
            }
        });
    }
    rsx! { g { {nodes.into_iter()} } }
}

fn render_grid_major() -> Element {
    let majors = [-0.75_f64, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75];
    rsx! {
        g {
            for v in majors.iter().copied() {
                line {
                    key: "vmaj-{v}",
                    class: "if-curve__grid-major",
                    x1: "{v}", y1: "-1.0", x2: "{v}", y2: "1.0",
                }
                line {
                    key: "hmaj-{v}",
                    class: "if-curve__grid-major",
                    x1: "-1.0", y1: "{v}", x2: "1.0", y2: "{v}",
                }
            }
        }
    }
}

fn render_identity() -> Element {
    rsx! {
        line {
            class: "if-curve__identity",
            x1: "-1.0", y1: "-1.0", x2: "1.0", y2: "1.0",
        }
    }
}

fn render_bezier_handle_lines(curve: &ResponseCurve) -> Element {
    let ResponseCurve::CubicBezier { segments, .. } = curve else {
        return rsx! { g {} };
    };
    rsx! {
        g {
            for (i, seg) in segments.iter().enumerate() {
                line {
                    key: "h1-{i}",
                    class: "if-curve__handle-line",
                    x1: "{seg.start.0}", y1: "{seg.start.1}",
                    x2: "{seg.control1.0}", y2: "{seg.control1.1}",
                }
                line {
                    key: "h2-{i}",
                    class: "if-curve__handle-line",
                    x1: "{seg.control2.0}", y1: "{seg.control2.1}",
                    x2: "{seg.end.0}", y2: "{seg.end.1}",
                }
            }
        }
    }
}

fn render_curve_path(state: &BodyState) -> Element {
    // 4-decimal precision = 0.0001 viewBox units, well below the 0.025
    // stroke width. Output is byte-stable across platforms (matters for
    // SSR snapshot assertions) and ~6 KB shorter than `{x},{y}` at 200
    // samples.
    let points = state
        .cached_path
        .iter()
        .map(|(x, y)| format!("{x:.4},{y:.4}"))
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        polyline {
            class: "if-curve__path",
            points: "{points}",
            fill: "none",
        }
    }
}

// Spec layer 7: anchors only.
fn render_anchors(curve: &ResponseCurve, state: &BodyState) -> Element {
    let bezier = matches!(curve, ResponseCurve::CubicBezier { .. });
    rsx! {
        g {
            for (i, &(x, y)) in state.cached_anchors.iter().enumerate() {
                if !(bezier && matches!(i % 4, 1 | 2)) {
                    circle {
                        key: "anchor-{i}",
                        class: "if-curve__anchor",
                        cx: "{x}", cy: "{y}", r: "0.04",
                    }
                }
            }
        }
    }
}

// Spec layer 8: bezier handle markers (diamonds), rendered AFTER
// anchors so handle markers stack on top when they coincide.
fn render_handle_markers(curve: &ResponseCurve, state: &BodyState) -> Element {
    let ResponseCurve::CubicBezier { .. } = curve else {
        return rsx! { g {} };
    };
    rsx! {
        g {
            for (i, &(x, y)) in state.cached_anchors.iter().enumerate() {
                if matches!(i % 4, 1 | 2) {
                    rect {
                        key: "handle-{i}",
                        class: "if-curve__handle-marker",
                        x: "{x - 0.022}", y: "{y - 0.022}",
                        width: "0.044", height: "0.044",
                        transform: "rotate(45 {x} {y})",
                    }
                }
            }
        }
    }
}

fn render_hover_ring(_curve: &ResponseCurve, state: &BodyState) -> Element {
    let Some(idx) = state.hovered_point else {
        return rsx! { g {} };
    };
    let Some(&(x, y)) = state.cached_anchors.get(idx) else {
        return rsx! { g {} };
    };
    rsx! {
        circle {
            class: "if-curve__hover-ring",
            cx: "{x}", cy: "{y}", r: "0.085",
            fill: "none",
        }
    }
}

fn render_drag_halo(_curve: &ResponseCurve, state: &BodyState) -> Element {
    let Some(drag) = state.dragging.as_ref() else {
        return rsx! { g {} };
    };
    let Some(&(x, y)) = state.cached_anchors.get(drag.point_index) else {
        return rsx! { g {} };
    };
    rsx! {
        circle {
            class: "if-curve__drag-halo",
            cx: "{x}", cy: "{y}", r: "0.07",
        }
    }
}

fn render_focus_ring(_curve: &ResponseCurve, state: &BodyState) -> Element {
    let Some(idx) = state.focused_point else {
        return rsx! { g {} };
    };
    let Some(&(x, y)) = state.cached_anchors.get(idx) else {
        return rsx! { g {} };
    };
    rsx! {
        circle {
            class: "if-curve__focus-ring",
            cx: "{x}", cy: "{y}", r: "0.105",
            fill: "none",
        }
    }
}

fn render_live(input: f64, output: f64) -> Element {
    rsx! {
        g {
            line {
                class: "if-curve__live-guide",
                x1: "-1.0", y1: "{output}", x2: "{input}", y2: "{output}",
            }
            circle {
                class: "if-curve__live-dot-halo",
                cx: "{input}", cy: "{output}", r: "0.07",
            }
            circle {
                class: "if-curve__live-dot",
                cx: "{input}", cy: "{output}", r: "0.04",
            }
        }
    }
}

fn render_tick_labels() -> Element {
    let xs = [(-1.0_f64, "-1"), (-0.5, "-.5"), (0.0, "0"), (0.5, ".5"), (1.0, "1")];
    let ys = [(-1.0_f64, "-1"), (0.0, "0"), (1.0, "1")];
    rsx! {
        g {
            class: "if-curve__ticks",
            for (x, lbl) in xs.iter().copied() {
                text {
                    key: "tx-{x}",
                    class: "if-curve__tick-label",
                    x: "{x}", y: "1.04",
                    text_anchor: "middle",
                    "{lbl}"
                }
            }
            for (y, lbl) in ys.iter().copied() {
                text {
                    key: "ty-{y}",
                    class: "if-curve__tick-label",
                    x: "-1.04", y: "{-y}",
                    text_anchor: "end",
                    "{lbl}"
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::render_plot`
Expected: PASS. Snapshots assert class names + presence of y-flip group + conditional live-dot rendering.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve
git commit -m "feat(response_curve): SVG render layers (grid, identity, anchors, live)"
```

---

### Task 9: `thumbnail.rs`, 28x14 stage-header preview

Glanceable curve preview for F9's stage header right slot. `viewBox="-1.05 -1.05 2.1 2.1"` with `preserveAspectRatio="none"` so the curve fills the wider-than-tall thumbnail. Single polyline; no grid, no anchors.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/thumbnail.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests.rs`:

```rust
#[test]
fn header_thumbnail_emits_svg_with_polyline_for_each_curve_kind() {
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::thumbnail;
    use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};

    let curves = [
        ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            false,
        )
        .unwrap(),
        ResponseCurve::cubic_spline(
            vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            false,
        )
        .unwrap(),
        ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-0.5, 0.5),
                control2: (0.5, -0.5),
                end: (1.0, 1.0),
            }],
            false,
        )
        .unwrap(),
    ];
    for c in curves {
        // Reuse the same harness pattern as `RenderHarness` above:
        // `#[derive(Clone, Props, PartialEq)]` + `#[component]`. A free
        // fn `fn h(curve) -> Element` is NOT a valid Dioxus component
        // and tuple props do not implement `Properties`.
        #[derive(Clone, Props, PartialEq)]
        struct ThumbHarnessProps { curve: ResponseCurve }
        #[component]
        fn ThumbHarness(props: ThumbHarnessProps) -> Element {
            thumbnail::header_thumbnail(&props.curve)
        }
        let mut vdom = VirtualDom::new_with_props(
            ThumbHarness,
            ThumbHarnessProps { curve: c },
        );
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("if-curve__thumbnail"), "thumbnail class missing: {html}");
        assert!(html.contains("polyline"), "thumbnail polyline missing");
        assert!(html.contains(r#"viewBox="-1.05 -1.05 2.1 2.1""#));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Wire `pub(crate) mod thumbnail;` into `response_curve/mod.rs`. Run:
`cargo test -p inputforge-gui-dx --lib response_curve::tests::header_thumbnail`
Expected: FAIL.

- [ ] **Step 3: Implement `thumbnail.rs`**

```rust
//! 28x14 inline-SVG curve thumbnail used in F9's stage-header right slot.

use dioxus::prelude::*;

use inputforge_core::processing::curves::{ResponseCurve, sample_curve_path};

const THUMBNAIL_SAMPLE_COUNT: usize = 30;

#[must_use]
pub(crate) fn header_thumbnail(curve: &ResponseCurve) -> Element {
    let samples = sample_curve_path(curve, THUMBNAIL_SAMPLE_COUNT);
    // 4-decimal precision = byte-stable across platforms for snapshot
    // tests; well below the 0.12 stroke width.
    let points = samples
        .iter()
        .map(|(x, y)| format!("{x:.4},{y:.4}"))
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        svg {
            class: "if-curve__thumbnail",
            width: "28",
            height: "14",
            view_box: "-1.05 -1.05 2.1 2.1",
            preserve_aspect_ratio: "none",
            "aria-hidden": "true",
            g {
                transform: "scale(1, -1)",
                polyline {
                    points: "{points}",
                    fill: "none",
                    stroke: "currentColor",
                    "stroke-width": "0.12",
                    "stroke-linecap": "round",
                    "stroke-linejoin": "round",
                    // `preserveAspectRatio="none"` stretches 2.1x2.1
                    // viewBox into 28x14 (2:1). Without
                    // non-scaling-stroke, vertical strokes become ~2x
                    // thicker than horizontal.
                    "vector-effect": "non-scaling-stroke",
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::header_thumbnail`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve
git commit -m "feat(response_curve): 28x14 stage-header thumbnail with sample_curve_path"
```

---

### Task 10: `toolbar.rs`, type tabs + symmetric switch + reset button

Toolbar above the plot. Three controls dispatch `EngineCommand::SetMapping` and push undo entries via the same `dispatch_curve_edit` helper used by the body. The toolbar receives all signals it needs as props rather than reading context, so it stays SSR-testable in isolation.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/toolbar.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests.rs`:

```rust
// `cmd_tx` is NOT a prop. The harness seeds it via `AppContext` per the
// `HarnessComponent` pattern in `frame/mapping_editor/tests.rs:82-168`.
// This avoids the `Sender: !PartialEq` problem entirely.
#[derive(Clone, Props, PartialEq)]
struct ToolbarHarnessProps {
    curve: ResponseCurve,
    stage_id: crate::frame::mapping_editor::undo_log::StageId,
    root_actions: Vec<inputforge_core::action::Action>,
    mapping_key: crate::frame::MappingKey,
}

#[component]
fn ToolbarHarness(props: ToolbarHarnessProps) -> Element {
    use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;
    use parking_lot::RwLock;
    use std::sync::{mpsc, Arc};
    let (cmd_tx, _rx) = mpsc::channel();
    let raw = RawHandles {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
    };
    use_context_provider(|| raw.clone());
    let meta = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let ctx = AppContext {
        state: Arc::clone(&raw.state),
        commands: raw.commands.clone(),
        settings: Arc::clone(&raw.settings),
        meta, config, live,
    };
    use_context_provider(|| ctx);
    crate::frame::mapping_editor::use_editor_state_provider();
    rsx! {
        Toolbar {
            curve: props.curve,
            stage_id: props.stage_id,
            root_actions: props.root_actions,
            mapping_key: props.mapping_key,
        }
    }
}

#[test]
fn toolbar_type_change_emits_set_mapping() {
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::toolbar::Toolbar;
    use inputforge_core::action::Action;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    let curve =
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap();
    let actions = vec![Action::ResponseCurve { curve: curve.clone() }];

    // SSR mount only verifies static markup. Click simulation is covered
    // by manual smoke tests + the keyboard/pointer pure-fn suites.
    let mapping_key = (
        "Default".to_owned(),
        InputAddress::Bound {
            device: DeviceId("dev".to_owned()),
            input: InputId::Axis { index: 0 },
        },
    );
    let stage_id = crate::frame::mapping_editor::undo_log::StageId(vec![
        crate::frame::mapping_editor::undo_log::StageIdSegment::Index(0),
    ]);
    let mut vdom = VirtualDom::new_with_props(
        ToolbarHarness,
        ToolbarHarnessProps {
            curve: curve.clone(),
            stage_id,
            root_actions: actions,
            mapping_key,
        },
    );
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Linear"), "Linear tab missing: {html}");
    assert!(html.contains("Spline"));
    assert!(html.contains("Bezier"));
    assert!(html.contains("if-switch"), "symmetric switch missing");
    assert!(html.contains("Reset"));
}
```

Wire `pub(crate) mod toolbar;` into `response_curve/mod.rs`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::toolbar_type_change_emits`
Expected: FAIL, `Toolbar` undefined.

- [ ] **Step 3: Implement `toolbar.rs`**

```rust
//! Toolbar above the plot: type segmented control + symmetric switch + reset.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::processing::curves::ResponseCurve;

use crate::components::{Button, ButtonSize, ButtonVariant, Switch};
use crate::components::tabs::{TabItem, Tabs};
use crate::context::{AppContext, ConfigSnapshot};
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{StageId, UndoKind};

use super::mutation;
use super::CurveType;

#[component]
pub(crate) fn Toolbar(
    curve: ResponseCurve,
    stage_id: StageId,
    root_actions: Vec<Action>,
    mapping_key: MappingKey,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let cmd_tx = ctx.commands.clone();
    let config_signal = ctx.config;
    let mut undo_log = editor.undo_log;

    let current_kind = match &curve {
        ResponseCurve::PiecewiseLinear { .. } => CurveType::PiecewiseLinear,
        ResponseCurve::CubicSpline { .. } => CurveType::CubicSpline,
        ResponseCurve::CubicBezier { .. } => CurveType::CubicBezier,
    };
    let current_kind_id = match current_kind {
        CurveType::PiecewiseLinear => "linear".to_owned(),
        CurveType::CubicSpline => "spline".to_owned(),
        CurveType::CubicBezier => "bezier".to_owned(),
    };
    let symmetric = matches!(
        &curve,
        ResponseCurve::PiecewiseLinear { symmetric: true, .. }
            | ResponseCurve::CubicSpline { symmetric: true, .. }
            | ResponseCurve::CubicBezier { symmetric: true, .. }
    );

    let curve_for_type = curve.clone();
    let actions_for_type = root_actions.clone();
    let key_for_type = mapping_key.clone();
    let stage_for_type = stage_id.clone();
    let cmd_for_type = cmd_tx.clone();
    let on_type_change = move |id: String| {
        let target = match id.as_str() {
            "linear" => CurveType::PiecewiseLinear,
            "spline" => CurveType::CubicSpline,
            "bezier" => CurveType::CubicBezier,
            _ => return,
        };
        if target == current_kind {
            return;
        }
        let Some(new) = mutation::convert_curve_type(&curve_for_type, target) else { return };
        let name = config_signal.read().mapping_names.get(&key_for_type.1).cloned();
        dispatch_curve_edit(
            &actions_for_type,
            &stage_for_type,
            new,
            &key_for_type,
            name,
            &cmd_for_type,
            &mut undo_log,
            format!(
                "curve: type {} -> {}",
                kind_label(current_kind),
                kind_label(target),
            ),
        );
    };

    let curve_for_sym = curve.clone();
    let actions_for_sym = root_actions.clone();
    let key_for_sym = mapping_key.clone();
    let stage_for_sym = stage_id.clone();
    let cmd_for_sym = cmd_tx.clone();
    let on_symmetric_change = move |evt: FormEvent| {
        // Switch renders <input type="checkbox">. evt.value() returns the
        // static `value` attribute (always "on") regardless of checked
        // state; use evt.checked() for the actual bit.
        let new_state = evt.data().checked();
        if new_state == symmetric {
            return;
        }
        let Some(new) = mutation::apply_symmetry(&curve_for_sym, new_state) else { return };
        let name = config_signal.read().mapping_names.get(&key_for_sym.1).cloned();
        dispatch_curve_edit(
            &actions_for_sym,
            &stage_for_sym,
            new,
            &key_for_sym,
            name,
            &cmd_for_sym,
            &mut undo_log,
            format!("curve: symmetric {}", if new_state { "on" } else { "off" }),
        );
    };

    let curve_for_reset = curve.clone();
    let actions_for_reset = root_actions.clone();
    let key_for_reset = mapping_key.clone();
    let stage_for_reset = stage_id.clone();
    let cmd_for_reset = cmd_tx.clone();
    let on_reset = move |_| {
        let new = mutation::default_identity_curve(&curve_for_reset);
        if new == curve_for_reset {
            return;
        }
        let name = config_signal.read().mapping_names.get(&key_for_reset.1).cloned();
        dispatch_curve_edit(
            &actions_for_reset,
            &stage_for_reset,
            new,
            &key_for_reset,
            name,
            &cmd_for_reset,
            &mut undo_log,
            "curve: reset".to_owned(),
        );
    };

    rsx! {
        div { class: "if-curve__toolbar",
            Tabs {
                value: current_kind_id,
                items: vec![
                    TabItem { id: "linear".to_owned(), label: "Linear".to_owned(), controls: None },
                    TabItem { id: "spline".to_owned(), label: "Spline".to_owned(), controls: None },
                    TabItem { id: "bezier".to_owned(), label: "Bezier".to_owned(), controls: None },
                ],
                onchange: on_type_change,
            }
            // `Switch::checked: ReadSignal<bool>` (see components/switch.rs:7).
            // The `bool` prop value is coerced through `IntoReadSignal`; no
            // wrapping `use_signal` is needed.
            Switch {
                checked: symmetric,
                onchange: on_symmetric_change,
                label: Some("Symmetric".to_owned()),
            }
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                onclick: on_reset,
                "Reset"
            }
        }
    }
}

fn kind_label(k: CurveType) -> &'static str {
    match k {
        CurveType::PiecewiseLinear => "linear",
        CurveType::CubicSpline => "spline",
        CurveType::CubicBezier => "bezier",
    }
}

// `name` is resolved by the caller via
// `ctx.config.read().mapping_names.get(mapping_key).cloned()`. Both the
// undo `before` snapshot and the engine command must carry the same
// `Some(name)` to preserve the user-set mapping name (mirrors F9
// amendment #2; see `name_field.rs:60-70` and `input_field.rs:87-103`).
pub(crate) fn dispatch_curve_edit(
    actions_before: &[Action],
    stage_id: &StageId,
    new_curve: ResponseCurve,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
    label: String,
) {
    let Some(new_actions) = replace_at_path(
        actions_before,
        stage_id,
        Action::ResponseCurve { curve: new_curve },
    ) else {
        return;
    };
    let before = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: name.clone(),
        actions: actions_before.to_vec(),
    };
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(target: "f10::response_curve", action = "set_mapping_drop_offline");
        return;
    }
    undo_log.write().push_edit(mapping_key.clone(), before, UndoKind::StageEdit, label);
}
```

Spot-check during implementation: confirm `Tabs::value` prop type (`String` vs `Signal<String>`) in `crates/inputforge-gui-dx/src/components/tabs.rs`. If the prop expects a signal, wrap `current_kind_id` in `use_signal(|| ...)`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::toolbar_type_change_emits`
Expected: PASS, markup contains all three tabs, the switch, and the reset button.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve
git commit -m "feat(response_curve): toolbar with type tabs, symmetric switch, reset button"
```

---

### Task 11: `ResponseCurveBody` component scaffolding (no interaction yet)

Mount the body, allocate the `Signal<BodyState>`, derive `cached_path` and `cached_anchors` whenever the projected curve changes, and render the toolbar + plot. Live-input projection lands in Task 14; pointer events in Task 12; keyboard in Task 13. This task just renders a static, correct plot.

The body takes a `root_actions: Vec<Action>` prop matching every other body in `pipeline/stage_body/mod.rs:34-78` (this prop seeds the initial render; F9's dispatcher passes the same value into every body). For LIVE projection (so cache rebuild stays reactive to undo replay, external edits, or sibling-stage edits), the body reads `ConfigSnapshot.selected_mapping_actions` from context inside its `use_effect`. The `curve` prop is similarly a one-way init seed; the live curve comes from `project_stage_curve(actions, stage_id, &curve)` where `actions` is the `Option<Vec<Action>>` from context, unwrapped to `&[]` when absent.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests.rs`:

```rust
#[test]
fn body_renders_static_plot_with_summary_and_anchors() {
    use crate::context::{AppContext, ConfigSnapshot, MetaSnapshot, LiveSnapshot, RawHandles};
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::ResponseCurveBody;
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};
    use inputforge_core::action::Action;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::sync::{Arc, mpsc};
    use parking_lot::RwLock;
    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;

    fn h() -> Element {
        let (cmd_tx, _rx) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        crate::patterns::live_capture::use_live_capture_provider();
        crate::frame::mapping_editor::use_editor_state_provider();
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta, config, live,
        };
        use_context_provider(|| ctx);

        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            false,
        )
        .unwrap();
        let stage_id = StageId(vec![StageIdSegment::Index(0)]);
        let key = (
            "Default".to_owned(),
            InputAddress::Bound {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
        );
        let root_actions = vec![Action::ResponseCurve { curve: curve.clone() }];
        rsx! {
            ResponseCurveBody {
                mapping_key: key,
                stage_id,
                curve,
                root_actions,
            }
        }
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("if-curve"), "body root class missing");
    assert!(html.contains("if-curve__plot"), "plot svg missing");
    assert!(html.contains("if-curve__path"), "polyline missing");
    assert!(html.contains("if-curve__toolbar"), "toolbar missing");
    // 3 anchors → 3 anchor circles.
    assert!(html.matches("if-curve__anchor").count() >= 3);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::body_renders_static_plot`
Expected: FAIL, `ResponseCurveBody` undefined.

- [ ] **Step 3: Implement `ResponseCurveBody`**

Append to `response_curve/mod.rs`:

```rust
use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::curves::{sample_curve_path, ResponseCurve};

use crate::context::{AppContext, ConfigSnapshot};
use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::at_path;
use crate::frame::mapping_editor::undo_log::StageId;

use self::state::{extract_anchors, BodyState};

const CURVE_SAMPLE_COUNT: usize = 200;

// `RESPONSE_CURVE_CSS` is registered centrally in `crates/inputforge-gui-dx/src/theme/mod.rs`
// alongside the other frame stylesheets (see lines 10-44 + 63-98 there for the pattern).
// Do NOT declare a per-component `Asset` here, and do NOT mount `Stylesheet { ... }`
// in this body's `rsx!`. Theme is the single owner of `<link rel="stylesheet">` mounts.

/// Project the curve at `stage_id` from the current root `actions`.
/// Falls back to the prop seed when projection fails (transient mid-edit
/// state). Helper extracted so Tasks 12, 13, 14 can share the projection.
fn project_stage_curve(actions: &[Action], stage_id: &StageId, fallback: &ResponseCurve) -> ResponseCurve {
    match at_path(actions, stage_id) {
        Some(Action::ResponseCurve { curve }) => curve.clone(),
        _ => fallback.clone(),
    }
}

#[component]
pub(crate) fn ResponseCurveBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    curve: ResponseCurve,
    /// Outermost actions vec for the mapping, threaded by F9's StageBody.
    /// Used as the initial-render seed; the live source is the context
    /// signal `ConfigSnapshot.selected_mapping_actions` (Option<Vec<Action>>).
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let config_signal = ctx.config;
    let mut body: Signal<BodyState> = use_signal(BodyState::default);

    // Reactivity: read the config signal inside the effect closure so
    // any change to `selected_mapping_actions` (own dispatch, undo
    // replay, external edit) re-fires this effect and the cache stays
    // in sync. Capturing only the prop `curve` would freeze the cache
    // at first-render values. `selected_mapping_actions` is `Option<Vec<Action>>`;
    // unwrap to `&[]` when absent (transient between mapping selection
    // and config push).
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

    // Each render: re-project the curve from the live config so toolbar
    // and render_plot see the freshest data. The prop `curve` and
    // `root_actions` are first-render seeds only. Clone the snapshot so
    // the read guard is dropped before we re-read for `stage_summary_for`.
    let cfg = config_signal.read().clone();
    let live_actions = cfg
        .selected_mapping_actions
        .clone()
        .unwrap_or_else(|| root_actions.clone());
    let live_curve = project_stage_curve(&live_actions, &stage_id, &curve);

    let body_read = body.read();
    // F9's existing summary (`Linear · 5 pts · sym` style) reused verbatim;
    // see `pipeline::stage::stage_summary_for` and `format_response_curve_summary`.
    let summary = pipeline::stage::stage_summary_for(
        &Action::ResponseCurve { curve: live_curve.clone() },
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
            // Live value is None at this scaffolding step; Task 14 wires it.
            { rendering::render_plot(&live_curve, &body_read, None, 240.0) }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::body_renders_static_plot`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve
git commit -m "feat(response_curve): static ResponseCurveBody scaffolding (toolbar + plot)"
```

---

### Task 12: Wire pointer events on the plot

Add `onpointerdown` / `onpointermove` / `onpointerup` / `ondoubleclick` / `oncontextmenu` to the SVG inside `ResponseCurveBody`. The handlers project Dioxus events to the pure-fn primitives from Task 6, write the resulting `BodyState` back to the signal, and dispatch `SetMapping` plus push undo on commit-points (drag end, double-click add, right-click remove).

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests.rs`:

```rust
#[test]
fn body_attaches_pointer_handlers_and_emits_data_attributes() {
    // SSR cannot drive PointerEvent dispatch, so this is a static
    // assertion: the plot must have the data-hovered/data-dragging
    // attributes (CSS-driven cursor) and SVG event-attribute names
    // present in the rendered markup.
    fn h() -> Element {
        // ... reuse the harness from `body_renders_static_plot` but
        // assert pointer handlers / data attrs are emitted ...
        unimplemented!("populate using body_renders_static_plot harness")
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // SVG attributes on rsx!-emitted elements show up as kebab-case.
    assert!(html.contains("data-hovered"));
    assert!(html.contains("data-dragging"));
}
```

(Drag and click logic itself is exhaustively covered by Task 6's pure-fn tests.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::body_attaches_pointer_handlers`
Expected: FAIL.

- [ ] **Step 3: Wire handlers in `ResponseCurveBody`**

Replace the `rendering::render_plot(...)` call inside `ResponseCurveBody` with a wrapper that owns a cached `PlotRect`, attaches handlers, sets data attributes, and emits the SVG plot. Because `MountedData::get_client_rect()` is async on Dioxus 0.7 (verified in `crates/inputforge-gui-dx/src/components/sortable/item.rs:100-148`), the rect is captured into a `Signal<Option<PlotRect>>` via `spawn(async ...)`. The first `pointermove` after mount is a silent no-op while the rect is `None`; subsequent events read the cached value. The aria-label moves to the focusable wrapper `<div>` so screen readers announce the role when the user tabs in.

```rust
use std::rc::Rc;

let mut plot_rect: Signal<Option<interaction::PlotRect>> = use_signal(|| None);
let mapping_key_for_evt = mapping_key.clone();
let stage_id_for_evt = stage_id.clone();
let cmd_tx = ctx.commands.clone();
let mut undo_log = use_context::<EditorState>().undo_log;
let mut malformed_hints = use_context::<EditorState>().malformed_hints;

let on_mounted = move |evt: MountedEvent| {
    let data = evt.data();
    spawn(async move {
        if let Ok(rect) = data.get_client_rect().await {
            plot_rect.set(Some(interaction::PlotRect {
                x: rect.origin.x,
                y: rect.origin.y,
                // Square plot: smaller of the two so circular hit zones
                // are not stretched if the wrapper is briefly non-square
                // mid-resize.
                size: rect.size.width.min(rect.size.height),
            }));
        }
    });
};

// Helper that turns a Dioxus PointerEvent into (cursor, PlotRect).
// Returns None when the rect cache is not yet populated.
let project_event = move |evt: &PointerEvent| -> Option<((f64, f64), interaction::PlotRect)> {
    let rect = (*plot_rect.peek()).clone()?;
    let cur = evt.client_coordinates();
    Some(((cur.x, cur.y), rect))
};
```

Add the handlers. Inside `on_pointer_down`, call `evt.set_pointer_capture()` (Dioxus 0.7 spelling: spot-check against `components/sortable/item.rs`) when a drag actually starts so events that exit the SVG keep streaming. Release on `pointerup`.

```rust
let on_pointer_down = move |evt: PointerEvent| {
    let Some((cursor, rect)) = project_event(&evt) else { return };
    let cfg = config_signal.read();
    let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
    let live_curve = project_stage_curve(actions, &stage_id_for_evt, &curve);
    drop(cfg);
    let prev = body.peek().clone();
    let (next, _, _) = interaction::handle_pointer_down(prev, &live_curve, cursor, &rect);
    if next.dragging.is_some() {
        // Pointer-capture API path: zero existing usages of
        // `set_pointer_capture` in the repo. Spot-check Dioxus 0.7
        // PointerEvent (`dioxus-html-0.7.6/src/events/pointer.rs`) before
        // committing this code: the call may be `evt.set_pointer_capture()`,
        // `evt.data().set_pointer_capture()`, or via the underlying
        // web_sys::PointerEvent through `evt.try_as_web_event()`. Without
        // capture, drags that exit the SVG drop their pointermove /
        // pointerup stream and the user gets a stuck drag. If capture is
        // not exposed on the synthetic event, the wrapper `<div>` keeps
        // receiving move/up events as long as the cursor stays inside the
        // wrapper rect, which is enough for the common case.
        let _ = evt.set_pointer_capture();
    }
    body.set(next);
};
let on_pointer_move = move |evt: PointerEvent| { /* project + handle_pointer_move; write back via body.set() */ };
let on_pointer_up = move |evt: PointerEvent| {
    /* call handle_pointer_up; on success dispatch via:
       let cfg = config_signal.read();
       let name = cfg.mapping_names.get(&mapping_key_for_evt.1).cloned();
       let actions = cfg.selected_mapping_actions.clone().unwrap_or_default();
       drop(cfg);
       toolbar::dispatch_curve_edit(&actions, &stage_id_for_evt, valid_curve,
           &mapping_key_for_evt, name, &cmd_tx, &mut undo_log, "curve: drag".to_owned());
       on validation failure write editor.malformed_hints.write().insert(stage_id, err)
       and skip dispatch. Always release pointer capture: let _ = evt.release_pointer_capture(); */
};
let on_double_click = move |evt: MouseEvent| { /* add point + dispatch with label `curve: add point at (x.xx, y.yy)` */ };
let on_context_menu = move |evt: MouseEvent| {
    // `prevent_default()` is sync on Dioxus 0.7 MouseEvent; spot-check
    // the exact spelling. If the API requires evt.data().prevent_default(),
    // adjust accordingly.
    evt.prevent_default();
    /* remove point + dispatch with label `curve: remove point` */
};

let body_snapshot = body.read();
let dragging_attr = body_snapshot.dragging.is_some().to_string();
let hovered_attr = body_snapshot.hovered_point.is_some().to_string();
drop(body_snapshot);

rsx! {
    div {
        class: "if-curve__plot-frame",
        tabindex: "0",
        "aria-label": "response curve",
        "data-hovered": "{hovered_attr}",
        "data-dragging": "{dragging_attr}",
        onpointerdown: on_pointer_down,
        onpointermove: on_pointer_move,
        onpointerup: on_pointer_up,
        ondoubleclick: on_double_click,
        oncontextmenu: on_context_menu,
        onmounted: on_mounted,
        { rendering::render_plot(&live_curve, &body.read(), None, 240.0) }
    }
}
```

The SVG inside `render_plot` keeps the `<title>response curve</title>` for screen readers that descend, but its own `aria-label` is removed (the focusable `<div>` now owns the label). See Task 8 step 3 to remove `"aria-label": "response curve"` from `<svg>` once Task 12 is in.

Each handler, after calling the pure fn:
- writes `next_state` back via `body.set(next)`;
- on `pointerup` validates, then calls `toolbar::dispatch_curve_edit(&actions, &stage_id_for_evt, valid_curve, &mapping_key_for_evt, name, &cmd_tx, &mut undo_log, "curve: drag".to_owned())` if validation passed; else writes `malformed_hints[stage_id]` with the validator error and skips dispatch;
- on `ondoubleclick` and `oncontextmenu` dispatches on success with labels `"curve: add point at (x.xx, y.yy)"` / `"curve: remove point"`. Always resolve `name` and `actions` from `config_signal.read()` at the moment of dispatch.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::body_attaches_pointer_handlers`
Expected: PASS, `data-hovered` and `data-dragging` attributes render.

Manual smoke (run in Task 18): drag an anchor, double-click to add, right-click hovered anchor to remove. Cursor changes per CSS rule on `data-dragging` / `data-hovered`.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs
git commit -m "feat(response_curve): pointer event wiring (drag, add, remove) with dispatch"
```

---

### Task 13: Wire keyboard handler into the body

Attach `onkeydown` to the plot wrapper (Task 12 already set `tabindex="0"` and `aria-label`), and route key events through `keyboard::handle_key` with a `now_ms` timestamp from `std::time::Instant`. Initial focus is NOT seeded: `focused_point` stays `None` until the user presses Tab, and `advance_focus` lands on `Some(0)` from `None` automatically.

Time source: `crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs:13` and `machine.rs:5` use `std::time::Instant`. Capture a baseline `Instant` once at component mount and compute `now_ms` as `Instant::now().saturating_duration_since(*baseline).as_millis() as u64`. `web_time` is NOT used; `Instant::EPOCH` does not exist on either `std::time::Instant` or `web_time::Instant`.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests.rs`:

```rust
#[test]
fn body_emits_tabindex_and_aria_label_on_plot() {
    // Reuse the harness pattern from `body_renders_static_plot` above.
    // The plot wrapper <div class="if-curve__plot-frame"> must carry
    // tabindex="0" and aria-label="response curve" (the latter moved
    // here from <svg> in Task 12 so screen readers announce on focus).
    fn h() -> Element {
        // (Same RawHandles+AppContext+EditorState seeding as
        // body_renders_static_plot, then mount ResponseCurveBody with
        // a 3-point identity curve.)
        unimplemented!("populate using body_renders_static_plot harness")
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains(r#"tabindex="0""#), "plot must be focusable");
    assert!(html.contains(r#"aria-label="response curve""#));
    // onkeydown listener is opaque in SSR markup; full key flow is
    // covered by Task 7 pure-fn tests.
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::body_emits_tabindex`
Expected: FAIL, `tabindex` not present yet.

- [ ] **Step 3: Wire keyboard**

Capture the time baseline once at component mount, then attach `onkeydown` to the plot frame `<div>`:

```rust
// Captured once per mount (Task 11 component body).
let time_baseline = use_signal(std::time::Instant::now);

let mapping_key_for_key = mapping_key.clone();
let stage_id_for_key = stage_id.clone();

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
    // wrap when the user reaches the end of the anchor list (the
    // outer page should advance focus past the plot). All other keys
    // are consumed locally.
    if !matches!(key, keyboard::KeyInput::Tab | keyboard::KeyInput::ShiftTab) {
        evt.prevent_default();
    }

    // Time source: std::time::Instant (matches live_capture). No
    // `Instant::EPOCH` is used; `web_time` is not pulled in.
    let now_ms = std::time::Instant::now()
        .saturating_duration_since(*time_baseline.peek())
        .as_millis() as u64;

    // Re-project curve and root actions from the live config so the
    // handler sees the freshest state (no stale prop closures).
    let cfg = config_signal.read();
    let actions: Vec<Action> = cfg
        .selected_mapping_actions
        .clone()
        .unwrap_or_default();
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
                &cmd_tx,
                &mut undo_log,
                label,
            );
        }
        Some(keyboard::KeyOutcome::MergeUndo) => {
            // Same-key burst within 250ms: dispatch the new curve to
            // the engine but do NOT push a new undo entry. The first
            // nudge of the burst already pushed an entry whose
            // `mapping_before` captures the pre-burst state, so undo
            // restores correctly. Redo replays the first nudge's
            // SetMapping (not the burst total); accepted as a
            // deliberate UX simplification.
            toolbar::dispatch_curve_edit_no_undo(
                &actions,
                &stage_id_for_key,
                new,
                &mapping_key_for_key,
                name,
                &cmd_tx,
            );
        }
        None => {
            // Escape revert: body-local only. The drag never
            // dispatched, so the engine state is already correct; no
            // dispatch is needed. (Pointer-up's revert path pulls a
            // pre-drag snapshot via `pre_drag_curve` and the same
            // no-dispatch invariant holds.)
        }
    }
};
```

Add `dispatch_curve_edit_no_undo` to `toolbar.rs`. Like `dispatch_curve_edit`, it threads `name` to preserve the user-set mapping name:

```rust
pub(crate) fn dispatch_curve_edit_no_undo(
    actions_before: &[Action],
    stage_id: &StageId,
    new_curve: ResponseCurve,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
) {
    let Some(new_actions) = replace_at_path(
        actions_before,
        stage_id,
        Action::ResponseCurve { curve: new_curve },
    ) else { return };
    let _ = cmd_tx.send(EngineCommand::SetMapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name,
        actions: new_actions,
    });
}
```

Reset coalesce state on focus loss: when the plot wrapper fires `onfocusout`, clear `last_nudge_at_ms` and `last_nudge_key` so the next nudge after refocus pushes a fresh undo entry rather than merging into a stale prior one.

```rust
let mut body_for_focusout = body;
let on_focus_out = move |_| {
    body_for_focusout.with_mut(|s| {
        s.last_nudge_at_ms = None;
        s.last_nudge_key = None;
    });
};
// Attach on the plot frame: `onfocusout: on_focus_out,`
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::body_emits_tabindex`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve
git commit -m "feat(response_curve): keyboard wiring with same-key 250ms undo coalesce"
```

---

### Task 14: Live tracking dot (top-level stages only)

Project the live input value through `evaluate_actions_through(actions, &state, &addr, stop_at)` for stages whose `stage_id` is exactly one `StageIdSegment::Index(n)` AND whose mapping key is bound to a real device (`InputAddress::Bound`). Connectivity check via `state.devices`. Pass the resulting `Some(input)` into `rendering::render_plot`.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append two tests to `tests.rs`:

```rust
#[test]
fn body_renders_live_dot_for_top_level_stage_with_connected_device() {
    // Seed AppState with a connected device pushing axis 0 = 0.4.
    // Mount body with stage_id = [Index(0)].
    // Assert html contains "if-curve__live-dot".
    // (Full harness deferred to implementation; uses the same pattern
    // as Task 11's body harness, plus a non-empty AppState.)
}

#[test]
fn body_omits_live_dot_for_nested_stage() {
    // Same seeding but stage_id = [Index(0), IfTrue, Index(0)].
    // Assert html does NOT contain "if-curve__live-dot".
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::body_renders_live_dot`
Expected: FAIL.

- [ ] **Step 3: Implement live projection**

Inside `ResponseCurveBody`, after reading the `body` signal but before rendering the plot:

```rust
use inputforge_core::pipeline::evaluate_actions_through;
use inputforge_core::types::InputValue;
use crate::frame::mapping_editor::undo_log::StageIdSegment;

// Two reads, two roles: `ctx.live` is a Signal<LiveSnapshot> updated
// at the engine's polling tick (~60Hz); reading it subscribes the
// body to that tick so the live dot re-renders on every poll. The
// actual input/output values come from `ctx.state` (the engine's
// authoritative AppState), evaluated through the same actions chain
// the engine uses, so the dot tracks the curve exactly.
let live_value: Option<f64> = (|| {
    // Gate on top-level stage only.
    let segs = &stage_id.0;
    let stop_at = match segs.as_slice() {
        [StageIdSegment::Index(n)] => *n,
        _ => return None,
    };
    // Gate on bound input. `Unbound` mappings have no device to read.
    let device_id = mapping_key.1.device()?;
    let _ = ctx.live.read(); // subscribe to ~60Hz polling tick
    let state_guard = ctx.state.try_read()?;
    // Connectivity check.
    let device_present = state_guard
        .devices
        .iter()
        .any(|d| &d.info.id == device_id && d.connected);
    if !device_present {
        return None;
    }
    // `actions` already pulled from config_signal in the body component
    // body (Task 11); reuse the local. If you re-read the signal here,
    // make sure it does not double-subscribe in a way that thrashes.
    let value = evaluate_actions_through(
        &actions,
        &state_guard,
        &mapping_key.1,
        stop_at,
    );
    drop(state_guard);
    match value {
        InputValue::Axis { value } => Some(value.value()),
        _ => None,
    }
})();
```

Pass `live_value` (instead of `None`) into `rendering::render_plot`.

Non-axis inputs (Button, Hat) silently produce `None` and the live dot is not rendered. This is intentional: ResponseCurve stages typically operate on axis-typed values, but the chain of actions before this stage may have transformed a button to an axis. The match arm returns `Some(f64)` only when the value at this stage's input boundary is axis-typed.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib response_curve::tests::body_renders_live_dot`
Expected: PASS, both tests.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs
git commit -m "feat(response_curve): live tracking dot via evaluate_actions_through (top-level only)"
```

---

### Task 15: REMOVED

The original Task 15 wired a `use_effect` keyed on `EditorState.external_edit_reset` to coordinate cache rebuild and focus clamp on external edits. That signal was removed from `EditorState` in commit `c9e7853` ("refactor(mapping-editor): drop external-edit reconciler, descope ac 27"). There is no token to subscribe to.

The defensive `clamp_focus_after_external_edit` pure helper survives but moves into Task 4 (`state.rs`), where its three unit tests (clamp down, clamp away when empty, no-op when in range) belong with the other `BodyState` mutators. The body's main `use_effect` from Task 11, which already re-fires when `config_signal` changes, can call the helper inline if the projected anchor count drops below `focused_point`. No new effect is needed.

Skip this task. Numbering is preserved so commit references and reviews stay aligned.

---

### Task 16: F9 dispatcher integration

Replace the `Action::ResponseCurve` branches in `pipeline/stage_body/mod.rs` (both `StageBody` and `header_right_slot`), and add an optional `aria_label_override: Option<String>` prop on F9's `StageHeader` so the ResponseCurve arm can override the accessible name on the existing `<button>`.

**Prerequisite:** F9 has shipped. The current placeholder lives at `pipeline/stage_body/placeholders.rs::ResponseCurvePlaceholder` and is dispatched from `pipeline/stage_body/mod.rs:96` (and the matching `header_right_slot` arm at `mod.rs:117` returns the default chevron). `StageHeader` lives at `pipeline/stage_header.rs`.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs` (replace the two `ResponseCurve` arms; add `mod response_curve;` declaration; remove the now-unused `placeholders::ResponseCurvePlaceholder` re-export if Tasks 11-15 are all landed).
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_header.rs` (add the new `aria_label_override` prop and wire it onto the existing `<button>`).
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/placeholders.rs` (delete the `ResponseCurvePlaceholder` component now that it has no caller).

- [ ] **Step 1: Write the failing test**

Append to `pipeline/tests.rs`:

```rust
#[test]
fn response_curve_stage_expanded_renders_f10_body_not_placeholder() {
    // Mount Pipeline with [Action::ResponseCurve { curve: identity }],
    // pre-expand stage 0.
    // Assert html contains "if-curve" (F10 root) AND does NOT contain
    // "F10 / F11 / F14 owns this body" (current placeholder caption from
    // pipeline/stage_body/placeholders.rs:12).
}

#[test]
fn response_curve_header_right_slot_emits_thumbnail_not_chevron() {
    // Same harness as above but check the collapsed header.
    // Assert html contains "if-curve__thumbnail" AND does NOT contain
    // the default chevron class "if-stage__chevron" (the shared Phosphor
    // icon used by every other variant; see pipeline/stage_body/mod.rs::default_chevron).
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib pipeline::tests::response_curve`
Expected: FAIL, F9's stub still in place.

- [ ] **Step 3: Replace the dispatcher arms**

In `pipeline/stage_body/mod.rs`, modify the two existing `Action::ResponseCurve { .. }` arms (currently at `mod.rs:96` in `StageBody` and `mod.rs:117` in `header_right_slot`):

`StageBody` (replaces `placeholders::ResponseCurvePlaceholder {}` at `mod.rs:96`):

```rust
Action::ResponseCurve { curve } => rsx! {
    response_curve::ResponseCurveBody {
        mapping_key: mapping_key.clone(),
        stage_id: stage_id.clone(),
        curve: curve.clone(),
        root_actions: root_actions.clone(),
    }
},
```

(F10 follows the same `root_actions: Vec<Action>` prop convention used by every other body in this dispatcher; see `mod.rs:34-78`. The body still uses `ConfigSnapshot.selected_mapping_actions` via context for the LIVE projection in its `use_effect`, but takes the prop for the initial mount.)

`header_right_slot` (replaces `default_chevron(expanded)` at `mod.rs:117`):

```rust
Action::ResponseCurve { curve } => response_curve::thumbnail::header_thumbnail(curve),
```

Spec does not give the F9 stage header a per-variant `header_summary` injection point, and F9's `pipeline::stage::stage_summary_for(action, cfg)` already returns `"Linear · 5 pts · sym"`-style output that `StageHeader` renders into `if-stage__summary`. F10 reuses that summary verbatim; no override is added at the summary level. The variant-specific accessible-name override happens at the `aria-label` level only (Step 4).

- [ ] **Step 4: Add aria_label_override prop on `StageHeader`**

Modify `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_header.rs:23-77`. Today `StageHeader` is a plain `<button class="if-stage__header">` that emits `aria-expanded` and `aria-controls` only; there is NO `aria-label` to "lift". Add a new prop:

```rust
#[component]
pub(crate) fn StageHeader(
    /* existing props */
    #[props(default)] aria_label_override: Option<String>,
) -> Element { ... }
```

When `Some(s)`, emit `aria-label: "{s}"` on the existing `<button>` (additive to `aria-expanded` / `aria-controls`); when `None`, omit the attribute entirely. Then, from the `Action::ResponseCurve` dispatcher arm in `StageBody`, pass:

```rust
aria_label_override: Some(format!(
    "Toggle stage body. Curve: {}",
    pipeline::stage::stage_summary_for(action, cfg.read().deref()),
)),
```

For every other Action variant the prop defaults to `None`. The format string is fixed by spec; the summary content is whatever `stage_summary_for` returns for that variant (so SR users hear the same human-readable curve summary that sighted users see in `if-stage__summary`).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib pipeline::tests::response_curve`
Expected: PASS, both tests.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs
git commit -m "feat(pipeline): wire F10 body + thumbnail into ResponseCurve dispatcher arms"
```

---

### Task 17: CSS file (`response_curve.css`)

Create the stylesheet referenced by `RESPONSE_CURVE_CSS` (Task 11). All colors compose from existing global tokens; nothing in `assets/tokens/` changes.

**Files:**
- Create: `crates/inputforge-gui-dx/assets/frame/response_curve.css`

- [ ] **Step 1: Skim existing patterns**

Read `crates/inputforge-gui-dx/assets/frame/mapping_editor.css` and `assets/components/tabs.css` for the established BEM + custom-property style. Match it.

- [ ] **Step 2: Write the stylesheet**

Create `crates/inputforge-gui-dx/assets/frame/response_curve.css`:

```css
.if-curve {
  --color-curve-plot-bg: var(--color-bg-sunken);
  --color-curve-grid-micro: rgba(255, 255, 255, 0.025);
  --color-curve-grid-major: rgba(255, 255, 255, 0.06);
  --color-curve-identity: var(--color-text-subtle);
  --color-curve-stroke: var(--color-primary);
  --color-curve-handle: rgb(from var(--color-primary) r g b / 0.40);
  --color-curve-anchor-fill: var(--color-text);
  --color-curve-anchor-stroke: var(--color-curve-plot-bg);

  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3);
  width: 100%;
}

.if-curve__toolbar {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--space-3);
}

.if-curve__plot-frame {
  display: flex;
  position: relative;
  width: clamp(240px, 100%, 480px);
  aspect-ratio: 1 / 1;
  cursor: default;
  outline: none;
}
.if-curve__plot-frame:focus-visible {
  outline: 2px solid var(--color-border-focus);
  outline-offset: 2px;
}
.if-curve__plot-frame[data-hovered="true"] { cursor: pointer; }
.if-curve__plot-frame[data-dragging="true"] { cursor: grabbing; }

.if-curve__plot {
  display: flex;
  width: 100%;
  height: 100%;
}

.if-curve__bg { fill: var(--color-curve-plot-bg); }
.if-curve__grid-micro { stroke: var(--color-curve-grid-micro); stroke-width: 0.005; vector-effect: non-scaling-stroke; }
.if-curve__grid-major { stroke: var(--color-curve-grid-major); stroke-width: 0.01; vector-effect: non-scaling-stroke; }
.if-curve__identity {
  stroke: var(--color-curve-identity);
  stroke-width: 0.01;
  stroke-dasharray: 0.02 0.05;
  vector-effect: non-scaling-stroke;
  fill: none;
}
.if-curve__handle-line {
  stroke: var(--color-curve-handle);
  stroke-width: 0.012;
  stroke-dasharray: 0.02 0.04;
  vector-effect: non-scaling-stroke;
}
.if-curve__path {
  stroke: var(--color-curve-stroke);
  stroke-width: 0.025;
  stroke-linecap: round;
  stroke-linejoin: round;
  filter: url(#if-curve-glow);
  vector-effect: non-scaling-stroke;
  fill: none;
}
.if-curve__anchor {
  fill: var(--color-curve-anchor-fill);
  stroke: var(--color-curve-anchor-stroke);
  stroke-width: 0.012;
  vector-effect: non-scaling-stroke;
}
.if-curve__handle-marker { fill: var(--color-curve-handle); }
.if-curve__hover-ring {
  stroke: var(--color-border-focus);
  stroke-width: 0.018;
  vector-effect: non-scaling-stroke;
  opacity: 0.55;
}
.if-curve__drag-halo {
  fill: var(--color-border-focus);
  opacity: 0.30;
}
.if-curve__focus-ring {
  stroke: var(--color-border-focus);
  stroke-width: 0.018;
  stroke-dasharray: 0.02 0.02;
  vector-effect: non-scaling-stroke;
}
.if-curve__live-guide {
  stroke: var(--color-live);
  stroke-width: 0.005;
  stroke-dasharray: 0.02 0.03;
  vector-effect: non-scaling-stroke;
  opacity: 0.5;
}
.if-curve__live-dot-halo { fill: var(--color-live); opacity: 0.18; }
.if-curve__live-dot {
  fill: var(--color-live);
  filter: url(#if-curve-glow);
}
.if-curve__tick-label {
  font-family: var(--font-mono);
  /* Unitless font-size inside SVG = user units (viewBox-relative).
     `0.075px` would resolve to 0.075 device pixels (sub-pixel, often
     floored to 0); `0.075` user units render as ~8.6 px at 240 px
     plot size (0.075 * 240 / 2.1). */
  font-size: 0.075;
  fill: var(--color-text-subtle);
}
.if-curve__thumbnail {
  width: 28px;
  height: 14px;
  color: var(--color-curve-stroke, var(--color-primary));
  fill: none;
}

```

`vector-effect: non-scaling-stroke` keeps stroke widths visually stable as the SVG scales. The unitless `font-size: 0.075` on `.if-curve__tick-label` is intentional: SVG resolves unitless lengths in user (viewBox) units, so the label renders at ~8.6 px when the plot is 240 px wide. A `0.075px` value would resolve to 0.075 device pixels and render as zero or a single black pixel.

No `@media (prefers-reduced-motion)` block is needed if every transition or animation in this stylesheet uses the global `--duration-*` tokens: `assets/tokens/motion.css:46-52` already collapses those tokens to `0ms` under the OS preference. If a transform-based animation (e.g. anchor pulse) has its own `@keyframes` rule, gate that specific rule with `@media (prefers-reduced-motion: reduce) { ... animation: none; }`, mirroring the pattern in `assets/frame/mapping_editor.css:218`. Do NOT blanket-override `transition-duration`/`animation-duration` on `.if-curve *`; that double-counts the token reset.

If a token referenced above (`--space-3`, `--color-live`, `--font-mono`, etc.) has a different name in the existing token sheets, fix the reference inline by reading `crates/inputforge-gui-dx/assets/tokens/*.css`. Do NOT add new tokens.

- [ ] **Step 3: Verify visually via `dx run`**

Stop here for SSR-only validation. Manual smoke is the next task.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/response_curve.css
git commit -m "feat(response_curve): instrument-grade plot styles with reduced-motion guard"
```

---

### Task 18: Manual smoke run + build sweep

**Files:** none modified.

- [ ] **Step 1: Build sweep**

```
cargo build -p inputforge-gui-dx
cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings
cargo test -p inputforge-core --lib processing::curves
cargo test -p inputforge-gui-dx --lib response_curve
cargo test -p inputforge-gui-dx --lib pipeline::tests::response_curve
```

(The `gui-dioxus` feature flag was removed when the egui crate was deleted; `inputforge-gui-dx` and `inputforge-app` define no features today. The pre-commit hook runs `cargo clippy --all-targets -- -D warnings` flag-free; mirror that here.)

All five expected: PASS.

- [ ] **Step 2: Manual smoke (Windows + WebView2 CDP at 9222)**

Launch the dioxus app:

```
dx run -p inputforge-app
```

Walk through the spec's interaction matrix. Record observed vs. expected for each row:

1. Select a mapping that has a `ResponseCurve` stage. Stage header right slot shows the 28x14 thumbnail; collapsed thumbnail tracks the curve shape.
2. Expand the stage. The plot renders with two-tier grid + dashed identity + tick labels.
3. Drag an anchor: cursor changes to `grabbing`, no engine dispatch during the drag, `SetMapping` fires once on pointer-up, undo log gains one entry labeled `curve: drag` (or your chosen drag label).
4. Right-click an anchor: anchor disappears, undo entry pushed.
5. Double-click an empty area inside the plot: anchor inserted, undo entry pushed.
6. Toolbar: switch type Linear → Bezier; toolbar's tabs swap; curve replaced; undo entry labeled `curve: type linear -> bezier`.
7. Toolbar: toggle Symmetric on; curve enforced to antisymmetric; undo entry `curve: symmetric on`.
8. Toolbar: click Reset on a non-identity curve; curve resets to identity of the same kind; undo entry `curve: reset`. Click Reset on an already-identity curve: nothing happens.
9. Tab into the plot. First focus highlights index 0. Tab through; bezier handles are visited; duplicate junctions are skipped.
10. Arrow keys: nudge by 0.01; Shift+Arrow: nudge by 0.10. Holding the same arrow key produces a single undo entry across the burst (250ms coalesce).
11. Enter: inserts a midpoint when focused on a non-rightmost anchor.
12. Delete / Backspace: removes the focused anchor when allowed.
13. Escape during a drag: reverts; not committed.
14. Live tracking: with a connected device pushing axis 0, the live dot tracks input along the curve. Disconnect the device: the dot disappears.
15. Place a curve stage inside a Conditional branch; expand it. Plot renders without a live dot (Conditional-nested suppression).
16. Reduced motion: enable the OS preference; transitions inside the plot are immediate.

Any discrepancy → file as a follow-up; this task does not gate on perfect parity.

- [ ] **Step 3: Commit notes (if anything was tweaked)**

If the smoke pass surfaced bug-fix-level changes (a misnamed token, a missing `evt.prevent_default()`, etc.), commit them as a follow-up under message style `fix(response_curve): <thing>`. Otherwise skip this step.

---

## Self-review checklist

After all 18 tasks:

- [ ] All spec sections covered: curve types (Q1), thumbnail right slot (Q2), instrument-grade visual floor (Q3), deferred extras recorded (Q4), toolbar layout (Q5), keyboard, live tracking, validation, edge cases.
- [ ] No `// TODO`, no placeholders, no `unimplemented!()` in shipping code (test stubs explicitly call `unimplemented!()` only inside test harness scaffolds that the task expects the implementer to fill in: those tasks must replace them before "PASS").
- [ ] Type consistency: every `dispatch_curve_edit` call uses the same `(actions_before, stage_id, new_curve, mapping_key, name, cmd_tx, undo_log, label)` signature, and every `dispatch_curve_edit_no_undo` call uses `(actions_before, stage_id, new_curve, mapping_key, name, cmd_tx)`. The `name` parameter is resolved at the call site via `ctx.config.read().mapping_names.get(&mapping_key.1).cloned()` (mirrors F9: `mapping_names` is keyed by the `InputAddress`, not by the full `(profile, address)` tuple, see `crates/inputforge-gui-dx/src/context.rs:65`).
- [ ] No swap-back to egui's `[output, input]` layout: `interaction.rs` sees engine-native `(input, output)` only; the engine-native invariant test in Task 6 enforces this.
- [ ] CSS tokens are all defined in existing `assets/tokens/*.css`; no new global tokens added.
- [ ] F9 dispatcher modifications are limited in scope: `Action::ResponseCurve` arms in `StageBody` and the per-arm header right slot, plus `StageHeader` extended with `aria_label_override: Option<String>`. Task 16's prerequisite gate confirmed before starting.
- [ ] No em-dash, en-dash, or `--` substitute anywhere in the plan: `Grep '[–—]'` returns 0 matches.
- [ ] No `web_time::Instant::EPOCH` references; time source is `std::time::Instant` against a baseline captured at component mount (matches `patterns/live_capture/mod.rs`).
- [ ] No `name: None` on `EngineCommand::SetMapping` or `Mapping::name` anywhere in F10 plan code blocks.
- [ ] Every SSR test uses the `HarnessComponent` + `HarnessProps` pattern; no `VirtualDom::new_with_props(free_fn, tuple)` shapes.
- [ ] `font-size: 0.075px` does not appear in `response_curve.css`; tick labels use unitless `font-size: 0.075` (viewBox-relative).
- [ ] Switch onchange uses `evt.data().checked()`; no `evt.value() == "true" \|\| evt.value() == "on"` fallback.
- [ ] Tab/ShiftTab keys do NOT call `evt.prevent_default()`, so the browser advances focus past the plot at wrap.
- [ ] Body's `use_effect` for cache rebuild reads `config_signal.read()` inside the closure so the effect re-fires whenever an external edit lands in the live config; does NOT depend solely on the `curve` prop (which is a one-way init seed). The `external_edit_reset` token referenced by the original plan was deleted in commit `c9e7853` and is not used.
- [ ] `set_pointer_capture` is called on `pointerdown` when a drag actually starts; released on `pointerup`.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-01-f10-curve-editor.md`. Two execution options:

**1. Subagent-Driven (recommended):** fresh subagent per task, two-stage review between tasks, fast iteration.
**2. Inline Execution:** execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints.

Which approach?
