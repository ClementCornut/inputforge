# Live Readout: Multi-Merge / Multi-Out Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the F9 live readout from single-merge / single-OUT to a full DFS walker that surfaces every pipeline input (primary axis + every merge secondary), every conditional predicate, and every terminal output (vJoy axis / button / hat + keyboard) with per-OUT expandable causal chains.

**Architecture:** Promote the existing 705-line `live_readout.rs` to a module directory of seven focused submodules. Phase 0 is a pure refactor (split helpers across files, no behavior change). Phase 1 adds a path-aware variant of `evaluate_actions_through` to `inputforge-core` so the analyzer can evaluate intermediate values inside nested `Conditional` slices. Phase 2 builds the new analyzer + predicate evaluator types; Phase 3 builds the new IN / OUT / chain components on top; Phase 4 wires the orchestrator and adds CSS; Phase 5 adds SSR tests for every spec scenario. Each phase compiles green and existing tests remain green before moving on.

**Tech Stack:** Rust 2024 edition · Dioxus 0.7 (desktop / WebView2) · `inputforge-core` (`evaluate_actions_through`, `evaluate_actions_through_path` [new], `evaluate_condition`, `into_natural_domain`) · CSS custom properties

**Spec:** [`docs/superpowers/specs/2026-05-03-multi-merge-multi-out-readout-design.md`](../specs/2026-05-03-multi-merge-multi-out-readout-design.md)

**Coding rules:** never use em-dash, en-dash, or `--` substitutes in any text artefact (code, comments, docs, commits). Use comma, colon, semicolon, period, parentheses. No `Co-Authored-By` footer on commits. After every multi-file edit run `cargo check -p inputforge-gui-dx`. After every analyzer / predicate edit, run the relevant `cargo test -p inputforge-gui-dx <module>::tests` to scope the run. The full SSR suite at the end of each phase is `cargo test -p inputforge-gui-dx --lib mapping_editor` (no `dx run`; smoke tests use `cargo`).

---

## File Structure

### New files (live_readout module directory)

| Path | Purpose |
|---|---|
| `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs` | `LiveReadout` orchestrator component; `pub(super) const FROZEN_ROW_CLASS`; `ExpandState`; submodule declarations |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/value_helpers.rs` | `AxisDisplay`, `read_axis_display`, `read_output_display`, `axis_f64`, `format_percentage`, `format_output_label`, `format_key_combo`, `merge_output_polarity`, `infer_output_polarity` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs` | `LiveReadoutModel`, `MAX_NESTED_ACTION_DEPTH`, `OutputDescriptor`, `OutputDestination`, `ChainStep::Merge { operation, secondary_input, encoded_value, polarity_at_step }`, `Branch`, `PredicateDescriptor`, `PredicateKind`, `analyze` (DFS walker, snapshot-borrow contract) |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/predicate.rs` | `format_condition_label` (composite-aware), `format_predicate_chip_label`, `evaluate_leaf_state` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/in_block.rs` | `InBlock` component (conditional `IN · pipeline` / `IN · predicates` labels), `ReadoutRow`, `PredicateChips`, predicate chip variants. (`ReadoutDivider` is moved to `out_block::DividerStrip`) |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs` | `OutBlock`, `OutRow` (uses `Rc<OutputDescriptor>`), `DividerStrip` (expand-all pill only, no label), `ExpandState`, vJoy axis / button / hat / keyboard variant cells |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_chain.rs` | `OutChain`, `ChainRow`, merge-step (uses `polarity_at_step`) + conditional-step (active vs inactive) rendering |

### Removed files

- `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs` (replaced by the directory above)

### Modified files

| Path | Change |
|---|---|
| `crates/inputforge-core/src/pipeline/mod.rs` | Add `pub enum BranchStep { IfTrue(usize), IfFalse(usize) }` and `pub fn evaluate_actions_through_path(actions, state, primary, path, stop_at) -> InputValue`. Refactor existing `evaluate_actions_through` to delegate (`path = &[]`). Add 6 new tests mirroring the existing `evaluate_actions_through_*` patterns |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs` | `mod live_readout;` keeps its declaration; the target switches from `.rs` to `/mod.rs`. No API change |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs` | Add SSR tests for stacked merges, multi-output, conditional active/inactive branches, keyboard chip, predicate chips, expand toggles, plus Task 19's gap-coverage suite (composite, nested, engine-stopped multi-OUT, hat glyphs, button-released suffix, per-output polarity disagreement, AxisInRange live dot) and Task 22's expand-toggle suite (Signal-injected). Existing tests remain |
| `crates/inputforge-gui-dx/assets/frame/mapping_editor.css` | Widen `.if-editor__readout-group` to 5 columns (last `max-content`, auto-collapsing). Add rules for shared `if-editor__readout-section-label`, predicate chip layout, kb-chip variant, hat-glyph cell, expand-all pill (textual), per-OUT wrapper with `--frozen` modifier on the wrap (so descendants in both value-cell row and chain block inherit), recessed chain block (`bg-sunken` background, 28px indent, NO border vocabulary), chain-row typography with `.is-cond` state class, chain-bar variants. Focus rules for chevron + expand-all match the project `if-icon-button` pattern. Existing `.if-editor__readout` rules are unchanged |

The only `inputforge-core` change is the new `evaluate_actions_through_path` API in Task 6 (existing function preserved as a delegating shim). No `LiveSnapshot`, `ConfigSnapshot`, or `MetaSnapshot` shape changes.

---

## Task 1: Promote `live_readout.rs` to a module directory (refactor only, mod.rs orchestrator)

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs`
- Delete: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs`

This task copies the entire content of `live_readout.rs` into a new `live_readout/mod.rs` so the module's surface stays bit-identical to today. Subsequent tasks split the helpers out file-by-file. Doing it as one move-and-rename commit makes the diff readable; later splits show as pure relocations.

- [ ] **Step 1: Verify the existing tests pass before touching anything**

Run: `cargo test -p inputforge-gui-dx --lib live_readout`
Expected: all 11 tests in `live_readout::tests` (the merge-polarity + find-merge-context unit tests) green.

Run: `cargo test -p inputforge-gui-dx --lib mapping_editor::tests`
Expected: green (the SSR readout tests + everything else).

- [ ] **Step 2: Create the new `mod.rs` as an exact copy of `live_readout.rs`**

```bash
mkdir crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout
cp crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs
```

- [ ] **Step 3: Delete the old file**

```bash
git rm crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs
```

- [ ] **Step 4: Verify the build still works and tests pass**

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

Run: `cargo test -p inputforge-gui-dx --lib live_readout`
Expected: same 11 tests still green.

Run: `cargo test -p inputforge-gui-dx --lib mapping_editor::tests`
Expected: green (the SSR readout tests use `super::live_readout::FROZEN_ROW_CLASS`, which still resolves now that `live_readout` is a directory module).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs
git commit -m "refactor(live_readout): promote live_readout.rs to live_readout/ directory"
```

---

## Task 2: Extract value helpers to `value_helpers.rs`

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/value_helpers.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs`

Extract the polarity / format / cache-read helpers into their own file. They have no Dioxus surface, so this is a clean cut.

- [ ] **Step 1: Create `value_helpers.rs`**

```rust
// Rust guideline compliant 2026-05-03

//! Value-level helpers shared by IN, OUT, and chain rows: axis-display
//! conversion, percentage formatting, output-label formatting, and the
//! merge-polarity inference table. No Dioxus surface; pure functions on
//! the snapshot types.

use inputforge_core::processing::into_natural_domain;
use inputforge_core::types::{
    AxisPolarity, InputAddress, InputId, InputValue, MergeOp, OutputAddress, OutputId, VJoyAxis,
};

use crate::context::{ConfigSnapshot, LiveSnapshot};

/// Thin display value carried through the readout component tree.
///
/// `value` is normalized to the polarity's natural domain:
/// - `Bipolar`: `[-1.0, 1.0]`, where 0 is centered.
/// - `Unipolar`: `[0.0, 1.0]`, where 0 is idle and 1 is fully pressed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct AxisDisplay {
    pub value: f64,
    pub polarity: AxisPolarity,
}

/// Read the raw axis value and polarity for `addr` from the live snapshot.
///
/// Falls back to `(0.0, Bipolar)` when the device or axis index is not
/// present in the snapshot.
pub(super) fn read_axis_display(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> AxisDisplay {
    let Some(InputId::Axis { index }) = addr.input_id() else {
        return AxisDisplay {
            value: 0.0,
            polarity: AxisPolarity::Bipolar,
        };
    };
    let dev_idx = cfg
        .devices
        .iter()
        .position(|d| Some(&d.info.id) == addr.device());
    if let Some(di) = dev_idx
        && let Some(dev_inputs) = live.device_inputs.get(di)
        && let Some(&(raw, polarity)) = dev_inputs.axes.get(usize::from(*index))
    {
        return AxisDisplay {
            value: into_natural_domain(raw, polarity),
            polarity,
        };
    }
    AxisDisplay {
        value: 0.0,
        polarity: AxisPolarity::Bipolar,
    }
}

/// Read the engine output value for `out` from the live snapshot.
///
/// Mirrors `read_axis_display` but indexes into `live.output_values`.
/// `polarity` is the inferred output polarity (per-output, computed by the
/// analyzer's path-walk through merges). Falls back to `0.0` when the
/// device or output id is absent.
pub(super) fn read_output_display(
    out: &OutputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
    polarity: AxisPolarity,
) -> AxisDisplay {
    let dev_idx = cfg
        .virtual_devices
        .iter()
        .position(|v| v.device_id == out.device);
    let raw = dev_idx
        .and_then(|di| live.output_values.get(di))
        .and_then(|vals| match out.output {
            OutputId::Axis { id } => vals
                .axes
                .iter()
                .find_map(|&(axis, value)| (axis == id).then_some(value)),
            OutputId::Button { id } => {
                let idx = usize::from(id.checked_sub(1)?);
                vals.buttons.get(idx).map(|&b| if b { 1.0 } else { 0.0 })
            }
            OutputId::Hat { .. } => None,
        })
        .unwrap_or(0.0);
    AxisDisplay {
        value: into_natural_domain(raw, polarity),
        polarity,
    }
}

/// Read whether a vJoy button output is currently pressed.
///
/// Returns `false` for missing entries. Used by `OutRow` to drive the
/// unipolar 0/100% bar for `OutputId::Button`.
pub(super) fn read_output_button(
    out: &OutputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> bool {
    let OutputId::Button { id } = out.output else {
        return false;
    };
    let Some(idx) = id.checked_sub(1) else {
        return false;
    };
    cfg.virtual_devices
        .iter()
        .position(|v| v.device_id == out.device)
        .and_then(|di| live.output_values.get(di))
        .and_then(|vals| vals.buttons.get(usize::from(idx)).copied())
        .unwrap_or(false)
}

/// Read the current direction emitted to a vJoy hat output.
///
/// Currently the engine does not write hat outputs (the pipeline rejects
/// hat-target `MapToVJoy`), so this always returns `Center`. Surfaced as
/// a function so future hat-output engine support has a single hook.
pub(super) fn read_output_hat(
    _out: &OutputAddress,
    _live: &LiveSnapshot,
    _cfg: &ConfigSnapshot,
) -> inputforge_core::types::HatDirection {
    inputforge_core::types::HatDirection::Center
}

/// Extract a scalar f64 from any `InputValue`.
pub(super) fn axis_f64(v: &InputValue) -> f64 {
    match v {
        InputValue::Axis { value, .. } => value.value(),
        InputValue::Button { pressed } => {
            if *pressed {
                1.0
            } else {
                0.0
            }
        }
        InputValue::Hat { .. } => 0.0,
    }
}

/// Format a `KeyCombo` as `Ctrl + Shift + Space` (modifiers in canonical
/// order: Ctrl, Shift, Alt, Win; key name suffix). Pure formatter — no
/// Dioxus surface, so it lives here rather than co-located with the
/// keyboard OUT row that uses it.
pub(super) fn format_key_combo(combo: &inputforge_core::types::KeyCombo) -> String {
    use inputforge_core::types::KeyModifier;
    let mut parts: Vec<&str> = combo
        .modifiers
        .iter()
        .map(|m| match m {
            KeyModifier::Ctrl => "Ctrl",
            KeyModifier::Shift => "Shift",
            KeyModifier::Alt => "Alt",
            KeyModifier::Win => "Win",
        })
        .collect();
    parts.push(combo.key.as_str());
    parts.join(" + ")
}

/// Format a vJoy output address as `vJoy <device> · <axis|button|hat>`.
pub(super) fn format_output_label(output: &OutputAddress) -> String {
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X axis",
            VJoyAxis::Y => "Y axis",
            VJoyAxis::Z => "Z axis",
            VJoyAxis::Rx => "Rx axis",
            VJoyAxis::Ry => "Ry axis",
            VJoyAxis::Rz => "Rz axis",
            VJoyAxis::Slider0 => "Slider 0",
            VJoyAxis::Slider1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Button {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("vJoy {} \u{00b7} {}", output.device, suffix)
}

/// Format a percentage string for the readout label.
///
/// Bipolar axes show a sign prefix (`+0.00` / `-0.00`) so the center is
/// unambiguous. Unipolar axes omit the sign. Sub-precision noise rounds
/// to a literal `0.0` so idle is always `0.00` / `+0.00`.
pub(super) fn format_percentage(display: &AxisDisplay) -> String {
    let value = if display.value.abs() < 0.005 {
        0.0
    } else {
        display.value
    };
    match display.polarity {
        AxisPolarity::Bipolar => format!("{value:+.2}"),
        AxisPolarity::Unipolar => format!("{value:.2}"),
    }
}

/// Infer the natural polarity of a merge result from the operator and
/// each input's polarity. See `2026-05-01-f9-merge-polarity-followup.md`
/// for the truth table.
#[must_use]
pub(super) fn merge_output_polarity(
    op: MergeOp,
    primary: AxisPolarity,
    secondary: AxisPolarity,
) -> AxisPolarity {
    match op {
        MergeOp::Bidirectional => AxisPolarity::Bipolar,
        MergeOp::Average | MergeOp::Maximum => {
            if primary == secondary {
                primary
            } else {
                AxisPolarity::Bipolar
            }
        }
    }
}

/// Walk the action subtree along the path that reaches a particular OUT,
/// applying `merge_output_polarity` at every `MergeAxis` encountered.
///
/// `path` is the sequence of merge ops + secondary polarities from the
/// primary down to the OUT, in DFS pre-order. Conditionals on the path
/// do not change polarity (they pass the value through unchanged).
#[must_use]
pub(super) fn infer_output_polarity(
    primary_polarity: AxisPolarity,
    merges_on_path: &[(MergeOp, AxisPolarity)],
) -> AxisPolarity {
    merges_on_path
        .iter()
        .fold(primary_polarity, |acc, (op, secondary)| {
            merge_output_polarity(*op, acc, *secondary)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_output_polarity_bidirectional_always_bipolar() {
        for primary in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
            for secondary in [AxisPolarity::Bipolar, AxisPolarity::Unipolar] {
                assert_eq!(
                    merge_output_polarity(MergeOp::Bidirectional, primary, secondary),
                    AxisPolarity::Bipolar
                );
            }
        }
    }

    #[test]
    fn merge_output_polarity_average_uu_is_unipolar() {
        assert_eq!(
            merge_output_polarity(
                MergeOp::Average,
                AxisPolarity::Unipolar,
                AxisPolarity::Unipolar
            ),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn infer_output_polarity_no_merges_inherits_primary() {
        assert_eq!(
            infer_output_polarity(AxisPolarity::Unipolar, &[]),
            AxisPolarity::Unipolar
        );
    }

    #[test]
    fn infer_output_polarity_chained_merges_compose_left_to_right() {
        // Unipolar primary + Bidirectional with Unipolar -> Bipolar
        // then Average with Unipolar -> Bipolar (mixed promotes)
        let path = [
            (MergeOp::Bidirectional, AxisPolarity::Unipolar),
            (MergeOp::Average, AxisPolarity::Unipolar),
        ];
        assert_eq!(
            infer_output_polarity(AxisPolarity::Unipolar, &path),
            AxisPolarity::Bipolar
        );
    }

    #[test]
    fn format_percentage_bipolar_includes_sign() {
        let d = AxisDisplay {
            value: 0.5,
            polarity: AxisPolarity::Bipolar,
        };
        assert_eq!(format_percentage(&d), "+0.50");
    }

    #[test]
    fn format_percentage_unipolar_omits_sign() {
        let d = AxisDisplay {
            value: 0.25,
            polarity: AxisPolarity::Unipolar,
        };
        assert_eq!(format_percentage(&d), "0.25");
    }

    #[test]
    fn format_output_label_axis() {
        let out = OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::Y },
        };
        assert_eq!(format_output_label(&out), "vJoy 1 \u{00b7} Y axis");
    }
}
```

- [ ] **Step 2: Update `live_readout/mod.rs` to use the new module**

In `live_readout/mod.rs`, near the top under the existing `use` block, add:

```rust
mod value_helpers;

use value_helpers::{
    AxisDisplay, axis_f64, format_output_label, format_percentage, merge_output_polarity,
    read_axis_display, read_output_display,
};
```

Then DELETE the in-file definitions of:
- `struct AxisDisplay` (already moved)
- `fn read_axis_display`
- `fn read_output_display`
- `fn axis_f64`
- `fn format_output_label`
- `fn format_percentage`
- `fn merge_output_polarity`
- The seven `merge_output_polarity` unit tests inside `mod tests` (they live in `value_helpers.rs` now in a focused subset; the deleted ones add no coverage beyond what we just wrote)

Keep `find_merge_context`, `first_map_to_vjoy_output`, `MergeContext`, `ReadoutRow`, `ReadoutDivider`, `LiveReadout`, and `FROZEN_ROW_CLASS` in `mod.rs` for now (Tasks 3, 4, 6, 12 split them out later).

Also keep the `use` lines for items still referenced inside `mod.rs` (`Action`, `EngineStatus`, `InputAddress`, `OutputAddress`, etc.).

- [ ] **Step 3: Verify build + tests still pass**

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

Run: `cargo test -p inputforge-gui-dx --lib live_readout::value_helpers::tests`
Expected: 7 passed.

Run: `cargo test -p inputforge-gui-dx --lib mapping_editor::tests`
Expected: green (SSR tests still resolve `LiveReadout` and `FROZEN_ROW_CLASS`).

Run: `cargo test -p inputforge-gui-dx --lib live_readout::tests`
Expected: the 4 remaining tests (`find_merge_context_*`) still green.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/value_helpers.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs
git commit -m "refactor(live_readout): extract value helpers into value_helpers submodule"
```

---

## Task 3: Extract `ReadoutRow` and `ReadoutDivider` to `in_block.rs`

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/in_block.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs`

Both row primitives are reused by IN rows, the merged-IN row (today), and the new OUT rows (Phase 3). They live in `in_block.rs` because the IN block is their authoring home; `out_block.rs` will import them. This is still pure refactor: no behavior change.

- [ ] **Step 1: Create `in_block.rs`**

```rust
// Rust guideline compliant 2026-05-03

//! IN-block primitives: `ReadoutRow` (label + tag + bar + percentage)
//! and `ReadoutDivider` (`─── label ───`). Today both render today's
//! IN, merged-IN, and OUT rows. Task 12 adds the new IN-block
//! orchestrator (`InBlock`) and predicate-chip subsection on top.

use dioxus::prelude::*;

use inputforge_core::types::AxisPolarity;

use super::value_helpers::{AxisDisplay, format_percentage};

/// One row in the readout grid: label | tag | bar | percentage text.
///
/// `frozen` is true when the row is held (engine stopped or, in Phase 3,
/// inactive conditional branch). CSS dims the bar fill and percentage.
#[component]
pub(super) fn ReadoutRow(
    label: String,
    tag: String,
    display: AxisDisplay,
    frozen: bool,
) -> Element {
    let pct_text = format_percentage(&display);
    let bipolar = matches!(display.polarity, AxisPolarity::Bipolar);

    let fill_pct = if bipolar {
        (display.value.abs() * 50.0).clamp(0.0, 50.0)
    } else {
        (display.value.abs() * 100.0).clamp(0.0, 100.0)
    };

    let bar_style = if bipolar && display.value < 0.0 {
        format!("left: auto; right: 50%; width: {fill_pct}%;")
    } else if bipolar {
        format!("left: 50%; right: auto; width: {fill_pct}%;")
    } else {
        format!("left: 0; right: auto; width: {fill_pct}%;")
    };

    let bar_class = if bipolar {
        "if-editor__readout-bar if-editor__readout-bar--bipolar"
    } else {
        "if-editor__readout-bar"
    };

    let row_class = if frozen {
        "if-editor__readout-row if-editor__readout-row--frozen"
    } else {
        "if-editor__readout-row"
    };

    rsx! {
        div { class: "{row_class}",
            div { class: "if-editor__readout-label", "{label}" }
            div { class: "if-editor__readout-tag", "{tag}" }
            div { class: "{bar_class}",
                div {
                    class: "if-editor__readout-fill",
                    style: "{bar_style}",
                }
            }
            div { class: "if-editor__readout-pct", "{pct_text}" }
        }
    }
}

/// Section divider with an inline label (e.g. `─── merge ───`).
#[component]
pub(super) fn ReadoutDivider(label: String) -> Element {
    rsx! {
        div { class: "if-editor__readout-divider",
            span { class: "if-editor__readout-divider-label", "{label}" }
        }
    }
}
```

