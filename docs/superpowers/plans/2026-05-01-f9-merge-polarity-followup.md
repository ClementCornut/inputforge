# F9 follow-up: polarity-aware merge in core + GUI live-readout fixes

Date: 2026-05-01 (revised)
Owner: Mapping editor live-readout layer (F9) + core pipeline merge
Depends on: `f7887d2 fix(live_readout): remap unipolar axes from bipolar-encoded raw to [0, 1]`

## Context

A first revision of this plan proposed a GUI-only fix for two bugs in the live readout's merge / OUT rows: the polarity tag was inherited from the primary input (wrong when a merge changes polarity, e.g. Bidirectional of two unipolar pedals = bipolar rudder), and the value was rendered without unipolar remap.

A code review caught a critical follow-on bug that the GUI-only approach cannot fix:

`MergeOp::Maximum` of two unipolar pedals does not behave as users expect. Unipolar inputs are bipolar-encoded with `-1` = idle and `+1` = full. The current `merge_axes` for `Maximum` picks the input with the larger absolute encoded value, so `merge_axes(0.0, -1.0, Maximum)` (left half-pressed, right idle) returns `-1.0` (idle wins over half-press) instead of `0.0` (left wins). The acceptance criterion *"`Maximum` with two unipolar pedals: bar tracks whichever pedal is more pressed"* is unsatisfiable without changing `merge_axes`.

Expanding scope into `inputforge-core` to make `merge_axes` polarity-aware also resolves a latent correctness issue affecting the runtime engine (not just the live readout): vJoy outputs of `Maximum`-merged pedals would also pick the wrong winner, just invisibly.

This revision absorbs the original GUI-only follow-up into a three-commit chain (mechanical polarity plumbing, core fix, GUI fix).

## Goal

1. `merge_axes` produces semantically-correct output for every `MergeOp` x polarity combination, including the unipolar `Maximum` case.
2. Live readout's merge `IN` row and `OUT` row use inferred output polarity and natural-domain remap, matching the per-input rows fixed by `f7887d2`.
3. Polarity is a first-class part of axis values throughout the pipeline (`InputValue::Axis`, `InputCache::get_axis`, `PipelineContext`), enabling future polarity-aware actions.

## Scope

In scope:
- Plumb `AxisPolarity` through `InputValue::Axis`, `InputCache::get_axis`, and `PipelineContext`.
- Polarity-aware `merge_axes` for all three current `MergeOp` variants.
- Shared `into_natural_domain(raw, polarity) -> f64` helper in `inputforge-core::processing` consumed by both core (`merge_axes`) and GUI (`live_readout::read_axis_display`).
- GUI helpers: `merge_output_polarity(op, primary, secondary) -> AxisPolarity` and `find_merge_context` returning a structured `MergeContext`.
- Live readout `IN` and `OUT` rows consume inferred polarity + natural-domain remap.
- Single top-level `MergeAxis` per pipeline (matches existing readout assumption).

Out of scope:
- `Action::Invert` polarity reasoning (inverting a unipolar pedal gives "opposite-rest" which is still unipolar in shape; encoding stays bipolar-encoded `[-1, 1]`. Encoded behavior unchanged. Document only.).
- `Action::ResponseCurve` / `Action::Deadzone` polarity reasoning (F10 / F11 own these; range stays `[-1, 1]` natural).
- `Action::Conditional` nested merges (existing readout comment at `live_readout.rs:231-233` documents the exclusion; this plan honors it).
- vJoy-axis-side polarity (the `OUT` row uses pipeline polarity, not configured vJoy axis polarity). Tracked as future work below.
- Multi-merge pipelines (chained merges): readout component renders one merge layout only.
- Polarity-aware `Action::ResponseCurve` interpretation: even after this plan, curves still operate on encoded `[-1, 1]`. F10 will need to revisit if/when curves should auto-remap.

Out-of-scope but tracked:
- vJoy-axis-side polarity (the `OUT` row consults pipeline polarity, not the configured vJoy axis polarity). If a user maps a unipolar pedal post-Bidirectional-merge to a half-range vJoy slider, the OUT readout will be off by a factor of 2. Defer until reported.

