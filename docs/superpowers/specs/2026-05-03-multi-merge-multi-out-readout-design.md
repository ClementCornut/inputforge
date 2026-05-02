# Live Readout: Multi-Merge / Multi-Out Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-05-03
**Parent spec:** [`2026-04-30-f9-mapping-editor-design.md`](./2026-04-30-f9-mapping-editor-design.md), live readout in the mapping editor frame
**Predecessor work:** commit `5c299cf` (`feat(live_readout): OUT row reads engine output cache, freezes when engine stops`) introduced the engine-cache OUT path and the `if-editor__readout-row--frozen` modifier; this spec extends both.
**Brainstorm artefacts:** `.superpowers/brainstorm/10989-1777761417/content/layout-v1.html`
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Product brief:** [`/PRODUCT.md`](../../PRODUCT.md)
**Action enum:** `crates/inputforge-core/src/action/mod.rs:24` (`Action`), `crates/inputforge-core/src/action/condition.rs` (`Condition`).

---

## Context

The live readout in the mapping editor today displays a single primary axis IN, an optional single merge secondary IN, an optional single merged-IN value (the result of the first top-level `MergeAxis`), and an optional single OUT (the first `MapToVJoy` found anywhere in the action tree). This was the right MVP for F9 because most early profiles have a flat pipeline with at most one merge and one output.

In reality the action tree can fan out in three orthogonal ways, individually and in combination:

1. **Stacked merges.** A pipeline can chain `MergeAxis` calls, each merging in another input axis. The current readout shows only the first top-level merge.
2. **Multiple terminal outputs.** A pipeline can hold several sibling `MapToVJoy` actions, plus `MapToKeyboard` actions. The current readout shows the first `MapToVJoy` and ignores the rest, including all keyboard outputs.
3. **Conditional branches.** `Conditional { condition, if_true, if_false }` routes the value through one of two sub-pipelines. Each branch can carry its own merges and outputs. At any moment only one branch's outputs are live; the other branch's destinations are inactive.

These can nest: a `Conditional`'s branches can contain more `Conditional`s, more `MergeAxis` actions, more terminal outputs.

The current readout silently hides everything past the first match for each role. A user with a real multi-merge / multi-out pipeline reads the readout as broken or as missing data: they can see their stick position move and one OUT respond, but they can't tell what their pedals are doing in the merge, can't see the second vJoy axis they routed through a conditional, and can't see the key combo their button-mapping emits.

This spec extends the readout to surface the full set of pipeline inputs (axes), the full set of routing predicates (conditional conditions), and the full set of terminal outputs (vJoy + keyboard), with per-OUT expansion to the causal chain of merges and conditionals that produced each output. The shipped engine-cache OUT path (commit `5c299cf`) and the `--frozen` modifier compose cleanly with the new "inactive conditional branch" treatment: both are the same visual state for "this OUT is not currently being driven", regardless of *why*.

---

## Confirmed design choices

The decisions below were validated in brainstorm one question at a time.

**Q1. Scope: D, all of the above (stacked merges, multi-output, conditional branches, all possibly nested).** The design must handle every combination, including deeply nested conditionals.

**Q2. Philosophy: C, compact hybrid.** A single fixed-rhythm IN block at the top, a single OUT block at the bottom, and an *on-demand* middle that surfaces merge progression and conditional state per OUT. Rejects flat tuning-only (no chain visibility) and full-tree-mirror (too tall for nested pipelines). Optimizes for "glance for tuning, expand for understanding."

**Q3. Conditional OUT semantics: A, show both, distinguish active vs inactive.** Both branches' terminal outputs appear in the OUT list. The branch matching the current predicate state renders live (CRT phosphor green); the other renders muted (text-muted bar / hollow chip), reusing the engine-stopped freeze treatment. Power-user honest: nothing is hidden, the full destination set is always on screen.

**Q4. IN block structure: B, two grouped subsections.** Pipeline inputs (primary axis + merge secondaries) render as bar rows in the existing `ReadoutRow` rhythm. Conditional predicates render as chip-style readouts in a second subsection. Mixing visual types in one continuous list (variant C) was rejected as visually noisy; axis-only IN (variant A) was rejected because hiding predicate state contradicts "Hardware is the protagonist."

**Q5. Expand mechanism: B, per-OUT expand with global override.** Each OUT row carries a chevron (when its causal chain is non-empty); clicking it toggles the chain block for that OUT only. A small "expand all" affordance on the divider strip flips all chevrons at once for the rare full-pipeline-inspection case. Rejects single-global-toggle (worst with deep nesting) and always-visible middle (contradicts the "glance for tuning" promise).