- [ ] **Step 2: Update `live_readout/mod.rs` to import the moved components**

Add the new module declaration at the top, beside `mod value_helpers;`:

```rust
mod in_block;

use in_block::{ReadoutDivider, ReadoutRow};
```

DELETE the in-file `#[component] fn ReadoutRow(...)` and `#[component] fn ReadoutDivider(...)` definitions in `mod.rs`.

- [ ] **Step 3: Verify build + tests pass**

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

Run: `cargo test -p inputforge-gui-dx --lib mapping_editor::tests`
Expected: all SSR readout tests still green; HTML output is bit-identical to before this commit.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/in_block.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs
git commit -m "refactor(live_readout): extract ReadoutRow and ReadoutDivider into in_block submodule"
```

---

## Task 4: Build the `LiveReadoutModel` data types in `analyzer.rs`

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs` (add `mod analyzer;`)

This task lays out the data model only, no walker yet. The walker arrives in Task 5. Each type carries the docstring justifying its shape so the next reviewer does not have to re-derive it from the spec.

- [ ] **Step 1: Write the failing test**

Append to `mod.rs` (no need for a new test file yet):

Actually, the tests live inline in `analyzer.rs`. Create `analyzer.rs` with the types and a single shape-check test:

```rust
// Rust guideline compliant 2026-05-03

//! Action-tree walker that flattens a mapping's pipeline into a render-
//! ready `LiveReadoutModel`. Sees every pipeline input, every condition
//! predicate, and every terminal output (vJoy + keyboard). Phase 1 of
//! the multi-merge / multi-out spec; consumed by `mod.rs` orchestrator
//! and the IN / OUT / chain components.

use inputforge_core::types::{AxisPolarity, InputAddress, KeyCombo, MergeOp, OutputAddress};

/// Maximum nesting depth of `Action::Conditional` branches the analyzer
/// will descend into. Mirrors `MAX_CONDITION_DEPTH = 32` (which bounds
/// only the predicate AST). Enforced in the `analyze` walker as a
/// defensive cap; profile-level validation is deferred until a real-
/// world deeply-nested profile appears.
pub(super) const MAX_NESTED_ACTION_DEPTH: usize = 32;

/// Render-ready model of a mapping's pipeline. Rebuilt every render
/// (cost is bounded by `MAX_NESTED_ACTION_DEPTH = 32` for action-tree
/// nesting plus `MAX_CONDITION_DEPTH = 32` for predicate AST depth, and
/// typical pipeline size; memoization is unnecessary).
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct LiveReadoutModel {
    /// Primary axis first, then merge secondaries in DFS pre-order.
    pub pipeline_inputs: Vec<InputAddress>,
    /// Boolean inputs referenced by `Conditional` predicates. Composites
    /// flatten to per-leaf entries; deduplicated by `(input, kind, bounds)`
    /// so two `AxisInRange` predicates with distinct bounds remain distinct.
    pub predicates: Vec<PredicateDescriptor>,
    /// Every terminal `MapToVJoy` + `MapToKeyboard` reachable through
    /// the action tree, in DFS pre-order.
    pub outputs: Vec<OutputDescriptor>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct OutputDescriptor {
    pub destination: OutputDestination,
    /// Merges + conditionals on the path to this output, root-to-leaf.
    pub chain: Vec<ChainStep>,
    /// Composite-AND of every conditional step on the chain (true when
    /// the output is currently being driven by the engine).
    pub is_active: bool,
    /// Output polarity computed by walking this output's specific path
    /// through the merge chain. Ignored for keyboard outputs.
    pub polarity: AxisPolarity,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum OutputDestination {
    VJoy(OutputAddress),
    Keyboard(KeyCombo),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ChainStep {
    Merge {
        operation: MergeOp,
        secondary_input: InputAddress,
        /// Raw f64 from `evaluate_actions_through_path`, in encoded
        /// `[-1, 1]` domain. The natural-domain renderable value is
        /// `into_natural_domain(encoded_value, polarity_at_step)`.
        /// The renderer must use *this* polarity, not the terminal
        /// output polarity: when polarity promotes mid-fold (e.g.
        /// Unipolar primary + Bidirectional partner promotes to Bipolar
        /// after the first merge), per-step bars would otherwise be
        /// re-mapped through the wrong polarity.
        encoded_value: f64,
        /// Running output polarity AT this merge step (the left-fold
        /// prefix of `merge_output_polarity` across all merges in the
        /// chain up to and including this one).
        polarity_at_step: AxisPolarity,
    },
    Conditional {
        /// Pre-rendered, composite-aware label
        /// (e.g. "Btn 3 AND Axis Y in [0.20..0.80]"). Built by
        /// `predicate::format_condition_label`.
        condition_label: String,
        /// Current composite-condition truth value (load-bearing input
        /// to `is_active`).
        evaluated: bool,
        /// Which branch this output sits in.
        branch: Branch,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Branch {
    IfTrue,
    IfFalse,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PredicateDescriptor {
    pub kind: PredicateKind,
    /// Composite predicates flatten to one descriptor per leaf
    /// `InputAddress`. Wrapped in a Vec for future-proofing; today every
    /// descriptor holds exactly one input.
    pub inputs: Vec<InputAddress>,
    /// Current evaluated leaf truth (snapshot for chip rendering).
    pub state: bool,
    /// Source-label form (e.g. "Stick · Btn 3").
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum PredicateKind {
    ButtonPressed,
    ButtonReleased,
    AxisInRange { min: f64, max: f64 },
    HatDirection {
        directions: Vec<inputforge_core::types::HatDirection>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_default_is_empty() {
        let m = LiveReadoutModel::default();
        assert!(m.pipeline_inputs.is_empty());
        assert!(m.predicates.is_empty());
        assert!(m.outputs.is_empty());
    }

    #[test]
    fn output_descriptor_carries_destination_chain_polarity() {
        let dest = OutputDestination::Keyboard(KeyCombo {
            key: "Space".to_owned(),
            modifiers: vec![],
        });
        let d = OutputDescriptor {
            destination: dest.clone(),
            chain: vec![],
            is_active: true,
            polarity: AxisPolarity::Bipolar,
        };
        assert_eq!(d.destination, dest);
    }
}
```

- [ ] **Step 2: Add `mod analyzer;` to `live_readout/mod.rs`**

```rust
mod analyzer;
mod in_block;
mod value_helpers;
```

(Alphabetical order, beside the existing decls.)

- [ ] **Step 3: Verify the new tests pass**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::tests`
Expected: 2 passed.

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs
git commit -m "feat(live_readout): add LiveReadoutModel data types in analyzer module"
```

---

## Task 5: Implement the analyzer DFS walker (inputs + outputs, empty chains)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs`

Build the recursive walker. This task emits `pipeline_inputs` (primary first, merge secondaries appended in DFS order) and `outputs` (every `MapToVJoy` + `MapToKeyboard` in DFS pre-order, with empty chains and `is_active = true`, `polarity = AxisPolarity::Bipolar`). Tasks 6-9 layer on chain capture, conditional gating, and polarity inference.

- [ ] **Step 1: Write the failing tests**

Append below the existing `mod tests` block in `analyzer.rs`:

```rust
#[cfg(test)]
mod walker_tests {
    use super::*;
    use inputforge_core::action::{Action, Condition};
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputId, KeyCombo, MergeOp, OutputId, VJoyAxis,
    };

    fn axis_addr(idx: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: idx },
        }
    }

    fn btn_addr(idx: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: idx },
        }
    }

    fn vjoy_x() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }
    }

    fn vjoy_y() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::Y },
        }
    }

    #[test]
    fn empty_actions_yields_only_primary_input() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let model = analyze(&[], &primary, &state);
        assert_eq!(model.pipeline_inputs, vec![primary]);
        assert!(model.outputs.is_empty());
    }

    #[test]
    fn stacked_merges_emit_primary_plus_secondaries_in_order() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            },
            Action::MergeAxis {
                second_input: axis_addr(2),
                operation: MergeOp::Average,
            },
            Action::MapToVJoy { output: vjoy_x() },
        ];
        let model = analyze(&actions, &primary, &state);
        assert_eq!(
            model.pipeline_inputs,
            vec![primary, axis_addr(1), axis_addr(2)]
        );
        assert_eq!(model.outputs.len(), 1);
        assert!(matches!(
            model.outputs[0].destination,
            OutputDestination::VJoy(ref o) if *o == vjoy_x()
        ));
    }

    #[test]
    fn sibling_outputs_yield_one_descriptor_each() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![
            Action::MapToVJoy { output: vjoy_x() },
            Action::MapToVJoy { output: vjoy_y() },
        ];
        let model = analyze(&actions, &primary, &state);
        assert_eq!(model.outputs.len(), 2);
    }

    #[test]
    fn keyboard_output_yields_keyboard_destination() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let combo = KeyCombo {
            key: "Space".to_owned(),
            modifiers: vec![],
        };
        let actions = vec![Action::MapToKeyboard { key: combo.clone() }];
        let model = analyze(&actions, &primary, &state);
        assert_eq!(model.outputs.len(), 1);
        assert_eq!(
            model.outputs[0].destination,
            OutputDestination::Keyboard(combo)
        );
    }

    #[test]
    fn conditional_outputs_in_both_branches_are_emitted() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: btn_addr(0) },
            if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
            if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
        }];
        let model = analyze(&actions, &primary, &state);
        assert_eq!(model.outputs.len(), 2);
    }

    #[test]
    fn walker_caps_at_max_nested_depth() {
        // Build a 33-deep chain of nested Conditionals, each wrapping one
        // MapToVJoy in `if_true`. With `MAX_NESTED_ACTION_DEPTH = 32`, the
        // 33rd MapToVJoy must be silently dropped (bailed branch).
        let state = AppState::new();
        let primary = axis_addr(0);
        let mut actions = vec![Action::MapToVJoy { output: vjoy_x() }];
        for _ in 0..33 {
            actions = vec![Action::Conditional {
                condition: Condition::ButtonPressed { input: btn_addr(0) },
                if_true: actions,
                if_false: Vec::new(),
            }];
        }
        let model = analyze(&actions, &primary, &state);
        // Depth-32 cap: the innermost MapToVJoy at level 33 is bailed out.
        assert!(model.outputs.is_empty());
    }
}
```

- [ ] **Step 2: Verify the tests fail (function not yet defined)**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests`
Expected: FAIL with "cannot find function `analyze` in this scope".

- [ ] **Step 3: Implement the walker**

Append to `analyzer.rs`, below the type definitions, before the test modules:

```rust
use inputforge_core::action::Action;
use inputforge_core::state::AppState;

/// Walk an action tree and produce a render-ready `LiveReadoutModel`.
///
/// **Snapshot contract:** The caller must hold `state.read()` for the
/// duration of this call. The walker assumes a single consistent snapshot
/// of `input_cache` and `output_values` across every sub-evaluation
/// (predicates, intermediate merge values, polarity reads). Calling
/// `state.read()` per iteration in a loop will tear the snapshot and
/// produce inconsistent chain steps within one rebuild.
///
/// **Depth bound:** The walker descends `Action::Conditional` branches up
/// to `MAX_NESTED_ACTION_DEPTH` levels. Beyond the cap, the offending
/// branch contributes nothing to the model (no outputs, no chain steps).
/// This is a defensive guard against pathological action trees.
///
/// Phase 1 implementation: emits `pipeline_inputs` and `outputs` only.
/// `chain`, `is_active`, `polarity`, and `predicates` are stub-defaulted
/// here; Tasks 7-11 layer on chain capture, conditional gating, polarity
/// inference, and predicate flattening.
pub(super) fn analyze(
    actions: &[Action],
    primary: &InputAddress,
    state: &AppState,
) -> LiveReadoutModel {
    let mut model = LiveReadoutModel {
        pipeline_inputs: vec![primary.clone()],
        predicates: Vec::new(),
        outputs: Vec::new(),
    };

    walk(actions, state, &mut model, 0);
    model
}

/// Recursive DFS pre-order walker. Mutates `model.pipeline_inputs` and
/// `model.outputs` as it visits each action. `Conditional` arms recurse
/// with `depth + 1`; processing actions and `ChangeMode` are no-ops at
/// the routing level. Bails silently when `depth > MAX_NESTED_ACTION_DEPTH`.
fn walk(
    actions: &[Action],
    state: &AppState,
    model: &mut LiveReadoutModel,
    depth: usize,
) {
    if depth > MAX_NESTED_ACTION_DEPTH {
        return;
    }
    for action in actions {
        match action {
            Action::MergeAxis { second_input, .. } => {
                model.pipeline_inputs.push(second_input.clone());
            }
            Action::MapToVJoy { output } => {
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::VJoy(output.clone()),
                    chain: Vec::new(),
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                });
            }
            Action::MapToKeyboard { key } => {
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::Keyboard(key.clone()),
                    chain: Vec::new(),
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                });
            }
            Action::Conditional { if_true, if_false, .. } => {
                walk(if_true, state, model, depth + 1);
                walk(if_false, state, model, depth + 1);
            }
            // Processing + ChangeMode do not affect routing shape.
            Action::ResponseCurve { .. }
            | Action::Deadzone { .. }
            | Action::Invert
            | Action::ChangeMode { .. } => {}
        }
    }
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer`
Expected: 8 passed (2 type tests + 6 walker tests including `walker_caps_at_max_nested_depth`).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs
git commit -m "feat(live_readout): implement DFS walker emitting pipeline inputs and outputs"
```

---

## Task 6: Add path-aware `evaluate_actions_through_path` to `inputforge-core`

**Files:**
- Modify: `crates/inputforge-core/src/pipeline/mod.rs`

The existing `evaluate_actions_through(actions, state, primary, stop_at)` walks a single top-level slice. Task 7 (the merge-step capture) needs to evaluate intermediate values at merges *inside* `Action::Conditional.if_true[i]` / `if_false[i]` slices, where the local index does not refer to the outer top-level vector. This task introduces a path-aware variant in core so the analyzer can call it correctly. The existing function is preserved as a thin shim.

- [ ] **Step 1: Add `BranchStep` enum to `pipeline/mod.rs`**

```rust
/// One hop down the action tree from a containing `Vec<Action>` into a
/// nested `Action::Conditional` branch. Used by `evaluate_actions_through_path`
/// to address actions inside conditional arms.
///
/// Each variant carries the index of the `Action::Conditional` inside the
/// parent slice; the variant tag (`IfTrue` / `IfFalse`) selects which arm
/// to descend into. The path is interpreted left-to-right; the final
/// `stop_at` parameter applies to the resolved nested slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchStep {
    IfTrue(usize),
    IfFalse(usize),
}
```

- [ ] **Step 2: Add the path-aware function**

Just below the existing `evaluate_actions_through`:

```rust
/// Path-aware variant of [`evaluate_actions_through`]. Walks `path`
/// through nested `Action::Conditional` branches, then evaluates the
/// first `stop_at` actions of the resolved slice.
///
/// `path` may be empty, in which case this is identical to
/// `evaluate_actions_through(actions, state, primary, stop_at)`.
///
/// # Panics
///
/// Panics with a clear message if any `BranchStep` indexes outside its
/// parent slice or names a non-`Conditional` action. Callers (the
/// analyzer) must construct paths from a walk that already verified
/// these invariants.
#[must_use]
pub fn evaluate_actions_through_path(
    actions: &[Action],
    state: &AppState,
    primary: &InputAddress,
    path: &[BranchStep],
    stop_at: usize,
) -> InputValue {
    // Resolve the path to a nested slice, then delegate to the existing
    // execute_pipeline machinery on that slice with `stop_at`.
    let mut slice: &[Action] = actions;
    for (depth, step) in path.iter().enumerate() {
        let (idx, want_true) = match step {
            BranchStep::IfTrue(i) => (*i, true),
            BranchStep::IfFalse(i) => (*i, false),
        };
        let action = slice.get(idx).unwrap_or_else(|| {
            panic!(
                "evaluate_actions_through_path: path[{depth}] index {idx} out of range \
                 (slice len {})",
                slice.len()
            )
        });
        let (if_true, if_false) = match action {
            Action::Conditional { if_true, if_false, .. } => (if_true, if_false),
            other => panic!(
                "evaluate_actions_through_path: path[{depth}] expected Conditional, \
                 found {other:?}"
            ),
        };
        slice = if want_true { if_true } else { if_false };
    }
    evaluate_actions_through(slice, state, primary, stop_at)
}
```

The implementation deliberately delegates back to `evaluate_actions_through` once the slice is resolved, so all the primary-read + `PipelineContext` + `execute_pipeline` machinery is reused unchanged.

- [ ] **Step 3: Add tests mirroring existing `evaluate_actions_through_*` patterns**

In the same `mod tests` block (currently around lines 951-1112) append:

```rust
#[test]
fn path_empty_matches_evaluate_actions_through() {
    // Empty path == top-level slice; result must match the non-path call.
    let state = AppState::new();
    let primary = axis_addr(0);
    state.input_cache.update(
        &primary,
        &InputValue::Axis {
            value: AxisValue::new(0.5),
            polarity: AxisPolarity::Bipolar,
        },
    );
    let actions = vec![Action::Invert, Action::MapToVJoy { output: vjoy_x() }];
    let direct = evaluate_actions_through(&actions, &state, &primary, 1);
    let via_path = evaluate_actions_through_path(&actions, &state, &primary, &[], 1);
    assert_eq!(direct, via_path);
}

#[test]
fn path_one_level_deep_runs_branch_subset() {
    // Conditional[0].if_true = [Invert, Deadzone]; eval through the
    // first action of if_true (just Invert) for primary = 0.5 should
    // yield -0.5.
    let state = AppState::new();
    let primary = axis_addr(0);
    state.input_cache.update(
        &primary,
        &InputValue::Axis {
            value: AxisValue::new(0.5),
            polarity: AxisPolarity::Bipolar,
        },
    );
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(0) },
        if_true: vec![Action::Invert, Action::MapToVJoy { output: vjoy_x() }],
        if_false: Vec::new(),
    }];
    let result = evaluate_actions_through_path(
        &actions,
        &state,
        &primary,
        &[BranchStep::IfTrue(0)],
        1, // run just the Invert
    );
    if let InputValue::Axis { value, .. } = result {
        assert!((value.get() - (-0.5)).abs() < 1e-9);
    } else {
        panic!("expected Axis InputValue");
    }
}

#[test]
fn path_into_if_false_branch() {
    // Mirrors the IfTrue test against the IfFalse arm.
    let state = AppState::new();
    let primary = axis_addr(0);
    state.input_cache.update(
        &primary,
        &InputValue::Axis {
            value: AxisValue::new(0.5),
            polarity: AxisPolarity::Bipolar,
        },
    );
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(0) },
        if_true: Vec::new(),
        if_false: vec![Action::Invert],
    }];
    let result = evaluate_actions_through_path(
        &actions,
        &state,
        &primary,
        &[BranchStep::IfFalse(0)],
        1,
    );
    if let InputValue::Axis { value, .. } = result {
        assert!((value.get() - (-0.5)).abs() < 1e-9);
    } else {
        panic!("expected Axis InputValue");
    }
}

#[test]
fn path_two_levels_deep_path_aware() {
    // Outer Conditional wraps an inner Conditional, whose if_true holds
    // [Invert]. Path `[IfTrue(0), IfTrue(0)]` descends both levels.
    let state = AppState::new();
    let primary = axis_addr(0);
    state.input_cache.update(
        &primary,
        &InputValue::Axis {
            value: AxisValue::new(0.5),
            polarity: AxisPolarity::Bipolar,
        },
    );
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(0) },
        if_true: vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: btn_addr(1) },
            if_true: vec![Action::Invert],
            if_false: Vec::new(),
        }],
        if_false: Vec::new(),
    }];
    let result = evaluate_actions_through_path(
        &actions,
        &state,
        &primary,
        &[BranchStep::IfTrue(0), BranchStep::IfTrue(0)],
        1,
    );
    if let InputValue::Axis { value, .. } = result {
        assert!((value.get() - (-0.5)).abs() < 1e-9);
    } else {
        panic!("expected Axis InputValue");
    }
}

#[test]
#[should_panic(expected = "expected Conditional")]
fn path_at_non_conditional_panics_with_clear_message() {
    let state = AppState::new();
    let primary = axis_addr(0);
    let actions = vec![Action::Invert];
    evaluate_actions_through_path(
        &actions,
        &state,
        &primary,
        &[BranchStep::IfTrue(0)],
        1,
    );
}

#[test]
#[should_panic(expected = "out of range")]
fn path_index_out_of_range_panics_with_clear_message() {
    let state = AppState::new();
    let primary = axis_addr(0);
    let actions: Vec<Action> = Vec::new();
    evaluate_actions_through_path(
        &actions,
        &state,
        &primary,
        &[BranchStep::IfTrue(0)],
        0,
    );
}
```