Known limitations after this plan:
- `Average` of mixed Unipolar+Bipolar inputs renders as Bipolar (per truth table). Combined with a bipolar primary at center plus a unipolar pedal at idle, the IN row reads `-50%` (bar half-grown leftward). Mathematically correct per "average of `0` and `-1` = `-0.5`", semantically confusing. Considered acceptable: the user must consciously construct this combo, and reframing it would require either (a) rejecting it at config time or (b) introducing per-input pre-remap to natural domain in the merge, which would change existing rudder-pedal behavior.
- `Maximum` with mixed Unipolar+Bipolar inputs: output polarity classified as Bipolar (safe default). The encoded winner is returned as-is, so a unipolar idle (encoded `-1`) winning over a bipolar near-center (encoded `0.05`) would display as `-100%`. Rare; documented; refine if reported.

## Approach

### Architectural decision: plumb polarity through the value, not via side-channels

`AxisPolarity` becomes a field of `InputValue::Axis` and a return component of `InputCache::get_axis`. Rejected alternatives:

- **Side-channel via `PipelineContext.input_polarity` + `InputCache::get_axis_polarity`**: smaller surface but creates two parallel data paths (encoded value via `get_axis`, polarity via `get_axis_polarity`). Refactor risk: a future `InputCache` impl could populate one and forget the other. Skipped.
- **Pre-remap to natural domain at the cache boundary**: would change the pipeline's `[-1, 1]` encoding contract that every existing action depends on. Out of question.

Plumbing polarity through the value is mechanical (~15-20 touch points, all of them constructors or matchers) and aligns with the type system: an axis read intrinsically has a polarity at the device layer, and that fact should not be lost when it enters the pipeline.

### Polarity inference table for `merge_output_polarity`

Used by the GUI to label IN row / OUT row polarity. `merge_axes` itself does not need this inference; it operates on the encoded values directly using each input's polarity.

| Op | Both Bipolar | Both Unipolar | Mixed (one of each) |
|---|---|---|---|
| `Bidirectional` (`a - b`) | Bipolar | **Bipolar** (rudder-pedal case) | Bipolar |
| `Average` (`(a + b) / 2`) | Bipolar | Unipolar | Bipolar |
| `Maximum` (natural-domain abs-greatest) | Bipolar | Unipolar | Bipolar |

Reasoning:
- `Bidirectional` is a difference. Difference of monotonic quantities is bipolar. Always bipolar.
- `Average` preserves polarity when inputs match. Mixed produces a value that swings through encoded zero in either direction; bipolar is safe.
- `Maximum` (after the natural-domain fix) returns one of its inputs. If both inputs share polarity, output has that polarity. Mixed: output polarity is unpredictable in a useful sense; bipolar is safe.

For `Bidirectional`, primary/secondary order matters semantically (`a - b` != `b - a`) but does not change inferred polarity (Bipolar regardless of input order). Test suite covers both orders explicitly.

### `merge_axes` behavior with polarity

Signature change:
```rust
pub fn merge_axes(
    first: f64,
    second: f64,
    operation: MergeOp,
    first_polarity: AxisPolarity,
    second_polarity: AxisPolarity,
) -> f64
```

Per-op behavior:
- `Bidirectional`: `(first - second).clamp(-1.0, 1.0)`. Polarities ignored (subtraction in encoded domain is correct for all polarity combos: encoded UU rudder pedals subtract to encoded bipolar `[-1, 1]` exactly as desired).
- `Average`: `f64::midpoint(first, second).clamp(-1.0, 1.0)`. Polarities ignored (UU averaged in encoded domain produces encoded value that the GUI re-interprets as unipolar; BB and mixed averaged in encoded domain produce bipolar values).
- `Maximum`: compare in natural domain. `into_natural_domain(first, first_polarity).abs() >= into_natural_domain(second, second_polarity).abs()` picks first; otherwise second. Return the winner's encoded value as-is. This fixes the half-press-vs-idle case.

