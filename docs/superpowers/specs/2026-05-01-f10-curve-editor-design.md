# F10, Curve Editor: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-05-01
**Parent spec:** [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md), Core Screens feature F10
**IA spec:** [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md)
**Predecessor (mounting feature):** [`2026-04-30-f9-mapping-editor-design.md`](./2026-04-30-f9-mapping-editor-design.md), pipeline + stage_body dispatcher + EditorState contract + `evaluate_actions_through` helper
**Brainstorm artefacts:** wireframes persisted under `.superpowers/brainstorm/556-1777625944/content/` (`welcome.html`, `q2-right-slot.html`, `q3-visual-direction.html`, `q5-layout.html`)
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Source to port:** `crates/inputforge-gui/src/widgets/curve_editor/` (egui implementation, ~970 LoC across `mod.rs` / `mutation.rs` / `interaction.rs` / `rendering.rs` / `symmetry.rs`)

---

## Context

F10 is the curve editor body that plugs into F9's pipeline stage_body dispatcher. F9 ships a placeholder body (`F10 / F11 / F14 owns this body` caption); F10 replaces only the `Action::ResponseCurve` branch of the dispatcher and the `Action::ResponseCurve` branch of `header_right_slot()`. The rest of F9 (StageBody dispatcher signature, EditorState provider, IconButton hit area, `aria-expanded` / `aria-controls`, keyboard shortcuts handler, drag-and-drop, undo log) is invariant.