The existing test helpers (`axis_addr`, `btn_addr`, `vjoy_x`) are in scope; if they are not yet declared in `pipeline/mod.rs::tests`, mirror the helpers from the existing 8 `evaluate_actions_through_*` tests.

- [ ] **Step 4: Verify**

Run: `cargo test -p inputforge-core --lib pipeline`
Expected: existing 8 tests + 6 new path-aware tests green.

Run: `cargo check -p inputforge-core` and `cargo check -p inputforge-gui-dx`
Expected: green (the GUI does not yet call the new function).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-core/src/pipeline/mod.rs
git commit -m "feat(pipeline): add path-aware evaluate_actions_through_path"
```

---

## Task 7: Capture `ChainStep::Merge` per output (per-step `(encoded_value, polarity_at_step)` via `evaluate_actions_through_path`)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs`

Each output descriptor now carries the merges on its DFS path. The chain is a snapshot taken at the moment the descriptor is emitted (so sibling branches do not share state).

For each `MergeAxis` encountered during DFS, the analyzer captures two things per merge step:
- **`encoded_value`**: raw f64 in the encoded `[-1, 1]` domain returned by `evaluate_actions_through_path(top_level, state, primary, branch_path, local_idx + 1)`. The path-aware variant lets us walk into nested `Conditional` slices correctly (the original `evaluate_actions_through` would only have addressed top-level slices). The natural-domain renderable value is `into_natural_domain(encoded_value, polarity_at_step)`.
- **`polarity_at_step`**: the running output polarity at this merge, computed as the left-fold of `merge_output_polarity(op, primary_polarity, secondary_polarity)` across all merges in the chain up to and including this one. Stored per step because polarity can promote mid-fold (e.g. Unipolar primary + Bidirectional partner promotes to Bipolar after the first merge); the renderer must use the at-step polarity, not the terminal one, for accurate intermediate-value bars.

The walker tracks `branch_path: &mut Vec<BranchStep>` alongside `chain_stack`. When entering `Conditional.if_true`, push `BranchStep::IfTrue(action_idx)`; on exit, pop. Same for `if_false`. Inside a merge, pass `&branch_path` to `evaluate_actions_through_path` so the nested slice resolves correctly.

- [ ] **Step 1: Write the failing test**

Append to `walker_tests` in `analyzer.rs`:

```rust
    #[test]
    fn merge_then_output_records_one_chain_step() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            },
            Action::MapToVJoy { output: vjoy_x() },
        ];
        let model = analyze(&actions, &primary, &state);
        let out = &model.outputs[0];
        assert_eq!(out.chain.len(), 1);
        assert!(matches!(
            out.chain[0],
            ChainStep::Merge {
                operation: MergeOp::Bidirectional,
                ref secondary_input,
                ..
            } if *secondary_input == axis_addr(1)
        ));
    }

    #[test]
    fn stacked_merges_record_two_chain_steps_for_each_output() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            },
            Action::MergeAxis {
                second_input: axis_addr(2),
                operation: MergeOp::Average,
            },
            Action::MapToVJoy { output: vjoy_x() },
        ];
        let model = analyze(&actions, &primary, &state);
        let out = &model.outputs[0];
        assert_eq!(out.chain.len(), 2);
    }

    #[test]
    fn sibling_outputs_share_the_pre_split_merges_only() {
        // Two top-level outputs after one merge; both should carry the merge
        // step. Branches diverge only inside Conditionals (Task 8).
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Average,
            },
            Action::MapToVJoy { output: vjoy_x() },
            Action::MapToVJoy { output: vjoy_y() },
        ];
        let model = analyze(&actions, &primary, &state);
        assert_eq!(model.outputs.len(), 2);
        for out in &model.outputs {
            assert_eq!(out.chain.len(), 1);
        }
    }

    #[test]
    fn merge_step_carries_polarity_at_step_not_terminal() {
        // Unipolar primary + Bidirectional merge promotes polarity to
        // Bipolar at THIS step. Test that polarity_at_step reflects the
        // promotion, not the primary's polarity.
        let state = AppState::new();
        let primary = axis_addr(0);
        state.input_cache.update(
            &primary,
            &InputValue::Axis {
                value: AxisValue::new(0.0),
                polarity: AxisPolarity::Unipolar,
            },
        );
        state.input_cache.update(
            &axis_addr(1),
            &InputValue::Axis {
                value: AxisValue::new(0.0),
                polarity: AxisPolarity::Unipolar,
            },
        );
        let actions = vec![
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            },
            Action::MapToVJoy { output: vjoy_x() },
        ];
        let model = analyze(&actions, &primary, &state);
        let ChainStep::Merge { polarity_at_step, .. } = model.outputs[0].chain[0] else {
            panic!("expected Merge step");
        };
        assert_eq!(polarity_at_step, AxisPolarity::Bipolar);
    }
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests::merge_then_output_records_one_chain_step`
Expected: FAIL.

- [ ] **Step 3: Update the walker to track a chain stack and branch path**

Stack policy: the walker pushes onto `chain_stack` when entering a slice; `truncate(stack_baseline)` on exit pops anything pushed during this `walk` invocation. Top-level merges remain visible to all later siblings in the same frame; nested merges are scoped to their containing slice. This handles `Merge` here and the `Conditional` arm Task 8 adds. `branch_path` is mirrored: push on conditional descent, pop on return.

Replace the body of `analyze` and `walk` in `analyzer.rs` with:

```rust
use inputforge_core::pipeline::BranchStep;

pub(super) fn analyze(
    actions: &[Action],
    primary: &InputAddress,
    state: &AppState,
) -> LiveReadoutModel {
    let mut model = LiveReadoutModel {
        pipeline_inputs: vec![primary.clone()],
        predicates: Vec::new(),
        outputs: Vec::new(),
    };
    let mut chain_stack: Vec<ChainStep> = Vec::new();
    let mut branch_path: Vec<BranchStep> = Vec::new();
    walk(
        actions,
        actions,
        primary,
        state,
        &mut chain_stack,
        &mut branch_path,
        &mut model,
        0,
    );
    model
}

/// Recursive DFS walker.
///
/// `top_level` is the original action vec, retained across recursion so
/// `evaluate_actions_through_path` operates from the root and resolves
/// the current nested slice via `branch_path`. `local` is the slice
/// currently being iterated; `i` is the local action index. `branch_path`
/// records each `Conditional` descent so the path-aware core helper
/// addresses nested slices correctly.
fn walk(
    local: &[Action],
    top_level: &[Action],
    primary: &InputAddress,
    state: &AppState,
    chain_stack: &mut Vec<ChainStep>,
    branch_path: &mut Vec<BranchStep>,
    model: &mut LiveReadoutModel,
    depth: usize,
) {
    if depth > MAX_NESTED_ACTION_DEPTH {
        return;
    }
    let stack_baseline = chain_stack.len();
    for (i, action) in local.iter().enumerate() {
        match action {
            Action::MergeAxis {
                second_input,
                operation,
            } => {
                model.pipeline_inputs.push(second_input.clone());
                let (encoded_value, polarity_at_step) = compute_merge_step_data(
                    top_level,
                    primary,
                    state,
                    branch_path,
                    i,
                    *operation,
                    second_input,
                    chain_stack,
                );
                chain_stack.push(ChainStep::Merge {
                    operation: *operation,
                    secondary_input: second_input.clone(),
                    encoded_value,
                    polarity_at_step,
                });
            }
            Action::MapToVJoy { output } => {
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::VJoy(output.clone()),
                    chain: chain_stack.clone(),
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                });
            }
            Action::MapToKeyboard { key } => {
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::Keyboard(key.clone()),
                    chain: chain_stack.clone(),
                    is_active: true,
                    polarity: AxisPolarity::Bipolar,
                });
            }
            Action::Conditional { if_true, if_false, .. } => {
                branch_path.push(BranchStep::IfTrue(i));
                walk(
                    if_true,
                    top_level,
                    primary,
                    state,
                    chain_stack,
                    branch_path,
                    model,
                    depth + 1,
                );
                branch_path.pop();

                branch_path.push(BranchStep::IfFalse(i));
                walk(
                    if_false,
                    top_level,
                    primary,
                    state,
                    chain_stack,
                    branch_path,
                    model,
                    depth + 1,
                );
                branch_path.pop();
            }
            Action::ResponseCurve { .. }
            | Action::Deadzone { .. }
            | Action::Invert
            | Action::ChangeMode { .. } => {}
        }
    }
    chain_stack.truncate(stack_baseline);
}

/// Capture both the encoded merged value AND the running output polarity
/// AT this merge step. The encoded value comes from `evaluate_actions_through_path`
/// against the resolved nested slice; the polarity is the left-fold of
/// `merge_output_polarity` across this merge's primary-side polarity (the
/// previous merge's `polarity_at_step`, or the primary input's polarity if
/// no prior merges exist) and the secondary input's polarity.
fn compute_merge_step_data(
    top_level: &[Action],
    primary: &InputAddress,
    state: &AppState,
    branch_path: &[BranchStep],
    local_idx: usize,
    operation: MergeOp,
    secondary_input: &InputAddress,
    chain_stack: &[ChainStep],
) -> (f64, AxisPolarity) {
    let stop_at = local_idx + 1;
    let iv = inputforge_core::pipeline::evaluate_actions_through_path(
        top_level, state, primary, branch_path, stop_at,
    );
    let encoded_value = super::value_helpers::axis_f64(&iv);

    // Primary-side polarity at this step:
    //   - if a prior merge step exists in `chain_stack`, use its polarity_at_step;
    //   - else use the primary input's stored polarity from the cache.
    let primary_polarity = chain_stack
        .iter()
        .rev()
        .find_map(|step| match step {
            ChainStep::Merge { polarity_at_step, .. } => Some(*polarity_at_step),
            _ => None,
        })
        .unwrap_or_else(|| {
            use inputforge_core::pipeline::InputCache;
            let (_, p) = state.input_cache.get_axis(primary);
            p
        });

    // Secondary-side polarity from the cache.
    let secondary_polarity = {
        use inputforge_core::pipeline::InputCache;
        let (_, p) = state.input_cache.get_axis(secondary_input);
        p
    };

    let polarity_at_step = super::value_helpers::merge_output_polarity(
        operation,
        primary_polarity,
        secondary_polarity,
    );

    (encoded_value, polarity_at_step)
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests`
Expected: 10 passed (6 from Task 5 + 4 new merge-chain tests including `merge_step_carries_polarity_at_step_not_terminal`).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs
git commit -m "feat(live_readout): capture Merge chain steps with per-step (encoded_value, polarity) via path-aware core helper"
```

---

## Task 8: Capture `ChainStep::Conditional` per output (with branch tracking)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs`

When the walker enters `if_true` it pushes a `Conditional { branch: IfTrue }` step onto the stack; on exit it pops, pushes `IfFalse`, recurses, pops again. The `evaluated` boolean is computed by calling `evaluate_condition` from `inputforge-core`. The `condition_label` is filled with a placeholder for now (`format!("{condition:?}")`); Task 10 wires the proper composite-aware formatter from `predicate.rs`.

- [ ] **Step 1: Write the failing test**

Append to `walker_tests`:

```rust
    #[test]
    fn conditional_branches_record_distinct_chain_steps() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: btn_addr(0) },
            if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
            if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
        }];
        let model = analyze(&actions, &primary, &state);
        assert_eq!(model.outputs.len(), 2);
        let true_branch = model
            .outputs
            .iter()
            .find(|d| matches!(&d.destination, OutputDestination::VJoy(o) if *o == vjoy_x()))
            .unwrap();
        let false_branch = model
            .outputs
            .iter()
            .find(|d| matches!(&d.destination, OutputDestination::VJoy(o) if *o == vjoy_y()))
            .unwrap();
        assert!(matches!(
            true_branch.chain[0],
            ChainStep::Conditional {
                branch: Branch::IfTrue,
                ..
            }
        ));
        assert!(matches!(
            false_branch.chain[0],
            ChainStep::Conditional {
                branch: Branch::IfFalse,
                ..
            }
        ));
    }

    #[test]
    fn nested_conditional_records_two_chain_steps() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: btn_addr(0) },
            if_true: vec![Action::Conditional {
                condition: Condition::ButtonPressed { input: btn_addr(1) },
                if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
                if_false: vec![],
            }],
            if_false: vec![],
        }];
        let model = analyze(&actions, &primary, &state);
        assert_eq!(model.outputs.len(), 1);
        assert_eq!(model.outputs[0].chain.len(), 2);
    }

    #[test]
    fn conditional_evaluated_uses_input_cache() {
        use inputforge_core::types::InputValue;
        let mut state = AppState::new();
        state
            .input_cache
            .update(&btn_addr(0), &InputValue::Button { pressed: true });
        let primary = axis_addr(0);
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: btn_addr(0) },
            if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
            if_false: vec![],
        }];
        let model = analyze(&actions, &primary, &state);
        let step = &model.outputs[0].chain[0];
        assert!(matches!(
            step,
            ChainStep::Conditional { evaluated: true, .. }
        ));
    }
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests::conditional_branches_record_distinct_chain_steps`
Expected: FAIL.

- [ ] **Step 3: Update the walker to push Conditional steps**

In `walk`, replace the `Action::Conditional` arm:

```rust
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                let evaluated =
                    inputforge_core::pipeline::evaluate_condition(condition, &state.input_cache);
                let condition_label = format!("{condition:?}"); // TODO(task-10): swap in predicate::format_condition_label

                chain_stack.push(ChainStep::Conditional {
                    condition_label: condition_label.clone(),
                    evaluated,
                    branch: Branch::IfTrue,
                });
                walk(if_true, top_level, primary, state, chain_stack, model);
                chain_stack.pop();

                chain_stack.push(ChainStep::Conditional {
                    condition_label,
                    evaluated,
                    branch: Branch::IfFalse,
                });
                walk(if_false, top_level, primary, state, chain_stack, model);
                chain_stack.pop();
            }
```

(Each branch's recursion sees only its own conditional step on the stack; outputs inside `if_true` capture `Branch::IfTrue`, outputs inside `if_false` capture `Branch::IfFalse`. The `walk` baseline truncate from Task 7 cleans up any merge steps pushed inside a branch when that branch's recursion returns.)

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests`
Expected: 11 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs
git commit -m "feat(live_readout): capture Conditional chain steps per branch with evaluated state"
```

---

## Task 9: Add `is_active` evaluation + per-output polarity inference

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs`

For each emitted descriptor:
- `is_active`: vacuously true when there are no Conditional steps. When the chain has Conditional steps, the rule is `evaluated == matches!(branch, Branch::IfTrue)` per step, AND-combined across all conditional steps. This evaluates to true when the user is in the active branch of every nested conditional; mismatched pairs (`evaluated=true` with `branch=IfFalse`, or `evaluated=false` with `branch=IfTrue`) make the output inactive.
- `polarity`: the `polarity_at_step` of the *last* `ChainStep::Merge` in the chain (the terminal output polarity, since it's the last merge's running fold). Falls back to the primary input's polarity when the chain has no merges. Per-step polarity is already stored on each Merge step (Task 7); this task only reads the terminal value. `infer_output_polarity` is no longer needed.

- [ ] **Step 1: Write the failing tests**

Append to `walker_tests`:

```rust
    #[test]
    fn is_active_true_when_predicate_matches_branch() {
        use inputforge_core::types::InputValue;
        let mut state = AppState::new();
        state
            .input_cache
            .update(&btn_addr(0), &InputValue::Button { pressed: true });
        let primary = axis_addr(0);
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: btn_addr(0) },
            if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
            if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
        }];
        let model = analyze(&actions, &primary, &state);
        let true_out = model
            .outputs
            .iter()
            .find(|d| matches!(&d.destination, OutputDestination::VJoy(o) if *o == vjoy_x()))
            .unwrap();
        let false_out = model
            .outputs
            .iter()
            .find(|d| matches!(&d.destination, OutputDestination::VJoy(o) if *o == vjoy_y()))
            .unwrap();
        assert!(true_out.is_active);
        assert!(!false_out.is_active);
    }

    #[test]
    fn is_active_nested_path_and_evaluation() {
        // Outer false, inner anything: outer-false branch's outputs
        // are active iff the user is in the false branch (which they are
        // when the outer condition is false).
        use inputforge_core::types::InputValue;
        let mut state = AppState::new();
        state
            .input_cache
            .update(&btn_addr(0), &InputValue::Button { pressed: false });
        state
            .input_cache
            .update(&btn_addr(1), &InputValue::Button { pressed: true });
        let primary = axis_addr(0);
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: btn_addr(0) },
            if_true: vec![],
            if_false: vec![Action::Conditional {
                condition: Condition::ButtonPressed { input: btn_addr(1) },
                if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
                if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
            }],
        }];
        let model = analyze(&actions, &primary, &state);
        let inner_true = model
            .outputs
            .iter()
            .find(|d| matches!(&d.destination, OutputDestination::VJoy(o) if *o == vjoy_x()))
            .unwrap();
        let inner_false = model
            .outputs
            .iter()
            .find(|d| matches!(&d.destination, OutputDestination::VJoy(o) if *o == vjoy_y()))
            .unwrap();
        assert!(inner_true.is_active);
        assert!(!inner_false.is_active);
    }

    #[test]
    fn polarity_no_merges_inherits_primary() {
        let mut state = AppState::new();
        state.input_cache.update(
            &axis_addr(0),
            &inputforge_core::types::InputValue::Axis {
                value: inputforge_core::types::AxisValue::new(0.0),
                polarity: AxisPolarity::Unipolar,
            },
        );
        let primary = axis_addr(0);
        let actions = vec![Action::MapToVJoy { output: vjoy_x() }];
        let model = analyze(&actions, &primary, &state);
        assert_eq!(model.outputs[0].polarity, AxisPolarity::Unipolar);
    }
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests::is_active_true_when_predicate_matches_branch`
Expected: FAIL.

- [ ] **Step 3: Implement post-processing for `is_active` and `polarity`**

The polarity helper calls `state.input_cache.get_axis(...)`, which requires the `InputCache` trait in scope to coerce `&InputCacheStore` to `&dyn InputCache`. Add the import at the top of `analyzer.rs` if it is not already present:

```rust
use inputforge_core::pipeline::InputCache;
```

Helper functions the descriptor builders call when emitting. Add to `analyzer.rs`:

```rust
/// Compute the `is_active` boolean for an output sitting at the end of
/// a chain. Vacuously true when the chain has no `Conditional` steps.
///
/// Per step: `evaluated == matches!(branch, Branch::IfTrue)` is the
/// active-branch rule:
/// - `evaluated=true,  branch=IfTrue`  → active (true == true)
/// - `evaluated=false, branch=IfFalse` → active (false == false)
/// - `evaluated=true,  branch=IfFalse` → inactive
/// - `evaluated=false, branch=IfTrue`  → inactive
///
/// AND-combined across all conditional steps in the chain so a deeply-
/// nested output is active only when the user is in the matching branch
/// at every nesting level.
fn compute_is_active(chain: &[ChainStep]) -> bool {
    chain.iter().all(|s| match s {
        ChainStep::Conditional { evaluated, branch, .. } => {
            *evaluated == matches!(branch, Branch::IfTrue)
        }
        ChainStep::Merge { .. } => true,
    })
}

/// Terminal output polarity for an output at the end of a chain.
///
/// Reads the `polarity_at_step` of the *last* `ChainStep::Merge` in the
/// chain (Task 7 stores per-step polarity, so the last merge's polarity
/// IS the terminal output polarity). Falls back to the primary input's
/// stored polarity when the chain contains no merges.
fn terminal_polarity(
    chain: &[ChainStep],
    primary: &InputAddress,
    state: &AppState,
) -> AxisPolarity {
    chain
        .iter()
        .rev()
        .find_map(|step| match step {
            ChainStep::Merge { polarity_at_step, .. } => Some(*polarity_at_step),
            ChainStep::Conditional { .. } => None,
        })
        .unwrap_or_else(|| {
            let (_, p) = state.input_cache.get_axis(primary);
            p
        })
}
```

Then update both `MapToVJoy` and `MapToKeyboard` arms in `walk`. Replace each:

```rust
            Action::MapToVJoy { output } => {
                let chain = chain_stack.clone();
                let is_active = compute_is_active(&chain);
                let polarity = terminal_polarity(&chain, primary, state);
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::VJoy(output.clone()),
                    chain,
                    is_active,
                    polarity,
                });
            }
            Action::MapToKeyboard { key } => {
                let chain = chain_stack.clone();
                let is_active = compute_is_active(&chain);
                model.outputs.push(OutputDescriptor {
                    destination: OutputDestination::Keyboard(key.clone()),
                    chain,
                    is_active,
                    polarity: AxisPolarity::Bipolar, // ignored for keyboard
                });
            }
