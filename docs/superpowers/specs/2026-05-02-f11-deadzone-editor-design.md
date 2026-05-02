# F11, Deadzone Editor: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-05-02
**Parent spec:** [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md), Core Screens feature F11
**IA spec:** [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md)
**Sibling instrument (visual + structural template):** [`2026-05-01-f10-curve-editor-design.md`](./2026-05-01-f10-curve-editor-design.md)
**Predecessor (mounting feature):** [`2026-04-30-f9-mapping-editor-design.md`](./2026-04-30-f9-mapping-editor-design.md), pipeline + stage_body dispatcher + EditorState contract
**Brainstorm artefacts:** wireframes persisted under `.superpowers/brainstorm/9530-1777752112/content/` (`q1-visualization.html`, `q1-visualization-v2.html`, `q2-body-layout.html`, `q3-thumbnail.html`).
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Product brief:** [`/PRODUCT.md`](../../PRODUCT.md)
**Engine source:** `crates/inputforge-core/src/processing/deadzone.rs` (`DeadzoneConfig` + `apply()`, validation in `new()`)

> Notation: feature codes (`F2`, `F8`, `F9`, `F10`, `F11`, `F16`, `F17`) refer to features in the parent rewrite spec `2026-04-24-egui-to-dioxus-rewrite-design.md` (Core Screens / Components feature index).

---

## Context

F11 is the deadzone editor body that plugs into F9's pipeline `StageBody` dispatcher. F9 ships a placeholder body (`placeholders::DeadzonePlaceholder`); F11 replaces only the `Action::Deadzone` arm of the dispatcher (`crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs:105`) and the `Action::Deadzone` arm of `header_right_slot()` (line 127, currently `default_chevron`). The rest of F9 (StageBody dispatcher signature, EditorState provider, IconButton hit area, `aria-expanded` / `aria-controls`, keyboard shortcuts handler, drag-and-drop, undo log) is invariant.

The master plan flags F11 as a signature feature alongside F10. PRODUCT.md says "live data is the contract" and "power-user defaults, no apologies. Density over whitespace. Numeric inputs over sliders when precision matters." DESIGN.md commits the Evolved Glass Cockpit visual language F10 already realizes, which F11 inherits without divergence. F5 commits the "input axis vs deadzone-applied output" framing with "drag handles on the curve" plus numeric input fields.

The egui implementation (`crates/inputforge-gui/src/widgets/deadzone_editor.rs`, deleted in commit `2271256` along with the rest of the egui crate) used a horizontal 5-zone bar plus four `DragValue` rows. F11 keeps the engine contract (`DeadzoneConfig::new(low, center_low, center_high, high)` validation gate, `apply(input) -> output` evaluation) and re-shapes the visualization to a square XY plot coherent with F10. Zone semantics are preserved as background bands behind the curve.

This spec was validated section-by-section in a brainstorm that picked one option per question (Q1-Q7 below). F11 implementation begins with a refactor task that extracts a small shared `instruments/` module from F10 before any F11-specific code lands.

---

## Confirmed design choices

The decisions below are recorded in order of confirmation.

**Q1. Visualization geometry: D1, curve plot with zone-banded background.** Square XY plot (input X, output Y) at `viewBox="-1.05 -1.05 2.1 2.1"`, identity diagonal reference, four threshold handles sit on the kinks of the deadzone curve. Five vertical zone bands sit behind the curve (under the layer stack): saturated bands `var(--color-error)` at 12% opacity (semantically = clipping, the genuine pain), ramp bands `var(--color-primary)` at 6% (semantically = action), dead band `var(--color-text-subtle)` at 10% (semantically = absence). Zone semantics from the egui original are recovered inside F10's plot scaffolding.

**Q2. Body layout: E1, numeric row above plot.** Four labelled `NumberInput` fields (`Low`, `CL`, `CH`, `High`) plus `Reset` (ghost button) on the right, in a single horizontal toolbar above the plot. Mirrors F10's toolbar slot. Tab order goes left-to-right through the fields, then into the plot. Field labels stay short to honour the dense-cluster typography rule. PRODUCT.md "numeric inputs over sliders when precision matters" + the small handle count (4) make this the right tradeoff vs F10's plot-only choice.

**Q3. Stage header right-slot: F1, mini zone bar (28×14).** A 5-zone strip rendered at the slot's natural 2:1 aspect ratio (no aspect distortion needed; the bar IS rectangular). Distinct at a glance from F10's curve thumbnail in a stack of mixed stages. Sat=red 55%, ramp=blue 30%, dead=`var(--color-bg-sunken)`, threshold marks as 0.4px white-with-50%-opacity vertical lines.

**Q4. Symmetric mode: G1, no symmetric toggle.** Engine-faithful, simplest. 4 independent handles always. The egui original had no symmetric toggle either; calibration handles the typical "stick is centered" case upstream. Marked as a possible follow-up in the deferred-items table if user demand surfaces.