**Q6. Keyboard live state: A now, B as future enhancement.** `MapToKeyboard` outputs render as key-combo chips. Filled (active) when the engine is running and the destination is on the active conditional branch; hollow (idle/frozen) otherwise. The chip does *not* track actual OS-level key-down events because there is no engine-side keyboard output cache. A future `KeyboardOutputCache` on `AppState` would make the chip mirror engine reality the same way vJoy bars do today; this is named in `Deferred / Future-B follow-ups` below.

**Layout treatment** (validated against `layout-v1.html`):
- IN block has two grouped subsections with small uppercase `IN · pipeline` and `IN · predicates` section labels (caption typography, `--color-text-subtle`).
- The divider strip between IN and OUT carries the existing `out` / `merge` label and an "expand all ▾" pill (visible only when at least one OUT has a non-empty chain).
- OUT chevrons sit in a new trailing column (right of the percentage), matching the established right-aligned numeric grid.
- Expanded chain block is indented under the OUT row with a 1px dashed `--color-border-strong` left border, distinct from the IN/OUT grid columns. Chain rows use smaller font + 4px-tall bars in `--color-processing` (teal) for merge intermediates; conditional outcomes render in `--color-control` (violet).

---

## Non-goals (out of scope)

- **Editing from the readout.** The readout remains read-only. Adding an output, removing a merge, or flipping a conditional happens in the pipeline stage list, not here.
- **Live keyboard press tracking.** Q6-A defers this to a future spec. The keyboard chip's "active" state in v1 reflects routing (conditional branch) only, not actual key-down events.
- **Persistent expand state.** Per-mapping expand memory was rejected for v1: the per-output expand state is transient and resets on mapping selection change.
- **Free-form predicate editing.** Conditional predicates render as read-only chips. Editing the condition stays in the conditional stage body.
- **Graphical wiring diagrams.** The expanded chain is a vertical outline, not a node-and-edge canvas. F9's editing surface is the action stage list; the readout is its tuning mirror, not a parallel visual editor.
- **`AxisInRange` predicate as a bar.** `AxisInRange` chips show the source label and a small mono `[low..high]` glyph, not a live bar with a highlighted band. Q4-B grouped predicates as chips uniformly; mixing axis bars into the predicate subsection was the rejected variant C.

---

## Architecture

### Module split

The current `live_readout.rs` is ~640 lines and the multi-merge / multi-out work will roughly double it. Promote the single file to a module directory:

```
crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/
├── mod.rs              # public LiveReadout component (orchestrator), AppContext wiring, FROZEN_ROW_CLASS const
├── analyzer.rs         # action-tree walker that produces a flat LiveReadoutModel
├── predicate.rs        # Condition evaluator (recursion through All/Any/Not) + chip-label formatter
├── value_helpers.rs    # AxisDisplay, read_axis_display, read_output_display, polarity inference
├── in_block.rs         # IN-section component: pipeline-axis ReadoutRows + predicate chip subsection
├── out_block.rs        # OUT-section component: per-row dispatch (axis bar / button bar / hat glyph / kb chip)
└── out_chain.rs        # expanded causal-chain rendering for one OutputDescriptor
```

Existing live-readout SSR tests in `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs` stay where they are (they exercise the full `MappingEditor` component, not `LiveReadout` directly); new analyzer / predicate-evaluator unit tests live in inline `#[cfg(test)] mod tests` blocks within each new submodule file.

Public API stays `pub(crate) fn LiveReadout(primary: InputAddress, actions: Vec<Action>) -> Element`. Existing `pub(super) const FROZEN_ROW_CLASS` stays exported from `mod.rs` so SSR tests reach it through `super::live_readout::FROZEN_ROW_CLASS`. No callers outside the module change.

### Data model

The analyzer produces a flat, render-ready model from the action tree in one DFS walk:

```rust
struct LiveReadoutModel {
    pipeline_inputs: Vec<InputAddress>,    // primary first, then merge secondaries in DFS order
    predicates: Vec<PredicateDescriptor>,  // boolean inputs referenced by Conditional conditions
    outputs: Vec<OutputDescriptor>,        // every terminal MapToVJoy + MapToKeyboard
    output_polarity: AxisPolarity,         // inferred once from the merge chain
}

struct OutputDescriptor {
    destination: OutputDestination,        // VJoy(OutputAddress) | Keyboard(KeyCombo)
    chain: Vec<ChainStep>,                 // merges + conditionals on the path to this output
    is_active: bool,                       // see "is_active evaluation" below
}

enum ChainStep {
    Merge {
        op: MergeOp,
        partner: InputAddress,
        intermediate_value: f64,           // pipeline-evaluated value at this stage (natural domain)
    },
    Conditional {
        predicate: PredicateDescriptor,
        evaluated: bool,                   // current predicate truth
        branch: Branch,                    // which branch this output sits in
    },
}

enum Branch { IfTrue, IfFalse }

struct PredicateDescriptor {
    kind: PredicateKind,                   // ButtonPressed | ButtonReleased | AxisInRange | HatDirection
    inputs: Vec<InputAddress>,             // composites flatten to per-leaf entries
    state: bool,                           // current evaluated value
    label: String,                         // source-label form, e.g. "Stick · Btn 3"
}

enum OutputDestination {
    VJoy(OutputAddress),
    Keyboard(KeyCombo),
}
```