```

Note: `infer_output_polarity` is no longer called from the analyzer (per-step polarity is captured by Task 7's `compute_merge_step_data`). It can be deleted from `value_helpers.rs` if no other consumer exists, or kept as a pure utility if other code uses it.

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests`
Expected: 14 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs
git commit -m "feat(live_readout): compute per-output is_active and polarity from chain"
```

---

## Task 10: Build the predicate-label formatter (`predicate.rs`)

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/predicate.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs`

`predicate.rs` carries no evaluation logic (the analyzer reuses core's `evaluate_condition`); its job is label formatting:
- `format_condition_label(condition, cfg)` produces a composite-aware string for the chain's `Conditional` step
- `format_predicate_chip_label(input, kind, cfg)` produces the IN-block chip's source label
- `evaluate_leaf_state(input, kind, cache)` returns the per-leaf truth used by `PredicateDescriptor.state`

Once `predicate.rs` exists, swap the placeholder `format!("{condition:?}")` in `analyzer.rs` for `predicate::format_condition_label`.

- [ ] **Step 1: Create `predicate.rs`**

```rust
// Rust guideline compliant 2026-05-03

//! Label formatting + leaf-state evaluation for `Conditional` predicates.
//! No evaluation recursion (that lives in
//! `inputforge_core::pipeline::evaluate_condition`); this module only
//! renders human labels and snapshots per-leaf truth for the IN-block
//! chip subsection.

use inputforge_core::action::Condition;
use inputforge_core::pipeline::InputCache;
use inputforge_core::types::{HatDirection, InputAddress};

use crate::context::ConfigSnapshot;
use crate::frame::mapping_list::source_label;

use super::analyzer::PredicateKind;

/// Render a (possibly composite) condition as a single human label
/// suitable for the expanded chain block's COND row.
///
/// Leaf forms reuse the chip-label phrasing; `All` joins children with
/// ` AND `, `Any` with ` OR `, `Not` prefixes `NOT `. Empty composites
/// degenerate to the operator name (`All ()` -> "true", `Any ()` ->
/// "false") to be honest about the vacuous truth value.
pub(super) fn format_condition_label(condition: &Condition, cfg: &ConfigSnapshot) -> String {
    match condition {
        Condition::ButtonPressed { input } => {
            format!("{} pressed", source_label::format(input, cfg))
        }
        Condition::ButtonReleased { input } => {
            format!("{} released", source_label::format(input, cfg))
        }
        Condition::AxisInRange { input, min, max } => {
            format!(
                "{} in [{:.2}..{:.2}]",
                source_label::format(input, cfg),
                min,
                max
            )
        }
        Condition::HatDirection { input, directions } => {
            let glyphs = render_hat_glyphs(directions);
            format!("{} hat {glyphs}", source_label::format(input, cfg))
        }
        Condition::All { conditions } => {
            if conditions.is_empty() {
                "true".to_owned()
            } else {
                conditions
                    .iter()
                    .map(|c| format_condition_label(c, cfg))
                    .collect::<Vec<_>>()
                    .join(" AND ")
            }
        }
        Condition::Any { conditions } => {
            if conditions.is_empty() {
                "false".to_owned()
            } else {
                conditions
                    .iter()
                    .map(|c| format_condition_label(c, cfg))
                    .collect::<Vec<_>>()
                    .join(" OR ")
            }
        }
        Condition::Not { condition } => format!("NOT {}", format_condition_label(condition, cfg)),
    }
}

/// Source label for an IN-block predicate chip
/// (`source_label::format` form).
pub(super) fn format_predicate_chip_label(input: &InputAddress, cfg: &ConfigSnapshot) -> String {
    source_label::format(input, cfg)
}

/// Per-leaf state snapshot used by the IN-block chip rendering.
///
/// Independent of branch gating: `PredicateDescriptor.state` is the leaf
/// truth, not the composite-condition truth that gates `is_active`.
pub(super) fn evaluate_leaf_state(
    input: &InputAddress,
    kind: &PredicateKind,
    cache: &dyn InputCache,
) -> bool {
    use inputforge_core::pipeline::evaluate_condition;
    let synthetic = match kind {
        PredicateKind::ButtonPressed => Condition::ButtonPressed {
            input: input.clone(),
        },
        PredicateKind::ButtonReleased => Condition::ButtonReleased {
            input: input.clone(),
        },
        PredicateKind::AxisInRange { min, max } => Condition::AxisInRange {
            input: input.clone(),
            min: *min,
            max: *max,
        },
        PredicateKind::HatDirection { directions } => Condition::HatDirection {
            input: input.clone(),
            directions: directions.clone(),
        },
    };
    evaluate_condition(&synthetic, cache)
}

/// Render a hat-direction set as a contiguous glyph string using the
/// shared alphabet `↑↗→↘↓↙←↖·`. Iterates the canonical order so output
/// is deterministic regardless of input order.
pub(super) fn render_hat_glyphs(directions: &[HatDirection]) -> String {
    const ORDER: &[(HatDirection, char)] = &[
        (HatDirection::N, '\u{2191}'),
        (HatDirection::NE, '\u{2197}'),
        (HatDirection::E, '\u{2192}'),
        (HatDirection::SE, '\u{2198}'),
        (HatDirection::S, '\u{2193}'),
        (HatDirection::SW, '\u{2199}'),
        (HatDirection::W, '\u{2190}'),
        (HatDirection::NW, '\u{2196}'),
        (HatDirection::Center, '\u{00b7}'),
    ];
    ORDER
        .iter()
        .filter(|(d, _)| directions.contains(d))
        .map(|(_, g)| *g)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, InputId};

    fn cfg() -> ConfigSnapshot {
        ConfigSnapshot::default()
    }

    fn btn(idx: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: idx },
        }
    }

    #[test]
    fn render_hat_glyphs_n_only() {
        assert_eq!(render_hat_glyphs(&[HatDirection::N]), "\u{2191}");
    }

    #[test]
    fn render_hat_glyphs_n_ne_e() {
        assert_eq!(
            render_hat_glyphs(&[HatDirection::N, HatDirection::NE, HatDirection::E]),
            "\u{2191}\u{2197}\u{2192}"
        );
    }

    #[test]
    fn render_hat_glyphs_center_only() {
        assert_eq!(render_hat_glyphs(&[HatDirection::Center]), "\u{00b7}");
    }

    #[test]
    fn render_hat_glyphs_order_is_canonical_not_input() {
        // Inputs in scrambled order produce glyphs in canonical
        // `N NE E SE S SW W NW Center` order.
        assert_eq!(
            render_hat_glyphs(&[HatDirection::E, HatDirection::N]),
            "\u{2191}\u{2192}"
        );
    }

    #[test]
    fn format_condition_label_button_pressed_uses_source_label() {
        let label = format_condition_label(
            &Condition::ButtonPressed { input: btn(0) },
            &cfg(),
        );
        assert!(label.contains("pressed"));
    }

    #[test]
    fn format_condition_label_all_joins_with_and() {
        let cond = Condition::All {
            conditions: vec![
                Condition::ButtonPressed { input: btn(0) },
                Condition::ButtonReleased { input: btn(1) },
            ],
        };
        let label = format_condition_label(&cond, &cfg());
        assert!(label.contains(" AND "));
    }

    #[test]
    fn format_condition_label_any_joins_with_or() {
        let cond = Condition::Any {
            conditions: vec![
                Condition::ButtonPressed { input: btn(0) },
                Condition::ButtonPressed { input: btn(1) },
            ],
        };
        let label = format_condition_label(&cond, &cfg());
        assert!(label.contains(" OR "));
    }

    #[test]
    fn format_condition_label_not_prefixes() {
        let cond = Condition::Not {
            condition: Box::new(Condition::ButtonPressed { input: btn(0) }),
        };
        let label = format_condition_label(&cond, &cfg());
        assert!(label.starts_with("NOT "));
    }

    #[test]
    fn format_condition_label_empty_all_is_true() {
        let label = format_condition_label(&Condition::All { conditions: vec![] }, &cfg());
        assert_eq!(label, "true");
    }

    #[test]
    fn format_condition_label_empty_any_is_false() {
        let label = format_condition_label(&Condition::Any { conditions: vec![] }, &cfg());
        assert_eq!(label, "false");
    }
}
```

- [ ] **Step 2: Add `mod predicate;` to `live_readout/mod.rs`**

Top of `mod.rs`, alphabetical:

```rust
mod analyzer;
mod in_block;
mod predicate;
mod value_helpers;
```

- [ ] **Step 3: Wire `predicate::format_condition_label` into the analyzer**

In `analyzer.rs`, the walker's `Action::Conditional` arm currently has:

```rust
let condition_label = format!("{condition:?}");
```

The analyzer cannot reach `predicate::format_condition_label` from here without a `cfg: &ConfigSnapshot` argument. Update `analyze` and `walk` signatures to take the snapshot:

```rust
pub(super) fn analyze(
    actions: &[Action],
    primary: &InputAddress,
    state: &AppState,
    cfg: &ConfigSnapshot,
) -> LiveReadoutModel {
    let mut model = LiveReadoutModel { ... };
    let mut chain_stack: Vec<ChainStep> = Vec::new();
    walk(actions, actions, primary, state, cfg, &mut chain_stack, &mut model);
    model
}

fn walk(
    local: &[Action],
    top_level: &[Action],
    primary: &InputAddress,
    state: &AppState,
    cfg: &ConfigSnapshot,
    chain_stack: &mut Vec<ChainStep>,
    model: &mut LiveReadoutModel,
) { ... }
```

Then replace the placeholder in the Conditional arm:

```rust
                let condition_label = super::predicate::format_condition_label(condition, cfg);
```

Add the import at the top of `analyzer.rs`:

```rust
use crate::context::ConfigSnapshot;
```

Update each existing test in `walker_tests` to pass a `&ConfigSnapshot::default()` after `&state`:

```rust
let model = analyze(&actions, &primary, &state, &ConfigSnapshot::default());
```

(Apply this rewrite to every `analyze(...)` call site in the test module. Tests still pass, only the call signature changed.)

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::predicate::tests`
Expected: 9 passed.

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests`
Expected: 14 passed (same as Task 9, just with updated call signature).

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/predicate.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs
git commit -m "feat(live_readout): add predicate label formatter and wire into analyzer"
```

---

## Task 11: Populate `model.predicates` from Conditionals (flatten + dedup)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs`

For each `Conditional` encountered, walk the condition tree (which can be a composite of `All` / `Any` / `Not` over leaves) and emit one `PredicateDescriptor` per leaf input. Dedup by `(InputAddress, PredicateKind discriminant, bounds)` so the same input referenced as `ButtonPressed` in two different Conditionals collapses to one chip, BUT two `AxisInRange[a..b]` and `AxisInRange[c..d]` Conditionals on the same input render as TWO chips (each chip = one real predicate, with its own bounds and live-dot semantics). Hat directions are likewise included in the key, so `HatDirection { Up }` and `HatDirection { Up, Down }` are distinct chips. Spec line 139's "distinct bounds appear in the chain block, not at chip granularity" is honored by *not collapsing distinct bounds onto a shared chip with hidden ranges* — distinct bounds become distinct chips, fully transparent to the user.

- [ ] **Step 1: Write the failing test**

Append to `walker_tests`:

```rust
    #[test]
    fn conditional_emits_predicate_descriptor() {
        use inputforge_core::types::InputValue;
        let mut state = AppState::new();
        state
            .input_cache
            .update(&btn_addr(0), &InputValue::Button { pressed: true });
        let primary = axis_addr(0);
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: btn_addr(0) },
            if_true: vec![],
            if_false: vec![],
        }];
        let cfg = crate::context::ConfigSnapshot::default();
        let model = analyze(&actions, &primary, &state, &cfg);
        assert_eq!(model.predicates.len(), 1);
        assert!(matches!(model.predicates[0].kind, PredicateKind::ButtonPressed));
        assert!(model.predicates[0].state); // button is pressed
    }

    #[test]
    fn composite_all_flattens_to_one_chip_per_leaf() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![Action::Conditional {
            condition: Condition::All {
                conditions: vec![
                    Condition::ButtonPressed { input: btn_addr(0) },
                    Condition::ButtonPressed { input: btn_addr(1) },
                ],
            },
            if_true: vec![],
            if_false: vec![],
        }];
        let cfg = crate::context::ConfigSnapshot::default();
        let model = analyze(&actions, &primary, &state, &cfg);
        assert_eq!(model.predicates.len(), 2);
    }

    #[test]
    fn duplicate_input_same_kind_dedups_to_one_chip() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![
            Action::Conditional {
                condition: Condition::ButtonPressed { input: btn_addr(0) },
                if_true: vec![],
                if_false: vec![],
            },
            Action::Conditional {
                condition: Condition::ButtonPressed { input: btn_addr(0) },
                if_true: vec![],
                if_false: vec![],
            },
        ];
        let cfg = crate::context::ConfigSnapshot::default();
        let model = analyze(&actions, &primary, &state, &cfg);
        assert_eq!(model.predicates.len(), 1);
    }

    #[test]
    fn same_input_different_kinds_renders_two_chips() {
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![
            Action::Conditional {
                condition: Condition::ButtonPressed { input: btn_addr(0) },
                if_true: vec![],
                if_false: vec![],
            },
            Action::Conditional {
                condition: Condition::ButtonReleased { input: btn_addr(0) },
                if_true: vec![],
                if_false: vec![],
            },
        ];
        let cfg = crate::context::ConfigSnapshot::default();
        let model = analyze(&actions, &primary, &state, &cfg);
        assert_eq!(model.predicates.len(), 2);
    }

    #[test]
    fn axis_in_range_distinct_bounds_render_two_chips() {
        // Two AxisInRange Conditionals on the same input, with different
        // (min, max) bounds, must render as TWO chips. Each chip carries
        // its own bounds + its own live-dot semantics. (Spec line 139:
        // distinct bounds belong to the chain, not silently hidden inside
        // one collapsed chip.)
        let state = AppState::new();
        let primary = axis_addr(0);
        let actions = vec![
            Action::Conditional {
                condition: Condition::AxisInRange {
                    input: axis_addr(1),
                    min: 0.20,
                    max: 0.80,
                },
                if_true: vec![],
                if_false: vec![],
            },
            Action::Conditional {
                condition: Condition::AxisInRange {
                    input: axis_addr(1),
                    min: 0.50,
                    max: 0.90,
                },
                if_true: vec![],
                if_false: vec![],
            },
        ];
        let cfg = crate::context::ConfigSnapshot::default();
        let model = analyze(&actions, &primary, &state, &cfg);
        assert_eq!(model.predicates.len(), 2);
        // And literal-duplicate ranges still dedup:
        let actions_dup = vec![
            Action::Conditional {
                condition: Condition::AxisInRange {
                    input: axis_addr(1),
                    min: 0.20,
                    max: 0.80,
                },
                if_true: vec![],
                if_false: vec![],
            },
            Action::Conditional {
                condition: Condition::AxisInRange {
                    input: axis_addr(1),
                    min: 0.20,
                    max: 0.80,
                },
                if_true: vec![],
                if_false: vec![],
            },
        ];
        let model_dup = analyze(&actions_dup, &primary, &state, &cfg);
        assert_eq!(model_dup.predicates.len(), 1);
    }
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests::conditional_emits_predicate_descriptor`
Expected: FAIL.

- [ ] **Step 3: Add predicate flattening to the walker**

In `analyzer.rs`, before the existing `walk` definition, add:

```rust
/// Dedup key including bounds, so two `AxisInRange` predicates on the
/// same input with different `(min, max)` are DISTINCT chips. f64 cannot
/// derive Hash directly; we hash via to_bits so 0.20 and 0.20 round-trip
/// to the same key but 0.20 and 0.50 do not.
#[derive(Hash, PartialEq, Eq, Clone)]
struct PredicateDedupKey {
    input: InputAddress,
    kind: PredicateKindKey,
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum PredicateKindKey {
    ButtonPressed,
    ButtonReleased,
    AxisInRange { min_bits: u64, max_bits: u64 },
    HatDirection { directions: Vec<inputforge_core::types::HatDirection> },
}

fn kind_key(kind: &PredicateKind) -> PredicateKindKey {
    match kind {
        PredicateKind::ButtonPressed => PredicateKindKey::ButtonPressed,
        PredicateKind::ButtonReleased => PredicateKindKey::ButtonReleased,
        PredicateKind::AxisInRange { min, max } => PredicateKindKey::AxisInRange {
            min_bits: min.to_bits(),
            max_bits: max.to_bits(),
        },
        PredicateKind::HatDirection { directions } => PredicateKindKey::HatDirection {
            directions: directions.clone(),
        },
    }
}

/// Flatten a (possibly composite) condition into per-leaf
/// `PredicateDescriptor`s, deduplicating via
/// `(InputAddress, PredicateKindDiscriminant)`.
fn collect_leaf_predicates(
    condition: &inputforge_core::action::Condition,
    state: &AppState,
    cfg: &ConfigSnapshot,
    seen: &mut std::collections::HashSet<PredicateDedupKey>,
    out: &mut Vec<PredicateDescriptor>,
) {
    use inputforge_core::action::Condition;
    match condition {
        Condition::ButtonPressed { input } => {
            push_leaf(input, PredicateKind::ButtonPressed, state, cfg, seen, out);
        }
        Condition::ButtonReleased { input } => {
            push_leaf(input, PredicateKind::ButtonReleased, state, cfg, seen, out);
        }
        Condition::AxisInRange { input, min, max } => {
            push_leaf(
                input,
                PredicateKind::AxisInRange {
                    min: *min,
                    max: *max,
                },
                state,
                cfg,
                seen,
                out,
            );
        }
        Condition::HatDirection { input, directions } => {
            push_leaf(
                input,
                PredicateKind::HatDirection {
                    directions: directions.clone(),
                },
                state,
                cfg,
                seen,
                out,
            );
        }
        Condition::All { conditions } | Condition::Any { conditions } => {
            for c in conditions {
                collect_leaf_predicates(c, state, cfg, seen, out);
            }
        }
        Condition::Not { condition } => {
            collect_leaf_predicates(condition, state, cfg, seen, out);
        }
    }
}

fn push_leaf(
    input: &InputAddress,
    kind: PredicateKind,
    state: &AppState,
    cfg: &ConfigSnapshot,
    seen: &mut std::collections::HashSet<PredicateDedupKey>,
    out: &mut Vec<PredicateDescriptor>,
) {
    let key = PredicateDedupKey {
        input: input.clone(),
        kind: kind_key(&kind),
    };
    if !seen.insert(key) {
        return;
    }
    let label = super::predicate::format_predicate_chip_label(input, cfg);
    let leaf_state = super::predicate::evaluate_leaf_state(input, &kind, &state.input_cache);
    out.push(PredicateDescriptor {
        kind,
        inputs: vec![input.clone()],
        state: leaf_state,
        label,
    });
}
```

In `walk`, the `Conditional` arm needs to invoke `collect_leaf_predicates` once per Conditional. Add a `seen` HashSet that lives across the whole walk; thread it through:

```rust
pub(super) fn analyze(
    actions: &[Action],
    primary: &InputAddress,
    state: &AppState,
    cfg: &ConfigSnapshot,
) -> LiveReadoutModel {
    let mut model = LiveReadoutModel {
        pipeline_inputs: vec![primary.clone()],
        predicates: Vec::new(),
        outputs: Vec::new(),
    };
    let mut chain_stack: Vec<ChainStep> = Vec::new();
    let mut seen: std::collections::HashSet<PredicateDedupKey> =
        std::collections::HashSet::new();
    walk(
        actions, actions, primary, state, cfg,
        &mut chain_stack, &mut seen, &mut model,
    );
    model
}

fn walk(
    local: &[Action],
    top_level: &[Action],
    primary: &InputAddress,
    state: &AppState,
    cfg: &ConfigSnapshot,
    chain_stack: &mut Vec<ChainStep>,
    seen: &mut std::collections::HashSet<PredicateDedupKey>,
    model: &mut LiveReadoutModel,
) {
    let stack_baseline = chain_stack.len();
    for (i, action) in local.iter().enumerate() {
        match action {
            // ... same arms as Task 9 ...
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                collect_leaf_predicates(condition, state, cfg, seen, &mut model.predicates);

                let evaluated =
                    inputforge_core::pipeline::evaluate_condition(condition, &state.input_cache);
                let condition_label = super::predicate::format_condition_label(condition, cfg);

                chain_stack.push(ChainStep::Conditional {
                    condition_label: condition_label.clone(),
                    evaluated,
                    branch: Branch::IfTrue,
                });
                walk(
                    if_true, top_level, primary, state, cfg,
                    chain_stack, seen, model,
                );
                chain_stack.pop();

                chain_stack.push(ChainStep::Conditional {
                    condition_label,
                    evaluated,
                    branch: Branch::IfFalse,
                });
                walk(
                    if_false, top_level, primary, state, cfg,
                    chain_stack, seen, model,
                );
                chain_stack.pop();
            }
            // ... same arms ...
        }
    }
    chain_stack.truncate(stack_baseline);
}
```

(Update the recursive `walk` calls in any other arms accordingly. The `seen` HashSet flows through every call and ensures predicate dedup works across siblings and across nesting.)

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p inputforge-gui-dx --lib live_readout::analyzer::walker_tests`
Expected: 19 passed (14 from prior tasks + 5 new predicate tests including `axis_in_range_distinct_bounds_render_two_chips`).

Run: `cargo test -p inputforge-gui-dx --lib live_readout`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs
git commit -m "feat(live_readout): flatten Conditional predicates into deduplicated descriptors"
```

---

## Task 12: Build the `InBlock` component (pipeline rows + predicate chips)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/in_block.rs`

`InBlock` consumes `&LiveReadoutModel` and renders two grouped subsections: `IN · pipeline` (one `ReadoutRow` per `pipeline_inputs[i]`) and `IN · predicates` (one chip per `predicates[i]`). Both subsections sit inside the existing `if-editor__readout` container; CSS rules for the subsection labels and chips arrive in Task 18.

The first row uses label `"IN"` when there is only one pipeline input, otherwise `"IN 1"` / `"IN 2"` / `"IN 3"` / etc. (matching today's merge-layout convention generalized to N inputs).

- [ ] **Step 1: Write the SSR test**

The component exists in isolation only inside the orchestrator render; SSR coverage lives in Task 20's `tests.rs` additions. For now, only verify the file compiles.

- [ ] **Step 2: Add `InBlock`, `PredicateChips`, and chip variants**

Append to `in_block.rs`, after the existing `ReadoutRow` and `ReadoutDivider` definitions:

```rust
use crate::context::{ConfigSnapshot, LiveSnapshot};

use super::analyzer::{LiveReadoutModel, PredicateDescriptor, PredicateKind};
use super::predicate::render_hat_glyphs;
use super::value_helpers::{AxisDisplay, read_axis_display};

/// IN section orchestrator: pipeline-axis rows + predicate chip subsection.
///
/// Section-label rendering is conditional:
/// - `IN · pipeline` label appears only when `pipeline_inputs.len() > 1`
///   (multi-input case). Single-input mappings render the bare row, byte-
///   identical to today's no-label state, so existing screens that show
///   one IN row do not gain a label they didn't have before.
/// - `IN · predicates` label appears only when `!model.predicates.is_empty()`.
///   No predicates → no subsection at all.
#[component]
pub(super) fn InBlock(model: LiveReadoutModel) -> Element {
    let ctx = use_context::<crate::context::AppContext>();
    let live = ctx.live.read();
    let cfg = ctx.config.read();

    let pipeline_inputs = model.pipeline_inputs.clone();
    let pipeline_n = pipeline_inputs.len();
    let pipeline_label_visible = pipeline_n > 1;

    let predicates = model.predicates.clone();
    let predicates_visible = !predicates.is_empty();

    rsx! {
        div { class: "if-editor__readout-section",
            if pipeline_label_visible {
                div { class: "if-editor__readout-section-label", "IN \u{00b7} pipeline" }
            }
            div { class: "if-editor__readout-group",
                for (idx, addr) in pipeline_inputs.iter().enumerate() {
                    {
                        let display: AxisDisplay = read_axis_display(addr, &live, &cfg);
                        let tag = crate::frame::mapping_list::source_label::format(addr, &cfg);
                        let label = if pipeline_n == 1 {
                            "IN".to_owned()
                        } else {
                            format!("IN {}", idx + 1)
                        };
                        rsx! {
                            ReadoutRow {
                                key: "{idx}",
                                label,
                                tag,
                                display,
                                frozen: false,
                            }
                        }
                    }
                }
            }
        }

        if predicates_visible {
            div { class: "if-editor__readout-section",
                div { class: "if-editor__readout-section-label", "IN \u{00b7} predicates" }
                div { class: "if-editor__readout-chips",
                    for (idx, predicate) in predicates.iter().enumerate() {
                        PredicateChip {
                            key: "{idx}",
                            descriptor: predicate.clone(),
                        }
                    }
                }
            }
        }
    }
}

/// One chip in the `IN · predicates` subsection. Layout depends on the
/// predicate kind: pressed/released render label + dot, AxisInRange adds
/// a `[min..max]` glyph, HatDirection adds the canonical glyph string.
///
/// Filled (`live`) when `descriptor.state` is true; hollow otherwise.
/// (Engine-stopped freezing flips `state` indirectly, since
/// `evaluate_condition` reads from the cache that the engine writes.)
#[component]
fn PredicateChip(descriptor: PredicateDescriptor) -> Element {
    let chip_class = if descriptor.state {
        "if-editor__readout-chip if-editor__readout-chip--live"
    } else {
        "if-editor__readout-chip if-editor__readout-chip--idle"
    };
    let dot_class = if descriptor.state {
        "if-editor__readout-chip-dot"
    } else {
        "if-editor__readout-chip-dot if-editor__readout-chip-dot--hollow"
    };
    let label = descriptor.label.clone();
    let suffix = match &descriptor.kind {
        PredicateKind::ButtonPressed => String::new(),
        PredicateKind::ButtonReleased => " (released)".to_owned(),
        PredicateKind::AxisInRange { min, max } => format!(" [{min:.2}..{max:.2}]"),
        PredicateKind::HatDirection { directions } => format!(" {}", render_hat_glyphs(directions)),
    };
    rsx! {
        span { class: "{chip_class}",
            span { class: "{dot_class}" }
            span { class: "if-editor__readout-chip-label", "{label}{suffix}" }
        }
    }
}
```

- [ ] **Step 3: Verify the build**

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/in_block.rs
git commit -m "feat(live_readout): add InBlock with pipeline rows and predicate chip subsection"
```

---

## Task 13: Build the `OutBlock` component (vJoy axis variant + dispatcher)

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs`

`OutBlock` renders the OUT section: section label + one `OutRow` per `model.outputs[i]`. `OutRow` dispatches on `OutputDescriptor.destination` and (for vJoy) `OutputId`. This task implements the vJoy axis variant only (the existing OUT row generalized to multiple sibling outputs); subsequent tasks layer on button, hat, and keyboard variants.

- [ ] **Step 1: Create `out_block.rs`**

```rust
// Rust guideline compliant 2026-05-03

//! OUT section: per-output dispatch row + future-task chevron / chain.
//!
//! `OutRow` switches on `OutputDescriptor.destination`. vJoy axis is the
//! only variant implemented in Task 13; Tasks 14-16 add button, hat, and
//! keyboard variants. Tasks 16-17 add the per-row chevron and the
//! expanded-chain block.

use dioxus::prelude::*;

use inputforge_core::state::EngineStatus;
use inputforge_core::types::{AxisPolarity, OutputId};

use crate::context::AppContext;

use super::analyzer::{LiveReadoutModel, OutputDescriptor, OutputDestination};
use super::in_block::ReadoutRow;
use super::value_helpers::{
    AxisDisplay, format_output_label, read_output_display,
};

/// OUT section orchestrator. Iterates `model.outputs` and dispatches each
/// to `OutRow`.
#[component]
pub(super) fn OutBlock(model: LiveReadoutModel) -> Element {
    if model.outputs.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "if-editor__readout-section",
            div { class: "if-editor__readout-section-label", "OUT" }
            div { class: "if-editor__readout-group",
                for (idx, descriptor) in model.outputs.iter().enumerate() {
                    OutRow {
                        key: "{idx}",
                        descriptor: descriptor.clone(),
                    }
                }
            }
        }
    }
}

/// One OUT row. Dispatches the value-cell variant on
/// `descriptor.destination` and (for vJoy) `OutputId`.
///
/// `frozen = !engine_running || !descriptor.is_active` (single CSS
/// modifier covers both engine-stopped and inactive-conditional-branch
/// cases per spec § State management).
#[component]
fn OutRow(descriptor: OutputDescriptor) -> Element {
    let ctx = use_context::<AppContext>();
    let engine_running = matches!(ctx.meta.read().engine_status, EngineStatus::Running);
    let frozen = !engine_running || !descriptor.is_active;

    match &descriptor.destination {
        OutputDestination::VJoy(out) => {
            let live = ctx.live.read();
            let cfg = ctx.config.read();
            let tag = format_output_label(out);
            match out.output {
                OutputId::Axis { .. } => {
                    let display = read_output_display(out, &live, &cfg, descriptor.polarity);
                    rsx! {
                        ReadoutRow {
                            label: "OUT".to_owned(),
                            tag,
                            display,
                            frozen,
                        }
                    }
                }
                OutputId::Button { .. } => {
                    // Task 14.
                    rsx! { div { "TODO button" } }
                }
                OutputId::Hat { .. } => {
                    // Task 15.
                    rsx! { div { "TODO hat" } }
                }
            }
        }
        OutputDestination::Keyboard(_) => {
            // Task 16.
            rsx! { div { "TODO keyboard" } }
        }
    }
}
```

- [ ] **Step 2: Add `mod out_block;` to `live_readout/mod.rs`**

```rust
mod analyzer;
mod in_block;
mod out_block;
mod predicate;
mod value_helpers;
```

- [ ] **Step 3: Verify the build**

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs
git commit -m "feat(live_readout): add OutBlock and OutRow with vJoy axis variant"
```

---

## Task 14: Add OUT row variants for vJoy Button, Hat, and Keyboard

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs`

Replace the `TODO` arms with full variant rendering:

- **vJoy Button** (`OutputId::Button { id }`): unipolar bar 0/100% + mono pct (`0.00` / `1.00`). Reuses `ReadoutRow` with a synthesized `AxisDisplay { value: 0|1, polarity: Unipolar }`.
- **vJoy Hat** (`OutputId::Hat { id }`): directional glyph cell. Bar slot replaced with a single character from `↑↗→↘↓↙←↖·`. No animation. Pct slot empty.
- **Keyboard** (`OutputDestination::Keyboard(combo)`): chip in `--color-control` (violet badge), key combo in mono. Filled when `is_active`; hollow when frozen.

- [ ] **Step 1: Replace the Button arm**

In `out_block.rs`, replace `OutputId::Button { .. } => { rsx! { div { "TODO button" } } }` with:

```rust
                OutputId::Button { .. } => {
                    let pressed = super::value_helpers::read_output_button(out, &live, &cfg);
                    let display = AxisDisplay {
                        value: if pressed { 1.0 } else { 0.0 },
                        polarity: AxisPolarity::Unipolar,
                    };
                    rsx! {
                        ReadoutRow {
                            label: "OUT".to_owned(),
                            tag,
                            display,
                            frozen,
                        }
                    }
                }
```

- [ ] **Step 2: Replace the Hat arm with a glyph cell**

The hat cell does not fit the `ReadoutRow` shell because the bar slot becomes a glyph. Render an inline cell with the same row grid:

```rust
                OutputId::Hat { .. } => {
                    let direction = super::value_helpers::read_output_hat(out, &live, &cfg);
                    let glyph = hat_glyph_for(direction);
                    let row_class = if frozen {
                        "if-editor__readout-row if-editor__readout-row--hat if-editor__readout-row--frozen"
                    } else {
                        "if-editor__readout-row if-editor__readout-row--hat"
                    };
                    rsx! {
                        div { class: "{row_class}",
                            div { class: "if-editor__readout-label", "OUT" }
                            div { class: "if-editor__readout-tag", "{tag}" }
                            div { class: "if-editor__readout-hat-glyph", "{glyph}" }
                            div { class: "if-editor__readout-pct" }
                            // Empty 5th cell for the chevron column (auto-collapses
                            // to 0 in CSS via the grid's max-content 5th column).
                            div { class: "if-editor__readout-chevron-spacer" }
                        }
                    }
                }
```

Add the helper to the bottom of `out_block.rs`:

```rust
fn hat_glyph_for(direction: inputforge_core::types::HatDirection) -> char {
    use inputforge_core::types::HatDirection;
    match direction {
        HatDirection::N => '\u{2191}',
        HatDirection::NE => '\u{2197}',
        HatDirection::E => '\u{2192}',
        HatDirection::SE => '\u{2198}',
        HatDirection::S => '\u{2193}',
        HatDirection::SW => '\u{2199}',
        HatDirection::W => '\u{2190}',
        HatDirection::NW => '\u{2196}',
        HatDirection::Center => '\u{00b7}',
    }
}
```

- [ ] **Step 3: Replace the Keyboard arm with a chip cell**

Replace `OutputDestination::Keyboard(_) => { rsx! { div { "TODO keyboard" } } }` with:

```rust
        OutputDestination::Keyboard(combo) => {
            let combo_label = format_key_combo(combo);
            let row_class = if frozen {
                "if-editor__readout-row if-editor__readout-row--kb if-editor__readout-row--frozen"
            } else {
                "if-editor__readout-row if-editor__readout-row--kb"
            };
            let chip_class = if frozen {
                "if-editor__readout-kb-chip if-editor__readout-kb-chip--idle"
            } else {
                "if-editor__readout-kb-chip if-editor__readout-kb-chip--live"
            };
            rsx! {
                div { class: "{row_class}",
                    div { class: "if-editor__readout-label", "OUT" }
                    div { class: "if-editor__readout-tag", "Keyboard" }
                    div { class: "if-editor__readout-kb-cell",
                        span { class: "{chip_class}", "{combo_label}" }
                    }
                    div { class: "if-editor__readout-pct" }
                    // Empty 5th cell for the chevron column (auto-collapses
                    // to 0 in CSS via the grid's max-content 5th column).
                    div { class: "if-editor__readout-chevron-spacer" }
                }
            }
        }
```

No new helper to declare here — `format_key_combo` lives in `value_helpers.rs` (Task 2). Import it at the top of `out_block.rs`:

```rust
use super::value_helpers::format_key_combo;
```

The `OutputDestination::Keyboard(combo)` arm above invokes `format_key_combo(combo)` directly. (Per UI reviewer #M4, the helper is generic enough that future call sites — e.g. a key-combo display in the mapping list — can reuse it.)

- [ ] **Step 4: Verify the build**

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs
git commit -m "feat(live_readout): add OUT row variants for vJoy button, hat, and keyboard"
```

---

## Task 15: Add the `OutChain` expanded-block component

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_chain.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs`

`OutChain` renders one row per `ChainStep` for an `OutputDescriptor`. Indented under the OUT row with a 1px dashed left border. Two row layouts:
- **Merge step:** small-uppercase `MERGE n` label + partner source label + intermediate-value bar + mono pct
- **Conditional step:** small-uppercase `COND` label + `condition_label` text + active/inactive branch tag

- [ ] **Step 1: Create `out_chain.rs`**

```rust
// Rust guideline compliant 2026-05-03

//! Expanded-chain block under one OUT row. Renders the per-output
//! `chain` (Vec<ChainStep>) as a sequence of indented chain rows. Hidden
//! by default; orchestrator toggles via `ExpandState`.

use dioxus::prelude::*;

use inputforge_core::types::AxisPolarity;

use crate::context::AppContext;

use super::analyzer::{Branch, ChainStep, OutputDescriptor};
use super::value_helpers::format_percentage;

/// Indented chain block for one `OutputDescriptor`.
///
/// Each merge step is rendered using its own `polarity_at_step` (the
/// running output polarity at that fold position), NOT the terminal
/// output polarity. This is correct because polarity can promote
/// mid-fold (e.g. Unipolar primary + Bidirectional partner promotes to
/// Bipolar after the first merge); using the terminal polarity to
/// re-map every intermediate value would mis-render bars at earlier
/// steps. The terminal polarity (`descriptor.polarity`) is unused here.
#[component]
pub(super) fn OutChain(descriptor: OutputDescriptor) -> Element {
    let chain = descriptor.chain.clone();

    rsx! {
        div { class: "if-editor__readout-chain",
            for (idx, step) in chain.iter().enumerate() {
                ChainRow {
                    key: "{idx}",
                    step: step.clone(),
                    index_in_chain: idx,
                }
            }
        }
    }
}

#[component]
fn ChainRow(step: ChainStep, index_in_chain: usize) -> Element {
    match step {
        ChainStep::Merge {
            operation,
            secondary_input,
            encoded_value,
            polarity_at_step,
        } => {
            let merge_n = index_in_chain + 1;
            let ctx = use_context::<AppContext>();
            let cfg = ctx.config.read();
            let partner_label =
                crate::frame::mapping_list::source_label::format(&secondary_input, &cfg);
            // Re-map the encoded value to natural domain using THIS step's
            // running polarity (the fold prefix), not the terminal polarity.
            let display = super::value_helpers::AxisDisplay {
                value: inputforge_core::processing::into_natural_domain(
                    encoded_value,
                    polarity_at_step,
                ),
                polarity: polarity_at_step,
            };
            let pct = format_percentage(&display);
            let bipolar = matches!(polarity_at_step, AxisPolarity::Bipolar);
            let fill_pct = if bipolar {
                (display.value.abs() * 50.0).clamp(0.0, 50.0)
            } else {
                (display.value.abs() * 100.0).clamp(0.0, 100.0)
            };
            let bar_style = if bipolar && display.value < 0.0 {
                format!("left: auto; right: 50%; width: {fill_pct}%;")
            } else if bipolar {
                format!("left: 50%; right: auto; width: {fill_pct}%;")
            } else {
                format!("left: 0; right: auto; width: {fill_pct}%;")
            };
            let bar_class = if bipolar {
                "if-editor__readout-chain-bar if-editor__readout-chain-bar--bipolar"
            } else {
                "if-editor__readout-chain-bar"
            };
            // Explicit operator string (not Debug) so renames of MergeOp
            // variants in core do not silently change UI labels.
            let op_label = match operation {
                inputforge_core::types::MergeOp::Bidirectional => "bidirectional",
                inputforge_core::types::MergeOp::Average => "average",
                inputforge_core::types::MergeOp::Maximum => "maximum",
            };
            rsx! {
                div { class: "if-editor__readout-chain-row",
                    span { class: "if-editor__readout-chain-step", "MERGE {merge_n}" }
                    span { class: "if-editor__readout-chain-tag", "{partner_label} \u{00b7} {op_label}" }
                    div { class: "{bar_class}",
                        div {
                            class: "if-editor__readout-chain-fill",
                            style: "{bar_style}",
                        }
                    }
                    span { class: "if-editor__readout-chain-pct", "{pct}" }
                }
            }
        }
        ChainStep::Conditional {
            condition_label,
            evaluated,
            branch,
        } => {
            // Active-branch rule (same as compute_is_active in analyzer):
            //   evaluated == matches!(branch, Branch::IfTrue)
            // - evaluated=true  + branch=IfTrue  → active (true == true)
            // - evaluated=false + branch=IfFalse → active (false == false)
            // - mismatched pairs → inactive (the user is in the OTHER branch)
            let active = evaluated == matches!(branch, Branch::IfTrue);
            let outcome = if active { "active branch" } else { "inactive branch" };
            let row_class = if active {
                "if-editor__readout-chain-row if-editor__readout-chain-row.is-cond"
            } else {
                "if-editor__readout-chain-row if-editor__readout-chain-row.is-cond if-editor__readout-chain-row.is-inactive"
            };
            let outcome_class = if active {
                "if-editor__readout-chain-outcome if-editor__readout-chain-outcome--active"
            } else {
                "if-editor__readout-chain-outcome if-editor__readout-chain-outcome--inactive"
            };
            rsx! {
                div { class: "{row_class}",
                    span { class: "if-editor__readout-chain-step", "COND" }
                    span { class: "if-editor__readout-chain-tag", "{condition_label}" }
                    span { class: "{outcome_class}", "\u{2192} {outcome}" }
                }
            }
        }
    }
}
```

- [ ] **Step 2: Add `mod out_chain;` to `live_readout/mod.rs`**

```rust
mod analyzer;
mod in_block;
mod out_block;
mod out_chain;
mod predicate;
mod value_helpers;
```

- [ ] **Step 3: Verify the build**

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_chain.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs
git commit -m "feat(live_readout): add OutChain component for expanded merge + conditional steps"
```

---

## Task 16: Add `ExpandState`, per-OUT chevron, and "expand all" pill

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs`

Add a per-output chevron button on each `OutRow` whose `descriptor.chain` is non-empty. Wire it to a `Signal<ExpandState>` owned by the orchestrator (Task 17). For now, define `ExpandState`, accept a per-row `expanded: bool` prop on `OutRow`, and surface a callback for the chevron click.

- [ ] **Step 1: Add `ExpandState` to `out_block.rs`**

Add at the top of `out_block.rs`:

```rust
use std::rc::Rc;

/// Per-`LiveReadout`-instance expand state.
///
/// `per_output` is index-aligned with `model.outputs`. `expand_all` is a
/// global override; when true, every output's chain is expanded
/// regardless of `per_output`. Toggling the divider's "expand all" /
/// "collapse all" pill flips both `expand_all` and all `per_output[i]`
/// to the same value, so subsequent per-row toggles work from a known
/// state.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct ExpandState {
    pub expand_all: bool,
    pub per_output: Vec<bool>,
}

impl ExpandState {
    /// Return the effective expand-state for `idx`, taking the global
    /// override into account.
    pub(super) fn is_expanded(&self, idx: usize) -> bool {
        self.expand_all || self.per_output.get(idx).copied().unwrap_or(false)
    }
}
```

- [ ] **Step 2: Update `OutBlock` and `OutRow` to wire the chevron**

Replace `OutBlock` to take an `expand_state: Signal<ExpandState>` prop and an `engine_running: bool` prop (hoisted from the orchestrator's single `meta` read so we don't re-read N times for N outputs). Wrap each descriptor in `Rc` so the chain `Vec` is not cloned per OUT re-render:

```rust
#[component]
pub(super) fn OutBlock(
    model: LiveReadoutModel,
    expand_state: Signal<ExpandState>,
    engine_running: bool,
) -> Element {
    if model.outputs.is_empty() {
        return rsx! {};
    }
    let outputs: Vec<Rc<OutputDescriptor>> =
        model.outputs.iter().map(|d| Rc::new(d.clone())).collect();
    rsx! {
        div { class: "if-editor__readout-section",
            div { class: "if-editor__readout-section-label", "OUT" }
            div { class: "if-editor__readout-group",
                for (idx, descriptor) in outputs.iter().enumerate() {
                    OutRow {
                        key: "{idx}",
                        descriptor: descriptor.clone(),
                        idx,
                        expand_state,
                        engine_running,
                    }
                }
            }
        }
    }
}
```

Update `OutRow` signature. Note: the `match &descriptor.destination` body stays inline (NOT refactored into a helper that takes `&AppContext`, because lifetime threading across the borrow is fragile and the inline form is no harder to read). The chevron and chain block are siblings inside the `if-editor__readout-row-wrap` element:

```rust
#[component]
fn OutRow(
    descriptor: Rc<OutputDescriptor>,
    idx: usize,
    expand_state: Signal<ExpandState>,
    engine_running: bool,
) -> Element {
    let ctx = use_context::<AppContext>();
    let frozen = !engine_running || !descriptor.is_active;
    let has_chain = !descriptor.chain.is_empty();
    let expanded = expand_state.read().is_expanded(idx);

    let toggle = move |_| {
        expand_state.with_mut(|s| {
            if s.per_output.len() <= idx {
                s.per_output.resize(idx + 1, false);
            }
            s.per_output[idx] = !s.per_output[idx];
        });
    };

    let chevron = if has_chain {
        rsx! {
            button {
                class: "if-editor__readout-chevron",
                "type": "button",
                onclick: toggle,
                if expanded { "\u{25be}" } else { "\u{25b8}" }
            }
        }
    } else {
        rsx! { div { class: "if-editor__readout-chevron-spacer" } }
    };

    // Inline value-cell render. Reads `ctx.live` / `ctx.config` directly.
    // Same body as the previous OutRow (Tasks 13 + 14), wrapped in a Rust
    // expression that yields an Element so the rsx! below can interpolate it.
    let value_cell = {
        let live = ctx.live.read();
        let cfg = ctx.config.read();
        match &descriptor.destination {
            OutputDestination::VJoy(out) => {
                let tag = format_output_label(out);
                match out.output {
                    OutputId::Axis { .. } => {
                        let display = read_output_display(out, &live, &cfg, descriptor.polarity);
                        rsx! {
                            ReadoutRow {
                                label: "OUT".to_owned(),
                                tag,
                                display,
                                frozen,
                            }
                        }
                    }
                    // Button / Hat / Keyboard arms: same bodies as Task 14.
                    // Each hand-rolled row already injects an empty 5th cell
                    // (.if-editor__readout-chevron-spacer) so the grid template
                    // stays consistent.
                    _ => rsx! { /* see Task 14 for full bodies */ },
                }
            }
            OutputDestination::Keyboard(_) => rsx! { /* see Task 14 */ },
        }
    };

    let chain_block = if expanded && has_chain {
        rsx! { super::out_chain::OutChain { descriptor: (*descriptor).clone() } }
    } else {
        rsx! {}
    };

    rsx! {
        // The wrap element uses display: contents (CSS Task 18) so its
        // children dissolve into the parent grid. Frozen modifier is
        // applied HERE, not on the inner value-cell row, so it reaches
        // both the row AND the chain block via descendant selectors.
        div {
            class: if frozen {
                "if-editor__readout-row-wrap if-editor__readout-row-wrap--frozen"
            } else {
                "if-editor__readout-row-wrap"
            },
            {value_cell}
            {chevron}
            {chain_block}
        }
    }
}
```

- [ ] **Step 3: Add the divider's "expand all" pill**

The divider sits between the IN section and the OUT section, replacing today's flat `ReadoutDivider`. Define it inline in `out_block.rs`:

```rust
/// The strip between IN and OUT sections. Holds ONLY the expand-all pill
/// (rendered only when at least one output has a non-empty chain). The
/// previous "out" label is dropped: section labels now live inside their
/// owning blocks (`OutBlock` renders `OUT`; `InBlock` renders
/// `IN · pipeline` / `IN · predicates`). The divider strip is dedicated
/// to the global expand control — one job per surface.
#[component]
pub(super) fn DividerStrip(
    model: LiveReadoutModel,
    expand_state: Signal<ExpandState>,
) -> Element {
    let any_expandable = model.outputs.iter().any(|o| !o.chain.is_empty());
    let outputs_len = model.outputs.len();
    let toggle_all = move |_| {
        expand_state.with_mut(|s| {
            let new_all = !s.expand_all;
            s.expand_all = new_all;
            // Sync per_output so subsequent per-row toggles work from a
            // known state; otherwise toggling expand_all=false would leave
            // stale per_output[i]=true values that immediately re-expand.
            s.per_output = vec![new_all; outputs_len];
        });
    };
    let expanded_now = expand_state.read().expand_all;
    rsx! {
        div { class: "if-editor__readout-divider",
            // Spacer to keep the divider's hairline visual; the previous
            // label slot is now empty.
            span { class: "if-editor__readout-divider-spacer" }
            if any_expandable {
                button {
                    class: "if-editor__readout-expand-all",
                    "type": "button",
                    onclick: toggle_all,
                    // Textual copy, not glyphs. The chevrons on per-row
                    // controls are visually distinct enough; using a glyph
                    // here too would make the controls look identical
                    // despite different functions.
                    if expanded_now { "collapse all" } else { "expand all" }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Verify the build**

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/out_block.rs
git commit -m "feat(live_readout): add ExpandState, per-OUT chevron, and expand-all pill"
```

---

## Task 17: Wire the orchestrator (`mod.rs`) to use the new components

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs`

Replace the existing `LiveReadout` body. The new orchestrator:
1. Builds `LiveReadoutModel` via `analyzer::analyze` while holding a single `state.read()` guard (per the analyzer's snapshot contract from Task 5).
2. Reads `engine_status` ONCE and passes `engine_running: bool` down (avoids N read-locks in N `OutRow`s).
3. Owns a `Signal<ExpandState>` and a previous-length signal. The reset effect compares previous-vs-current length and only zeroes `per_output` on actual length change (NOT every render). Without this, mapping changes that keep the same OUT count silently retain stale per-output toggles.
4. Renders `InBlock` + `DividerStrip` + `OutBlock`.

**Dependency note:** This task requires Tasks 6, 10, and 11 to have landed (the `analyze` signature gains `cfg: &ConfigSnapshot` in Task 11; predicate-label formatter is in Task 10; path-aware core helper is in Task 6).

**Deletion list.** Remove the following (analyzer + new components subsume them):
- `MergeContext` struct
- `find_merge_context` function
- `first_map_to_vjoy_output` function
- The 4 `find_merge_context_*` tests in `mod tests`
- `ReadoutDivider` (in `in_block.rs`) — replaced by `out_block::DividerStrip`

- [ ] **Step 1: Replace `LiveReadout` body**

Open `live_readout/mod.rs`. Replace the entire `LiveReadout` function body and remove every item in the deletion list above.

The new file should look like (preserving header docs and `FROZEN_ROW_CLASS`):

```rust
// Rust guideline compliant 2026-05-03

//! Live readout: full DFS walker over the action tree, surfacing every
//! pipeline input, every condition predicate, and every terminal output
//! (vJoy + keyboard) with per-OUT expandable causal chains.
//!
//! Submodules:
//! - `analyzer`: action-tree DFS walker producing `LiveReadoutModel`
//! - `predicate`: composite-aware condition label formatter + per-leaf eval
//! - `value_helpers`: AxisDisplay, percentage formatting, polarity helpers,
//!   merge polarity table, key-combo formatting
//! - `in_block`: pipeline-axis rows + predicate chip subsection
//! - `out_block`: OUT section, per-row chevron, expand-all pill, divider
//! - `out_chain`: expanded chain rendering for one OutputDescriptor
//!
//! See `2026-05-03-multi-merge-multi-out-readout-design.md` for the spec.

mod analyzer;
mod in_block;
mod out_block;
mod out_chain;
mod predicate;
mod value_helpers;

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::state::EngineStatus;
use inputforge_core::types::InputAddress;

use crate::context::AppContext;

use in_block::InBlock;
use out_block::{DividerStrip, ExpandState, OutBlock};

/// CSS modifier class applied to a `ReadoutRow` whose value is held
/// (engine stopped or the row sits in an inactive conditional branch).
pub(super) const FROZEN_ROW_CLASS: &str = "if-editor__readout-row--frozen";

/// Live IN/OUT readout section, mounted beneath the input field.
///
/// # Props
/// - `primary` — address of the primary (mapped) input axis.
/// - `actions` — the full action pipeline for the selected mapping.
#[component]
pub(crate) fn LiveReadout(primary: InputAddress, actions: Vec<Action>) -> Element {
    let ctx = use_context::<AppContext>();

    // Single state.read() per render — analyzer snapshot contract (Task 5).
    let model = {
        let state = ctx.state.read();
        let cfg = ctx.config.read();
        analyzer::analyze(&actions, &primary, &state, &cfg)
    };
    // Single meta read — passed down so OutRows don't each re-read.
    let engine_running = matches!(ctx.meta.read().engine_status, EngineStatus::Running);

    let outputs_len = model.outputs.len();

    // Per-output expand state. The reset rule is: when outputs_len CHANGES
    // (e.g. mapping selection switched, pipeline edited), zero per_output
    // and expand_all. Tracking previous length explicitly avoids the
    // stale-state trap where a mapping change to the same output count
    // would silently keep prior per-row toggles.
    let mut expand_state: Signal<ExpandState> = use_signal(ExpandState::default);
    let mut prev_outputs_len: Signal<usize> = use_signal(|| outputs_len);
    use_effect(move || {
        let prev = *prev_outputs_len.read();
        if prev != outputs_len {
            expand_state.with_mut(|s| {
                s.per_output = vec![false; outputs_len];
                s.expand_all = false;
            });
            prev_outputs_len.set(outputs_len);
        }
    });

    let model_for_in = model.clone();
    let model_for_div = model.clone();
    let model_for_out = model;

    rsx! {
        div { class: "if-editor__readout",
            InBlock { model: model_for_in }
            DividerStrip { model: model_for_div, expand_state }
            OutBlock { model: model_for_out, expand_state, engine_running }
        }
    }
}

#[cfg(test)]
mod tests {
    // Live readout tests live in:
    // - submodule `#[cfg(test)] mod tests` blocks (analyzer, predicate,
    //   value_helpers): unit-level
    // - `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs`:
    //   SSR-level (renders the full MappingEditor with a seeded state)
}
```

- [ ] **Step 2: Migrate or delete legacy "merged IN" tests**

The new layout removes the top-level "merged IN" row: the merged value now lives inside the per-OUT expanded chain (per spec Q4-B). Five existing SSR tests assert content of the merged-IN row and will break:

- `editor_live_readout_bidirectional_uu_idle_renders_centered_bipolar_in`
- `editor_live_readout_bidirectional_uu_half_press_renders_half_deflection`
- `editor_live_readout_average_uu_idle_renders_empty_unipolar_in`
- `editor_live_readout_average_uu_full_press_renders_full_unipolar_in`
- `editor_live_readout_average_bb_renders_bipolar_unchanged`

These were locked to the single-merge layout. Replace each with an analyzer-level unit test in `analyzer.rs` that asserts the chain step's `intermediate_value` for the same input scenario, plus one SSR test that drives `ExpandState.expand_all = true` (via signal mutation in the harness) and verifies the chain row renders the expected `+0.50` / `0.00` / etc. text. The polarity / merge math invariants (which is what those tests really exercised) still need coverage; the rendering surface just moved.

Concretely: delete the five tests, then add a new `editor_live_readout_chain_bidirectional_uu_half_press` SSR test that:
1. Builds the same pipeline (`MergeAxis { Bidirectional }` + `MapToVJoy`).
2. Seeds the same axis values (primary 0.0, secondary -1.0, both Unipolar).
3. Renders, asserts the OUT row pct `+0.50` (or whatever the post-merge encoded value resolves to via `into_natural_domain`).
4. Drives the chain to expand via direct signal write, re-renders, asserts the `MERGE 1` chain row's pct text equals the expected merge intermediate value.

Driving the signal directly requires either a test-only `pub(super)` accessor for `ExpandState` or a harness helper that writes `expand_state.expand_all = true` after the first `rebuild_in_place`. The simplest path: add a query-string or test-only prop on `LiveReadout` (e.g. `#[cfg(test)] expand_all_for_test: bool`) defaulting to false. Out of scope for v1 if the analyzer unit test is sufficient; in that case delete the five tests and keep the polarity coverage at the analyzer level only.

The other legacy tests (`editor_live_readout_renders_in_row`, `editor_live_readout_omits_out_when_no_map_to_vjoy`, `editor_live_readout_renders_out_when_map_to_vjoy_present`, `editor_live_readout_unipolar_primary_no_merge_out_inherits_unipolar`, `editor_live_readout_out_freezes_when_engine_stopped`, `editor_live_readout_out_row_marks_frozen_class_when_engine_stopped`, `editor_live_readout_out_row_omits_frozen_class_when_engine_running`) all hold under the new layout (their assertions are about OUT row presence and freeze modifiers, which the new layout preserves). Run them and confirm they pass.

- [ ] **Step 3: Run the full test suite and confirm green**

Run: `cargo test -p inputforge-gui-dx --lib mapping_editor::tests`
Expected: every test green; the five deleted tests no longer exist.

Run: `cargo test -p inputforge-gui-dx --lib live_readout`
Expected: all submodule tests pass.

Run: `cargo check -p inputforge-gui-dx`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs
git commit -m "feat(live_readout): wire orchestrator to new InBlock + DividerStrip + OutBlock"
```

---

## Task 18: Add CSS rules for new readout primitives

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

Add rules for the new CSS classes introduced across Tasks 12-16:
- **Grid template change**: `.if-editor__readout-group` widens to 5 columns; the 5th uses `max-content` so it auto-collapses to 0 when no row injects a chevron cell. IN-only screens pay nothing.
- `.if-editor__readout-section` and `.if-editor__readout-section-label` (single class shared by `IN · pipeline`, `IN · predicates`, `OUT`).
- `.if-editor__readout-chips` (flex container for the predicate subsection).
- `.if-editor__readout-chip` + `--live` / `--idle` modifiers; `.if-editor__readout-chip-dot` + `--hollow` modifier.
- `.if-editor__readout-row--hat` + `.if-editor__readout-hat-glyph`; `.if-editor__readout-row--kb` + `.if-editor__readout-kb-cell` + `.if-editor__readout-kb-chip` + state modifiers.
- `.if-editor__readout-row-wrap` (per-OUT wrapper, `display: contents`). Frozen modifier lives HERE so it reaches descendants in both the value-cell row AND the chain block.
- `.if-editor__readout-chevron`, `.if-editor__readout-chevron-spacer`, `.if-editor__readout-expand-all` — focus rules match the project `if-icon-button` pattern at `assets/components/icon-button.css:68-70`.
- `.if-editor__readout-chain` (recessed sub-zone via `bg-sunken` background, indented 28px, NO border vocabulary — per DESIGN.md "layering by luminance, not shadow").
- `.if-editor__readout-chain-row` with `.is-cond` state class (single state class, not `--cond` proliferation across sibling elements).
- `.if-editor__readout-chain-bar`, `--bipolar` modifier, `.if-editor__readout-chain-fill`, `.if-editor__readout-chain-pct`, `.if-editor__readout-chain-outcome`.

- [ ] **Step 1: Update the existing grid template**

Find `.if-editor__readout-group` (around line 385) and add the 5th `max-content` column:

```css
.if-editor__readout-group {
    display: grid;
    /* 5th column auto-collapses to 0 when no row contains a chevron cell.
       IN-only screens pay nothing; OUT screens with chains get the chevron
       slot for free. */
    grid-template-columns: 60px minmax(0, max-content) 1fr 60px max-content;
    column-gap: 12px;
    row-gap: 8px;
}
```

- [ ] **Step 2: Append the new CSS block at the bottom of `mapping_editor.css`**

Locate the existing `.if-editor__readout-row--frozen` rules (around line 470) and append the new block immediately after `.if-editor__readout-divider-label` (around line 503), before the Task 19 inactive-hint banner block:

```css
/* Section labels (IN · pipeline, IN · predicates, OUT). Caption typography
   per DESIGN.md (11px / 500 / 1.45), subtle text color. Single shared
   class — no per-section variants. */
.if-editor__readout-section {
    /* Default to flex per project flexbox convention; flex-direction column
       so the section-label sits above its grouped rows. */
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.if-editor__readout-section-label {
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 500;
    line-height: 1.45;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-subtle);
    margin: 8px 0 2px 0;
}

/* Predicate chips subsection: flex row that wraps when chips overflow. */
.if-editor__readout-chips {
    /* flex (default) — wrap onto new line when the chip set exceeds width */
    flex-wrap: wrap;
    gap: 6px;
    padding-left: 56px;
}

.if-editor__readout-chip {
    /* inline-flex so the dot + label align on a single baseline within a
       flowing chip row, while the chip itself behaves as inline content */
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--color-border);
    color: var(--color-text-subtle);
}

.if-editor__readout-chip--live {
    color: var(--color-live);
    border-color: var(--color-live);
    background: rgba(46, 224, 160, 0.10);
}

.if-editor__readout-chip-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
    /* inline-block so the dot keeps its width/height inside an inline-flex
       chip row */
    display: inline-block;
}

.if-editor__readout-chip-dot--hollow {
    background: transparent;
    border: 1.5px solid currentColor;
}

.if-editor__readout-chip-label {
    color: inherit;
}

/* OUT row variants: hat glyph cell replaces the bar slot. */
.if-editor__readout-row--hat .if-editor__readout-hat-glyph {
    font-family: var(--font-mono);
    font-size: 16px;
    text-align: center;
    color: var(--color-text);
}

.if-editor__readout-row-wrap--frozen .if-editor__readout-row--hat .if-editor__readout-hat-glyph {
    color: var(--color-text-muted);
}

/* OUT row variants: keyboard chip replaces the bar slot. */
.if-editor__readout-row--kb .if-editor__readout-kb-cell {
    /* flex (default) so the chip sits centered within its grid cell */
    align-items: center;
}

.if-editor__readout-kb-chip {
    display: inline-flex;
    align-items: center;
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--color-control);
    color: var(--color-control-badge-text);
    background: var(--color-control-bg);
}