**Q5. Shared instrument tokens: lift to `assets/tokens/instruments.css`.** F10 spec explicitly flagged this as the F11 decision point ("extract on second use"). New CSS file declares `--instr-grid-major`, `--instr-axis-cross`, `--instr-identity-stroke`. Rust constant `instruments::INSTR_GLOW_STDDEV: f64 = 0.012` (consumed by SVG `feGaussianBlur` per body, since SVG attributes can't read CSS vars). F10's `response_curve.css` is amended in the same PR to consume the new tokens; F10's `.if-curve` block keeps only its truly-curve-specific tokens.

**Q6. Live tracking inside Conditional: F11 v1 mirrors F10 v1 punt.** Same gate as F10 (`stage_id.0.len() == 1` → no live dot for nested-in-Conditional deadzone stages). Same Open Question carried into F16 (where the natural fix is a `&StageId`-aware extension to `evaluate_actions_through` shared by both editors).

**Q7. Visual extras: F11 v1 floor matches F10 v1 floor exactly.** No position trail, no snap-to-quarter feedback, no snap-to-axis in v1. Recorded as deferred for `impeccable:bolder` during implementation. Coherence is the explicit constraint: if F10 ships without trail and F11 ships with it, the two editors feel mismatched.

---

## Non-goals (out of scope for F11)

- **Position trail** and **snap-to-quarter feedback.** Recorded as deferred; `impeccable:bolder` may add them.
- **Symmetric mode toggle** and paired-handle dragging. Q4 = G1.
- **Y-axis dragging on handles.** Each handle's Y is fixed (`Low` / `High` at y=±1, `CenterLow` / `CenterHigh` at y=0); only X is editable. Cursor delta in Y is ignored during drag. Y is determined by the deadzone semantics, not the user.
- **Per-handle numeric tooltip on hover.** The numeric row above the plot already exposes all four values; per-handle floating callouts would duplicate.
- **Reset confirmation dialog.** Reset is a one-key undo away (`Ctrl+Z`); a confirmation dialog would violate F5's "auto-commit + session undo" model.
- **Deadzone presets / save-as-template.** Future feature.
- **Right-click context menu on handles.** F11 silently ignores right-click on handles. Future feature if user demand surfaces.
- **Sound feedback on snap or drag-end.** Out of scope.
- **Custom evaluator (e.g., user-supplied formula for the active ramp).** Out of scope.
- **`aria-live` announcement on refused operations.** Deferred to `impeccable:harden`.

---

## Architecture

### Mount points

F11 replaces two arms in `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`:

```rust
// Before (F9 placeholders / default chevron):
Action::Deadzone { .. } => rsx! { placeholders::DeadzonePlaceholder {} },
// header_right_slot:
Action::Deadzone { .. } => default_chevron(expanded),

// After (F11):
Action::Deadzone { config } => rsx! {
    deadzone::DeadzoneBody {
        mapping_key: mapping_key.clone(),
        stage_id: stage_id.clone(),
        config: config.clone(),
        root_actions: root_actions.clone(),
    }
},
// header_right_slot:
Action::Deadzone { config } => deadzone::thumbnail::header_thumbnail(config),
```

F9's `StageBody` props (`mapping_key`, `stage_id`, `action`, `root_actions`), `EditorState` provider, IconButton 32×32 hit area, `aria-expanded`, `aria-controls`, keyboard shortcuts handler, drag-and-drop, and undo log are all invariant.

### Module structure

F11 lives at `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/`, mirroring F10's `response_curve/`:

| File | Responsibility |
|---|---|
| `mod.rs` | `DeadzoneBody` Dioxus component (entry point); SSR-mountable; threads `EditorState` + `ConfigSnapshot` + `LiveSnapshot` into the layered submodules. Re-exports `thumbnail::header_thumbnail` for F9's `header_right_slot` dispatcher. Owns the SVG `<defs>` block (the `if-instr-glow` filter). |
| `state.rs` | `BodyState` struct (drag in-flight, hovered handle, focused handle, pre-drag snapshot, embedded `NudgeCoalesce`) and `DragInProgress` substruct. Pure types; no Signals. `HandleId` enum (`Low | CenterLow | CenterHigh | High`). |
| `mutation.rs` | Pure handle-mutation functions: `adjacent_bounds(handle, config) -> (f64, f64)`, `with_handle(config, handle, new_x) -> Result<DeadzoneConfig>` (delegating to `DeadzoneConfig::new`), `default_config() -> DeadzoneConfig` (alias for `DeadzoneConfig::default()`), `handle_positions(config) -> [(f64, f64); 4]` returning the four viewBox `(x, y)` coords. No new engine code; no helpers added to `inputforge-core`. |
| `interaction.rs` | Pure pointer-event handler functions: `handle_pointer_down`, `handle_pointer_move`, `handle_pointer_up`. Each takes `(BodyState, DeadzoneConfig, MockPointerEvent) -> (BodyState, Option<DeadzoneConfig>, ChangedFlag)`. Mirrors F10's purity pattern; unit-testable without Dioxus types. |
| `keyboard.rs` | Pure keyboard handler `handle_key(BodyState, DeadzoneConfig, KeyInput, now_ms) -> (BodyState, Option<DeadzoneConfig>, KeyOutcome)`. Tab / Shift-Tab / Arrow / Shift+Arrow / Home / End / Escape semantics. Same `KeyOutcome::PushUndo` / `MergeUndo` / `None` shape as F10, sharing the 250ms coalesce decision via `instruments::nudge_coalesce`. |
| `rendering.rs` | SVG render functions taking `(config, body_state, live_value) -> Element`. Private fns: `render_zone_bands`, `render_grid`, `render_axis_cross`, `render_identity_guide`, `render_curve_path`, `render_handles`, `render_focus_ring`, `render_live_tracking`. Reads CSS custom properties via classes; no inline color literals. |
| `thumbnail.rs` | `header_thumbnail(config)` returns the F1 mini zone bar at `viewBox="0 0 28 14"` (no aspect distortion). Sat = `var(--color-error)` 55%, ramp = `var(--color-primary)` 30%, dead = `var(--color-bg-sunken)`, threshold marks at 0.4px stroke. No grid, no live marker, no interactivity. |
| `toolbar.rs` | Numeric row component: 4 `NumberInput` fields + `Reset` (`Button` ghost variant). Each numeric input dispatches `SetMapping` on commit (Enter or blur) via `instruments::stage_dispatch::dispatch_stage_edit`. |
| `tests.rs` | SSR mount tests for the body; pure-fn tests for interaction / keyboard / mutation; thumbnail snapshot equality tests; live-tracking projection tests. |

### Shared instruments module (the Q5 lift, code half)

F11 begins with a refactor task that extracts four cross-instrument concerns from F10 into a sibling module:

```
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/
├── instruments/                           ← NEW (extracted from F10 in F11 task 1)
│   ├── mod.rs                             re-exports + INSTR_GLOW_STDDEV constant
│   ├── live_axis.rs                       compute_live_axis_value(stage_id, &addr, &ctx, &actions) -> Option<f64>
│   ├── bridge.rs                          BridgeEvent, BRIDGE_JS_TEMPLATE, mount_mouse_bridge(plot_dom_id, dispatch_fn) -> on_mounted closure
│   ├── nudge_coalesce.rs                  NudgeCoalesce struct + should_merge(now_ms, key) -> bool
│   └── stage_dispatch.rs                  dispatch_stage_edit, dispatch_stage_edit_no_undo (generic over Action variant)
├── response_curve/                        (F10, refactored in same task to consume instruments::*)
└── deadzone/                              (F11, consumes instruments::*)
```

**`mount_mouse_bridge` signature.** Each editor builds a per-instrument dispatch closure that captures its `Signal<BodyState>`, `Signal<ConfigSnapshot>`, mapping_key, stage_id, undo_log, malformed_hints, and engine cmd_tx; the shared helper signature is intentionally narrow:

```rust
pub fn mount_mouse_bridge(
    plot_dom_id: &str,
    dispatch_fn: impl Fn(BridgeEvent) + 'static,
) -> EventHandler<MountedEvent>
```

The shared module owns: `BridgeEvent` (kind + viewBox-projected position + plot rect), `BRIDGE_JS_TEMPLATE` (the JS string), the `on_mounted` factory that injects the template via `document::eval` and forwards parsed messages to `dispatch_fn`, and the listener-cleanup logic from commit `55ed19c`. F10's existing `dispatch_bridge_event` (10-parameter helper at `mod.rs:284`) becomes the body of F10's per-instrument closure passed as `dispatch_fn`; F11 writes its own equivalent against `BodyState`'s deadzone-shaped fields. This is the principled boundary between "shared infrastructure" and "per-editor state".

**Extraction rules.** Each helper in `instruments/` is moved from F10 with documented mechanical edits:

- `compute_live_value` renames to `compute_live_axis_value` (clarity now that it's shared); F10's call site updates to the new name.
- `dispatch_curve_edit` and `dispatch_curve_edit_no_undo` generalize to `dispatch_stage_edit` / `dispatch_stage_edit_no_undo` accepting `Action` instead of `Action::ResponseCurve`; F10's call sites pass `Action::ResponseCurve { ... }` explicitly.
- `last_nudge_at_ms` and `last_nudge_key` collapse into a `NudgeCoalesce` struct in `instruments::nudge_coalesce`; F10's `BodyState` embeds the struct instead of carrying the two fields directly. The merge-vs-push decision moves to `NudgeCoalesce::should_merge(now_ms, key) -> bool`.
- The SVG glow filter ID renames `if-curve-glow` to `if-instr-glow` in `<defs>`, in `response_curve.css`, and in the new `instruments::INSTR_GLOW_STDDEV: f64 = 0.012` constant.
- The bridge helpers (`BridgeEvent`, `BRIDGE_JS_TEMPLATE`, `dispatch_bridge_event`, `stage_id_dom_id`, `on_mounted` factory) move to `instruments::bridge`; F10's call sites import from there.

F10's existing tests are updated mechanically to reference the new symbol names and assert against the `NudgeCoalesce` struct shape. **Behavior is unchanged by construction:** dispatch payloads, undo entries, coalesce timing, and rendered SVG are identical pre- and post-extraction. F10's `mod.rs`, `keyboard.rs`, `toolbar.rs`, and `state.rs` are amended to import from `instruments::*` instead of carrying the implementations inline. The refactor task ships F10 and F11 changes in a single PR.

**What is explicitly NOT extracted** (deferred until a third instrument forces it):

- A generic `InstrumentPlot` wrapper component. Premature abstraction at 2 uses; F10 and F11 each render their own SVG layer stack with their own `<defs>`, and that's fine.
- A generic hit-test helper. F10 hit-tests against a variable anchor list; F11 against a fixed 4-element handle list. Shapes diverge enough that a single helper would carry both branches awkwardly.
- A generic `BodyState`. Different per-editor fields. They share `last_nudge_at_ms` + `last_nudge_key` only, and that pair lives in `nudge_coalesce.rs` as a small `NudgeCoalesce` struct each editor embeds.

### Engine integration

F11 reads from `inputforge-core` via `DeadzoneConfig` (already serializable, validated, evaluated). F11 writes to the engine via `EngineCommand::SetMapping` only, never `std::fs` or any direct profile manipulation. F11 introduces **no new engine commands and no new core types**, in contrast to F10 which added `inputforge_core::processing::curves::sample_curve_path`. The deadzone curve geometry is so trivial (5 fixed-shape segments derived from 4 thresholds) it's drawn directly in `rendering.rs` from the four `DeadzoneConfig` getters; no engine-side helper would earn its place.

`instruments::live_axis::compute_live_axis_value` (extracted from F10's `compute_live_value`) consumes `evaluate_actions_through(actions, &state, &addr, stage_index)` for the live-input projection. F11 is the second consumer of that helper after F10.

### Coordinate convention

Engine `DeadzoneConfig` stores four `f64` thresholds, each in `[-1, 1]`. The SVG `<svg>` uses `viewBox="-1.05 -1.05 2.1 2.1"` with an inner `<g transform="scale(1, -1)">` to flip y so positive-output is up. Pointer events project from screen to viewBox coordinates and operate on engine-native `(input, output)` directly; the y-flip is purely an SVG render-time concern. Tick labels render in a separate non-flipped `<g>` so text is not mirrored. This mirrors F10's coordinate handling exactly.

---

## State shapes & data flow

### Local body state

Per-mounted-component, held in a `Signal<BodyState>` inside `DeadzoneBody`:

```rust
struct BodyState {
    dragging: Option<DragInProgress>,
    hovered_handle: Option<HandleId>,
    /// Keyboard-focused handle; intentionally separate from `hovered_handle`.
    focused_handle: Option<HandleId>,
    /// Snapshot taken at drag start, used to revert on validation failure.
    pre_drag_config: Option<DeadzoneConfig>,
    /// Same-(stage_id, key) nudge coalesce timing; embedded shared shape.
    nudge_coalesce: instruments::nudge_coalesce::NudgeCoalesce,
}

struct DragInProgress {
    handle: HandleId,
    /// X bounds derived once at drag start from neighboring thresholds.
    bounds: (f64, f64),
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum HandleId { Low, CenterLow, CenterHigh, High }
```

`focused_handle` exists separately from `hovered_handle` so keyboard navigation is undisturbed by mouse motion and screen-reader users have a stable focus.

No `cached_path` or `cached_anchors`. F10 caches them because curves are arbitrarily-shaped and re-sampling is non-trivial; F11's "curve" is 6 fixed points derived from 4 thresholds, cheap to recompute per render.

### Inputs read

Via `use_context::<AppContext>()` and the StageBody props provided by F9's dispatcher:

| Source | Field | Use |
|---|---|---|
| `ConfigSnapshot.selected_mapping_actions` | `Option<Vec<Action>>` | Locate this stage's `Action::Deadzone { config }` at the F9-provided `stage_id`. |
| `ConfigSnapshot.selected_mapping_key` | `Option<MappingKey>` | Threaded into `SetMapping` dispatch and `UndoLog::push_edit`. |
| `LiveSnapshot` (the F1 ~60Hz polling Signal) | input value at `mapping_key.1` | Feeds `instruments::live_axis::compute_live_axis_value(stage_id, &addr, &ctx, &actions)` for the live tracking dot. |
| `EditorState.expanded_stages` | `HashSet<StageId>` | Read-only; F9's dispatcher already gates whether this body renders. |
| `EditorState.malformed_hints` | `HashMap<StageId, String>` | Write target for invalid commits (read by F9's stage header). |
| `inputforge_core::state::AppState.devices` (via `AppContext.state` lock) | connectivity of `mapping_key.1.device` | Distinguish "real zero signal" from "no device"; if absent or disconnected, `compute_live_axis_value` returns `None` and no live dot or guides render. |

### Outputs written

| Target | Trigger | Payload |
|---|---|---|
| `EngineCommand::SetMapping` (via `AppContext.commands.send`) | drag-end · numeric input commit (Enter/blur) · keyboard nudge (per press, with 250ms coalesce) · Reset (only if changed) | Full new `actions: Vec<Action>` produced by `instruments::stage_dispatch::dispatch_stage_edit(actions, stage_id, Action::Deadzone { config: new }, ...)`. Engine-offline (`Err`): silently dropped via `let _ =`. |
| `EditorState.undo_log` (via `push_edit`) | paired with each `SetMapping` (subject to nudge coalesce) | `UndoKind::StageEdit`, labels: `deadzone: low -0.85 -> -0.90`, `deadzone: drag center high`, `deadzone: reset`. |
| `EditorState.malformed_hints[stage_id]` | every commit attempt (write-only on change) | Empty when `DeadzoneConfig::new(...)` succeeds; populated with the validator's error string when it fails. F9's stage header reads this map. |

### Drag pipeline (the hot path)

1. **Pointer down** on the plot. Hit-test (10px screen-space radius) against the four positions returned by `mutation::handle_positions(config)`. If hit, set `dragging = Some(DragInProgress { handle, bounds })` where `bounds` come from `mutation::adjacent_bounds(handle, config)`:
   - `Low`: `(-1.0, config.center_low() - 0.001)`
   - `CenterLow`: `(config.low() + 0.001, config.center_high())`
   - `CenterHigh`: `(config.center_low(), config.high() - 0.001)`
   - `High`: `(config.center_high() + 0.001, 1.0)`

   Snapshot `pre_drag_config = Some(config.clone())`. Capture pointer via the `instruments::bridge::mount_mouse_bridge` JS bridge. **No engine dispatch.**

2. **Pointer move.** While `dragging.is_some()`, convert cursor X to viewBox x, clamp to `bounds`, build a candidate `DeadzoneConfig` via `mutation::with_handle(config, handle, new_x)`. The candidate replaces the body's local working copy. Display re-renders from the local clone. Engine and `ConfigSnapshot` are not touched. The handle Y is locked per-handle (Low/High at y=±1, CenterLow/CenterHigh at y=0); cursor Y delta is ignored.

3. **Pointer up.** Validate via `DeadzoneConfig::new(low, center_low, center_high, high)` (the engine's canonical gate). If `Ok`: dispatch `SetMapping` via `instruments::stage_dispatch::dispatch_stage_edit`, push undo entry. If `Err`: revert local state to `pre_drag_config`, write `malformed_hints[stage_id]` with the engine's error string, no dispatch. Either branch: clear `dragging`, clear `pre_drag_config`.

The `mutation::adjacent_bounds` clamp at step 2 should make step 3 validation always pass in practice; the validation gate exists as defense-in-depth for race conditions (external edit during drag).

### External-edit reconciliation

The polling task drives `ConfigSnapshot` at ~60Hz. F11's outer reactive scope reads `selected_mapping_actions` and projects this stage's `DeadzoneConfig` into a memoized signal.

- **`dragging.is_none()` and projected config changed:** body re-renders from the new config. `pre_drag_config = None`. `focused_handle` is invariant to config changes (handles always exist).
- **`dragging.is_some()` and projected config changed:** drop the external update; the local clone wins until pointer-up. Matches F9's external-edit task convention; protects against a competing writer mid-drag.
- **Stage disappears entirely** (action at `stage_id` no longer `Deadzone`): F9's dispatcher unmounts this body before render. F11 doesn't defend against missing-stage internally; if invariant is violated, trace-log and render an inert error placeholder.

---

## SVG rendering

### Plot frame

Single `<svg>` with `viewBox="-1.05 -1.05 2.1 2.1"` and `preserveAspectRatio="xMidYMid meet"` (square, no stretch). Width responsive: `clamp(240px, 100%, 480px)`, height = width via CSS `aspect-ratio: 1 / 1`. Inner `<g transform="scale(1, -1)">` flips y; tick labels render in a separate non-flipped group so text is not mirrored. **F10 frame parity, byte-for-byte.**

### Layer stack (back to front)

| # | Element | Class | Notes |
|---|---|---|---|
| 1 | Background `<rect>` filling the viewBox | `.if-deadzone__bg` | `fill: var(--color-bg-sunken)` |
| 2 | Zone bands (Q1 = D1), 5 vertical `<rect>`s under `<g transform="scale(1,-1)">` | `.if-deadzone__zone--sat`, `--ramp`, `--dead` | Sat: `fill: var(--color-error); opacity: 0.12`. Ramp: `fill: var(--color-primary); opacity: 0.06`. Dead: `fill: var(--color-text-subtle); opacity: 0.10`. X bounds derived from `(low, center_low, center_high, high)` per render. |
| 3 | Major grid (vertical and horizontal lines at `-0.75, -0.5, -0.25, 0.25, 0.5, 0.75` on both axes; origin excluded, owned by layer 4) | `.if-deadzone__grid-major` | `stroke: var(--instr-grid-major)`, 1px, `vector-effect: non-scaling-stroke` |
| 4 | Axis cross (x=0 and y=0) | `.if-deadzone__axis-cross` | `stroke: var(--instr-axis-cross)`, 1px, `vector-effect: non-scaling-stroke` |
| 5 | Identity reference `<line>` from (-1,-1) to (1,1) | `.if-deadzone__identity` | `stroke: var(--instr-identity-stroke)`, 1px, `stroke-dasharray: 4 6`, `opacity: 0.55`, `fill: none`, `vector-effect: non-scaling-stroke` |
| 6 | Deadzone curve `<polyline>`, 6 points: `(-1, -1)`, `(low, -1)`, `(center_low, 0)`, `(center_high, 0)`, `(high, 1)`, `(1, 1)` | `.if-deadzone__path` | `stroke: var(--color-deadzone-curve)`, 1.75px, `stroke-linejoin: round`, `filter: url(#if-instr-glow)`, `vector-effect: non-scaling-stroke`, `fill: none` |
| 7 | Handle markers, 4 `<circle r="0.022">` at the four kinks: `(low, -1)`, `(center_low, 0)`, `(center_high, 0)`, `(high, 1)` | `.if-deadzone__handle` | `fill: var(--color-deadzone-handle-fill)`, `stroke: var(--color-deadzone-handle-stroke)`, 1px. Hit-test still uses `HIT_RADIUS_PX = 10.0` from `instruments::interaction` (per F10 `interaction.rs:85`), so click targets stay generous regardless of visual size. |
| 8 | Hover ring (only if `hovered_handle.is_some()`) | `.if-deadzone__hover-ring` | `<circle r="0.085">`, no fill, `stroke: var(--color-border-focus)`, 1.5px, opacity 0.55 |
| 9 | Drag halo (only if `dragging.is_some()`) | `.if-deadzone__drag-halo` | `<circle r="0.07">`, `fill: var(--color-border-focus)`, opacity 0.30 |
| 10 | Keyboard focus ring (only if `focused_handle.is_some()` AND host element matches `:focus-visible`) | `.if-deadzone__focus-ring` | `<circle r="0.105">`, no fill, dashed, `stroke: var(--color-border-focus)`, 1.5px, `stroke-dasharray: "2 2"` |
| 11 | Live guide horizontal `<line>` at `y = output`, x from -1 to `input` | `.if-deadzone__live-guide` | `stroke: var(--color-live)`, 0.5px, `stroke-dasharray: 2 3`, opacity 0.5 |
| 12 | Live guide vertical `<line>` at `x = input`, y from 0 to `output` (anchored at the axis cross so negative outputs run upward to the dot from below) | `.if-deadzone__live-guide` | Same style as 11. |
| 13 | Live dot halo `<circle r="0.07">` | `.if-deadzone__live-dot-halo` | `fill: var(--color-live)`, opacity 0.18 |
| 14 | Live dot core `<circle r="0.04">` | `.if-deadzone__live-dot` | `fill: var(--color-live)`, `filter: url(#if-instr-glow)` |
| 15 (separate non-flipped `<g>`) | Tick labels. X axis: `-1`, `-.5`, `0`, `.5`, `1` (5 labels at the major-grid half-integer positions). Y axis: `-1`, `0`, `1` (3 labels). | `.if-deadzone__tick-label` | JetBrains Mono, `fill: var(--color-text-subtle)`. X labels render below the plot (`y="1.04"`, `text-anchor="middle"`); y labels render inside the left edge (`x="-0.98"`, `text-anchor="start"`, `dominant-baseline="central"`). Mirrors F10 exactly per `rendering.rs:284-323`. |

**Layer placement relative to the y-flip group.** Layer 1 (background) and layer 15 (tick labels) render at the SVG root, outside the `<g transform="scale(1, -1)">` group. Layers 2-14 render inside the flipped group. This mirrors F10's actual rendering at `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/rendering.rs:47-74`.

**Vertical live guide divergence.** The vertical guide is a deliberate F10 divergence justified by deadzone bipolarity: anchoring at `y = 0` produces a symmetric reading for positive and negative outputs, where F10's response curve does not need this affordance because its identity diagonal already implies the output sign. The horizontal guide remains anchored at the left edge (`x = -1`) to match F10 verbatim.

**Class scoping rule.** All CSS classes are component-scoped (`.if-deadzone__*` here, `.if-curve__*` in F10). Only **tokens** lift to the shared `assets/tokens/instruments.css` (the Q5 lift). Each component's CSS file declares its own classes that consume the shared tokens. This keeps each component's CSS self-contained while preventing token drift; lifting classes themselves at 2 uses would be premature abstraction.

### Glow filter

Defined once in the body's `<defs>` block, ID renamed from F10's `if-curve-glow` to **`if-instr-glow`** as part of the Q5 lift:

```xml
<filter id="if-instr-glow" x="-50%" y="-50%" width="200%" height="200%">
  <feGaussianBlur stdDeviation="0.012" />
</filter>
```

`stdDeviation = 0.012` viewBox units ≈ 1.4px at a 240×240 rendered size. The Rust source pins this to `pub const INSTR_GLOW_STDDEV: f64 = 0.012;` exposed by `instruments::mod` so F10 and F11 can't drift.

### Design tokens (the Q5 lift, CSS half)

Component-scoped tokens at the top of `assets/frame/deadzone.css`:

```css
.if-deadzone {
  /* Zone-band semantics (composed entirely from existing global tokens) */
  --color-deadzone-zone-sat: var(--color-error);
  --color-deadzone-zone-ramp: var(--color-primary);
  --color-deadzone-zone-dead: var(--color-text-subtle);

  /* Curve + handles */
  --color-deadzone-curve: var(--color-primary);
  --color-deadzone-handle-fill: var(--color-text);
  --color-deadzone-handle-stroke: var(--color-bg-sunken);

  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3);
  width: 100%;
}
```

New shared file `assets/tokens/instruments.css`:

```css
:root {
  --instr-grid-major: rgba(150, 165, 220, 0.18);
  --instr-axis-cross: rgba(150, 165, 220, 0.32);
  --instr-identity-stroke: var(--color-text-subtle);
}
```

`response_curve.css` is amended in the same PR: `--color-curve-grid-major` and `--color-curve-axis-cross` become scoped references (`--color-curve-grid-major: var(--instr-grid-major);`) so existing selectors `.if-curve__grid-major { stroke: var(--color-curve-grid-major); }` are unchanged. The `.if-curve` block keeps only its truly-curve-specific tokens (`--color-curve-stroke`, `--color-curve-handle`, `--color-curve-anchor-fill`, `--color-curve-anchor-stroke`).

**Zero new global tokens introduced.** Zone-band tints compose entirely from existing `--color-error`, `--color-primary`, `--color-text-subtle`, `--color-bg-sunken`, with opacity carrying the "12% / 6% / 10%" tint specification. Respects DESIGN.md "One Action Color Rule" (the deadzone curve and ramp band both use `--color-primary`, semantically aligned because both signal "input is being acted on") and the "Subordinate Categories Rule" (the saturated red is the genuine clipping signal, not decorative). DESIGN.md is not modified.

Exact alpha values are starting points; `impeccable:frontend-design` pins final values during implementation.

### Reduced motion

A single CSS rule covers all transitions inside the body, mirroring F10:

```css
@media (prefers-reduced-motion: reduce) {
  .if-deadzone * {
    transition-duration: 0ms !important;
    animation-duration: 0ms !important;
  }
}
```

The live tracking dot has no CSS transition; it updates per polling frame, which is the truthful behavior.

### Toolbar (Q2 = E1)

Layout: a `<div class="if-deadzone__toolbar">` placed above the plot. Each numeric input is composed via the existing `Field` form-row wrapper (`crates/inputforge-gui-dx/src/components/field.rs`) wrapping an extended `NumberInput` (see "F2 coordination" below). Children left-to-right:

```
Field { label: "Low".into(), for_id: Some("dz-low".into()), error: malformed_hint.clone(),
    NumberInput {
        id: Some("dz-low".into()),
        value: low_signal,
        min: -1.0,
        max: config.center_low() - 0.001,
        step: 0.01,
        precision: Some(2),
        oncommit: handle_low_commit,
    }
}
```

The `CL` / `CH` / `High` rows follow the same shape with their own `min` / `max` derived from the neighbour thresholds. After the four `Field` blocks: a `<div class="if-deadzone__toolbar-spacer" />` and the Reset button:

```
Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, onclick: on_reset, "Reset" }
```

The Reset variant and size match F10's `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/toolbar.rs:194-197`.

`min` / `max` come from the engine invariant (`low < center_low <= center_high < high`); `NumberInput`'s stepper buttons clamp to those bounds, but every commit still goes through `DeadzoneConfig::new(...)` validation as the canonical gate. Number formatting: 2 decimals via `precision: Some(2)`, JetBrains Mono with `font-feature-settings: "tnum" 1` from existing F2 styling.

---

## Interactions

### Pointer hit-testing

All hit-tests run in screen space so the 10px radius is consistent across plot sizes. The body resolves the SVG's `getBoundingClientRect()` per pointer event (via `instruments::bridge`) and projects each entry of `mutation::handle_positions(config)` from viewBox to screen pixels, then computes Euclidean distance to the cursor. Ties broken by lowest `HandleId` discriminant (`Low` first).

### Pointer event handlers (in `interaction.rs`)

| Event | Handler | Behavior |
|---|---|---|
| `onpointerdown` (primary button only; `event.button != 0` returns the input state unchanged) | `handle_pointer_down` | Hit-test, if hit, set `dragging = Some(DragInProgress { handle, bounds })`, snapshot `pre_drag_config`, capture pointer. If miss, no-op. Right-click on a handle never starts a drag and never dispatches. |
| `onpointermove` | `handle_pointer_move` | While `dragging.is_some()`: convert cursor X to viewBox x, clamp to `bounds`, build candidate via `mutation::with_handle`, replace local working copy. **No dispatch.** Else: hit-test → update `hovered_handle`. |
| `onpointerup` | `handle_pointer_up` | While `dragging.is_some()`: validate via `DeadzoneConfig::new`. Valid → dispatch + undo. Invalid → revert + write `malformed_hints[stage_id]`. Either way: clear `dragging`, `pre_drag_config`. |
| `oncontextmenu` | (handled at wrapper level, calls `event.prevent_default()`) | Suppresses webview menu. F11 has no right-click semantics. |
| `onpointerleave` | (no special handling) | Pointer capture (set at down via the JS bridge) ensures `pointerup` fires regardless of position. Hover state clears via the next `onpointermove` outside the plot. |

Cursor styles are CSS-driven via `data-` attributes (mirrors F10):

```css
.if-deadzone__plot-frame { cursor: default; }
.if-deadzone__plot-frame[data-hovered="true"] { cursor: pointer; }
.if-deadzone__plot-frame[data-dragging="true"] { cursor: grabbing; }
```

### Keyboard interactions (in `keyboard.rs`)

The plot is `tabindex="0"`. On focus, `focused_handle` defaults to `Some(HandleId::Low)`. All handlers are pure fns.

| Key | Action | Dispatch |
|---|---|---|
| `Tab` / `Shift+Tab` | Move `focused_handle` forward / backward through `[Low, CenterLow, CenterHigh, High]`. Wrap at end, release focus to next focusable element via browser default (no `prevent_default`). On focus reentry into the plot, `focused_handle` retains its last value; if `None` (e.g., first focus), defaults to `Some(HandleId::Low)`. | No |
| `ArrowLeft` / `ArrowRight` | Nudge focused handle's x by ∓0.01, clamped via `mutation::adjacent_bounds`. | Yes: `SetMapping` + undo per press, subject to 250ms same-`(stage_id, key)` coalesce via `instruments::nudge_coalesce` |
| `Shift+Arrow*` | Nudge step = 0.10. | Yes, same coalesce |
| `Home` / `End` | Focus first / last handle (`Low` / `High`). | No |
| `Escape` | If `dragging.is_some()`: revert to `pre_drag_config`, clear drag. Else no-op. | No (revert is local) |
| `Enter`, `Delete`, `Backspace`, `ArrowUp`, `ArrowDown` | Silent no-op. The deadzone has no add/remove and no Y-axis movement. ArrowUp/Down are deliberately swallowed (`prevent_default`) so users who picked the wrong key don't accidentally scroll the page. | No |

**Per-press dispatch with one local coalesce.** Each arrow nudge produces its own `SetMapping` and undo entry; same-`(stage_id, key)` repeats arriving within 250ms merge into the prior undo entry. The shared `instruments::nudge_coalesce::NudgeCoalesce` struct embedded in `BodyState` carries the timing state; the merge decision matches F10's `KeyOutcome::PushUndo` / `MergeUndo` shape exactly. Broader cross-stage / cross-key coalescing remains deferred to F16 polish or `impeccable:harden`.

### Numeric input commits (in `toolbar.rs`)

The four `NumberInput` fields commit when the user finishes editing: Enter pressed (via the new `oncommit` prop, see "F2 coordination" below) or input loses focus. Free-typing fires the underlying `oninput` only and does not dispatch. Commit handler:

1. Read all four current values (the changed field plus the other three from current `config`).
2. Build candidate via `DeadzoneConfig::new(low, center_low, center_high, high)`.
3. If `Ok`: dispatch `SetMapping` via `instruments::stage_dispatch::dispatch_stage_edit`, push undo entry with label `deadzone: <field> <old> -> <new>`. Clear `malformed_hints[stage_id]` and the `Field`'s `error` slot.
4. If `Err`: write `malformed_hints[stage_id]` with the validator's error string and mirror it into the affected `Field`'s `error` prop so the message renders inline next to the offending input. Do not dispatch. The field's underlying value reverts on next render (driven by `ConfigSnapshot`).

The `NumberInput` stepper buttons clamp to `min` / `max` client-side, so stepper-driven inputs always commit valid candidates. Free-typed values can violate `min` / `max` (the native `<input type="number">` does not block them); the canonical `DeadzoneConfig::new` gate at step 2 catches those at commit time.

### Reset button (in `toolbar.rs`)

Build `DeadzoneConfig::default()` (which is `low=-1.0, center_low=0.0, center_high=0.0, high=1.0` per `crates/inputforge-core/src/processing/deadzone.rs:55`). If equal to current config (`PartialEq` on `DeadzoneConfig`; thresholds are bounded `f64` and the validator forbids NaN-producing inputs), no-op (no dispatch, no undo). Else dispatch + push undo with label `deadzone: reset`. Mirrors F10's Reset variant and size (`Secondary` / `Sm`) and behavior.

### Live tracking dot (Q6 punt)

Same gate as F10 v1: only render the dot when `stage_id.0 == [StageIdSegment::Index(n)]` (top-level stages). Algorithm:

```text
1. If stage_id.0 != [Index(n)] → skip; no dot, no guides.
2. input = instruments::live_axis::compute_live_axis_value(stage_id, &mapping_key.1, &ctx, &actions)
3. If None (gate failed: button/hat input, missing/disconnected device, etc.) → skip.
4. output = config.apply(input)
5. Render horizontal guide from (-1, output) to (input, output),
   vertical guide from (input, 0) to (input, output) (anchored at the axis cross),
   dot at (input, output).
```

The polling Signal fires the body's reactive scope at ~60Hz; live tracking re-projects automatically. No explicit RAF loop. The dot's projection of `input` along the curve naturally lands on a band: in the dead zone (output=0), in a ramp (linearly between -1/+1 and 0), or saturated (output=±1). The user reads "where am I in the deadzone curve" without reading numbers.

### Stage header summary

Already shipped at `pipeline/stage.rs:255-275` (committed during F9 work):

```
Action::Deadzone { config } => "inner X% · outer Y%"
   inner = (center_high - center_low) * 100
   outer = (1.0 - high) * 100
```

F11 keeps this verbatim; no change needed. The header reads at-a-glance against neighbouring stage summaries.

### Stage header thumbnail (Q3 = F1)

Per Q3: a 28×14 inline SVG mini zone bar replaces F9's default chevron in the right-slot. Renders inside the F2 IconButton's invariant 32×32 hit area (per F9 spec line 325-326, IconButton 32x32 hit area is invariant). `viewBox="0 0 28 14"` with default `preserveAspectRatio` (no aspect distortion needed; the bar IS rectangular). Five `<rect>` zones positioned from the four thresholds mapped from `[-1, 1]` to `[0, 28]`. Threshold marks as 0.4px white-with-50%-opacity vertical lines. No grid, no live marker, no interactivity. The IconButton's `aria-label` shifts from `"Toggle stage body"` (chevron) to `"Toggle stage body. Deadzone: inner X% · outer Y%"` so screen readers announce the toggle action.

---

## Validation, malformed handling, and edge cases

### Validation flow

`DeadzoneConfig::new(low, center_low, center_high, high)` is the **single validation gate**. It enforces `low < center_low <= center_high < high` and returns `EngineError::InvalidConfig` with a descriptive `reason` string on failure (per `crates/inputforge-core/src/processing/deadzone.rs:67-89`). Called at every commit point: drag-end, numeric input commit, keyboard nudge release, Reset.

- **Validation passes:** clear `malformed_hints[stage_id]` if previously populated, dispatch `SetMapping`, push undo.
- **Validation fails (drag):** revert `BodyState.dragging`-related state to `pre_drag_config`, write the engine error string to `malformed_hints[stage_id]`, **no dispatch**.
- **Validation fails (numeric input commit):** the candidate `DeadzoneConfig` was built on a clone; `BodyState` was never mutated. Write the error to `malformed_hints[stage_id]`, no dispatch. The field's underlying value reverts on next render via `ConfigSnapshot`. `pre_drag_config` is the only snapshot field on `BodyState`; non-drag handlers do not need a sibling field.
- **Validation fails (keyboard nudge):** clone-only candidate; no `BodyState` mutation. If the candidate was produced by `mutation::with_handle` after `adjacent_bounds` clamping (the default path), the candidate equals the current config; the dispatch handler detects this no-op and skips both `SetMapping` and `push_edit`. `malformed_hints` is not written. If the candidate somehow bypassed clamping (defensive only; no current code path produces this), write the engine error to `malformed_hints[stage_id]` and skip dispatch.

The drag path is the only one with explicit "revert" semantics because it's the only path that mutates a working copy; all other paths validate against a candidate clone with no `BodyState` side effect.

**Intra-Action mutations only.** F11's mutations replace the inner `config` of `Action::Deadzone { config }` at the existing `stage_id`; they never add or remove actions in the pipeline. The F9 structural-mutation invariant ("clear `expanded_stages` and `malformed_hints` on positional StageId changes") does **not** apply to F11: `stage_id` is stable across all F11 commits, so existing entries in those maps remain valid.

### Edge cases

| Case | Behavior |
|---|---|
| Empty pipeline / no `selected_mapping_actions` | F9's dispatcher doesn't mount the body. Not defended internally. |
| Body mounted but `stage_id` resolves to a non-`Deadzone` action | F9 invariant violation. Trace-log + render inert error placeholder. |
| Pointer-up outside SVG | The JS bridge captures pointer; `pointerup` fires regardless of position. Commit logic runs as usual. |
| Window resize mid-drag | Hit-testing reads `getBoundingClientRect()` per event via the JS bridge; resize during drag is safe. |
| Tab into the body before `selected_mapping_actions` resolves | `focused_handle = None` until first render; arrow keys no-op. |
| Live signal address is a button or hat | `instruments::live_axis::compute_live_axis_value` returns `None`. No live dot, no guides. |
| Device for the primary `InputAddress` missing or disconnected in `state.devices` | Same: returns `None`. No live dot, no guides. |
| Drag a handle past its neighbor | `mutation::adjacent_bounds` clamps the X to `(neighbor + 0.001, ...)` or `(..., neighbor - 0.001)` before the candidate config is built. The drag never produces an invalid `DeadzoneConfig`; the cursor just stops moving the handle. |
| Numeric input typed below `min` or above `max` | The native `<input type="number">` does not block free-typed out-of-bounds values; only stepper buttons clamp. `oncommit` fires with the typed value, and the canonical `DeadzoneConfig::new` gate rejects it. The error renders inline via `Field`'s `error` slot and in `malformed_hints[stage_id]`. |
| Numeric input typed equal to a neighbor (e.g., `low == center_low`) | F2 `NumberInput`'s `max` is `center_low - 0.001`, so the client clamp prevents this. If it slips through (paste, programmatic set), `DeadzoneConfig::new` rejects with a clear message in `malformed_hints`. |
| Numeric input typed equal to `center_low == center_high` (zero-width center) | This is **valid** per the engine invariant (`center_low <= center_high`, equality permitted). No special handling. |
| User pastes `0,5` (comma decimal) into a NumberInput | The browser's native `<input type="number">` parser ignores comma-decimal in en locales; `valueAsNumber` is `NaN` and `oncommit` is suppressed. F11 sees no commit. Locale-aware parsing is out of F11 scope. |
| Body unmounted mid-drag (e.g., user collapses the stage while dragging) | The shared `instruments::bridge::mount_mouse_bridge` detaches its JS listener on Dioxus unmount; the active drag does not commit, `pre_drag_config` is dropped with the body, and a fresh mount starts a fresh `BodyState`. F11 inherits the listener-cleanup fix from commit `55ed19c` (originally landed on F10). |
| Hover over the curve away from a handle | `hovered_handle = None`, no hover ring. Cursor stays `default` (per `data-hovered="false"`). |
| Keyboard nudge would push handle past neighbor | `mutation::adjacent_bounds` clamps; the dispatched config equals the current one; the dispatch handler detects the no-op and skips both `SetMapping` and `push_edit`. |
| Two `Deadzone` stages in the same pipeline both expanded | Each `DeadzoneBody` instance has its own `BodyState` Signal; live-dot, focus, and drag are independent per stage. |
| Reset on a config already at default | Post-mutation equality check; if equal, skip dispatch and undo. Matches F10's Reset behavior. |
| Config becomes invalid via external edit (defensive, shouldn't happen) | The engine validates `SetMapping` payloads before persisting (`DeadzoneConfig` is constructed via `new()` only); F11 trusts engine state. |
| User holds an arrow key (auto-repeat) | Each repeat fires a separate `KeyDown`. Same-`(stage_id, key)` repeats within 250ms merge into the prior undo entry per `instruments::nudge_coalesce`. |
| User clicks a numeric input mid-drag | The drag's pointer capture means `pointerup` fires on the SVG before focus moves; the drag commits normally, then the click reaches the field. |
| User Tab-cycles from the plot into the numeric inputs while a drag is in flight | Drags require pointer down→up to commit; Tab doesn't trigger `pointerup`. The drag stays in flight visually until the user clicks elsewhere or presses Escape. Acceptable defensive behavior; the user's mouse is still captured. (`Escape` clears the drag without committing, per the keyboard table.) |
| Trackpad / non-Windows | Project ships WebView2 / Windows-only per `CLAUDE.md`; out of F11 scope. |
| Refused operations have no per-event feedback | Operations the engine invariants make impossible (drag past neighbor, type below `min`) are silent: no toast, no shake, no `malformed_hints` write. `malformed_hints` is reserved for "candidate config that bypassed client clamp and `DeadzoneConfig::new` rejected". Richer per-operation feedback (e.g., an `aria-live="polite"` region announcing "low cannot exceed center_low") is deferred to `impeccable:harden`. |

---

## Deferred to `impeccable` (recorded so the ideas don't get lost)

These items the brainstorm surfaced and did not commit to F11's floor. Listed here so subsequent agents and the impeccable phase have a complete index.

| Idea | Origin | Where it lands |
|---|---|---|
| Position trail (3-5 fading dots tracking recent live signal positions) | Q7 (mirrors F10's deferred move) | `impeccable:bolder` or `impeccable:delight` during F11 implementation |
| Snap-to-quarter visual feedback (grid line brightens when handle near 0, ±0.25, ±0.5, ±0.75) | Q7 | `impeccable:bolder` |
| Snap-to-axis (handle clamps to zero when within 0.02) | Q7 | `impeccable:bolder` |
| Cross-stage / cross-key undo coalescing | Section "Keyboard interactions" | F16 polish or `impeccable:harden`. F11 v1 already coalesces same-`(stage_id, key)` repeats within 250ms via `instruments::nudge_coalesce`. |
| Symmetric mode toggle (paired-handle dragging) | Q4 (G1 picked) | Possible follow-up if user demand surfaces; currently committed as out-of-scope |
| `aria-live="polite"` announcement on refused operations ("Low cannot exceed Center−") | Section "Edge cases" | `impeccable:harden` |
| First-time onboarding hint ("drag a handle to set a threshold") | (no specific origin) | `impeccable:onboard` if user testing surfaces confusion. Default ship: no hint, the visualization is self-evident. |
| Right-click context menu on handles (e.g., "set to zero", "copy value") | (no specific origin) | Future feature only if user demand surfaces |
| Custom evaluator (user-supplied formula for the active ramp) | (no specific origin) | Out of F11 scope |
| Deadzone presets / save-as-template | (no specific origin) | Future feature |
| Sound feedback on snap or drag-end | (no specific origin) | Out of scope |
| Number-field width tuning | Section "Toolbar" | `impeccable:layout` once the four fields render side-by-side at 1280px and at min-width 800px |
| Reset button distinct `aria-label` ("Reset deadzone to default") | Section "Toolbar" | `impeccable:harden` |

---

## Testing strategy

TDD throughout, pure logic before render, mirroring F8 / F9 / F10.

| Layer | Cases |
|---|---|
| `instruments/live_axis.rs` (extracted from F10) | Top-level `Index(n)` → projects via `evaluate_actions_through`. Nested `IfTrue` / `IfFalse` → returns `None`. Button input → `None`. Disconnected device → `None`. Bound axis at expected value → `Some(f64)`. Tested once for both editors; refactor verifies F10 and F11 get the same answer for the same fixture. |
| `instruments/bridge.rs` (extracted from F10) | Bridge mounting test: `mount_mouse_bridge` returns a closure; the closure spawns a task that polls `BridgeEvent`s; mock the `document::eval` channel and verify dispatched events round-trip. F10's existing tests are extended (not duplicated) to cover the extracted helper. |
| `instruments/nudge_coalesce.rs` (extracted from F10) | Pure-fn tests: `should_merge` returns `true` for same key within 250ms, `false` for different key, `false` past 250ms, `false` when state has no prior nudge. |
| `instruments/stage_dispatch.rs` (extracted from F10) | `dispatch_stage_edit` builds the new actions vec via `replace_at_path`, sends one `SetMapping`, pushes one undo entry. `dispatch_stage_edit_no_undo` skips the undo push. Mocks `mpsc::Sender<EngineCommand>`; assert receiver got expected payload. F10's two existing dispatch tests move with the function. |
| F11 `mutation.rs` | `adjacent_bounds(handle, config)` returns the neighbor-derived `(min, max)` for each `HandleId`. `with_handle(config, handle, x)` produces the candidate respecting bounds clamp. `default_config()` round-trips `DeadzoneConfig::default()`. `handle_positions(config)` returns the 4 `(x, y)` viewBox coords (Low/High at y=±1, CenterLow/CenterHigh at y=0). |
| F11 `interaction.rs` | Pure-fn tests for each handler. Given seed `(BodyState, DeadzoneConfig, MockPointerEvent)`, assert returned `(BodyState', Option<DeadzoneConfig'>, ChangedFlag)`. Cases: hit, miss, drag-then-validate-pass, drag-then-validate-fail (engineered via concurrent external edit during drag), pointer-up-without-down (no-op). Hit-test invariant: only X coordinate matters; Y delta during drag ignored. |
| F11 `keyboard.rs` | Pure-fn tests per key path: nudge ±0.01, nudge ±0.10, nudge clamped at neighbor, Escape during drag (revert), Escape with no drag (no-op), Enter / Delete / ArrowUp / ArrowDown (silent no-op), Tab order `Low → CL → CH → High`. Coalesce shape: same-key within 250ms returns `KeyOutcome::MergeUndo`; cross-key or past 250ms returns `KeyOutcome::PushUndo`. |
| F11 `thumbnail.rs` | Snapshot equality on rendered SVG `rect` attributes for canonical configs: default (full range), aggressive (low=-0.5, high=0.5), zero-width-center (cl=ch=0.0), wide-dead (cl=-0.3, ch=0.3). Asserts byte-stability of the zone bar geometry. |
| F11 `mod.rs` SSR | Mount `DeadzoneBody` via Dioxus `VirtualDom` + `dioxus_ssr::render`. Cases: default config renders 4 handles at expected viewBox positions; live-input absent → no live dot; live-input present → dot at expected `(input, output)` projection; malformed dispatch (forced via mock) → `malformed_hints[stage_id]` populated; pointer-up without down → no dispatch. Mirrors F10's SSR pattern. |
| F11 `toolbar.rs` | SSR test: simulating Enter on a `NumberInput` produces a `DeadzoneConfig::new` call, emits `EngineCommand::SetMapping`, and updates `Field`'s `error` slot on validation failure. Reset on default config produces no dispatch. Reset on non-default config produces `SetMapping` with `DeadzoneConfig::default()`. |
| F2 `NumberInput` (extension) | Two new pure-fn tests: `oncommit` fires on Enter with the post-clamp value; `oncommit` fires on blur with the post-clamp value. Existing `NumberInput` tests are unaffected (new prop is opt-in). |

`egui_kittest` snapshot tests don't apply (the egui crate was deleted in commit `2271256`). F11 ships its own SSR + pure-fn tests; coverage is acceptably reduced. The pure-fn split makes the bulk of the logic exhaustively testable.

---

## F10 coordination (the Q5 lift in flight)

The `instruments/` Rust module + `assets/tokens/instruments.css` introduced by F11 land in **F11's first implementation task**, before any F11-specific code. F10 changes that the refactor task ships in the same PR:

- `mod.rs`: `compute_live_value` is moved to `instruments::live_axis::compute_live_axis_value` (function rename for clarity now that it's shared); F10's call site updates to the new name.
- `mod.rs`: the JS bridge (`BridgeEvent`, `BRIDGE_JS_TEMPLATE`, `dispatch_bridge_event`, `stage_id_dom_id`, `on_mounted` factory) moves to `instruments::bridge`; F10 imports from there.
- `state.rs` + `keyboard.rs`: the `last_nudge_at_ms` + `last_nudge_key` pair moves into a `NudgeCoalesce` struct in `instruments::nudge_coalesce`; F10's `BodyState` embeds the struct instead of carrying the two fields directly. The merge-vs-push decision moves to `NudgeCoalesce::should_merge(now_ms, key) -> bool`.
- `toolbar.rs`: `dispatch_curve_edit` and `dispatch_curve_edit_no_undo` move to `instruments::stage_dispatch::dispatch_stage_edit` / `dispatch_stage_edit_no_undo`, generic over `Action` variant; F10's call sites pass `Action::ResponseCurve { ... }` explicitly where they previously implied it.
- `response_curve.css`: `--color-curve-grid-major` and `--color-curve-axis-cross` become scoped references (`--color-curve-grid-major: var(--instr-grid-major);`) so existing selectors `.if-curve__grid-major { stroke: var(--color-curve-grid-major); }` are unchanged. The `.if-curve` block keeps only its truly-curve-specific tokens (`--color-curve-stroke`, `--color-curve-handle`, `--color-curve-anchor-fill`, `--color-curve-anchor-stroke`).
- `mod.rs` + `response_curve.css`: the SVG glow filter ID renames from `if-curve-glow` to `if-instr-glow`. The `<filter id>` in F10's `<defs>`, the `filter: url(...)` references in `response_curve.css`, and the Rust constant exposed as `instruments::INSTR_GLOW_STDDEV: f64 = 0.012` all align under the shared name.

**Test impact.** F10's pure-fn tests on `mutation.rs`, `interaction.rs`, `keyboard.rs`, and `thumbnail.rs` are unaffected (no signature change). F10's SSR mount tests in `mod.rs` need mechanical updates where they assert the old function name (`compute_live_value` → `compute_live_axis_value`) or the old glow filter ID (`if-curve-glow` → `if-instr-glow`); behavior is unchanged. F10's dispatch tests (the two in `toolbar.rs`) update their import path. F10's `state.rs` test for `BodyState::default` updates to assert the embedded `NudgeCoalesce` field instead of the two prior fields.

The shared visual language is enforced by tokens (`--instr-grid-major`, `--instr-axis-cross`, `--instr-identity-stroke`) and code (`instruments::INSTR_GLOW_STDDEV`). Drift between F10 and F11 is structurally impossible at the grid / axis / glow / identity layers. Each editor still owns its own curve / handle styling and category-specific tokens.

---

## F2 coordination (the smallest possible NumberInput extension)

`NumberInput` (`crates/inputforge-gui-dx/src/components/number_input.rs`) today exposes `value`, `oninput`, `onstep`, `min`, `max`, `step`, `precision`, `disabled`, `id`, `size`. F11 lands one new prop on the same component:

```rust
/// Emits the post-parse, post-clamp value when the user finishes editing
/// (Enter pressed or input loses focus). Free-typing fires `oninput` only.
oncommit: Option<EventHandler<f64>>,
```

Internally the component attaches `onkeydown` (matching `Enter`, calls `prevent_default`) and `onfocusout` to its underlying `<input>`; both branches read the input's current value, parse via the browser's native `<input type="number">` `valueAsNumber`, clamp to `[min, max]`, and dispatch `oncommit` exactly once per commit. F11's toolbar is the only initial consumer; the prop is `Option`-typed so existing call sites are unaffected. No client-side locale parsing is added (matches today's behavior; locale support is out of F11 scope).

Composition with the existing `Field` form-row wrapper (`crates/inputforge-gui-dx/src/components/field.rs`) provides labelling and error slots; F11 mirrors the malformed-hint state into Field's `error` prop so per-field errors render inline next to the affected input, in addition to F9's stage-header `malformed_hints` summary.

**Test impact.** F2's existing `NumberInput` tests are unaffected (new prop is opt-in). F11 adds two `NumberInput` tests (also recorded in the testing strategy table): `oncommit` fires on Enter, `oncommit` fires on blur, both with the post-clamp value.

---

## Open questions and deferred items

- **Per-press undo coalescing.** F11 ships per-press dispatch with one local coalesce (same as F10): same-`(stage_id, key)` keyboard nudges within 250ms merge into the prior undo entry via `instruments::nudge_coalesce`. Broader cross-stage / cross-key coalescing remains deferred to F16 polish or `impeccable:harden`.
- **Live-input projection inside Conditional branches.** F11 v1 mirrors F10 v1: top-level stages only. The natural fix is a small extension to `evaluate_actions_through` (or a sibling helper in `inputforge-core`) that takes a `&StageId` and threads the seed through nested branches. F11 makes the same Open Question as F10; the lift, when it comes, lifts both editors at once.
- **Asymmetric calibration interaction with deadzone.** A non-centered stick (where neutral output is at, say, +0.04 instead of 0) will see the deadzone's `[center_low, center_high]` band slightly off-center too. Calibration is supposed to handle this upstream (the calibrated value enters the pipeline already centered); F11 trusts that contract. If user reports surface "my deadzone is asymmetric on a calibrated stick", that's a calibration bug, not an F11 bug.
- **Numeric input scroll-wheel behavior.** F2 `NumberInput` may or may not accept scroll-wheel-to-increment; F11 doesn't override either way. If the F2 behavior is "no scroll wheel", users get keyboard nudge as the precision affordance.
- **Number field width.** Numeric input width is left unspecified; defaults to F2's `NumberInput` natural sizing. Specific width may need revisiting in `impeccable:layout` once the four fields render side-by-side at 1280px and at min-width 800px.

---

## Next steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce the focused implementation plan for F11. The plan's first task is the `instruments/` extraction from F10 (refactor-only, no behavior change). Subsequent tasks build F11 on top of the extracted helpers.

---

## Appendix, brainstorm artefacts

Browser-rendered wireframes from the F11 brainstorm session, persisted under `.superpowers/brainstorm/9530-1777752112/content/`:

- `q1-visualization.html`, three initial visualization geometries (curve plot, zone bar, hybrid).
- `q1-visualization-v2.html`, three impeccable-refined options (zone-banded background, identity-delta shading, clipping caps).
- `q2-body-layout.html`, three numeric input placements (toolbar row above, two-column grid below, inline callouts).
- `q3-thumbnail.html`, two stage header right-slot thumbnails (mini zone bar, mini D1 curve).