Returning the encoded value preserves the pipeline's `[-1, 1]` convention: downstream actions (curves, deadzone, MapToVJoy) continue to operate without polarity awareness. The GUI is the only consumer that re-interprets via `merge_output_polarity` + `into_natural_domain`.

### Shared `into_natural_domain` helper

Lives in `crates/inputforge-core/src/processing/polarity.rs` (new file):

```rust
pub fn into_natural_domain(raw: f64, polarity: AxisPolarity) -> f64 {
    match polarity {
        AxisPolarity::Bipolar => raw.clamp(-1.0, 1.0),
        AxisPolarity::Unipolar => f64::midpoint(raw, 1.0).clamp(0.0, 1.0),
    }
}
```

Re-exported via `crates/inputforge-core/src/processing/mod.rs`. Consumers:
- `merge_axes` (for the `Maximum` natural-domain comparison).
- GUI `read_axis_display` (replaces inline match at `live_readout.rs:198`).
- GUI merge `IN` row and `OUT` row construction (new call sites).

The clamp added to the bipolar arm differs from the current inline match in `read_axis_display` (which has no clamp). Pre-existing behavior: a calibration-drifted raw of `1.1` rendered as `1.05` via `f64::midpoint(1.1, 1.0)` (unipolar) or as `1.1` raw (bipolar). Post-change: it renders as `1.0` in both arms. This is a fix, not a regression: the live readout should never show >100%. Documented as an intentional behavior change in the relevant commit message.

### GUI: `find_merge_context` and call-site updates

Replaces `first_merge_index` (currently `live_readout.rs:234-238`):

```rust
struct MergeContext {
    op: MergeOp,
    secondary: InputAddress,
    secondary_polarity: AxisPolarity,
}

fn find_merge_context(actions: &[Action], cache: &dyn InputCache) -> Option<MergeContext> {
    actions.iter().find_map(|action| match action {
        Action::MergeAxis { second_input, operation } => {
            let (_, polarity) = cache.get_axis(second_input);
            Some(MergeContext {
                op: *operation,
                secondary: second_input.clone(),
                secondary_polarity: polarity,
            })
        }
        _ => None,
    })
}
```

Returns `Option<MergeContext>` (`None` when there is no top-level merge, falls back to existing primary-polarity OUT behavior).

Live readout call sites (`merged_in_value` lines 43-55, `out_value` lines 57-69) consume `MergeContext`:
- Compute `output_polarity = merge_output_polarity(ctx.op, primary_polarity, ctx.secondary_polarity)`.
- For `merged_in_value.polarity` and `out_value.polarity`: use `output_polarity` instead of `primary_value.polarity`.
- For value display: pass through `into_natural_domain(raw, output_polarity)` in the format layer (matches the per-input fix from `f7887d2`).

## Files to modify

### Core (`crates/inputforge-core`)

- `src/types/input.rs` (line 56-60): add `polarity: AxisPolarity` field to `InputValue::Axis` variant. Update derives if needed.
- `src/processing/polarity.rs` (new file): `into_natural_domain` helper + unit tests.
- `src/processing/mod.rs`: re-export `into_natural_domain`.
- `src/pipeline/mod.rs`:
  - line 48-57 `InputCache` trait: change `get_axis` return type from `f64` to `(f64, AxisPolarity)`.
  - line 60-65 `PipelineContext`: no field change required (existing `input_value: InputValue` now carries polarity via the variant change).
  - line 134-140 `Action::MergeAxis` handler: destructure secondary's `(value, polarity)` from cache, extract primary's polarity from `ctx.input_value`, pass both into `merge_axes`.
- `src/pipeline/merge.rs`: update `merge_axes` signature + `Maximum` arm + extend test suite.
- `src/pipeline/test_helpers.rs` line 41 `MockCache`: update `get_axis` impl to return `(f64, AxisPolarity)`. Default to `Bipolar` for existing test fixtures.
- `src/state/cache.rs` line 101 `InputCacheStore`: update `get_axis` impl to return paired `(value, polarity)` from the stored snapshot.
- `src/engine/run.rs` line 962 `InputValue::Axis { value }` matcher: update to bind polarity (use `..` if not needed locally).
- `src/engine/output_handler.rs` line 190 `InputValue::Axis` constructor: populate `polarity` from the originating event source.
- `src/engine/tests.rs` (lines 77, 367, 418, 922): update `InputValue::Axis { value }` constructors to include `polarity`. Default to `Bipolar` unless the test specifically validates polarity behavior.
- `src/types/input.rs` line 124 (test): update construction to include polarity.