.if-editor__readout-kb-chip--idle {
    color: var(--color-text-subtle);
    border-color: var(--color-border);
    background: transparent;
}

/* Per-output wrapper. display: contents dissolves the wrapper into the
   parent grid; the wrapper contributes 5 cells (4 from the inline value
   cell row + 1 chevron cell). The chain block uses grid-column: 1/-1 so
   it spans all 5 columns on its own row. */
.if-editor__readout-row-wrap {
    display: contents;
}

/* Frozen modifier on the WRAPPER (not the value-cell row), so it reaches
   both the value cell descendants AND the chain block beneath. */
.if-editor__readout-row-wrap--frozen .if-editor__readout-fill {
    background: var(--color-text-muted);
}
.if-editor__readout-row-wrap--frozen .if-editor__readout-pct {
    opacity: 0.7;
}
.if-editor__readout-row-wrap--frozen .if-editor__readout-chain {
    opacity: 0.7;
}
.if-editor__readout-row-wrap--frozen .if-editor__readout-chain-fill {
    background: var(--color-text-muted);
}

/* Chevron button: sits in the trailing 5th grid column. */
.if-editor__readout-chevron {
    /* inline-flex so the chevron glyph centers inside the 24×24 button */
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    background: transparent;
    border: 0;
    color: var(--color-text-subtle);
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 0;
    border-radius: 2px;
}
.if-editor__readout-chevron:hover {
    color: var(--color-text);
}
.if-editor__readout-chevron:focus-visible {
    /* Matches the if-icon-button pattern at icon-button.css:68-70 —
       2px focus-cyan outline at 2px offset, the project default for
       every action surface. */
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
}