The master plan flags F10 as a signature feature. Reference quality bar: synthesizer envelope editors (Bitwig, Ableton's tools), DAW LFO designers, color-grading curve tools (DaVinci Resolve, Lightroom). The curve editor is the primary tool of the tuning session, and the user explicitly permits it to push past safe defaults.

The egui implementation already covers the feature surface (PiecewiseLinear / CubicSpline / CubicBezier curves, drag editing, double-click add, right-click remove, symmetric mode, live-input tracking). The master plan risk note forbids re-deriving the bezier math: `mutation.rs` is **ported verbatim** with two changes — `egui_plot::PlotPoint` becomes `(f64, f64)` tuples, and the `[output, input]` storage swap from the egui port is unwound (F10 stores points in engine-native `(input, output)` order).

This spec was validated section-by-section in a five-question brainstorm; choices Q1-Q5 below are recorded in order of confirmation.

---

## Confirmed design choices

**Q1. Curve types.** All three engine variants are kept: `PiecewiseLinear`, `CubicSpline`, `CubicBezier`. Parity with the egui editor; respects the master-plan risk note about not re-deriving bezier math.

**Q2. Stage header right-slot — 28×14 SVG curve thumbnail.** F10 replaces F9's default chevron with a tiny live curve preview. F9's contract for the header (32×32 IconButton hit area, `aria-expanded`, `aria-controls`) is invariant; only the visual content of the slot changes. Rationale: a stack of multiple curve stages reads at a glance ("this one is the deadband, that one is the expo") without expanding each.

**Q3. Visual ambition floor — instrument-grade.** Two-tier grid (micro at 0.1, major at ±0.5 / ±0.25 / 0), tick labels along axes, dashed identity reference, white anchors on a dark plot, subtle glow on the curve and live dot, hover focus ring. Reads as a precision instrument; matches the master-plan reference quality bar. `impeccable:bolder` and `impeccable:delight` push from this floor in implementation.

**Q4. Optional visual extras — both deferred to impeccable, recorded so the ideas are preserved.** Position trail (3-5 fading dots tracking recent live signal) and snap-to-quarter visual feedback (grid line brightens when a dragged point nears a quarter line) are not in F10's floor. They are valuable signature moves; `impeccable:bolder` may add them. Listed in the deferred-items table below.

**Q5. Body layout — toolbar above plot, no numeric input fields.** Layout, top-to-bottom: toolbar (type segmented control + symmetric switch + Reset button), then square plot. Numeric per-point inputs are out of scope for F10 (they double the body's surface and a11y work; deferred to a future feature or `impeccable:onboard` if user demand surfaces). Reset is new (egui has no explicit reset; users currently resort to type-conversion); F10 ships it as a small QoL.

---

## Non-goals (out of scope for F10)

- **Position trail** and **snap-to-quarter feedback.** Recorded as deferred; `impeccable:bolder` may add them.
- **Per-point numeric input fields.** Power-user tuning via exact value typing is deferred. Keyboard nudge with `Shift+Arrow` covers the precision case at 10% step; finer step sizes are deferred.
- **Curve presets / save-as-template.** Future feature.
- **Right-click context menu beyond remove.** Right-click removes the hovered point (F10 keeps egui parity); a richer menu (rename, duplicate, copy values) is deferred.
- **Symmetric Bezier handle pairing.** Both handles of an anchor remain independent; symmetric handle pairing across the anchor is deferred. The curve's `symmetric` flag (mirror across origin) is supported as today.
- **Sound feedback on snap or drag-end.** Out of scope.
- **Bezier handle dependency / pinned handles.** Out of scope.
- **Custom curve evaluators (e.g., user-supplied formula).** Out of scope.

---

## Architecture

### Module structure

F10 is a self-contained submodule mounted by F9's StageBody dispatcher when the action variant is `Action::ResponseCurve { curve }`. Files live under `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/`:

| File | Responsibility |
|---|---|
| `mod.rs` | `ResponseCurveBody` Dioxus component (entry point); SSR-mountable; threads `EditorState` + `ConfigSnapshot` + `LiveSnapshot` into the layered submodules. Exports `header_summary(curve) -> String` and `header_thumbnail(curve) -> Element` for F9's `header_right_slot` dispatcher to call. Owns the SVG `<defs>` block (the glow filter is defined once per body instance). |
| `state.rs` | `BodyState` struct (drag in-flight, hovered point, focused point, pre-drag snapshot, sampled-path cache, anchor cache, dirty flag) and `DragInProgress` substruct. Pure types; no Signals. |
| `mutation.rs` | Direct port of `crates/inputforge-gui/src/widgets/curve_editor/mutation.rs` lines 16-625 with two surface changes: `egui_plot::PlotPoint` → `(f64, f64)` tuples, and storage convention swap unwound (engine-native `(input, output)` throughout). Functions exported: `adjacent_x_bounds`, `update_point_in_curve`, `reconstruct_curve`, `default_identity_curve`, `convert_curve_type`, `apply_symmetry` (ported from `symmetry.rs`), `add_control_point`, `remove_control_point`, `split_bezier_segment` (private). Pure; unit-tested before any rendering lands. |
| `interaction.rs` | Pure pointer-event handler functions: `handle_pointer_down`, `handle_pointer_move`, `handle_pointer_up`, `handle_double_click`, `handle_context_menu`. Each takes `(BodyState, ResponseCurve, PointerEvent) -> (BodyState, Option<ResponseCurve>, ChangedFlag)`. Mirrors F8's `handle_key()` purity pattern; unit-testable without Dioxus types. |
| `keyboard.rs` | Pure keyboard handler `handle_key(BodyState, ResponseCurve, KeyEvent) -> (BodyState, Option<ResponseCurve>, ChangedFlag)`. Tab / Shift-Tab / Arrow / Shift+Arrow / Home / End / Enter / Delete / Backspace / Escape semantics. |
| `rendering.rs` | SVG render functions taking `(curve, body_state, live_value, theme_tokens) -> Element`. Private fns: `render_grid`, `render_identity_guide`, `render_tick_labels`, `render_curve_path`, `render_bezier_handles`, `render_control_points`, `render_focus_ring`, `render_live_tracking`. Reads CSS custom properties via classes — no inline color literals. |
| `thumbnail.rs` | `header_thumbnail(curve)` returns a 28×14 inline SVG using `viewBox="-1.05 -1.05 2.1 2.1"` and `preserveAspectRatio="none"`. Reuses `inputforge_core::processing::curves::sample_curve_path` (a small new helper extracted from the existing egui `rebuild_cache`). 30-sample polyline. |
| `toolbar.rs` | Toolbar component above the plot: F2 `Tabs` used as a 3-option segmented control (`Linear` / `Spline` / `Bezier`), F2 `Switch` (`Symmetric`), F2 `Button` (ghost variant, `Reset`). Calls into `mutation.rs` then dispatches commit. |
| `tests.rs` | SSR mount tests for the body; pure-fn tests for interaction and keyboard handlers; thumbnail snapshot equality tests; live-tracking projection tests. |

**CSS:** `crates/inputforge-gui-dx/assets/frame/response_curve.css` — owns `.if-curve__plot`, `.if-curve__grid-micro`, `.if-curve__grid-major`, `.if-curve__identity`, `.if-curve__path`, `.if-curve__handle-line`, `.if-curve__anchor`, `.if-curve__handle-marker`, `.if-curve__hover-ring`, `.if-curve__drag-halo`, `.if-curve__focus-ring`, `.if-curve__live-guide`, `.if-curve__live-dot`, `.if-curve__live-dot-halo`, `.if-curve__tick-label`, `.if-curve__toolbar`, `.if-curve__thumbnail`. CSS custom properties are scoped to `.if-curve` (see Design tokens).

**Engine side, one new helper:** `inputforge_core::processing::curves::sample_curve_path(curve: &ResponseCurve, samples: usize) -> Vec<(f64, f64)>`, extracted from the existing egui `rebuild_cache` function. Pure, lives next to `ResponseCurve`. Returns `samples`-length `Vec<(input, output)>` in engine-native order. For `CubicBezier` it samples parametrically (by `t`) per segment; for `PiecewiseLinear` / `CubicSpline` it samples evenly by input. Used by both `rendering.rs` (200 samples) and `thumbnail.rs` (30 samples). Replaces direct `bezier_x` / `bezier_y` calls in F10's render path.

### Coordinate convention

Engine `ResponseCurve` stores points as `(x, y) = (input, output)`. The egui port swaps these to `[output, input]` for `egui_plot`'s y-up convention; F10 unwinds that swap. The SVG `<svg>` uses `viewBox="-1.05 -1.05 2.1 2.1"` with an inner `<g transform="scale(1, -1)">` to flip y so positive-output is up. `mutation.rs` operates in engine-native `(input, output)` directly. Bezier math is coordinate-agnostic; the change is mechanical.

Tick labels render in a separate non-flipped `<g>` outside the scale-flip so text is not mirrored.

### Engine integration

F10 reads from `inputforge-core` via `ResponseCurve` (already serializable, validated, evaluated) and the new `sample_curve_path` helper. F10 writes to the engine via `EngineCommand::SetMapping` only, never `std::fs` or any direct profile manipulation. F9's `evaluate_actions_through(actions, &state, &addr, stop_at)` is consumed unchanged for live-input projection; F10 is its first non-test consumer.

---

## State shapes & data flow

### Local body state

Per-mounted-component, held in a `Signal<BodyState>` inside `ResponseCurveBody`:

```rust
struct BodyState {
    dragging: Option<DragInProgress>,
    hovered_point: Option<usize>,
    focused_point: Option<usize>,           // separate from hover; keyboard a11y
    pre_drag_curve: Option<ResponseCurve>,
    cached_path: Vec<(f64, f64)>,           // 200 samples; engine-native (input, output)
    cached_anchors: Vec<(f64, f64)>,        // flat list of draggable points (anchors + bezier handles)
    cache_dirty: bool,
}

struct DragInProgress {
    point_index: usize,                     // index into cached_anchors
    bounds: (f64, f64),                     // x bounds; computed once at drag start
}
```

`focused_point` exists separately from `hovered_point` so keyboard navigation is undisturbed by mouse motion and screen-reader users have a stable focus.

### Inputs read

Via `use_context::<AppContext>()` and the StageBody props provided by F9's dispatcher:

| Source | Field | Use |
|---|---|---|
| `ConfigSnapshot.selected_mapping_actions` | `Option<Vec<Action>>` | Locate this stage's `Action::ResponseCurve { curve }` at the F9-provided `stage_id` path; rebuild caches when the curve changes externally and dragging is idle. |
| `ConfigSnapshot.selected_mapping_key` | `Option<MappingKey>` | Threaded into `SetMapping` dispatch and `UndoLog::push_edit`. |
| `LiveSnapshot` (the F1 ~60Hz polling Signal) | input value at `mapping_key.1` (the primary `InputAddress`) | Feeds `evaluate_actions_through(actions, &state, &addr, stage_index)` for the live tracking dot. |
| `EditorState.expanded_stages` | `HashSet<StageId>` | Read-only — F9's dispatcher already gates whether this body renders. |
| `inputforge_core::state::AppState.devices` (via `AppContext.state` lock) | connectivity of `mapping_key.1.device` | Distinguish "real zero signal" from "no device"; if the device for the primary `InputAddress` is missing or disconnected → don't render the live dot. |

### Outputs written

| Target | Trigger | Payload |
|---|---|---|
| `EngineCommand::SetMapping` (via `AppContext.commands.send`) | drag-end · double-click add · right-click remove · type change · symmetric toggle · reset · keyboard nudge (per press) · keyboard insert · keyboard delete | Full new `actions: Vec<Action>` produced by `replace_at_path(root_actions, stage_id, Action::ResponseCurve { curve: new })`. Engine-offline (`Err`): silently dropped via `let _ =`. |
| `EditorState.undo_log` (via `push_edit`) | paired with each `SetMapping` | `UndoKind::StageEdit`, label per F9's convention, e.g., `curve: linear 5pt -> linear 5pt sym`, `curve: type linear -> bezier`, `curve: add point at (0.32, 0.18)`, `curve: reset`. |
| `EditorState.malformed_hints[stage_id]` | every render (write-only on change) | Empty string when valid; populated when validation fails. F9's stage header reads this map and surfaces the hint in the summary slot. |

### Drag pipeline (the hot path)

1. **Pointer down** on the plot. Hit-test (10px screen-space radius) against `cached_anchors` → if hit, set `dragging = Some(...)`, snapshot `pre_drag_curve = Some(curve.clone())`, capture pointer (`event.target.set_pointer_capture()`). **No engine dispatch.**
2. **Pointer move.** While `dragging.is_some()`, convert cursor to viewBox coordinates, apply `mutation::adjacent_x_bounds` + `mutation::update_point_in_curve` against a **local clone** of the curve held in `BodyState`. Set `cache_dirty = true`. Display re-renders from the local clone. Engine and `ConfigSnapshot` are not touched. This realizes F9's "drag interactions coalesce intermediate positions in local body state and dispatch `SetMapping` only on drag-end".
3. **Pointer up.** Validate via `mutation::reconstruct_curve(&local_clone)`. If `Some(valid)`: dispatch `SetMapping` with the full new actions vec, push undo entry. If `None`: revert to `pre_drag_curve`, write `malformed_hints[stage_id]` with the validator's error string, no dispatch. Either branch: clear `dragging`, clear `pre_drag_curve`, set `cache_dirty = true`.

### External-edit reconciliation

The polling task drives `ConfigSnapshot` at ~60Hz. F10's outer reactive scope reads `selected_mapping_actions` and projects this stage's `ResponseCurve` into a memoized signal.

- **`dragging.is_none()` and projected curve changed:** rebuild caches, blank `pre_drag_curve`, clamp `focused_point` to the new anchor count (`focused_point = focused_point.filter(|i| *i < new_anchor_count)`).
- **`dragging.is_some()` and projected curve changed:** drop the external update (the local clone wins until pointer-up). Matches F9's external-edit task convention; protects against a competing writer mid-drag.
- **Stage disappears entirely** (action at `stage_id` no longer `ResponseCurve`): F9's dispatcher unmounts this body before render. F10 doesn't defend against missing-stage internally; if invariant is violated, trace-log and render an inert error placeholder.

---

## SVG rendering

### Plot frame

Single `<svg>` with `viewBox="-1.05 -1.05 2.1 2.1"` and `preserveAspectRatio="xMidYMid meet"` (square, no stretch). Width responsive: `clamp(240px, 100%, 480px)`, height = width via CSS `aspect-ratio: 1 / 1`. Inner `<g transform="scale(1, -1)">` flips y; tick labels render in a separate non-flipped group so text is not mirrored.

### Layer stack (back to front)

| # | Element | Style |
|---|---|---|
| 1 | Background `<rect>` filling the viewBox | `fill: var(--color-curve-plot-bg)` |
| 2 | Micro grid (10 vertical + 10 horizontal at 0.1 spacing, skipping major positions) | `stroke: var(--color-curve-grid-micro)`, 0.5px |
| 3 | Major grid (lines at ±0.5, ±0.25, 0 on both axes) | `stroke: var(--color-curve-grid-major)`, 1px |
| 4 | Identity reference `<line>` from (-1,-1) to (1,1) | `stroke: var(--color-curve-identity)`, 1px, `stroke-dasharray: "2 5"` |
| 5 | Bezier handle dashed lines (only when `CubicBezier`) — anchor-to-handle for each of the four control points | `stroke: var(--color-curve-handle)`, 1px, `stroke-dasharray: "2 4"` |
| 6 | Curve path — `<polyline>` from `cached_path` | `stroke: var(--color-curve-stroke)`, 2.2px, `filter: url(#if-curve-glow)`, `stroke-linecap: round` |
| 7 | Anchor markers — `<circle r="0.04">` per non-handle point | `fill: var(--color-curve-anchor-fill)`, `stroke: var(--color-curve-anchor-stroke)` 1px |
| 8 | Bezier handle markers (only when `CubicBezier`, indices with `local % 4 in [1, 2]`) — `<rect>` rotated 45° (diamond), ~0.03 size | `fill: var(--color-curve-handle)` |
| 9 | Hover ring (only if `hovered_point.is_some()`) — `<circle r="0.085">`, no fill | `stroke: var(--color-focus-cyan)`, 1.5px, opacity 0.55 |
| 10 | Drag halo (only if `dragging.is_some()`) — `<circle r="0.07">` filled | `fill: var(--color-focus-cyan)`, opacity 0.30 |
| 11 | Keyboard focus ring (only if `focused_point.is_some()` AND host element matches `:focus-visible`) — `<circle r="0.105">`, no fill, dashed | `stroke: var(--color-focus-cyan)`, 1.5px, `stroke-dasharray: "2 2"` |
| 12 | Live guide — horizontal `<line>` at `y = output`, x from -1 to `input` | `stroke: var(--color-warning)`, 0.5px, `stroke-dasharray: "2 3"`, opacity 0.5 |
| 13 | Live dot halo — `<circle r="0.07">` | `fill: var(--color-warning)`, opacity 0.18 |
| 14 | Live dot core — `<circle r="0.04">` | `fill: var(--color-warning)`, `filter: url(#if-curve-glow)` |
| 15 (separate non-flipped group) | Tick labels (`-1`, `-.5`, `0`, `.5`, `1` on x; `1`, `0`, `-1` on y) | JetBrains Mono 8px, `fill: var(--color-text-subtle)` |

### Glow filter

Defined once in the body's `<defs>` block:

```xml
<filter id="if-curve-glow" x="-50%" y="-50%" width="200%" height="200%">
  <feGaussianBlur stdDeviation="0.012" />
</filter>
```

`stdDeviation` is in viewBox units; `0.012` ≈ `1.2px` at a 240×240 rendered size. Subtle.

### Design tokens

Component-scoped to `.if-curve`, declared at the top of `assets/frame/response_curve.css`. **`assets/tokens/colors.css` is not modified** — these tokens are used only by F10.

```css
.if-curve {
  --color-curve-plot-bg: var(--color-bg-sunken);
  --color-curve-grid-micro: rgba(255, 255, 255, 0.025);
  --color-curve-grid-major: rgba(255, 255, 255, 0.06);
  --color-curve-identity: var(--color-text-subtle);
  --color-curve-stroke: var(--color-focus-cyan);
  --color-curve-handle: rgba(105, 196, 224, 0.40);
  --color-curve-anchor-fill: var(--color-text);
  --color-curve-anchor-stroke: var(--color-curve-plot-bg);
}
```

Exact alpha values are starting points; `impeccable:frontend-design` pins final values during implementation. If F11 (Deadzone editor) decides to share visual language with F10 — the master plan calls them "coherent" — the shared subset (grid tiers, identity, focus ring) can be lifted to a new `assets/tokens/instruments.css` at that point. Extract-on-second-use, not now.

### Reduced motion

A single CSS rule covers all transitions inside the body:

```css
@media (prefers-reduced-motion: reduce) {
  .if-curve * {
    transition-duration: 0ms !important;
    animation-duration: 0ms !important;
  }
}
```

The live tracking dot has no CSS transition — it updates per polling frame, which is the truthful behavior. The toolbar's segmented control transitions and any focus-ring fade are covered by the rule above.

### Toolbar

Layout: a `<div class="if-curve__toolbar">` placed above the plot. Three children, left-to-right:

- **Type segmented control.** F2 `Tabs` component used as a tab strip without panels (one tab per curve kind: `Linear` / `Spline` / `Bezier`). Tabs already provides keyboard arrow navigation between options. The tab-strip-without-panels pattern is documented as a supported usage of F2 Tabs; if it is not supported in the current build, F10 introduces a thin `<SegmentedControl>` wrapper in `toolbar.rs` (a styled `<div role="tablist">` with three `<button role="tab">` children, ARIA-equivalent to Tabs). Decision lands during implementation; spec commits the surface, not the component name.
- **Symmetric switch.** F2 `Switch` component, bound to the `symmetric` flag of whichever curve variant is current.
- **Reset button.** F2 `Button`, ghost variant, label `Reset`.

Each control dispatches `SetMapping` immediately on change and pushes a paired undo entry. See *Toolbar handlers* below.

---

## Interactions

### Pointer hit-testing

All hit-tests run in screen space so the 10px radius is consistent across plot sizes. The body resolves the SVG's `getBoundingClientRect()` per pointer event and projects each `cached_anchors` entry from viewBox to screen pixels, then computes Euclidean distance to the cursor. Anchors and Bezier handles share the same 10px radius. Ties broken by lowest index.

### Pointer event handlers

| Event | Handler (in `interaction.rs`) | Behavior |
|---|---|---|
| `onpointerdown` (primary button) | `handle_pointer_down` | Hit-test → if hit, set `dragging = Some(DragInProgress { point_index, bounds })`, snapshot `pre_drag_curve`, capture pointer. If miss, no-op. |
| `onpointermove` | `handle_pointer_move` | While `dragging.is_some()`: convert cursor to viewBox coords, apply `mutation::update_point_in_curve` against the local clone, set `cache_dirty`. **No dispatch.** Else: hit-test → update `hovered_point`. |
| `onpointerup` | `handle_pointer_up` | While `dragging.is_some()`: validate via `mutation::reconstruct_curve`. Valid → dispatch + undo. Invalid → revert + write `malformed_hints[stage_id]`. Either way: clear `dragging`, `pre_drag_curve`. |
| `ondoubleclick` | `handle_double_click` | Cursor → viewBox; `mutation::add_control_point` on a local clone; if returns `true`, dispatch + push undo with label `curve: add point at (x, y)`. |
| `oncontextmenu` | `handle_context_menu` | `event.prevent_default()` always (suppresses webview menu). If `hovered_point.is_some()`: `mutation::remove_control_point` on a local clone; if `true`, dispatch + push undo. If no hover, no-op. |
| `onpointerleave` | (no special handling) | Pointer capture (`set_pointer_capture` at down) ensures `pointerup` fires regardless of position. Hover state clears via the next `onpointermove` outside the plot. |

Cursor styles are CSS-driven via `data-` attributes:

```css
.if-curve__plot { cursor: default; }
.if-curve__plot[data-hovered="true"] { cursor: pointer; }
.if-curve__plot[data-dragging="true"] { cursor: grabbing; }
```

### Keyboard interactions

The plot is `tabindex="0"`. On focus, `focused_point` defaults to `Some(0)`. All handlers are pure fns in `keyboard.rs`.

| Key | Action | Dispatch |
|---|---|---|
| `Tab` / `Shift+Tab` | Move `focused_point` forward / backward through `cached_anchors`. Wrap at end → release focus to next focusable element via browser default. | No |
| `ArrowLeft` / `ArrowRight` | Nudge focused point's x by ∓0.01, clamped via `mutation::adjacent_x_bounds`. | Yes — `SetMapping` + undo per press |
| `ArrowUp` / `ArrowDown` | Nudge focused point's y by ±0.01, clamped to [-1, 1]. | Yes — per press |
| `Shift+Arrow*` | Nudge step = 0.10. | Yes — per press |
| `Home` / `End` | Focus first / last index. | No |
| `Enter` | Insert a new control point at the midpoint between focused index and its right neighbor (only at anchor positions; bezier handles no-op on Enter). | Yes — `add` undo |
| `Delete` / `Backspace` | `mutation::remove_control_point(focused_point)`. Returns `false` for edge / center / handle → silent no-op. | Yes if removed |
| `Escape` | If `dragging.is_some()`: revert to `pre_drag_curve`, clear drag. Else no-op. | No (revert is local) |

**Per-press dispatch is intentional.** Each arrow nudge produces its own undo entry, mirroring DAW envelope-editor convention. Per-press undo coalescing (merging consecutive same-stage same-key presses) is deferred to F16 polish or `impeccable:harden`; it would be a single-place change in `UndoLog::push_edit` with no schema impact.

### Toolbar handlers (in `toolbar.rs`)

| Control | Behavior | Dispatch |
|---|---|---|
| Type segmented (F2 `Tabs`) | `mutation::convert_curve_type(curve, target)` → if `Some(new)`, dispatch + undo with label `curve: type linear -> bezier`. If returns `None`, no-op. | Yes |
| Symmetric switch (F2 `Switch`) | `mutation::apply_symmetry(curve, on)` → if `Some(new)`, dispatch + undo with label `curve: symmetric on` / `curve: symmetric off`. | Yes |
| Reset button (F2 `Button` ghost) | `mutation::default_identity_curve(curve)` (preserves current type + symmetric) → compare to current; if equal, no-op. Else dispatch + undo with label `curve: reset`. | Yes (only if changed) |

### Live tracking dot

The body receives `root_actions: Vec<Action>` and `stage_id: StageId` as props from F9's StageBody dispatcher. The stage's index in the top-level pipeline is read from `stage_id`.

**Top-level stages only.** F10 v1 renders the live tracking dot only when this curve stage sits at the top level of the pipeline (`stage_id.0.len() == 1`, single `StageIdSegment::Index(n)`). Curve stages nested inside a `Conditional` branch render their full body (axes, grid, anchors, drag, keyboard) but suppress the live dot, because `evaluate_actions_through` operates on a flat action slice and the seed-value logic for a nested branch requires walking the path. This lift is small and is the natural extension once F11 (which faces the same problem) ships; tracked under Open Questions.

```text
1. If stage_id.0 != [StageIdSegment::Index(n)] (i.e., nested) → skip; no dot.
2. Let stage_index = n.
3. Read input value via:
     evaluate_actions_through(
         actions: &root_actions,
         state:   &state_read_guard,
         addr:    &mapping_key.1,            // primary InputAddress
         stop_at: stage_index,                // value entering THIS stage
     )
4. Match on the returned InputValue:
     InputValue::Axis { value }   → input = value.value() (f64 in [-1, 1])
     InputValue::Button { .. }    → don't render (curve doesn't apply)
     InputValue::Hat { .. }       → don't render
5. Connectivity check: locate the DeviceState for mapping_key.1.device in state.devices.
   If absent or DeviceState.connected == false → don't render dot or guide.
   (The InputCache trait does not expose presence; a missing device returns a defensive
   zero, which we want to suppress visually.)
6. output = curve.evaluate(input)
7. Render the horizontal guide line from (-1, output) to (input, output), and the dot at (input, output).
```

The polling Signal fires the body's reactive scope at ~60Hz; live tracking re-projects automatically. No explicit RAF loop. If state-lock acquisition fails or the projected `(input, output)` equals the prior frame's, skip re-projection (cheap optimization, not correctness).

### Stage header summary

F9 default: `5 points · symmetric` / `5 points`. F10 refines to prepend the curve kind for at-a-glance distinction in stacked stages:

| Curve | Summary |
|---|---|
| `PiecewiseLinear`, asymmetric, 5 points | `linear · 5pt` |
| `CubicSpline`, symmetric, 5 points | `spline · 5pt · sym` |
| `CubicBezier`, asymmetric, 2 segments | `bezier · 2seg` |
| `CubicBezier`, symmetric, 2 segments | `bezier · 2seg · sym` |

`2seg` for bezier reads more naturally than `8pt` (a 2-segment bezier exposes 8 control points but the user thinks in segments).

### Stage header thumbnail

Per Q2: a 28×14 inline SVG curve preview replaces F9's default chevron in the right-slot. Renders inside the F2 IconButton's invariant 32×32 hit area, leaving room for the IconButton's own padding. `viewBox="-1.05 -1.05 2.1 2.1"` with `preserveAspectRatio="none"` so the curve fills the wider-than-tall thumbnail. Single `<polyline>` from `sample_curve_path(curve, 30)`, stroked in `--color-curve-stroke` at 0.12 viewBox units. No grid, no anchors, no live dot — the thumbnail is glanceable, not interactive. The IconButton's `aria-label` shifts from `"Toggle stage body"` (chevron) to `"Toggle stage body. Curve: <summary>"` so screen readers still announce the toggle action.

---

## Validation, malformed handling, and edge cases

### Validation flow

`mutation::reconstruct_curve` is the single validation gate. It delegates to `ResponseCurve::piecewise_linear` / `cubic_spline` / `cubic_bezier`, which already enforce `>= 2 points, strictly increasing x` and bezier endpoint continuity. Called at every commit point: drag-end, double-click add, right-click remove, keyboard nudge release, keyboard insert, keyboard delete, type change, symmetric toggle, reset.

- **Validation passes:** clear `malformed_hints[stage_id]` if previously populated, dispatch `SetMapping`, push undo.
- **Validation fails:** revert local state to `pre_drag_curve` (drag) or to the action snapshot taken before the mutation attempt (non-drag). Write the engine error string to `malformed_hints[stage_id]`. **No dispatch** — the engine never sees an invalid curve.

### Edge cases

| Case | Behavior |
|---|---|
| Empty pipeline / no `selected_mapping_actions` | F9's dispatcher doesn't mount the body. Not defended internally. |
| Body mounted but `stage_id` resolves to a non-`ResponseCurve` action | F9 invariant violation. Trace-log + render inert error placeholder. |
| Pointer-up outside SVG | Pointer capture means `pointerup` fires on the SVG; commit logic runs as usual. |
| Window resize mid-drag | Hit-testing reads `getBoundingClientRect()` per event; resize during drag is safe. |
| Tab into the body before `selected_mapping_actions` resolves | `focused_point = None`; arrow keys no-op until first render. |
| Live signal address is a button or hat | No live dot rendered. |
| Device for the primary `InputAddress` missing or disconnected in `state.devices` | No live dot rendered. |
| Bezier with focus on a handle, user presses `Enter` | Silent no-op; Enter only inserts at anchor positions. |
| Reset on a curve already at identity | Post-mutation equality check; if equal, skip dispatch and undo. |
| Two `ResponseCurve` stages in the same pipeline both expanded | Each `ResponseCurveBody` instance has its own `BodyState` Signal; live-dot, focus, and drag are independent per stage. |
| Curve becomes invalid via external edit (defensive, shouldn't happen) | The engine validates `SetMapping` payloads before persisting; F10 trusts engine state. If it occurs, the validator's error appears in `malformed_hints` on the next render. |
| User holds an arrow key (auto-repeat) | Each repeat fires a separate `KeyDown` and dispatches separately. Per-press undo entries pile up; coalescing is deferred. |

---

## Deferred to `impeccable` (recorded so the ideas don't get lost)

These are the items the brainstorm surfaced and did not commit to F10's floor. Listed here so subsequent agents and the impeccable phase have a complete index.

| Idea | Origin | Where it lands |
|---|---|---|
| Position trail (3-5 fading dots tracking recent live signal positions) | Q4(i) | `impeccable:bolder` or `impeccable:delight` during F10 implementation |
| Snap-to-quarter visual feedback (grid line brightens when point near 0, ±0.25, ±0.5, ±0.75) | Q4(ii) | `impeccable:bolder` |
| Per-point numeric input fields (x/y typing per anchor) | Q5 (option C) | Future feature or F16 polish if user demand surfaces |
| Pulsing live dot, gradient curve stroke, area-under-curve fill | Q3 (option C tricks) | `impeccable:bolder` / `impeccable:delight` |
| Per-press undo coalescing (merge consecutive same-stage same-key keyboard nudges) | Section 4 | F16 polish — single-place change in `UndoLog::push_edit` |
| Curve presets ("save as template", "load library curve") | — | Future feature, beyond F10 scope |
| Right-click context menu on points beyond simple remove | — | Future feature |
| Symmetric Bezier handles (handle1 mirrors handle2 across the anchor) | — | Possible follow-up; today both handles are independent within a segment, the curve's `symmetric` flag mirrors across origin only |
| Per-stage palette sharing with F11 (lift `--color-curve-*` to `assets/tokens/instruments.css`) | Section 3 | F11 brainstorm decides; extract-on-second-use |
| Snap-to-axis (axis-clean lines when y near 0, x near 0, etc.) | — | `impeccable:bolder` |
| Sound feedback on snap or drag-end | — | Future feature |

---

## Testing strategy

TDD throughout, pure logic before render, mirroring F8 / F9.

| Layer | Cases |
|---|---|
| `mutation.rs` | Port all existing tests from `crates/inputforge-gui/src/widgets/curve_editor/mutation.rs` `tests` module. New tests for the coordinate-convention swap (engine-native `(input, output)` instead of egui's `[output, input]`). New tests covering symmetric center-freeze for piecewise/spline; bezier handle interleaving; `apply_symmetry` round-trip; `convert_curve_type` preserves symmetric flag. |
| `interaction.rs` | Pure-fn tests for each handler. Given seed `(BodyState, ResponseCurve, MockPointerEvent)`, assert returned `(BodyState', Option<ResponseCurve'>, ChangedFlag)`. Hit, miss, drag-then-validate-pass, drag-then-validate-fail, double-click-add, double-click-add-reject, right-click-remove, right-click-no-hover. |
| `keyboard.rs` | Pure-fn tests per key path: nudge ±0.01, nudge ±0.10, nudge clamped at edge, Enter at anchor, Enter on handle (no-op), Delete edge (no-op), Delete center symmetric (no-op), Tab order with bezier (anchors + handles interleaved), Escape during drag (revert). |
| `thumbnail.rs` | Snapshot equality on rendered SVG `d` attribute for canonical curves: identity-piecewise, identity-bezier, asymmetric-piecewise, two-segment-bezier. Asserts byte-stability of the path data. |
| `mod.rs` SSR | Mount `ResponseCurveBody` via Dioxus `VirtualDom` + `dioxus_ssr::render`. Cases: identity curve renders 3 anchors and `linear · 3pt` summary; live-input absent → no live dot; live-input present → dot at expected viewBox coords; malformed → `malformed_hints` populated, no dispatch. Mirrors F8's SSR pattern. |
| `toolbar.rs` | SSR test: type change emits a `convert_curve_type` mutation; symmetric toggle calls `apply_symmetry`; reset emits `default_identity_curve`. Assert `mpsc::Receiver` saw the expected `EngineCommand::SetMapping`. |
| `inputforge_core::processing::curves::sample_curve_path` | Round-trip: 200-sample identity ≈ x for piecewise; bezier samples are continuous (no jumps > step). Used by both F10 render and F10 thumbnail. |

`egui_kittest` snapshot tests for the egui crate's curve editor remain in place until F17 deletes the egui crate. F10 ships its own SSR + pure-fn tests; no equivalent of `egui_kittest` exists for Dioxus today (parent plan open question). Coverage is acceptably reduced; the pure-fn split makes the bulk of the logic exhaustively testable.

---

## F11 coordination

The master plan declares F10 and F11 must feel coherent: same animation timing, same precision feel, same instrumented-ness. F10 commits the visual foundation:

- Two-tier grid (micro at 0.1, major at quarter / half / origin)
- Identity reference (dashed)
- White anchors on dark plot
- Subtle glow on the curve and live dot
- Hover ring + drag halo + keyboard focus ring
- Tick labels in JetBrains Mono 8px

F11 should adopt the same visual language. If F11's brainstorm agrees, the shared subset of CSS custom properties (grid tiers, identity, focus-cyan ring, live-dot warning-amber) lifts to `assets/tokens/instruments.css` at that point. F10 declares its tokens scoped to `.if-curve` to keep the lift cheap.

---

## Open questions and deferred items

- **Per-press undo coalescing.** Default (per-press undo) ships in F10. If keyboard nudge proves noisy in practice, `impeccable:harden` or F16 polish adds same-stage same-key consecutive-press merging in `UndoLog::push_edit`. Single-point change.
- **Bezier `Enter` at handle.** Currently silent no-op. If user testing shows confusion, the next-anchor neighbor could be used instead. Default ship: no-op.
- **Per-point numeric inputs.** Out of F10 scope. Reconsider after F16 polish if user demand surfaces.
- **Live-input on multi-input mappings.** F10 reads the primary `InputAddress` only. For mappings whose primary input is upstream of a `MergeAxis` stage, the live dot is the merged value at this stage's input — already correct behavior via `evaluate_actions_through`. No special handling needed.
- **Live tracking inside Conditional branches.** F10 v1 suppresses the live dot for curve stages nested inside a Conditional. Lifting this requires walking the StageId path and seeding the sub-pipeline's input from the outer pipeline's value at the Conditional's position. F11 faces the same problem; the natural fix is a small extension to `evaluate_actions_through` (or a sibling helper) that takes a `&StageId` and threads the seed. Tracked here so F11's brainstorm and implementation can decide where the helper lives.
- **Trail / snap-to-quarter visual.** Deferred to `impeccable:bolder`. Both have concrete implementation sketches in the brainstorm artefacts; neither blocks F10.

---

## Next steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce the focused implementation plan for F10.
3. F11 (Deadzone editor) brainstorm follows; F11's brainstorm should explicitly review F10's visual choices and decide whether to share them via the `instruments.css` lift.

---

## Appendix, brainstorm artefacts

Browser-rendered wireframes from the F10 brainstorm session, persisted under `.superpowers/brainstorm/556-1777625944/content/`:

- `welcome.html`, F10 scope framing.
- `q2-right-slot.html`, default chevron vs 28×14 curve thumbnail, with three sample curves.
- `q3-visual-direction.html`, Restrained / Instrument-grade / Bold-signature side-by-side at full plot scale.
- `q5-layout.html`, Egui-parity / Toolbar-above / Toolbar-above-with-numeric-inputs.
- `waiting.html`, decision summary.