### GUI (`crates/inputforge-gui-dx`)

- `src/frame/mapping_editor/live_readout.rs`:
  - lines 43-55 `merged_in_value`: consume `MergeContext`, compute output polarity via `merge_output_polarity`, apply `into_natural_domain` for display value.
  - lines 57-69 `out_value`: consume `MergeContext` when present, fall back to primary's polarity when no merge. Apply `into_natural_domain`.
  - line 178-206 `read_axis_display`: replace inline `f64::midpoint(raw, 1.0)` at line 198 with `into_natural_domain(raw, polarity)` call. Drop the local match.
  - lines 234-238 `first_merge_index`: replace with `find_merge_context`. Update callers.
  - Add `merge_output_polarity(op, p1, p2) -> AxisPolarity` (pure helper, near other pipeline-walk helpers).
- `src/context.rs` (~line 113): `DeviceInputValues::axes` already carries polarity. Verify the GUI's `InputCacheStore` impl propagates polarity to `InputCache::get_axis`.

### GUI alternate backend (`crates/inputforge-gui`)

- `src/app.rs` line 810 `InputValue::Axis` constructor: update to include polarity. Default to `Bipolar` if it does not have access to the live polarity.

### Tests

- `crates/inputforge-core/src/pipeline/merge.rs`: add `Maximum` polarity-combination tests covering the half-press-vs-idle case for UU. Existing 14 tests stay valid (with default `Bipolar` polarity for both inputs preserves current math).
- `crates/inputforge-core/src/processing/polarity.rs`: unit tests for `into_natural_domain` covering bipolar passthrough, unipolar `[-1, 1] -> [0, 1]` remap, out-of-range clamp on both arms.
- `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs` (test module): unit tests for `merge_output_polarity` covering all 9 op-x-polarity combinations, plus order swap for Bidirectional. New SSR tests: rudder-pedals UU Bidirectional, Average UU at idle and both-pressed, Bipolar+Bipolar Average regression, Unipolar primary + no-merge OUT inheritance.

## Tasks

Three-commit chain. Each commit independently compiles and passes tests.

### Task 1: Plumb polarity through `InputValue::Axis` and `InputCache` (mechanical refactor, no behavior change)

Commit message: `refactor(core): carry AxisPolarity in InputValue::Axis and InputCache::get_axis`

- Add `polarity: AxisPolarity` field to `InputValue::Axis`.
- Change `InputCache::get_axis` return type to `(f64, AxisPolarity)`.
- Update both `InputCache` impls (`MockCache`, `InputCacheStore`) and the one call site in `pipeline/mod.rs:138`.
- Update all `InputValue::Axis` constructors and matchers across the workspace. Use `Bipolar` as the default for existing fixtures.
- Update event sources (engine/run.rs, engine/output_handler.rs) to pass through polarity from the device snapshot.
- Verify: `cargo check --workspace`, `cargo test --workspace` passes with no functional changes.

### Task 2: Polarity-aware `merge_axes` + `into_natural_domain` helper

Commit message: `fix(core): merge_axes Maximum compares in natural domain to fix unipolar pedals`

- Add `crates/inputforge-core/src/processing/polarity.rs` with `into_natural_domain` + tests.
- Re-export from `processing/mod.rs`.
- Change `merge_axes` signature to take both polarities.
- Implement natural-domain comparison for `Maximum`. `Bidirectional` and `Average` arms unchanged but accept the new params.
- Update the call site at `pipeline/mod.rs:138-139` to pass primary's polarity (extract from `ctx.input_value`) and secondary's polarity (from cache).
- Add `Maximum` polarity-combination tests in `merge.rs` (UU half-press wins over UU idle; BB unchanged; mixed bipolar default).
- Add regression tests confirming `Bidirectional` and `Average` math is unchanged for UU + BB + mixed.
- Verify: full test suite passes; `cargo clippy --workspace -- -D warnings` clean.