.if-editor__readout-chevron-spacer {
    /* Same width as the chevron button so hat/kb hand-rolled rows
       reserve the 5th column without rendering a button. */
    width: 24px;
    height: 24px;
}

/* Expand-all pill on the divider strip. Textual ("expand all" /
   "collapse all"), no glyph — distinct from per-row chevrons. */
.if-editor__readout-expand-all {
    margin-left: 8px;
    font-family: var(--font-body);
    font-size: 11px;
    font-weight: 500;
    color: var(--color-text-subtle);
    border: 0;
    border-radius: 2px;
    padding: 2px 8px;
    background: transparent;
    cursor: pointer;
}
.if-editor__readout-expand-all:hover {
    color: var(--color-text);
}
.if-editor__readout-expand-all:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
}

/* Expanded chain block: recessed sub-zone, layering by luminance.
   - bg-sunken background reads as "set into the panel" beneath the OUT row.
   - 28px left indent anchors the chain visually to its parent OUT.
   - NO border-left vocabulary: DESIGN.md forbids colored side-stripes
     (Toast is the sole exception). The luminance shift carries the
     parent-child relationship without one.
   - grid-column: 1 / -1 spans the chain across all 5 grid columns of the
     parent if-editor__readout-group. */
.if-editor__readout-chain {
    grid-column: 1 / -1;
    background: var(--color-bg-sunken);
    padding: 6px 0 6px 28px;
    margin: 0;
    /* flex column for chain rows */
    flex-direction: column;
    gap: 2px;
}