The walker:
- For `Action::MergeAxis { second_input, op }`: appends `second_input` to `pipeline_inputs`; pushes a `Merge` step (with the partial pipeline-evaluated intermediate value) onto the current chain stack.
- For `Action::MapToVJoy { output }`: emits an `OutputDescriptor { destination: VJoy(output.clone()), chain: <chain stack snapshot>, is_active: <AND of all conditional steps> }`.
- For `Action::MapToKeyboard { key }`: same as `MapToVJoy` but with `Keyboard(key.clone())`.
- For `Action::Conditional { condition, if_true, if_false }`: builds a `PredicateDescriptor` (composites flattened to one leaf entry per distinct input address; the `predicates` vec is deduplicated using the input address as the key, so the same button referenced by two conditionals shows once in the IN block); pushes a `Conditional { branch: IfTrue }` step, recurses into `if_true`; pops, pushes `Conditional { branch: IfFalse }`, recurses into `if_false`; pops.
- Processing actions (`ResponseCurve`, `Deadzone`, `Invert`) are ignored by the analyzer (they don't change pipeline shape, only value).
- `Action::ChangeMode` is ignored (not a routing concern at the readout level).

The model is rebuilt every render. Cost is O(action-tree size), bounded by the user's mapping; the existing render path already clones the actions vec per render, so no new allocation pressure beyond what's already there. Memoization is unnecessary at the size of typical pipelines (single-digit nodes).

### `is_active` evaluation

For each `OutputDescriptor`, `is_active` is the AND over every `Conditional` step in the chain of:

```
predicate.state == (branch == Branch::IfTrue)
```

i.e. for an `IfTrue` branch the predicate must currently evaluate true; for an `IfFalse` branch it must evaluate false. An output sitting inside no `Conditional` (chain has only `Merge` steps or is empty) is unconditionally active. Predicate evaluation itself runs through `evaluate_predicate` below; the analyzer caches the result on each `Conditional` chain step (`evaluated`) so the renderer doesn't re-evaluate.

### Predicate evaluator

`predicate.rs` exposes one function:

```rust
fn evaluate_predicate(cond: &Condition, state: &AppState) -> bool
```

It reads `state.input_cache.get_button` / `get_axis` / `get_hat` and recurses through `Condition::All` / `Any` / `Not` short-circuiting where appropriate. The analyzer takes the `state.read()` lock once per render and feeds the guard to all sub-evaluations, mirroring the existing merged-IN evaluation pattern.

### Output polarity inference

`value_helpers.rs::infer_output_polarity` walks the same DFS path used by the analyzer, applying the existing `merge_output_polarity` table for each merge encountered. With multiple terminal outputs, the same polarity applies to every OUT (the value flowing through the pipeline at each terminal point shares one f64; polarity is a display concern, not a runtime concern). Keyboard outputs ignore polarity (they're chip-rendered).

---

## Components

```
LiveReadout (mod.rs, orchestrator)
├── builds LiveReadoutModel via analyzer
├── owns ExpandState signal (per_output: Vec<bool>, expand_all: bool)
├── reads engine_status from ctx.meta
│
├── InBlock (in_block.rs)
│   ├── ReadoutRow × N (one per pipeline_inputs[i] axis, existing component, frozen=false)
│   └── PredicateChips (one chip per predicates[i], filled when descriptor.state == true)
│
├── DividerStrip (mod.rs inline)
│   ├── existing "merge" / "out" label
│   └── "expand all ▾" pill (rendered only when ∃ output with non-empty chain)
│
└── OutBlock (out_block.rs)
    ├── OutRow × N (one per outputs[i])
    │   ├── label / tag / value-cell / pct / chevron columns
    │   ├── value-cell switches on destination + OutputId variant
    │   ├── frozen = !engine_running || !descriptor.is_active
    │   └── chevron rendered only when descriptor.chain is non-empty
    │
    └── OutChain (out_chain.rs, rendered when expand_all || per_output[i])
        └── ChainRow × M (one per chain[m])
```

### `OutRow` value-cell variants

The `OutRow` wrapper dispatches on `OutputDescriptor.destination` and (for vJoy) `OutputId`:

| Variant | Cell layout |
|---|---|
| `VJoy(OutputId::Axis { id })` | bar (existing rendering, polarity-aware) + mono pct |
| `VJoy(OutputId::Button { id })` | unipolar bar 0/100% + mono pct (`0.00` / `1.00`) |
| `VJoy(OutputId::Hat { id })` | directional glyph cell — single character `↑↗→↘↓↙←↖·` rendered in the bar slot, no animation. Pct slot empty. |
| `Keyboard(key_combo)` | chip in `--color-control` (violet badge), key combo in mono. Filled when `is_active` per Q6-A; hollow when frozen. Bar slot replaced by chip cell. Pct slot empty. |

`OutRow` reuses the existing `ReadoutRow` layout shell where possible: same column grid, same `--frozen` modifier class application. The four cell variants share the `if-editor__readout-row` BEM root; chip and glyph variants get sub-modifiers (`--kb`, `--hat`).

### Predicate chip variants (`in_block.rs`)

| Predicate kind | Chip layout |
|---|---|
| `ButtonPressed { input }` | source label + filled green dot (filled = pressed) |
| `ButtonReleased { input }` | source label with " (released)" suffix + filled dot when *currently released* |
| `AxisInRange { input, range }` | source label + small mono `[low..high]` glyph + filled green dot when in range |
| `HatDirection { input, direction }` | source label + direction glyph (`↑↗→↘↓↙←↖`) + filled green dot when matching |

Composites (`All` / `Any` / `Not`) flatten in the analyzer: each leaf condition becomes one chip. The composite operator is *not* shown in the predicate subsection. The conditional outcome inside the expanded chain *does* show the predicate evaluation as a single boolean (after composite combination).

### Expanded chain rendering (`out_chain.rs`)

Indented block under the OUT row, left-bordered with 1px dashed `--color-border-strong` (matches `layout-v1.html`). One row per `ChainStep`:

| Step | Layout |
|---|---|
| `Merge { op, partner, intermediate_value }` | small-uppercase `MERGE n` label in `--color-output` (gold), partner source label in muted text, smaller bar (4px tall) in `--color-processing` (teal) showing intermediate value, mono pct in `--color-text-muted` |
| `Conditional { predicate, evaluated, branch }` | small-uppercase `COND` label in `--color-control` (violet), predicate label in muted text, → "active branch" or "inactive branch" tag (violet for active, text-subtle for inactive) |

The chain block respects the engine-stopped freeze: when `--frozen` is on the parent OUT row, the chain bars and chips inherit the muted treatment.

---

## State management

### Expand state

A single `Signal<ExpandState>` local to `LiveReadout`:

```rust
struct ExpandState {
    expand_all: bool,
    per_output: Vec<bool>,   // index-aligned with model.outputs
}
```

- `per_output` resets to all-false when `model.outputs.len()` changes (mapping selection changed, or the user added/removed an output stage in the mapping editor). Implemented via a `use_effect` (or equivalent length-watch) that compares previous and new lengths.
- `expand_all` is a global override. Click "expand all" once → both `expand_all = true` and every `per_output[i] = true`. Click again → both false. Per-row chevrons remain interactive when `expand_all` is false.
- Transient by design: not persisted across mapping selection or app restart. Storage complexity not worth the small UX gain at v1.

### Engine-stopped + branch-inactive freeze composition

One CSS class — `if-editor__readout-row--frozen`, already shipped — handles both cases. `OutRow` computes `frozen = !engine_running || !descriptor.is_active`. Truth table:

| Engine | Conditional branch | Row visual |
|---|---|---|
| Running | active | live (green bar / green chip) |
| Running | inactive | frozen (muted bar / hollow chip) |
| Stopped | active | frozen (muted bar / hollow chip) |
| Stopped | inactive | frozen (muted bar / hollow chip) |

Identical visual state for "this OUT is not currently being driven", regardless of *why*. The user reads the engine pill (top bar) plus the offline banner for the engine-stopped case; the active-branch indicator inside the expanded chain explains the inactive-branch case. No new CSS modifier is added; `--frozen` overloads to cover both semantics.

### Empty-pipeline edge cases

| Pipeline shape | Render |
|---|---|
| No merges, no conditionals, one MapToVJoy | today's behavior preserved. No chevron on OUT row. No "expand all" pill. |
| No outputs at all | OUT block omitted (today's behavior). |
| No pipeline inputs beyond primary, no predicates | IN block has just one row. No `IN · predicates` subsection rendered. |
| No engine, no actions, fresh profile | identical to today |
| Outputs all in inactive conditional branches simultaneously (e.g. condition on a button never pressed) | every OUT row carries `--frozen`; chevrons still work; expanded chain reveals the inactive branch state. |

---

## Testing

SSR tests in `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/tests.rs` (or split across submodule test files), extending the existing harness:

| Scenario | Assertion |
|---|---|
| Stacked merge: 2 merges, 1 OUT | analyzer emits IN1+IN2+IN3 in DFS order; chain has 2 `Merge` steps with correct intermediate values; OUT pct matches end-of-pipeline value |
| Multi-output: 1 input, 2 sibling MapToVJoy | both OUTs render with correct cache values; neither has a chevron (chain empty); pcts independent |
| Conditional: 1 predicate, 2 OUTs (one per branch) | both OUTs render; the matching branch's OUT has `is_active == true` (no `--frozen`); the other carries `--frozen` |
| Conditional with composite (`All { conditions: [A, B] }`) | analyzer flattens to 2 chips; evaluator AND-combines; OUT active iff both A and B are true |
| Keyboard OUT, conditional active | renders as kb chip; chip filled (`--color-control` background) |
| Keyboard OUT, conditional inactive | chip hollow; `--frozen` on the row |
| Nested conditional (2 levels) | path-AND active evaluation: inner OUT only active if *both* outer and inner predicates match the path branches |
| Engine stopped + active conditional | row carries `--frozen`; expand chevron still works; chain rows muted |
| Expand toggle | clicking chevron sets `per_output[i] = true`; chain rows render; clicking again collapses |
| Expand all | global toggle expands every chain at once; per-row chevrons follow |
| Selection change resets expand | model rebuild zeroes `per_output` |
| Predicate chips: ButtonPressed | chip dot filled when button cache is true, hollow when false |
| Predicate chips: AxisInRange | chip range glyph rendered; dot filled when axis cache value is inside range |

Helpers extended:
- Existing `live_snapshot_with_axes_and_outputs` and `add_vjoy_device` reused for OUT-row tests.
- New helper `seeded_profile_with_pipeline(actions, axis_polarities, axis_values, button_states, hat_states)` to seed `input_cache` for predicate evaluation tests.
- Existing `harness_with_live_and_status` reused for engine-stopped tests.
- Existing `pub(super) const FROZEN_ROW_CLASS` reused as the assertion target.

---

## Deferred / Future-B follow-ups

- **`KeyboardOutputCache` on `AppState`.** Per Q6-B, a future spec adds a per-mapping "currently held keys" cache that the engine writes alongside the existing `OutputCacheStore`. The kb chip would then mirror engine reality (filled when key is actually being held), the same way vJoy bars mirror their cache today. Trigger to revisit: when a user hits a tuning case where they need to confirm "did the engine emit the key at the right moment?" without switching to a target game.
- **Persistent expand state per mapping.** If users start regularly inspecting the same OUT across selection changes, store `ExpandState` keyed by `MappingKey` in `EditorState`. Trivial to add; deferred until usage warrants.
- **`AxisInRange` predicate as a live bar.** Q4-B picked the all-chips approach for visual rhythm consistency. If users want to see "where is my axis relative to the threshold band right now?" without expanding, swap `AxisInRange` chips for a small bar variant with a highlighted band. Keep button / hat predicates as chips (they're inherently boolean).
- **Editing affordances on the readout.** Long-term, hovering an OUT row could show "go to this stage in the editor" or "remove this output". Out of scope for v1; the readout stays read-only.

---

## Files touched by the implementation

- New module directory `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/` with the seven files listed in **Module split** above.
- Existing `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs` deleted (its content distributes into the new module).
- `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`: update the `mod live_readout;` declaration target (no API change visible to callers).
- `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`: new rules for predicate chip layout, chip filled/hollow states, kb-chip variant, hat-glyph cell, expand-all pill, expanded-chain block (indented + dashed left border), chain row typography, and the chain-bar variant. Existing `--frozen` rules unchanged.
- New analyzer / predicate-evaluator unit tests in inline `#[cfg(test)] mod tests` blocks within `analyzer.rs` and `predicate.rs`. New SSR tests for the rendered output (chain expand/collapse, predicate chips, kb chip) extend the existing `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs` (same harness, same `FROZEN_ROW_CLASS` import path).

No core / runtime changes. No `LiveSnapshot`, `ConfigSnapshot`, or `MetaSnapshot` shape changes. The existing analyzer-input data is sufficient.