### Task 3: GUI live readout polarity inference + remap

Commit message: `fix(live_readout): infer merge polarity for IN and OUT rows, remap unipolar`

- Add `merge_output_polarity` helper in `live_readout.rs` with unit tests for all 9 combinations + Bidirectional input-order check.
- Replace `first_merge_index` with `find_merge_context` returning `Option<MergeContext>`.
- Replace `read_axis_display`'s inline polarity match (line 198) with `into_natural_domain` call. Note the implicit clamp behavior change in the commit message.
- Update `merged_in_value` (lines 43-55) and `out_value` (lines 57-69) to consume `MergeContext`, compute inferred polarity, and apply `into_natural_domain` for display.
- Update SSR tests:
  - Add: rudder-pedals UU Bidirectional (IN bipolar centered at idle, swings on differential).
  - Add: Average UU at idle and both-pressed.
  - Add: Bipolar+Bipolar Average regression (IN bipolar unchanged).
  - Add: Unipolar primary + no merge + MapToVJoy (OUT inherits primary's unipolar polarity, no `MergeContext`).
  - Verify existing `editor_live_readout_renders_*` family still passes.
- Manual smoke against running `dx run -p inputforge-app --no-default-features --features gui-dioxus`:
  - User's Thrustmaster pedals + Bidirectional: IN bar centered at idle, swings left/right on differential press; OUT mirrors.
  - Average UU: IN tracks press depth, idle empty, both-pressed full.
  - **Maximum UU half-press**: IN tracks the more-pressed pedal: this is the case fixed by Task 2; if it works, the core fix landed correctly.
- Verify: `cargo clippy --workspace -- -D warnings` clean; SSR tests green; manual smoke passes.

## Test strategy

**Pure unit tests:**
- `into_natural_domain` (in `inputforge-core`):
  - Bipolar passthrough: `into_natural_domain(0.5, Bipolar) == 0.5`, `(-0.5, Bipolar) == -0.5`, `(0.0, Bipolar) == 0.0`.
  - Unipolar remap: `(-1.0, Unipolar) == 0.0`, `(0.0, Unipolar) == 0.5`, `(1.0, Unipolar) == 1.0`.
  - Out-of-range clamp: `(1.5, Bipolar) == 1.0`, `(1.5, Unipolar) == 1.0`, `(-1.5, Bipolar) == -1.0`, `(-1.5, Unipolar) == 0.0`.
- `merge_axes` (in `inputforge-core/src/pipeline/merge.rs`):
  - `Maximum` UU half-press wins over UU idle: `merge_axes(0.0, -1.0, Maximum, Unipolar, Unipolar) == 0.0`.
  - `Maximum` UU full-press wins over UU half-press: `merge_axes(1.0, 0.0, Maximum, Unipolar, Unipolar) == 1.0`.
  - `Maximum` UU both idle: returns `-1.0` (tied at natural `0`, picks first).
  - `Maximum` BB unchanged: existing test cases stay green with explicit `Bipolar, Bipolar`.
  - `Maximum` mixed B+U: tied or not, returns winner's encoded value.
  - `Bidirectional` and `Average` unchanged across all polarity combos.
- `merge_output_polarity` (in GUI):
  - 9 cases (3 ops x 3 polarity pairs: BB / UU / BU).
  - `Bidirectional` order swap: `(B, U)` and `(U, B)` both yield Bipolar.
  - `Average` and `Maximum` commutative parity check: `(p1, p2)` matches `(p2, p1)`.

**SSR tests** (in `inputforge-gui-dx`):
- New: rudder-pedals UU Bidirectional, idle. IN row format `+0.00`, bar centered.
- New: rudder-pedals UU Bidirectional, left full + right idle. IN row format `+1.00`, bar grown right.
- New: Average UU, both idle. IN row format `0.00` unipolar, bar empty.
- New: Average UU, both full. IN row format `1.00` unipolar, bar full.
- New: BB Average regression. IN row format and polarity match current behavior.
- New: Unipolar primary, no merge, MapToVJoy present. OUT polarity inherits primary's unipolar.
- Existing `editor_live_readout_renders_*` family: should still pass.

**Manual smoke** (after Task 3 lands):
- Thrustmaster pedals + Bidirectional (rudder convention): IN centered at idle, swings on differential.
- Two pedals + Average: IN tracks combined depth.
- Two pedals + `Maximum`: IN tracks the more-pressed pedal **including the half-press-vs-idle case**.

## Edge cases / risks

- **Polarity classification timing.** SDL3 polarity heuristic is "lazy + deferred re-probe in the first 2 seconds". If the user opens the editor before re-probe completes, the per-input polarity may temporarily report bipolar for a unipolar pedal. The merge inference would then compute the wrong result. Acceptable: re-probe completes within seconds and the readout updates live.
- **Mixed Unipolar+Bipolar Average UX.** Documented as a known limitation. The truth table calls it Bipolar; downstream rendering shows what looks like a "wrong" `-50%` at neutral. Not fixed here.
- **`Maximum` mixed-polarity tie-break.** When natural-domain magnitudes are equal, picks the first. Same as the existing tie-break.
- **Out-of-range merge outputs.** `merge_axes` continues to clamp internally for Bidirectional and Average. `into_natural_domain` clamps as defense-in-depth. Maximum returns the winner's input as-is, already in `[-1, 1]` if the cache layer respects the contract.
- **`InputValue::Axis` blast radius.** ~15-20 touch points across the workspace. All mechanical. Risk is forgetting one and producing a compile error, which is the desirable failure mode.
- **`InputCache::get_axis` return-type change.** Two impls (`MockCache`, `InputCacheStore`). Compile errors will surface every call site immediately.
- **Subtract semantics on rudder pedals.** F9 spec uses `Bidirectional` (`first - second`) for rudder pedals where idle = `-1` for both. Diff is `-1 - (-1) = 0`. `into_natural_domain(0, Bipolar) = 0`, formatted `+0.00`.
- **OUT polarity when there is no merge.** `find_merge_context` returns `None`; OUT inherits primary's polarity (current behavior, unchanged).
- **`Action::Invert` of merged unipolar pedal output.** Inverts the encoded `[-1, 1]` value. Visually correct given the user explicitly asked to invert.

## Acceptance criteria

- [ ] `cargo test --workspace` passes (existing 14 merge tests + new polarity tests + new SSR tests).
- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] User's rudder-pedals setup (two Thrustmaster pedals + `Bidirectional`) shows IN bar centered at idle, swinging left when left exceeds right and right when right exceeds left. OUT mirrors.
- [ ] User's combined-throttle setup (two pedals + `Average`) shows IN bar empty at idle, full at both-pressed, intermediate when one is partial. OUT tracks IN.
- [ ] **`Maximum` with two unipolar pedals tracks whichever pedal is more pressed**, including the half-press-vs-idle case (Task 2 validation).
- [ ] Bipolar primary stick + bipolar secondary axis + any merge op: behavior visually unchanged from today (regression-pinned by SSR test).
- [ ] No regressions on the `f7887d2` per-input fix.
- [ ] Mid-pipeline behavior unchanged for any non-merge action (curves, deadzone, invert, MapToVJoy, MapToKeyboard).
- [ ] No spec amendment required.

## Estimated effort

Three sittings totaling 3-4 hours:
- Task 1 (polarity plumbing): 60-90 min. Mechanical but spans the workspace; compile-driven.
- Task 2 (`merge_axes` polarity-aware + helper): 45-60 min. Small surface, careful test-table coverage.
- Task 3 (GUI live readout): 60-90 min. Multiple SSR tests + manual smoke loop.

## Open questions

None blocking. Tracked for after the plan lands:
- Should the `OUT` row consult the configured vJoy axis polarity instead of inheriting pipeline polarity?
- Should `Action::ResponseCurve` interpret unipolar inputs in natural domain when authoring? F10 territory.
- Should `merge_axes` for `Bidirectional` and `Average` use natural-domain math too?