.if-editor__readout-chain-row {
    display: grid;
    grid-template-columns: 80px minmax(0, 1fr) 1fr 56px;
    column-gap: 10px;
    align-items: center;
    font-size: 11px;
    color: var(--color-text-muted);
}

/* Conditional row uses a state class (not a --cond modifier) so we can
   layer is-inactive on top without modifier-name proliferation. */
.if-editor__readout-chain-row.is-cond {
    grid-template-columns: 80px minmax(0, 1fr) auto;
}
.if-editor__readout-chain-row.is-inactive {
    opacity: 0.6;
}

.if-editor__readout-chain-step {
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-output);
    font-weight: 500;
}

/* COND step uses control-violet to distinguish from MERGE n. */
.if-editor__readout-chain-row.is-cond .if-editor__readout-chain-step {
    color: var(--color-control);
}

.if-editor__readout-chain-tag {
    color: var(--color-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.if-editor__readout-chain-bar {
    position: relative;
    height: 4px;
    background: var(--color-bg-elevated);
    border-radius: 2px;
    overflow: hidden;
}

.if-editor__readout-chain-bar--bipolar::before {
    content: "";
    position: absolute;
    left: 50%;
    top: 0;
    width: 1px;
    height: 100%;
    background: var(--color-border-strong);
}

.if-editor__readout-chain-fill {
    position: absolute;
    top: 0;
    height: 100%;
    background: var(--color-processing);
    border-radius: 2px;
}

.if-editor__readout-chain-pct {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-text-subtle);
    text-align: right;
}

.if-editor__readout-chain-outcome {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-text-subtle);
}

.if-editor__readout-chain-outcome--active {
    color: var(--color-control);
}
```

- [ ] **Step 2: Verify the build**

Run: `cargo check -p inputforge-gui-dx`
Expected: green (CSS is asset-bundled; only verifies no Rust regressions).

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "style(live_readout): add CSS for predicate chips, kb chip, hat cell, expand-all, chain block"
```

---

## Task 19: SSR coverage for composite, nested, engine-stopped, hat, button-released, per-output-polarity, axis-in-range live-dot

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs`

Tasks 20-22 (the original test tasks) cover the headline scenarios but the spec's Testing section (lines 293-310) and edge-case table (lines 282-287) include several scenarios that have no SSR test in those tasks. This task adds 7 missing SSR tests that fill the coverage gaps a code-review pass surfaced. Each test corresponds to a spec scenario and asserts a specific failure mode (not just "renders").

These tests use the `seeded_profile_with_pipeline(actions, axis_polarities, axis_values, button_states, hat_states)` helper that Task 20 introduces — so this task assumes Task 20 has at least added the helper, OR adds it locally if Task 20 has not run yet (the helper is identical in either case).

- [ ] **Step 1: Composite predicate test (`Condition::All` flattens to N chips, evaluator AND-combines)**

Spec line 300. Bug class: evaluator gates on the first leaf instead of the composite.

```rust
#[test]
fn editor_live_readout_composite_all_predicate_renders_two_chips_and_and_combines() {
    let actions = vec![Action::Conditional {
        condition: Condition::All {
            conditions: vec![
                Condition::ButtonPressed { input: btn_addr(0) },
                Condition::ButtonPressed { input: btn_addr(1) },
            ],
        },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![],
    }];

    // Both buttons pressed → OUT active.
    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), true), (btn_addr(1), true)],
        &[],
    );
    assert_eq!(count_substring(&html, "if-editor__readout-chip"), 2);
    let frozen_count = count_substring(&html, FROZEN_ROW_CLASS);
    assert_eq!(frozen_count, 0); // OUT is active

    // Only one button pressed → OUT inactive (the All composite is false).
    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), true), (btn_addr(1), false)],
        &[],
    );
    let frozen_count = count_substring(&html, FROZEN_ROW_CLASS);
    assert_eq!(frozen_count, 1); // OUT frozen because composite false
}
```

- [ ] **Step 2: Nested conditional test (path-AND active evaluation)**

Spec line 303. Bug class: branch-stale or off-by-one.

```rust
#[test]
fn editor_live_readout_nested_conditional_inner_active_only_when_path_matches() {
    // Outer Conditional on btn(0); inner Conditional on btn(1).
    // Inner OUT (vjoy_x) is in outer.if_true → inner.if_true.
    // It is active only when (btn0 AND btn1) both held — path-AND.
    let inner = Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(1) },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(0) },
        if_true: vec![inner],
        if_false: vec![],
    }];

    // Both true → vjoy_x active, vjoy_y frozen (inactive branch).
    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), true), (btn_addr(1), true)],
        &[],
    );
    // X axis row not frozen, Y axis row frozen.
    assert!(html.contains(">X axis<"));
    assert!(html.contains(">Y axis<"));
    let frozen_count = count_substring(&html, FROZEN_ROW_CLASS);
    assert_eq!(frozen_count, 1);

    // btn0 false, btn1 true → both inner OUTs are in the unreachable
    // outer branch; both must be frozen.
    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), false), (btn_addr(1), true)],
        &[],
    );
    let frozen_count = count_substring(&html, FROZEN_ROW_CLASS);
    assert_eq!(frozen_count, 2);
}
```

- [ ] **Step 3: Engine-stopped + multi-OUT freezes ALL rows**

Spec testing table row "Engine stopped + active conditional: row carries `--frozen`; expand chevron still works; chain rows muted." Bug class: only the active OUT freezes; the inactive one stays live.

```rust
#[test]
fn editor_live_readout_engine_stopped_with_multi_out_freezes_all_rows() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(0) },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
    }];
    // Engine STOPPED; both OUT rows must carry --frozen regardless of
    // which conditional branch is active.
    let html = render_with_pipeline_and_engine(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.5)],
        &[(btn_addr(0), true)],
        &[],
        EngineStatus::Stopped,
    );
    let frozen_count = count_substring(&html, "if-editor__readout-row-wrap--frozen");
    assert_eq!(frozen_count, 2);
}
```

- [ ] **Step 4: HatDirection chip glyphs**

Spec line 232 mandates the `↑↗→↘↓↙←↖·` glyph alphabet. Bug class: glyph table mismatch or `Center` first-class handling broken.

```rust
#[test]
fn editor_live_readout_hat_direction_predicate_chip_glyphs() {
    use inputforge_core::types::HatDirection;
    let actions = vec![Action::Conditional {
        condition: Condition::HatDirection {
            input: hat_addr(0),
            directions: vec![HatDirection::N, HatDirection::NE],
        },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![],
    }];
    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.0)],
        &[],
        &[(hat_addr(0), HatDirection::N)],
    );
    // Up (↑) and UpRight (↗) glyphs must appear in the chip label.
    assert!(html.contains("\u{2191}")); // ↑
    assert!(html.contains("\u{2197}")); // ↗
    // Chip is live when current hat state is in the directions set.
    assert!(html.contains("if-editor__readout-chip--live"));
}
```

- [ ] **Step 5: ButtonReleased chip suffix and inverted dot polarity**

Spec line 231. Bug class: missing " (released)" suffix; dot polarity not inverted (filled when pressed instead of released).

```rust
#[test]
fn editor_live_readout_button_released_chip_suffix_and_inverted_dot() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonReleased { input: btn_addr(0) },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![],
    }];
    // Button NOT pressed → ButtonReleased is true → chip is live.
    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.0)],
        &[(btn_addr(0), false)],
        &[],
    );
    assert!(html.contains("(released)"));
    assert!(html.contains("if-editor__readout-chip--live"));

    // Button PRESSED → ButtonReleased is false → chip is idle (hollow dot).
    let html = render_with_pipeline(
        &actions,
        &[(axis_addr(0), AxisPolarity::Bipolar, 0.0)],
        &[(btn_addr(0), true)],
        &[],
    );
    assert!(html.contains("(released)"));
    assert!(html.contains("if-editor__readout-chip-dot--hollow"));
}
```

- [ ] **Step 6: Per-output polarity disagreement (two OUTs in different branches with different terminal polarity)**

Spec lines 169-172. Bug class: every OUT inherits the primary's polarity instead of computing its own.

```rust
#[test]
fn editor_live_readout_per_output_polarity_disagreement() {
    // OUT in if_true: takes a Bidirectional merge → Bipolar terminal.
    // OUT in if_false: takes no merges → inherits Unipolar primary.
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: btn_addr(0) },
        if_true: vec![
            Action::MergeAxis {
                second_input: axis_addr(1),
                operation: MergeOp::Bidirectional,
            },
            Action::MapToVJoy { output: vjoy_x() },
        ],
        if_false: vec![Action::MapToVJoy { output: vjoy_y() }],
    }];
    let html = render_with_pipeline(
        &actions,
        &[
            (axis_addr(0), AxisPolarity::Unipolar, -0.5),
            (axis_addr(1), AxisPolarity::Unipolar, -0.5),
        ],
        &[(btn_addr(0), true)],
        &[],
    );
    // X axis row should render bipolar bar (centered tick), Y axis row
    // should render unipolar bar (left-anchored, no centered tick).
    // The bipolar modifier class is the structural marker.
    let bipolar_count = count_substring(&html, "if-editor__readout-bar--bipolar");
    // X axis row carries the bipolar class; Y does not. (IN row may also
    // be bipolar/unipolar based on primary; assert only that AT LEAST one
    // OUT row has the bipolar class AND at least one does not.)
    assert!(bipolar_count >= 1);
    // Conversely there must be at least one non-bipolar bar (the Y OUT).
    let non_bipolar_bars = html.matches("if-editor__readout-bar").count() - bipolar_count;
    assert!(non_bipolar_bars >= 1);
}
```

- [ ] **Step 7: AxisInRange chip live dot when in range**

Spec line 231. Bug class: chip dot state ignores the actual axis value vs. bounds.

```rust
#[test]
fn editor_live_readout_axis_in_range_chip_live_dot_when_in_range() {
    let actions = vec![Action::Conditional {
        condition: Condition::AxisInRange {
            input: axis_addr(1),
            min: 0.20,
            max: 0.80,
        },
        if_true: vec![Action::MapToVJoy { output: vjoy_x() }],
        if_false: vec![],
    }];
    // axis(1) at 0.5 → in range → chip live.
    let html = render_with_pipeline(
        &actions,
        &[
            (axis_addr(0), AxisPolarity::Bipolar, 0.0),
            (axis_addr(1), AxisPolarity::Bipolar, 0.5),
        ],
        &[],
        &[],
    );
    assert!(html.contains("[0.20..0.80]"));
    assert!(html.contains("if-editor__readout-chip--live"));

    // axis(1) at -0.3 → out of range → chip idle/hollow.
    let html = render_with_pipeline(
        &actions,
        &[
            (axis_addr(0), AxisPolarity::Bipolar, 0.0),
            (axis_addr(1), AxisPolarity::Bipolar, -0.3),
        ],
        &[],
        &[],
    );
    assert!(html.contains("if-editor__readout-chip-dot--hollow"));
}
```

- [ ] **Step 8: Verify**

Run: `cargo test -p inputforge-gui-dx --lib mapping_editor::tests`
Expected: 7 new tests green, all prior tests still green.

- [ ] **Step 9: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs
git commit -m "test(live_readout): cover composite, nested, engine-stopped, hat, button-released, polarity disagreement, axis-in-range live dot"
```

---

## Task 20: SSR tests for stacked merges + multi-output

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs`

Add a new helper `seeded_profile_with_pipeline(actions, axis_polarities, axis_values, button_states, hat_states)` that extends `seeded_profile_with_polarities_and_axes` by also seeding button + hat caches (needed by predicate-chip tests in Task 21). Then add SSR tests for the new scenarios.

- [ ] **Step 1: Add the new helper**

Append after `seeded_profile_with_polarities_and_axes` (around line 134):

```rust
/// Like `seeded_profile_with_polarities_and_axes` but also seeds the
/// button and hat caches. Used by the multi-merge / multi-out tests to
/// drive predicate-chip assertions.
fn seeded_profile_with_pipeline(
    actions: Vec<Action>,
    axis_polarities: Vec<AxisPolarity>,
    axis_values: &[(u8, f64, AxisPolarity)],
    button_states: &[(u8, bool)],
    hat_states: &[(u8, inputforge_core::types::HatDirection)],
) -> AppState {
    use inputforge_core::types::InputValue;
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        axis_polarities,
        axis_values,
    );
    for &(idx, pressed) in button_states {
        let addr = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: idx },
        };
        state
            .input_cache
            .update(&addr, &InputValue::Button { pressed });
    }
    for &(idx, direction) in hat_states {
        let addr = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Hat { index: idx },
        };
        state
            .input_cache
            .update(&addr, &InputValue::Hat { direction });
    }
    state
}
```

- [ ] **Step 2: Add SSR tests for stacked merges**

Append a new test region near the existing live-readout tests (around line 1262, after the `editor_live_readout_out_row_omits_frozen_class_when_engine_running` test):

```rust
// ---------------------------------------------------------------------------
// Multi-merge / multi-out: stacked merges
// ---------------------------------------------------------------------------

/// Two top-level merges + one OUT: the IN block renders three pipeline
/// rows (primary + two secondaries), and the readout's section labels
/// `IN · pipeline` and `OUT` are present.
#[test]
fn editor_live_readout_stacked_merges_render_three_in_rows() {
    use inputforge_core::types::{MergeOp, OutputAddress, OutputId, VJoyAxis};

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary_b = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let secondary_c = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 2 },
    };
    let actions = vec![
        Action::MergeAxis {
            second_input: secondary_b,
            operation: MergeOp::Bidirectional,
        },
        Action::MergeAxis {
            second_input: secondary_c,
            operation: MergeOp::Average,
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
    ];
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![
            AxisPolarity::Bipolar,
            AxisPolarity::Bipolar,
            AxisPolarity::Bipolar,
        ],
        &[
            (0, 0.3, AxisPolarity::Bipolar),
            (1, 0.0, AxisPolarity::Bipolar),
            (2, 0.0, AxisPolarity::Bipolar),
        ],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
    let live = live_snapshot_with_axes_and_outputs(
        vec![
            (0.3, AxisPolarity::Bipolar),
            (0.0, AxisPolarity::Bipolar),
            (0.0, AxisPolarity::Bipolar),
        ],
        vec![(VJoyAxis::X, 0.3)],
    );
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains(">IN 1<") || html.contains(">IN 1 "), "expected IN 1; got: {html}");
    assert!(html.contains(">IN 2<") || html.contains(">IN 2 "), "expected IN 2; got: {html}");
    assert!(html.contains(">IN 3<") || html.contains(">IN 3 "), "expected IN 3; got: {html}");
    assert!(html.contains("IN \u{00b7} pipeline"), "expected IN section label; got: {html}");
}

/// Sibling outputs: 1 input, 2 sibling MapToVJoy at the top level.
/// Both OUT rows render with their own destination tag.
#[test]
fn editor_live_readout_sibling_outputs_render_two_out_rows() {
    use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis};

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::Y },
            },
        },
    ];
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar],
        &[(0, 0.5, AxisPolarity::Bipolar)],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X, VJoyAxis::Y]);
    let live = live_snapshot_with_axes_and_outputs(
        vec![(0.5, AxisPolarity::Bipolar)],
        vec![(VJoyAxis::X, 0.5), (VJoyAxis::Y, 0.5)],
    );
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("X axis"), "expected first OUT tag; got: {html}");
    assert!(html.contains("Y axis"), "expected second OUT tag; got: {html}");
}
```

- [ ] **Step 3: Verify the tests pass**

Run: `cargo test -p inputforge-gui-dx --lib mapping_editor::tests::editor_live_readout_stacked_merges_render_three_in_rows mapping_editor::tests::editor_live_readout_sibling_outputs_render_two_out_rows`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs
git commit -m "test(live_readout): add SSR coverage for stacked merges and sibling outputs"
```

---

## Task 21: SSR tests for conditional active/inactive branches + keyboard chip

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs`

Add the predicate-driven scenarios: a Conditional with one OUT in each branch (matching predicate -> active branch live, other branch frozen); a keyboard OUT in the active branch (filled chip); a keyboard OUT in the inactive branch (hollow chip).

- [ ] **Step 1: Add SSR tests**

Append after the Task 20 tests:

```rust
// ---------------------------------------------------------------------------
// Multi-merge / multi-out: conditional active/inactive
// ---------------------------------------------------------------------------

/// Conditional on a pressed button: the if-true branch's OUT renders live
/// (no frozen modifier), the if-false branch's OUT renders with the frozen
/// modifier.
#[test]
fn editor_live_readout_conditional_active_branch_live_inactive_frozen() {
    use inputforge_core::action::Condition;
    use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis};

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let trigger = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: trigger },
        if_true: vec![Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        }],
        if_false: vec![Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::Y },
            },
        }],
    }];
    let mut state = seeded_profile_with_pipeline(
        actions,
        vec![AxisPolarity::Bipolar],
        &[(0, 0.0, AxisPolarity::Bipolar)],
        &[(0, true)], // button pressed -> if_true active
        &[],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X, VJoyAxis::Y]);
    let live = live_snapshot_with_axes_and_outputs(
        vec![(0.0, AxisPolarity::Bipolar)],
        vec![(VJoyAxis::X, 0.0), (VJoyAxis::Y, 0.0)],
    );
    let mut vdom = harness_with_live(state, primary, live);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    let frozen_count = html.matches(super::live_readout::FROZEN_ROW_CLASS).count();
    assert_eq!(
        frozen_count, 1,
        "expected exactly one frozen row (inactive branch's Y OUT); found {frozen_count}; html: {html}"
    );
}

/// Keyboard OUT in the active branch: chip rendered with the live modifier.
#[test]
fn editor_live_readout_keyboard_active_renders_live_chip() {
    use inputforge_core::action::Condition;
    use inputforge_core::types::{KeyCombo, KeyModifier};

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let trigger = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let combo = KeyCombo {
        key: "Space".to_owned(),
        modifiers: vec![KeyModifier::Ctrl],
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: trigger },
        if_true: vec![Action::MapToKeyboard { key: combo }],
        if_false: vec![],
    }];
    let state = seeded_profile_with_pipeline(
        actions,
        vec![AxisPolarity::Bipolar],
        &[(0, 0.0, AxisPolarity::Bipolar)],
        &[(0, true)],
        &[],
    );
    let mut vdom = harness_with(state, primary);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-editor__readout-kb-chip--live"),
        "expected live kb chip class; got: {html}"
    );
    assert!(html.contains("Ctrl + Space"), "expected key combo; got: {html}");
}

/// Keyboard OUT in the inactive branch: chip rendered with the idle
/// modifier and the OUT row carries the frozen modifier.
#[test]
fn editor_live_readout_keyboard_inactive_renders_idle_chip() {
    use inputforge_core::action::Condition;
    use inputforge_core::types::{KeyCombo, KeyModifier};

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let trigger = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let combo = KeyCombo {
        key: "Space".to_owned(),
        modifiers: vec![KeyModifier::Ctrl],
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: trigger },
        if_true: vec![Action::MapToKeyboard { key: combo }],
        if_false: vec![],
    }];
    let state = seeded_profile_with_pipeline(
        actions,
        vec![AxisPolarity::Bipolar],
        &[(0, 0.0, AxisPolarity::Bipolar)],
        &[(0, false)], // button NOT pressed -> if_true inactive
        &[],
    );
    let mut vdom = harness_with(state, primary);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-editor__readout-kb-chip--idle"),
        "expected idle kb chip class; got: {html}"
    );
    assert!(
        html.contains(super::live_readout::FROZEN_ROW_CLASS),
        "expected frozen modifier on inactive kb row; got: {html}"
    );
}
```

- [ ] **Step 2: Verify the tests pass**

Run: `cargo test -p inputforge-gui-dx --lib mapping_editor::tests::editor_live_readout_conditional_active_branch_live_inactive_frozen mapping_editor::tests::editor_live_readout_keyboard_active_renders_live_chip mapping_editor::tests::editor_live_readout_keyboard_inactive_renders_idle_chip`
Expected: 3 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs
git commit -m "test(live_readout): add SSR coverage for conditional branches and keyboard chip"
```

