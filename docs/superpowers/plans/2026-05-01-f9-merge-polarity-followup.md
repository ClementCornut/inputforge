# F9 follow-up: merge / OUT readout polarity inference

Date: 2026-05-01
Owner: Mapping editor live-readout layer (F9)
Depends on: `f7887d2 fix(live_readout): remap unipolar axes from bipolar-encoded raw to [0, 1]`

## Problem

The live readout's merge-result `IN` row and `OUT` row both bypass `read_axis_display`. They inherit `primary_value.polarity` directly:

```rust
// live_readout.rs:43-55, simplified
let merged_in_value = merge_index.map(|idx| {
    let iv = inputforge_core::pipeline::evaluate_actions_through(...);
    AxisDisplay {
        value: axis_f64(&iv),
        polarity: primary_value.polarity,
    }
});
let out_value = out_present.then(|| {
    let iv = inputforge_core::pipeline::evaluate_actions_through(...);
    AxisDisplay {
        value: axis_f64(&iv),
        polarity: primary_value.polarity,
    }
});
```

That's wrong on two axes:

1. **Polarity tag.** A merge can change the polarity of its output relative to its inputs. The current code assumes the merge preserves polarity (it inherits primary's polarity). Counter-example: `MergeOp::Bidirectional` of two unipolar pedals (rudder pedals) is `(left - right).clamp(-1.0, 1.0)`: idle when both pressed equally, swings positive when right pedal exceeds left, negative when left exceeds right. Result is structurally bipolar even though both inputs are unipolar.

2. **Value encoding.** The engine's pipeline computes everything in the bipolar-encoded `[-1, 1]` range regardless of natural polarity. When the merge's natural polarity is unipolar, the value still arrives in `[-1, 1]`. The readout currently doesn't remap, so a unipolar merge result at idle reads `-1.00` (raw) instead of `0.00` (natural) and the bar fills 100%.

This is the same class of bug the `f7887d2` fix addressed for per-input rows, but the merge / OUT path was deliberately deferred because polarity inference per merge op is non-trivial.

## Scope

In scope:
- Polarity inference for the three current `MergeOp` variants: `Bidirectional`, `Average`, `Maximum`.
- `IN` row (merged) and `OUT` row of the live readout consume the inferred polarity.
- Remap `[-1, 1]` to `[0, 1]` when the inferred polarity is unipolar (matches the per-input remap from `f7887d2`).
- Single-merge case: pipeline contains exactly one top-level `MergeAxis` action. This is the structure the merge layout already assumes (see `live_readout.rs:209` `first_merge_index`).

Out of scope:
- Polarity changes from `Action::Invert` (range stays `[-1, 1]` either way; semantic polarity unchanged).
- Polarity changes from `Action::ResponseCurve` / `Action::Deadzone` (range stays `[-1, 1]` natural; F10 / F11 own these).
- `Action::Conditional` polarity inference: the merge layout handler ignores nested merges (per `live_readout.rs` comment "merges nested inside Conditional do not trigger the merge layout"), so this stays consistent with the existing scope.
- vJoy-axis-side polarity (the `OUT` row currently uses pipeline polarity, not the configured vJoy axis polarity). Different concern; deferred unless the user reports it.
- Multi-merge pipelines (chained merges): the existing readout component only renders one merge layout, so this is scope-consistent.

## Approach

Single source of truth: a pure helper `merge_output_polarity(op, primary, secondary) -> AxisPolarity` that captures the inference table. Used by both `IN` row and `OUT` row in `live_readout.rs`.

### Polarity inference table

| Op | Both Bipolar | Both Unipolar | Mixed (one of each) |
|---|---|---|---|
| `Bidirectional` (`a - b`) | Bipolar | **Bipolar** (the rudder-pedal case) | Bipolar |
| `Average` (`(a + b) / 2`) | Bipolar | Unipolar | Bipolar |
| `Maximum` (`abs-greatest`) | Bipolar | Unipolar | Bipolar |

Reasoning:
- `Bidirectional` is fundamentally a difference. Differences of monotonic quantities (unipolar) are bipolar. Differences of bipolar are also bipolar. Always bipolar.
- `Average` preserves polarity when inputs match; mixing unipolar and bipolar inputs produces a value that can swing through zero in either direction, so safest to call it bipolar.
- `Maximum` (largest-absolute-value) returns one of its inputs literally. If both inputs share polarity, the output has that polarity. Mixed inputs make the output unpredictable; bipolar is the safe default.

This table is the **only** load-bearing piece of new behavior. Everything else is plumbing.

### Where the polarity gets applied

After computing `merged_in_value` and `out_value`:
- Compute the merge op's output polarity via the helper.
- For `merged_in_value.polarity`: use the inferred polarity (instead of inheriting primary).
- For `out_value.polarity`: use the inferred polarity (the OUT row sits after the merge, so it has the merge's output polarity).
- For values: when inferred polarity is unipolar, apply the same `f64::midpoint(raw, 1.0).clamp(0.0, 1.0)` remap used in `read_axis_display`.

Extract the remap into a small `into_natural_domain(raw: f64, polarity: AxisPolarity) -> f64` helper. Used by both `read_axis_display` and the new merge / OUT path. Clamping included for defense-in-depth (out-of-range raw values from calibration drift).

### Pipeline walk to find the merge

The existing `first_merge_index` helper (`live_readout.rs:209`) walks top-level actions for the first `MergeAxis`. Extend it to also return the merge op and its `second_input` polarity (looked up from the live snapshot the same way `read_axis_display` does for the primary). The result becomes:

```rust
struct MergeContext {
    op: MergeOp,
    secondary: InputAddress,
    secondary_polarity: AxisPolarity,
}
```

Used by both `IN` (merged) and `OUT` rows.

## Files to touch

- `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs`
  - Add `merge_output_polarity(op, primary, secondary) -> AxisPolarity` helper (pure, unit-testable).
  - Add `into_natural_domain(raw, polarity) -> f64` helper (pure, unit-testable). Replaces the inline match in `read_axis_display`.
  - Walk merge to a `MergeContext` instead of returning just an index.
  - Apply inferred polarity + remap to `merged_in_value` and `out_value`.

- No CSS changes.
- No spec amendments (F9 spec doesn't pin polarity inference; this is implementation detail).

## Tasks

Single-commit-friendly. One task because the changes interlock: introducing the helpers without using them is dead code, and using them without introducing them is broken.

1. **Land the merge polarity inference end-to-end.**
   - Add `merge_output_polarity` + `into_natural_domain` helpers with unit tests covering:
     - All 3x3 combinations of merge op + input polarities for `merge_output_polarity`.
     - Boundary values (`-1`, `0`, `1`) and out-of-range (`-1.5`, `1.5`) for `into_natural_domain` on both polarities.
   - Refactor `first_merge_index` -> `find_merge_context` returning the structured tuple.
   - Update `merged_in_value` and `out_value` construction in `LiveReadout` to consume the new helpers.
   - Update the existing SSR test for merge layout (`editor_live_readout_renders_in_row` family) if any selectors break; add one new SSR test for "rudder pedals -> bipolar merge result" (Bidirectional with two unipolar inputs producing the bipolar `IN` row).

## Test strategy

**Pure unit tests (no Dioxus runtime needed):**
- `merge_output_polarity` truth-table coverage. Nine cases (3 ops x 3 input polarity pairs: BB / UU / BU). Plus parity check that the result is symmetric for commutative ops (`Average`, `Maximum`) and asymmetric is fine for `Bidirectional`.
- `into_natural_domain`: bipolar passthrough; unipolar `[-1, 1]` -> `[0, 1]`; out-of-range clamping; `-0.0` normalization (per the format-layer guard's intent).

**SSR tests:**
- New: rudder-pedals scenario. Two unipolar pedal inputs at idle (`-1` raw each), merged via `Bidirectional`. Assert the `IN` row format is bipolar (`+0.00`) and the bar fill is anchored at center (`right` and `left` both at 50% per the bipolar zero state).
- New: `Average` of two unipolar pedals. Both at idle: `IN` row reads `0.00` unipolar, bar empty. Both fully pressed: `IN` reads `1.00`, bar full.
- Update if needed: existing `editor_live_readout_renders_out_when_map_to_vjoy_present` should still pass; OUT polarity inheritance changes only when there's a merge in the tree.

**Manual smoke** (after the implementation lands):
- The user's Thrustmaster pedals + `Bidirectional` (rudder convention): IN row should sit centered at idle, swing left/right on differential press.
- `Average` mode: IN should track the average press depth, idle empty, both-pressed full.
- `Maximum` mode: IN should track the more-pressed pedal.

## Edge cases / risks

- **Polarity classification timing.** The SDL3 polarity heuristic is "lazy + deferred re-probe in the first 2 seconds". If the user clicks the editor before re-probe completes, the per-input polarity may temporarily report bipolar for a unipolar pedal. The merge inference would then compute the wrong result. Acceptable: re-probe completes within a couple of seconds, and the readout updates live. Documented behavior, no fix needed.
- **`Maximum` with mixed-polarity inputs.** The truth table calls this bipolar. In practice this combo is rare (most users merge two same-polarity inputs). If it bites, refine to "inherit the input the merge picked at this poll tick" — but that requires tracking per-tick which input won, which is overkill until a user reports it.
- **Out-of-range merge outputs.** `merge_axes` already clamps to `[-1, 1]` internally. The `into_natural_domain` clamp is then a no-op for normal pipeline values, but defends against future merge implementations that forget to clamp.
- **`Subtract` semantics on rudder pedals.** F9 spec uses the `Bidirectional` op (`first - second`) for rudder pedals where idle = `-1` for both. The diff is `-1 - (-1) = 0`, which is "centered" in bipolar terms. Convert via `into_natural_domain(_, Bipolar)` = identity, format as `+0.00`. ✓
- **OUT polarity when there's no merge.** Falls back to the primary input's polarity (current behavior, unchanged). The new code path triggers only when `find_merge_context` returns `Some`.

## Acceptance criteria

- [ ] User's rudder-pedals setup (two Thrustmaster pedals + `Bidirectional`) shows IN bar centered at idle, swinging left when left pedal exceeds right and right when right exceeds left. OUT row mirrors IN row's centerline behavior.
- [ ] User's combined-throttle setup (two pedals + `Average`) shows IN bar empty at idle, full at both-pressed, intermediate when one pedal is partial. OUT row tracks IN.
- [ ] `Maximum` with two unipolar pedals: bar tracks whichever pedal is more pressed. Both idle = empty.
- [ ] Bipolar primary stick + bipolar secondary axis + any merge op: behavior unchanged from today.
- [ ] No regressions on the `f7887d2` per-input fix.
- [ ] Workspace clippy clean, all tests pass.
- [ ] No spec amendment required (verify by re-reading F9 spec choice 7 on merge layout; the polarity inference is implementation-side).

## Estimated effort

One focused implementation pass. ~30 minutes for code + ~30 minutes for tests + 5 minutes for the manual smoke. Smaller than any prior F9 task; can be picked up in a single sitting.

## Open questions

None blocking implementation. One open question for after landing: should the OUT row consult the configured vJoy axis polarity instead of inheriting pipeline polarity? Currently vJoy axis polarity isn't surfaced anywhere user-visible, and the engine treats vJoy outputs as bipolar `[-1, 1]` internally. If a user maps a unipolar pedal (post-`Bidirectional` merge bipolar) to a vJoy axis they configured as a bipolar X, the OUT readout already matches. If they configure the vJoy axis as a half-range slider, the readout would be wrong by a factor of 2. Out of scope here; flag if a user reports it.