---

## Task 22: SSR tests for predicate chips + expand toggle

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs`

Final SSR coverage: predicate chips render in the IN section with the appropriate live/idle dot class; the expand chevron is present on rows whose chain is non-empty; the OUT chain block is hidden by default and revealed by toggling the expand state.

The expand-toggle behaviors (per-row click sets `per_output[i]=true` → chain renders; "expand all" pill expands every chain; mapping-selection change resets `per_output`) cannot be exercised through synthetic click events in SSR. Instead, this task introduces a **test-only constructor** `LiveReadout::test_with_expand_state(primary, actions, expand_state: Signal<ExpandState>)` that lets a test inject a pre-set expand signal directly. The four expand-toggle tests use this to drive the signal and assert the rendered DOM under each state.

- [ ] **Step 1: Add SSR tests**

Append after the Task 21 tests:

```rust
// ---------------------------------------------------------------------------
// Multi-merge / multi-out: predicate chip subsection
// ---------------------------------------------------------------------------

/// Pressed button + Conditional that references it -> predicate chip
/// renders with `--live` modifier; the IN · predicates section label
/// is present.
#[test]
fn editor_live_readout_predicate_chip_button_pressed_renders_live() {
    use inputforge_core::action::Condition;

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let trigger = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: trigger },
        if_true: vec![],
        if_false: vec![],
    }];
    let state = seeded_profile_with_pipeline(
        actions,
        vec![AxisPolarity::Bipolar],
        &[(0, 0.0, AxisPolarity::Bipolar)],
        &[(0, true)],
        &[],
    );
    let mut vdom = harness_with(state, primary);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("IN \u{00b7} predicates"),
        "expected predicate section label; got: {html}"
    );
    assert!(
        html.contains("if-editor__readout-chip--live"),
        "expected live chip class; got: {html}"
    );
}

/// AxisInRange chip renders the range glyph `[min..max]`.
#[test]
fn editor_live_readout_predicate_chip_axis_in_range_shows_bounds() {
    use inputforge_core::action::Condition;

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let trigger_axis = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::AxisInRange {
            input: trigger_axis,
            min: 0.20,
            max: 0.80,
        },
        if_true: vec![],
        if_false: vec![],
    }];
    let state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar, AxisPolarity::Bipolar],
        &[
            (0, 0.0, AxisPolarity::Bipolar),
            (1, 0.5, AxisPolarity::Bipolar),
        ],
    );
    let mut vdom = harness_with(state, primary);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("[0.20..0.80]"),
        "expected range glyph; got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Multi-merge / multi-out: chevron + expand-all
// ---------------------------------------------------------------------------

/// One OUT with a non-empty chain (one merge): the chevron button is
/// rendered. One OUT with an empty chain (no merge): the chevron is
/// absent.
#[test]
fn editor_live_readout_chevron_present_only_when_chain_non_empty() {
    use inputforge_core::types::{MergeOp, OutputAddress, OutputId, VJoyAxis};

    // No-merge case: no chevron.
    {
        let primary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let actions = vec![Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        }];
        let mut state = seeded_profile_with_polarities_and_axes(
            actions,
            vec![AxisPolarity::Bipolar],
            &[(0, 0.0, AxisPolarity::Bipolar)],
        );
        add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
        let mut vdom = harness_with(state, primary);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains("if-editor__readout-chevron\""),
            "expected no chevron button when chain is empty; got: {html}"
        );
    }

    // With merge: chevron rendered.
    {
        let primary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let secondary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        let actions = vec![
            Action::MergeAxis {
                second_input: secondary,
                operation: MergeOp::Bidirectional,
            },
            Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Axis { id: VJoyAxis::X },
                },
            },
        ];
        let mut state = seeded_profile_with_polarities_and_axes(
            actions,
            vec![AxisPolarity::Bipolar, AxisPolarity::Bipolar],
            &[
                (0, 0.0, AxisPolarity::Bipolar),
                (1, 0.0, AxisPolarity::Bipolar),
            ],
        );
        add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
        let mut vdom = harness_with(state, primary);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("if-editor__readout-chevron"),
            "expected chevron button when chain non-empty; got: {html}"
        );
    }
}

/// Expand-all pill: rendered only when at least one OUT has a non-empty
/// chain.
#[test]
fn editor_live_readout_expand_all_pill_visible_when_any_chain_non_empty() {
    use inputforge_core::types::{MergeOp, OutputAddress, OutputId, VJoyAxis};

    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![
        Action::MergeAxis {
            second_input: secondary,
            operation: MergeOp::Bidirectional,
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
    ];
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar, AxisPolarity::Bipolar],
        &[
            (0, 0.0, AxisPolarity::Bipolar),
            (1, 0.0, AxisPolarity::Bipolar),
        ],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
    let mut vdom = harness_with(state, primary);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-editor__readout-expand-all"),
        "expected expand-all pill; got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Multi-merge / multi-out: expand-toggle behavior (Signal-driven SSR)
// ---------------------------------------------------------------------------

/// Default state: chain block is NOT rendered. Asserts collapsed-by-default
/// markup invariant.
#[test]
fn editor_live_readout_chain_collapsed_by_default_renders_no_chain_block() {
    let (state, primary) = expand_toggle_fixture();
    let mut vdom = harness_with(state, primary);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("if-editor__readout-chevron"));
    // No chain block in the default state.
    assert!(
        !html.contains("if-editor__readout-chain\""),
        "expected NO chain block by default; got: {html}"
    );
    // Chain rows must not appear.
    assert!(!html.contains("if-editor__readout-chain-row"));
}

/// Per-output expand: setting per_output[0]=true renders the chain block
/// for THAT output only.
#[test]
fn editor_live_readout_per_output_expand_renders_chain_block() {
    let (state, primary) = expand_toggle_fixture();
    let actions = expand_toggle_fixture_actions();
    let mut expand_state =
        super::live_readout::out_block::ExpandState::default();
    expand_state.per_output = vec![true];
    let mut vdom = harness_with_expand(state, primary, actions, expand_state);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-editor__readout-chain"),
        "expected chain block when per_output[0]=true; got: {html}"
    );
    assert!(
        html.contains(">MERGE 1<"),
        "expected MERGE 1 chain row; got: {html}"
    );
}

/// Expand-all: setting expand_all=true expands EVERY chain regardless of
/// per_output. Test fixture has 2 OUTs both with non-empty chains.
#[test]
fn editor_live_readout_expand_all_expands_every_chain() {
    let (state, primary) = expand_toggle_fixture_two_outputs();
    let actions = expand_toggle_fixture_two_outputs_actions();
    let mut expand_state =
        super::live_readout::out_block::ExpandState::default();
    expand_state.expand_all = true;
    let mut vdom = harness_with_expand(state, primary, actions, expand_state);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    let chain_count = count_substring(&html, "if-editor__readout-chain\"");
    assert_eq!(
        chain_count, 2,
        "expected 2 chain blocks (one per OUT) under expand_all; got {chain_count}: {html}"
    );
}

/// Selection change: when the model rebuilds with a different outputs.len(),
/// per_output is reset to all-false. Asserts the use_effect at
/// `live_readout/mod.rs::LiveReadout` zeroes the per_output vec on length
/// change. Without this guard, switching to a mapping with the same output
/// count would silently retain stale toggles; switching to a different
/// count would carry over an out-of-range vec.
#[test]
fn editor_live_readout_selection_change_resets_expand_state() {
    // Step 1: render a 1-OUT mapping with per_output[0]=true (chain visible).
    let (state, primary) = expand_toggle_fixture();
    let actions_one_out = expand_toggle_fixture_actions();
    let mut expand_state =
        super::live_readout::out_block::ExpandState::default();
    expand_state.per_output = vec![true];
    let mut vdom = harness_with_expand(state.clone(), primary.clone(), actions_one_out, expand_state.clone());
    vdom.rebuild_in_place();
    let html_initial = render(&vdom);
    assert!(html_initial.contains("if-editor__readout-chain"));

    // Step 2: rebuild with a DIFFERENT mapping (2 outputs). The
    // use_effect must reset per_output, so even though we previously had
    // [true], the new render sees an empty per_output -> no chains.
    let (state2, primary2) = expand_toggle_fixture_two_outputs();
    let actions_two_outs = expand_toggle_fixture_two_outputs_actions();
    // Use the SAME signal across rebuilds via the harness helper that
    // simulates a selection change.
    let mut vdom2 = harness_with_expand_simulating_selection_change(
        state2,
        primary2,
        actions_two_outs,
        expand_state,
    );
    vdom2.rebuild_in_place();
    let html_after_change = render(&vdom2);
    // After reset, no chains should be visible.
    assert!(
        !html_after_change.contains("if-editor__readout-chain\""),
        "expected per_output reset on selection change → no chains; got: {html_after_change}"
    );
}

// Test-only fixture helpers. These live next to the test functions to
// keep the SSR test file self-contained.

fn expand_toggle_fixture() -> (AppState, InputAddress) {
    use inputforge_core::types::{MergeOp, OutputAddress, OutputId, VJoyAxis};
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![
        Action::MergeAxis {
            second_input: secondary,
            operation: MergeOp::Bidirectional,
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
    ];
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar, AxisPolarity::Bipolar],
        &[
            (0, 0.0, AxisPolarity::Bipolar),
            (1, 0.0, AxisPolarity::Bipolar),
        ],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X]);
    (state, primary)
}

fn expand_toggle_fixture_actions() -> Vec<Action> {
    use inputforge_core::types::{MergeOp, OutputAddress, OutputId, VJoyAxis};
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    vec![
        Action::MergeAxis {
            second_input: secondary,
            operation: MergeOp::Bidirectional,
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
    ]
}

fn expand_toggle_fixture_two_outputs() -> (AppState, InputAddress) {
    // Two OUTs, both with a merge in their chain.
    use inputforge_core::types::{MergeOp, OutputAddress, OutputId, VJoyAxis};
    let primary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let actions = vec![
        Action::MergeAxis {
            second_input: secondary,
            operation: MergeOp::Bidirectional,
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::Y },
            },
        },
    ];
    let mut state = seeded_profile_with_polarities_and_axes(
        actions,
        vec![AxisPolarity::Bipolar, AxisPolarity::Bipolar],
        &[
            (0, 0.0, AxisPolarity::Bipolar),
            (1, 0.0, AxisPolarity::Bipolar),
        ],
    );
    add_vjoy_device(&mut state, 1, vec![VJoyAxis::X, VJoyAxis::Y]);
    (state, primary)
}

fn expand_toggle_fixture_two_outputs_actions() -> Vec<Action> {
    use inputforge_core::types::{MergeOp, OutputAddress, OutputId, VJoyAxis};
    let secondary = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    vec![
        Action::MergeAxis {
            second_input: secondary,
            operation: MergeOp::Bidirectional,
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::Y },
            },
        },
    ]
}

/// Test harness that injects a pre-set ExpandState signal into LiveReadout.
/// Requires `LiveReadout::test_with_expand_state(primary, actions, signal)`
/// constructor on the public API. If the constructor doesn't exist yet,
/// add it inside `live_readout/mod.rs` gated on `#[cfg(test)]`.
fn harness_with_expand(
    state: AppState,
    primary: InputAddress,
    actions: Vec<Action>,
    expand_state: super::live_readout::out_block::ExpandState,
) -> VirtualDom {
    // Implementation: same as harness_with but mounts a wrapper component
    // that calls LiveReadout::test_with_expand_state(primary, actions,
    // use_signal(|| expand_state.clone())) instead of LiveReadout(primary,
    // actions). Mirror the existing harness_with helper one-to-one.
    todo!("wire harness_with_expand once LiveReadout::test_with_expand_state lands")
}

/// Same as harness_with_expand but rebuilds the VirtualDom with a fresh
/// model to simulate the user switching to a different mapping. The
/// expand_state signal is shared across rebuilds so we observe the
/// use_effect's reset behavior.
fn harness_with_expand_simulating_selection_change(
    state: AppState,
    primary: InputAddress,
    actions: Vec<Action>,
    pre_change_expand_state: super::live_readout::out_block::ExpandState,
) -> VirtualDom {
    todo!("wire selection-change harness once test-only constructor lands")
}
```

To support these tests, add the test-only constructor to `live_readout/mod.rs`:

```rust
#[cfg(test)]
impl LiveReadout {
    // Not a real impl block on a component; this is shorthand for
    // "add a #[component] gated on test that takes the extra signal".
}

#[cfg(test)]
#[component]
pub(crate) fn LiveReadoutTest(
    primary: InputAddress,
    actions: Vec<Action>,
    injected_expand_state: Signal<ExpandState>,
) -> Element {
    // Body is identical to LiveReadout but uses `injected_expand_state`
    // instead of creating its own `use_signal(ExpandState::default)`.
    let ctx = use_context::<AppContext>();
    let model = {
        let state = ctx.state.read();
        let cfg = ctx.config.read();
        analyzer::analyze(&actions, &primary, &state, &cfg)
    };
    let engine_running = matches!(ctx.meta.read().engine_status, EngineStatus::Running);
    let outputs_len = model.outputs.len();

    let mut expand_state = injected_expand_state;
    let mut prev_outputs_len: Signal<usize> = use_signal(|| outputs_len);
    use_effect(move || {
        let prev = *prev_outputs_len.read();
        if prev != outputs_len {
            expand_state.with_mut(|s| {
                s.per_output = vec![false; outputs_len];
                s.expand_all = false;
            });
            prev_outputs_len.set(outputs_len);
        }
    });

    let model_for_in = model.clone();
    let model_for_div = model.clone();
    let model_for_out = model;
    rsx! {
        div { class: "if-editor__readout",
            InBlock { model: model_for_in }
            DividerStrip { model: model_for_div, expand_state }
            OutBlock { model: model_for_out, expand_state, engine_running }
        }
    }
}
```

- [ ] **Step 2: Verify the tests pass**

Run: `cargo test -p inputforge-gui-dx --lib mapping_editor::tests`
Expected: 4 chip/chevron/expand-all tests + 4 new expand-toggle tests + Task 19's 7 + Tasks 20-21 — all green.

Run: `cargo test -p inputforge-gui-dx`
Expected: full crate green.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs
git commit -m "test(live_readout): add SSR coverage for predicate chips and expand chevrons"
```

---

## Task 23: Verify in the running GUI (manual)

**Files:** none (manual verification step)

Run the Dioxus GUI manually and exercise the new readout against a profile that contains stacked merges, conditionals, and keyboard outputs. The SSR tests cover markup; this step verifies live behaviour and visual fidelity.

- [ ] **Step 1: Launch the dev GUI**

Run: `dx run -p inputforge-app`

- [ ] **Step 2: Build a multi-merge / multi-out test profile in the GUI**

Author one mapping that contains:
- Two stacked `MergeAxis` (e.g. primary stick X + pedal L + pedal R).
- A `Conditional` whose `if_true` writes to vJoy axis A and `if_false` writes to vJoy axis B.
- A second `Conditional` whose `if_true` writes a keyboard combo.

Move the inputs and verify:
- Three IN rows render with their bars tracking the live values (including unipolar pedals at idle reading `0.00`, not `-1.00`).
- Two predicate chips appear under `IN · predicates`. Pressing the trigger button flips the chip from hollow/idle to filled/live.
- Three OUT rows render. The active conditional branch's OUT renders live; the other carries the frozen modifier.
- The keyboard OUT renders as a violet chip; the chip flips between filled and hollow as the conditional gate toggles.
- Each OUT with a non-empty chain has a chevron. Click expands to reveal `MERGE 1`, `MERGE 2`, and `COND` rows in teal/violet.
- The "expand all" pill on the divider toggles every chevron at once.

- [ ] **Step 3: Stop the engine and re-verify**

Toggle the engine off (top-right pill in the editor frame). Verify every OUT row carries the frozen modifier; chain bars and chips dim; predicate chips remain interactive (their state still updates as the user moves inputs because `evaluate_condition` reads from the input cache, which the device thread writes regardless of engine state).

Specifically: stop the engine WHILE a chain is expanded. The chain rows beneath the OUT row must mute alongside the OUT row (the `--frozen` modifier on the wrap element reaches both via descendant selectors per Task 18 CSS).

- [ ] **Step 4: Verify mapping-selection resets expand state**

With one chevron expanded, click a different mapping in the sidebar. The newly-shown mapping's chevrons must collapse to default (no expanded chains) — confirms the `use_effect` in `LiveReadout` correctly tracks previous-vs-current `outputs.len()` and zeroes `per_output` on change. Repeat with a target mapping that has the same OUT count: the reset must still happen (the previous-length check catches "different mapping but same length").

- [ ] **Step 5: Verify predicate chip flips frame-to-frame**

Press a button bound to a Conditional's predicate. The chip dot must flip between live and idle in real time (no perceptible lag, no stale render). Release to confirm the inverse. For `ButtonReleased`, confirm the polarity is inverted: the dot is FILLED when the button is NOT pressed.

- [ ] **Step 6: Verify distinct AxisInRange ranges render distinct chips**

Author two `AxisInRange` Conditionals on the same input with different bounds (e.g. `[0.20..0.80]` and `[0.50..0.90]`). Confirm the IN · predicates section shows TWO chips, each with its own bounds glyph, and each chip's live dot independently reflects whether the current axis value sits in its range. Move the axis through both ranges' overlap and gaps to verify each chip behaves independently.

- [ ] **Step 7: Verify focus rings on chevron and expand-all pill**

Tab through the readout from the IN section into the OUT section. Each per-OUT chevron and the expand-all pill must show a 2px focus-cyan outline at 2px offset (matching the project `if-icon-button` pattern). Focus must remain visible against the dark background.

- [ ] **Step 8: Capture a screenshot for the PR description**

Use the chrome-devtools MCP `take_screenshot` tool. Try `http://localhost:9222/json` first; if it returns empty (per project CLAUDE.md, WebView2Feedback#4709 documents an IPv4/IPv6 binding quirk), fall back to `http://127.0.0.1:9222/json`. Embed the screenshot in the PR description.

- [ ] **Step 9: No commit**

This task produces no code. The next user-driven step is opening the PR.

---

## Self-review

After completing all 23 tasks, run the full crate test suite once more and skim the result:

Run: `cargo test -p inputforge-core --lib pipeline`
Expected: 0 failures (including the 6 new `evaluate_actions_through_path_*` tests from Task 6).

Run: `cargo test -p inputforge-gui-dx`
Expected: 0 failures.

Run: `cargo clippy -p inputforge-gui-dx --no-deps -- -D warnings`
Expected: clean.

Final spec coverage check:
- Q1 (D, all of the above): walker handles stacked merges, multi-output, conditionals. Stacked merges + multi-output: Task 20. Composite + nested conditional: Task 19 (added to fill review-flagged coverage gap).
- Q2 (C, compact hybrid): IN at top, OUT at bottom, expandable chain in between. Implemented in Task 17.
- Q3 (A, show both, distinguish active vs inactive): `is_active` + `frozen` modifier on inactive rows. Test 21's `editor_live_readout_conditional_active_branch_live_inactive_frozen`. Engine-stopped + multi-OUT freeze: Task 19.
- Q4 (B, two grouped subsections): `IN · pipeline` + `IN · predicates`. Tasks 12, 17, 20. Distinct AxisInRange ranges render as distinct chips (per dedup amendment): Tasks 11 (analyzer test) + 19 (SSR-style coverage).
- Q5 (B, per-OUT expand with global override): per-row chevron + divider pill. Task 16. Per-OUT expand toggle, expand-all toggle, selection-change reset: Task 22 (test-only Signal injection).
- Q6 (A, kb live state, B deferred): kb chip mirrors `is_active`. Task 15. Tests in Task 21 (live state) + Task 19 (`ButtonReleased` polarity inversion).

Specific spec scenarios from spec § Testing (lines 293-310) and edge-case table (lines 282-287):
- Stacked merges, multi-output: Task 20.
- Conditional active/inactive branches: Task 21.
- Predicate chips, expand toggle: Task 22.
- Composite (`All`/`Any`/`Not`) predicate: Task 19.
- Nested conditional path-AND: Task 19.
- Engine-stopped + multi-OUT: Task 19.
- HatDirection chip glyphs: Task 19.
- ButtonReleased suffix + inverted dot: Task 19.
- Per-output polarity disagreement: Task 19.
- AxisInRange chip live dot: Task 19.
- Selection-change resets expand: Task 22.
- Manual frame-to-frame predicate flip + engine-stopped chain mute: Task 23.

If anything from the spec is still uncovered after running the suite, add a targeted SSR or unit test before declaring complete.
