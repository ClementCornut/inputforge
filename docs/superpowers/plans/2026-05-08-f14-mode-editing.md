# F14, Mode Editing (ChangeMode action editor) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cut `ModeChangeStrategy::Previous` and `::Cycle` from the engine, then ship the `ChangeMode` action editor body for the surviving `SwitchTo` (Set) and `Temporary` (Hold) strategies.

**Architecture:** Three sequential parts. Part A removes the Previous and Cycle variants from `inputforge-core` and every dependent test, fixture, and call site (compiles green between tasks because tests are removed alongside their code). Part B extends the F2 `Select` primitive with per-option `disabled` and `class` so the GUI can render an orphaned mode name as an error-tinted disabled `<option>`. Part C builds the `ChangeModeBody` Dioxus component (segmented Set/Hold pill row plus target-mode `Select`), wires it into the `StageBody` dispatcher, threads the three malformed-hint conditions through `EditorState::malformed_hints`, and trims `format_mode_strategy`. Acceptance is verified at the end with the engine grep gate and a focused SSR test pass.

**Tech Stack:** Rust 2024, Dioxus 0.7 (RSX, `Signal`, `use_context`), serde, dioxus_ssr (tests), `cargo nextest` for execution. Workspace crates: `inputforge-core` (engine), `inputforge-gui-dx` (Dioxus GUI, hosts F2 primitives and stage bodies).

---

## File Structure

### `crates/inputforge-core/`

- **Modify** `src/action/mode_change.rs`: remove `ModeChangeStrategy::Previous`, `ModeChangeStrategy::Cycle { modes }`, the `CycleModes` newtype with its `new`/`with_renamed`/`modes` API, both `Serialize`/`Deserialize` impls for `CycleModes`, and the `mode_change_strategy_cycle_serde_roundtrip` / `mode_change_strategy_previous_serde_roundtrip` / every `cycle_modes_*` / `with_renamed_*` test.
- **Modify** `src/action/mod.rs:11`: drop `CycleModes` from the `pub use mode_change::{...}` re-export, leaving only `ModeChangeStrategy`.
- **Modify** `src/mode/state.rs`: remove `ModeState::go_previous` (lines 88-91), `ModeState::cycle` (lines 113-138), the `use crate::action::CycleModes;` import (line 3), and the `go_previous_pops_stack` / `cycle_advances` / `cycle_wraps_around` / `cycle_from_outside_list` / `cycle_nonexistent_mode_in_list` / `cycle_clears_stack` tests.
- **Modify** `src/engine/output_handler.rs:101-134`: the match in `apply_mode_change` collapses to two arms (`SwitchTo`, `Temporary`); remove the `Previous` and `Cycle` arms.
- **Modify** `src/types/address.rs`: add `is_button_shaped()` to `InputId` (returns `true` only for `Button { .. }`) and to `InputAddress` (delegates to the inner `InputId` when `Bound`, returns `false` for `Unbound`). Add tests covering all three `InputId` variants and both `InputAddress` variants.
- **Modify** `src/engine/tests.rs`: remove `process_outputs_previous_mode` (lines 1404-1435), `process_outputs_cycle_mode` (lines 1437-1472), `rename_mode_rejects_when_cycle_would_collapse` (lines 3091-3149), and `rename_mode_rewrites_cycle_action` (lines 3255-3305). Strip `CycleModes` from the `use` import on line 17. The `process_outputs_mode_change_no_op` test stays.
- **Modify** `src/profile/mod.rs`: remove `check_cycle_rename` (lines 520-543), the `Cycle` arm in `rewrite_mode_in_action` (lines 561-580), `rename_mode_refs_rewrites_cycle_entries` (lines 1569-1601), `rename_mode_refs_rejects_cycle_collision` (lines 1603-1641), and the `rename_mode_refs_byte_identity_unchanged_subtree` setup that constructs Cycle (lines 1643-1700+, scope verified at task time). Update the `rename_mode_refs` doc comment (lines 405-419) to drop the cycle pre-validation references. Update the call site of `check_cycle_rename` (locate at task time).

### `crates/inputforge-gui-dx/`

- **Modify** `src/components/select.rs`: introduce `pub struct SelectOption { value: String, label: String, disabled: bool, class: Option<String> }` and change the `options` prop on the `Select` component from `Vec<(String, String)>` to `Vec<SelectOption>`. Render each `<option>` with its own `disabled` and `class` attributes.
- **Modify** `src/frame/bulk_map/mod.rs`: migrate the four `Select` call sites at lines 361, 369, 377, 620 to the `SelectOption` shape (all `disabled: false`, `class: None`). Build helpers used to produce options must produce `Vec<SelectOption>`.
- **Modify** `src/frame/mapping_editor/pipeline/stage_body/predicate.rs`: migrate the kind picker `Select` at line 931 (and the `kind_options: Vec<...>` builder above it) to `Vec<SelectOption>`.
- **Modify** `src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs`: migrate `op_options` (lines 363-369) and the `Select` call site at line 385 to `Vec<SelectOption>`.
- **Modify** `src/frame/mapping_editor/pipeline/stage_body/map_to_vjoy.rs`: migrate `device_options` (lines 148-157), `output_options` (lines 173-202), and both `Select` call sites at lines 360 and 368 to `Vec<SelectOption>`.
- **Create** `src/frame/mapping_editor/pipeline/stage_body/change_mode.rs`: exports `pub(crate) fn ChangeModeBody(mapping_key, stage_id, strategy: ModeChangeStrategy, root_actions: Vec<Action>) -> Element`. Owns the segmented Set/Hold pill row, the target-mode `Select`, the three malformed-hint priorities, and dispatch through `dispatch_stage_edit`.
- **Modify** `src/frame/mapping_editor/pipeline/stage_body/mod.rs`: replace the `Action::ChangeMode { .. }` arm at line 115 with a `change_mode::ChangeModeBody { ... }` mount; replace the `mod placeholders;` declaration on line 25 with `mod change_mode;`; remove the `Action::ChangeMode { .. } => default_chevron(expanded),` line in `header_right_slot` (line 138) so the catch-all handles it.
- **Delete** `src/frame/mapping_editor/pipeline/stage_body/placeholders.rs` once nothing references `ChangeModePlaceholder`.
- **Modify** `src/frame/mapping_editor/pipeline/stage.rs:344-355`: rewrite `format_mode_strategy` to two arms only (`SwitchTo` -> `Set <mode>`, `Temporary` -> `Hold <mode>`).
- **Modify** `src/frame/mapping_editor/pipeline/tests.rs:1030-1043`: rewrite `placeholder_bodies_show_spec_caption` so it no longer references `ModeChangeStrategy::Previous` or the `F14 owns this body` caption (the whole test goes away because there is no more placeholder).
- **Modify** `assets/frame/mapping_editor.css`: add `.if-stage__body-change-mode` to the existing 2-column grid rule at lines 281-290; add the `.if-stage__body-strategy` pill-row rule (segmented control).
- **Create** `src/frame/top_bar/mode_tabs/context_menu_dispatch_test.rs` (or co-locate inside `context_menu.rs` `#[cfg(test)] mod tests`): SSR test that mounts `ModeTabContextMenu` with an enabled "Set as default" item, simulates the click, and asserts an `EngineCommand::SetDefaultMode { name }` arrives on the channel. Required because the existing `flags_for` tests cover only the disabled-flag derivation, not the dispatch.

Each created or heavily modified `.rs` file stays under the project's normal size budget (the new `change_mode.rs` is one component plus its hint helpers; the body has a single rsx tree with two grid rows).

---

## Sequencing

The plan runs in three parts:

- **Part A (Tasks 1-7):** Engine cut. Shipping order minimises broken-build windows by removing each variant's tests in the same task as the code they cover.
- **Part B (Tasks 8-9):** F2 `Select` primitive extension. Sequenced before the GUI body because the body depends on per-option `disabled` and `class`.
- **Part C (Tasks 10-18):** GUI body, dispatcher rewire, summary trim, and acceptance pass.

`cargo build -p inputforge-core --tests` and `cargo build -p inputforge-gui-dx --tests` should both stay green at every commit boundary.

---

## Part A: Engine cut

### Task 1: Remove `Previous` from `ModeChangeStrategy` and its tests

**Files:**
- Modify: `crates/inputforge-core/src/action/mode_change.rs:15`
- Modify: `crates/inputforge-core/src/action/mode_change.rs:118-125` (test `mode_change_strategy_previous_serde_roundtrip`)

- [ ] **Step 1: Delete the `Previous` variant**

In `crates/inputforge-core/src/action/mode_change.rs`, remove line 15 from the enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum ModeChangeStrategy {
    SwitchTo { mode: String },
    Temporary { mode: String },
    Cycle { modes: CycleModes },
}
```

The serde tag stays the same shape, so `SwitchTo` / `Temporary` / `Cycle` continue to round-trip byte-identically.

- [ ] **Step 2: Delete the `Previous` round-trip test**

Remove `mode_change_strategy_previous_serde_roundtrip` (lines 118-125). The other two round-trip tests stay.

- [ ] **Step 3: Build to confirm intermediate compile errors come only from the engine cut chain**

Run: `cargo build -p inputforge-core --tests`
Expected: errors point only to `engine/output_handler.rs` (`Previous` arm), `engine/tests.rs` (`process_outputs_previous_mode`), `mode/state.rs` (`go_previous`), and any pipeline tests referencing `Previous`. No surprises elsewhere.

- [ ] **Step 4: Note the error list**

Capture the file paths and line numbers from the `cargo build` output, the next tasks consume them. (Do not commit yet; Tasks 1+2 ship together because the build is broken until Task 2 lands.)

### Task 2: Remove `Cycle`, `CycleModes`, and the dependent state machine from core

**Files:**
- Modify: `crates/inputforge-core/src/action/mode_change.rs` (full sweep)
- Modify: `crates/inputforge-core/src/action/mod.rs:11`
- Modify: `crates/inputforge-core/src/mode/state.rs`
- Modify: `crates/inputforge-core/src/engine/output_handler.rs:101-134`
- Modify: `crates/inputforge-core/src/engine/tests.rs:17, 1404-1472`

- [ ] **Step 1: Strip `Cycle` and `CycleModes` from `mode_change.rs`**

The post-cut file is exactly:

```rust
// Rust guideline compliant 2026-03-02

use serde::{Deserialize, Serialize};

/// Strategy for changing the active input mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum ModeChangeStrategy {
    SwitchTo { mode: String },
    Temporary { mode: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_change_strategy_switch_to_serde_roundtrip() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "combat".to_owned(),
        };
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("\"strategy\":\"switch_to\""));
        let back: ModeChangeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, back);
    }

    #[test]
    fn mode_change_strategy_temporary_serde_roundtrip() {
        let strategy = ModeChangeStrategy::Temporary {
            mode: "combat".to_owned(),
        };
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("\"strategy\":\"temporary\""));
        let back: ModeChangeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, back);
    }
}
```

The `use std::collections::HashSet;` and `use crate::error::{EngineError, Result};` imports go away; the only crate-internal imports left are `serde::{Deserialize, Serialize}`.

- [ ] **Step 2: Drop `CycleModes` from the `mode_change` re-export**

In `crates/inputforge-core/src/action/mod.rs`, change line 11 from:

```rust
pub use mode_change::{CycleModes, ModeChangeStrategy};
```

to:

```rust
pub use mode_change::ModeChangeStrategy;
```

- [ ] **Step 3: Remove `go_previous` and `ModeState::cycle` from `state.rs`**

In `crates/inputforge-core/src/mode/state.rs`:

- Remove the `use crate::action::CycleModes;` import (line 3).
- Remove the entire `pub fn go_previous(&mut self)` method including its doc comment (lines 88-91).
- Remove the entire `pub fn cycle(&mut self, modes: &CycleModes, tree: &ModeTree) -> Result<()>` method including its doc comment (lines 113-137).
- Remove the `// --- go_previous ---` and `// --- cycle ---` test sections including every test inside them: `go_previous_pops_stack`, `cycle_advances`, `cycle_wraps_around`, `cycle_from_outside_list`, `cycle_nonexistent_mode_in_list`, `cycle_clears_stack` (lines 259-341).

The post-cut `state.rs` keeps `new`, `current`, `switch_to`, `push_temporary`, `pop_temporary`, `clear_stack_entries`, and `rename_in_place`.

- [ ] **Step 4: Collapse `apply_mode_change` to two arms**

In `crates/inputforge-core/src/engine/output_handler.rs`, replace the four-arm `match strategy` (lines 101-134) with:

```rust
match strategy {
    ModeChangeStrategy::SwitchTo { mode } => {
        if let Err(e) = mode_state.switch_to(mode, tree) {
            tracing::warn!(
                mode,
                error = %e,
                "SwitchTo failed, skipping"
            );
        }
    }
    ModeChangeStrategy::Temporary { mode } => match mode_state.push_temporary(mode, tree) {
        Ok(()) => {
            callbacks.register(triggering_input.clone(), ReleaseCallback::PopTemporaryMode);
        }
        Err(e) => {
            tracing::warn!(
                mode,
                error = %e,
                "Temporary mode push failed, skipping"
            );
        }
    },
}
```

- [ ] **Step 5: Remove engine-level Previous and Cycle tests**

In `crates/inputforge-core/src/engine/tests.rs`:

- Line 17: change `use crate::action::{Action, Condition, CycleModes, Mapping, ModeChangeStrategy};` to `use crate::action::{Action, Condition, Mapping, ModeChangeStrategy};`.
- Remove `process_outputs_previous_mode` (lines 1404-1435).
- Remove `process_outputs_cycle_mode` (lines 1437-1472).
- Keep `process_outputs_mode_change_no_op` (the SwitchTo-to-current-mode no-op test).

- [ ] **Step 6: Run the core test suite**

Run: `cargo test -p inputforge-core`
Expected: every test passes. No `Previous`, `Cycle`, or `CycleModes` references remain in the compiled output.

- [ ] **Step 7: Commit Tasks 1+2 together**

```bash
git add crates/inputforge-core/src/action/mode_change.rs \
        crates/inputforge-core/src/action/mod.rs \
        crates/inputforge-core/src/mode/state.rs \
        crates/inputforge-core/src/engine/output_handler.rs \
        crates/inputforge-core/src/engine/tests.rs
git commit -m "refactor(core): drop ModeChangeStrategy::Previous and ::Cycle"
```

(The conventional-commits skill prescribes the scope and verb. The single message covers both variants because the build was broken between Task 1 and Task 2 and a single commit is the smallest reviewable unit that compiles.)

### Task 3: Remove the profile-level Cycle rename machinery and tests

**Files:**
- Modify: `crates/inputforge-core/src/profile/mod.rs:405-419` (doc), `:419-451` (`rename_mode_refs` body), `:520-543` (`check_cycle_rename` removal), `:548-595` (`rewrite_mode_in_action` Cycle arm removal), `:1570-1602` (test `rename_mode_refs_rewrites_cycle_entries`), `:1604-1640` (test `rename_mode_refs_rejects_cycle_collision`), `:1641-1705` (test `rename_mode_refs_atomic_when_cycle_collides`)
- Modify: `crates/inputforge-core/src/engine/tests.rs:3091-3149` (test `rename_mode_rejects_when_cycle_would_collapse`), `:3255-3305` (test `rename_mode_rewrites_cycle_action`)
- Modify: `crates/inputforge-core/src/engine/run.rs:691` (call site of `rename_mode_refs`, only if return type changes)

- [ ] **Step 1: Delete `check_cycle_rename`**

In `crates/inputforge-core/src/profile/mod.rs`, remove the entire function `check_cycle_rename` (lines 520-543). The only caller is the pre-validation loop at lines 423-429 inside `rename_mode_refs`:

```rust
// Pre-validate cycle-rename safety on a clone of the action graphs.
// Returns Err early without touching self.
for mapping in &self.mappings {
    for action in &mapping.actions {
        check_cycle_rename(action, from, to)?;
    }
}
```

Delete those seven lines. After this removal, `rename_mode_refs` no longer has any failure path of its own.

- [ ] **Step 2: Strip the `Cycle` arm from `rewrite_mode_in_action`**

The function reduces to:

```rust
fn rewrite_mode_in_action(action: &mut Action, from: &str, to: &str) -> bool {
    use crate::action::ModeChangeStrategy as M;
    match action {
        Action::ChangeMode {
            strategy: M::SwitchTo { mode } | M::Temporary { mode },
        } => {
            if mode == from {
                to.clone_into(mode);
                true
            } else {
                false
            }
        }
        Action::Conditional {
            if_true, if_false, ..
        } => {
            let mut changed = false;
            for a in if_true {
                changed |= rewrite_mode_in_action(a, from, to);
            }
            for a in if_false {
                changed |= rewrite_mode_in_action(a, from, to);
            }
            changed
        }
        _ => false,
    }
}
```

- [ ] **Step 3: Update `rename_mode_refs` doc**

The doc comment for `rename_mode_refs` (lines 405-418) loses the "Pre-validates `CycleModes`" paragraph and the `# Errors` note about `InvalidConfig` for cycle-collapse, because the function no longer fails for that reason. The signature itself does not change (still returns `Result<usize>`); the residual `Result` is only because `CycleModes` validation lived inside the same function, but other failure paths (e.g., snapshot writes) may still exist. Verify the actual remaining error sources at task time and update the doc to match.

After the cut, `rename_mode_refs` has no remaining error path. Change its signature from `pub fn rename_mode_refs(&mut self, from: &str, to: &str) -> Result<usize>` to `pub fn rename_mode_refs(&mut self, from: &str, to: &str) -> usize`, change the `Ok(0)` early return at line 421 to `return 0;`, change the trailing `Ok(touched)` at line 450 to `touched`, and update the single non-test caller at `crates/inputforge-core/src/engine/run.rs:691` (`let touched = profile.rename_mode_refs(&from, &to)?;` becomes `let touched = profile.rename_mode_refs(&from, &to);`). Test call sites at `crates/inputforge-core/src/profile/mod.rs:1511, 1517, 1552, 1740` change from `.unwrap()` to bare expressions.

- [ ] **Step 4: Delete the cycle-only profile tests**

Remove from `crates/inputforge-core/src/profile/mod.rs`:

- `rename_mode_refs_rewrites_cycle_entries` (lines 1569-1601)
- `rename_mode_refs_rejects_cycle_collision` (lines 1603-1641)
- `rename_mode_refs_atomic_when_cycle_collides` (lines 1641-1705): remove wholesale. The test's setup constructs a `Cycle` strategy at lines 1670-1680 to provoke the collision rejection path; after the cut that path no longer exists, so the byte-identity assertion at lines 1695-1699 has no triggering condition. Atomic rollback is moot once `rename_mode_refs` becomes infallible.

- [ ] **Step 5: Delete the cycle-only engine integration tests**

Remove from `crates/inputforge-core/src/engine/tests.rs`:

- `rename_mode_rejects_when_cycle_would_collapse` (lines 3091-3149).
- `rename_mode_rewrites_cycle_action` (lines 3255-3305).

The `rename_mode_rewrites_switch_to_action` and `rename_mode_rewrites_temporary_action` tests stay.

- [ ] **Step 6: Run the test suite to confirm green**

Run: `cargo test -p inputforge-core`
Expected: all surviving tests pass. The `rename_mode_refs_rewrites_change_mode_actions` style tests (acceptance follow-up #6) remain.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-core/src/profile/mod.rs \
        crates/inputforge-core/src/engine/tests.rs
git commit -m "refactor(core): drop Cycle rename machinery and dependent tests"
```

### Task 4: Sweep the GUI for surviving Previous/Cycle references

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs:344-355`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs:1030-1043`

- [ ] **Step 1: Trim `format_mode_strategy` to two arms**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs:344-355`, replace the function with:

```rust
/// Format a [`ModeChangeStrategy`] to a concise one-line description.
fn format_mode_strategy(strategy: &ModeChangeStrategy) -> String {
    match strategy {
        ModeChangeStrategy::SwitchTo { mode } => format!("Set {mode}"),
        ModeChangeStrategy::Temporary { mode } => format!("Hold {mode}"),
    }
}
```

The Pop and Cycle arms go away. The function is exhaustive over the new two-variant enum; no wildcard arm is needed.

- [ ] **Step 2: Remove `placeholder_bodies_show_spec_caption`**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs:1030-1043`, delete the entire test function (and its preceding `// --- Task 27 ---` separator if it now has no following content within that section). After this removal, no test references `ModeChangeStrategy::Previous` or the placeholder caption.

- [ ] **Step 3: Build the GUI crate**

Run: `cargo build -p inputforge-gui-dx --tests`
Expected: the crate compiles. (The `placeholders.rs` file is still in the dispatcher and the placeholder body is still mounted; both go away in Task 11.)

- [ ] **Step 4: Run the gui-dx test suite**

Run: `cargo test -p inputforge-gui-dx`
Expected: all tests pass. The placeholder-bodies test is gone, the F14 body still hits the placeholder in the dispatcher.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git commit -m "refactor(mapping-editor): drop Previous/Cycle arms from format_mode_strategy"
```

### Task 5: Add `is_button_shaped` to `InputId` and `InputAddress`

**Files:**
- Modify: `crates/inputforge-core/src/types/address.rs` (add impl methods + tests)

- [ ] **Step 1: Write the failing tests**

Append to the `mod tests` block in `crates/inputforge-core/src/types/address.rs`:

```rust
#[test]
fn input_id_button_shaped_only_for_button_variant() {
    assert!(InputId::Button { index: 0 }.is_button_shaped());
    assert!(!InputId::Axis { index: 0 }.is_button_shaped());
    assert!(!InputId::Hat { index: 0 }.is_button_shaped());
}

#[test]
fn input_address_button_shaped_for_bound_button_only() {
    let bound_button = InputAddress::Bound {
        device: DeviceId("d".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let bound_axis = InputAddress::Bound {
        device: DeviceId("d".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let bound_hat = InputAddress::Bound {
        device: DeviceId("d".to_owned()),
        input: InputId::Hat { index: 0 },
    };
    let unbound = InputAddress::Unbound;

    assert!(bound_button.is_button_shaped());
    assert!(!bound_axis.is_button_shaped());
    assert!(!bound_hat.is_button_shaped());
    assert!(!unbound.is_button_shaped());
}
```

- [ ] **Step 2: Run the failing tests to confirm they fail with method-not-found**

Run: `cargo test -p inputforge-core types::address::tests::input_id_button_shaped_only_for_button_variant types::address::tests::input_address_button_shaped_for_bound_button_only`
Expected: FAIL, "no method named `is_button_shaped`".

- [ ] **Step 3: Add the impls**

After the existing `impl InputAddress { ... }` block (before the `// Helper structs for serialise.` comment), add:

```rust
impl InputId {
    /// Returns `true` when this input is button-shaped (discrete press/release).
    ///
    /// `Hat` and `Axis` return `false`. The runtime auto-release lifecycle for
    /// [`crate::action::ModeChangeStrategy::Temporary`] (`PopTemporaryMode`)
    /// only fires on real button releases, so this predicate is the gate that
    /// keeps Hold from being authored on inputs that would never auto-revert.
    #[must_use]
    pub const fn is_button_shaped(&self) -> bool {
        matches!(self, Self::Button { .. })
    }
}

impl InputAddress {
    /// Returns `true` when this address points at a button-shaped input.
    ///
    /// `Unbound` returns `false`. See [`InputId::is_button_shaped`].
    #[must_use]
    pub const fn is_button_shaped(&self) -> bool {
        matches!(self, Self::Bound { input, .. } if input.is_button_shaped())
    }
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test -p inputforge-core types::address::tests::input_id_button_shaped_only_for_button_variant types::address::tests::input_address_button_shaped_for_bound_button_only`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-core/src/types/address.rs
git commit -m "feat(core): add is_button_shaped to InputId and InputAddress"
```

### Task 6: Engine grep gate

**Files:** None (verification-only)

- [ ] **Step 1: Run the grep gate**

Run each:

```bash
rg "ModeChangeStrategy::Previous" crates/
rg "ModeChangeStrategy::Cycle" crates/
rg "CycleModes" crates/
rg "go_previous" crates/
rg "fn cycle\b" crates/inputforge-core/src/mode/state.rs
rg 'strategy = "previous"' .
rg 'strategy = "cycle"' .
```

Expected: every command returns zero matches except possibly hits inside `docs/superpowers/specs/2026-05-08-f14-mode-editing-design.md` and inside this plan file (those describe the historical state and are acceptable). If any code or test file matches, return to Tasks 1-4 and finish the removal.

- [ ] **Step 2: Run the full workspace test suite**

Run: `cargo test --workspace`
Expected: all tests pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: zero warnings. Pay attention to dead-code lints if any remain (every removed function should also have its caller removed; if a private helper became dead, drop it).

- [ ] **Step 4: Commit if any clippy fixes were necessary**

```bash
git add <touched files>
git commit -m "chore(core): clean up dead code after Previous/Cycle removal"
```

(Skip if no changes were necessary.)

### Task 7: Verify mode-rename test surfaces the existing helper

**Files:** None (verification-only, but may add a test if missing)

- [ ] **Step 1: Locate the SwitchTo/Temporary rename test**

Run: `rg "rename_mode_refs_rewrites_change_mode_actions|rename_mode_rewrites_switch_to_action|rename_mode_rewrites_temporary_action" crates/inputforge-core/src`
Expected: at least the SwitchTo and Temporary engine integration tests exist (verified at lines 3151 and 3203 of `engine/tests.rs` pre-cut). If the spec's referenced `rename_mode_refs_rewrites_change_mode_actions` test was inside profile/mod.rs and got removed in Task 3 alongside Cycle, replace it with a SwitchTo+Temporary version inline.

- [ ] **Step 2: Run the surviving rename tests**

Run: `cargo test -p inputforge-core -- rename_mode`
Expected: PASS.

(No commit if no source changes were necessary.)

---

## Part B: F2 Select primitive extension

### Task 8: Introduce `SelectOption` and migrate the primitive

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/select.rs`

- [ ] **Step 1: Write the failing tests**

Replace the existing `mod tests` block in `crates/inputforge-gui-dx/src/components/select.rs` (currently containing `select_marks_matching_option_selected` only at `select.rs:75-105`) with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    #[test]
    fn select_marks_matching_option_selected() {
        #[expect(non_snake_case, reason = "Dioxus components are PascalCase by convention")]
        fn Harness() -> Element {
            let value = use_signal(|| "b".to_owned());
            let value_ro: ReadSignal<String> = value.into();
            rsx! {
                Select {
                    value: value_ro,
                    onchange: move |_| {},
                    options: vec![
                        SelectOption { value: "a".into(), label: "A".into(), disabled: false, class: None },
                        SelectOption { value: "b".into(), label: "B".into(), disabled: false, class: None },
                    ],
                }
            }
        }

        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        assert!(
            html.contains(r#"<option value="b" selected=true>B</option>"#),
            "matching option should be selected: {html}"
        );
    }

    #[test]
    fn select_renders_per_option_disabled_and_class() {
        #[expect(non_snake_case, reason = "Dioxus components are PascalCase by convention")]
        fn Harness() -> Element {
            let value = use_signal(|| "live".to_owned());
            let value_ro: ReadSignal<String> = value.into();
            rsx! {
                Select {
                    value: value_ro,
                    onchange: move |_| {},
                    options: vec![
                        SelectOption {
                            value: "live".into(),
                            label: "Combat".into(),
                            disabled: false,
                            class: None,
                        },
                        SelectOption {
                            value: "ghost".into(),
                            label: "ghostly mode".into(),
                            disabled: true,
                            class: Some("if-select__option--orphan".into()),
                        },
                    ],
                }
            }
        }

        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        let ghost_idx = html
            .find(r#"<option value="ghost""#)
            .expect("orphan option must render");
        let ghost_slice = &html[ghost_idx..ghost_idx + 200];
        assert!(
            ghost_slice.contains("disabled=true"),
            "orphan option must carry disabled=true: {ghost_slice}"
        );
        assert!(
            ghost_slice.contains("if-select__option--orphan"),
            "orphan option must carry its class: {ghost_slice}"
        );

        let live_idx = html
            .find(r#"<option value="live""#)
            .expect("non-orphan option must render");
        let live_slice = &html[live_idx..live_idx + 200];
        assert!(
            !live_slice.contains("disabled=true"),
            "non-orphan option must not carry disabled=true: {live_slice}"
        );
    }
}
```

- [ ] **Step 2: Run the failing tests**

Run: `cargo test -p inputforge-gui-dx components::select::tests`
Expected: FAIL on `SelectOption` not found (compile error).

- [ ] **Step 3: Replace the primitive**

Rewrite `crates/inputforge-gui-dx/src/components/select.rs` to:

```rust
use dioxus::prelude::*;

use super::merge_class;
use crate::components::Icon;
use crate::components::text_input::InputSize;
use crate::icons::{Icon as IconKind, IconSize};

/// One option in a [`Select`]. `disabled` and `class` are per-option so a
/// surface (e.g. F14's stage-mode dropdown) can render an orphaned reference
/// as a disabled, error-tinted option without forking the primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
    pub class: Option<String>,
}

#[component]
pub fn Select(
    value: ReadSignal<String>,
    onchange: Option<EventHandler<FormEvent>>,
    options: Vec<SelectOption>,
    #[props(default)] disabled: bool,
    /// HTML `id` for label↔input coupling when wrapped in `Field`.
    #[props(default)]
    id: Option<String>,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-select--sm",
        InputSize::Md => "if-select--md",
        InputSize::Lg => "if-select--lg",
    };
    let combined = merge_class("if-select", size_class, class.as_deref());
    let selected_value = value.read().clone();
    let change_handler = move |evt: FormEvent| {
        if let Some(h) = &onchange {
            h.call(evt);
        }
    };
    rsx! {
        span { class: "if-select-wrapper",
            if let Some(ref id_val) = id {
                select {
                    class: "{combined}",
                    id: "{id_val}",
                    value: "{selected_value}",
                    disabled,
                    onchange: change_handler,
                    for opt in options.iter() {
                        option {
                            value: "{opt.value}",
                            selected: opt.value == selected_value,
                            disabled: opt.disabled,
                            class: opt.class.clone().unwrap_or_default(),
                            "{opt.label}"
                        }
                    }
                }
            } else {
                select {
                    class: "{combined}",
                    value: "{selected_value}",
                    disabled,
                    onchange: change_handler,
                    for opt in options.iter() {
                        option {
                            value: "{opt.value}",
                            selected: opt.value == selected_value,
                            disabled: opt.disabled,
                            class: opt.class.clone().unwrap_or_default(),
                            "{opt.label}"
                        }
                    }
                }
            }
            Icon {
                name: IconKind::ChevronDown,
                size: IconSize::Sm,
                class: "if-select-wrapper__chevron".to_owned(),
            }
        }
    }
}
```

- [ ] **Step 4: Re-export `SelectOption`**

In `crates/inputforge-gui-dx/src/components/mod.rs`, change `pub use select::Select;` to `pub use select::{Select, SelectOption};`.

- [ ] **Step 5: Build the crate (call sites are still on the old shape, expect failures)**

Run: `cargo build -p inputforge-gui-dx --tests`
Expected: errors at every call site listed in Task 9 (option-tuple no longer matches `Vec<SelectOption>`). The `select.rs` test file itself compiles.

- [ ] **Step 6: Do not commit yet**

The build is broken until Task 9 migrates every call site. Tasks 8 and 9 ship in one commit.

### Task 9: Migrate every `Select` call site

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs:23, 361, 369, 377, 620` (and the option-builder helpers above each)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs:55, 931`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs:53, 363-369, 385`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_vjoy.rs:38, 148-157, 173-202, 360, 368`

- [ ] **Step 1: Add the import at every call site**

In each file modified by this task, add `SelectOption` next to the existing `Select` import:

- `bulk_map/mod.rs:23`: `use crate::components::{Button, ButtonVariant, Checkbox, Field, Select, SelectOption};`
- `predicate.rs:55`: `use crate::components::{NumberInput, Select, SelectOption};`
- `merge_axis.rs:53`: `use crate::components::{Select, SelectOption};`
- `map_to_vjoy.rs:38`: `use crate::components::{Select, SelectOption};`

- [ ] **Step 2: Migrate every `Vec<(String, String)>` option builder**

For each call site, rewrite the option builder. The mechanical transform is:

```rust
// Before:
let foo_options: Vec<(String, String)> = items
    .iter()
    .map(|x| (x.value(), x.label()))
    .collect();

// After:
let foo_options: Vec<SelectOption> = items
    .iter()
    .map(|x| SelectOption {
        value: x.value(),
        label: x.label(),
        disabled: false,
        class: None,
    })
    .collect();
```

Apply to:

- `merge_axis.rs:363-369` (`op_options`).
- `map_to_vjoy.rs:148-157` (`device_options`) and `:173-202` (`output_options`, including the fallback branch).
- `predicate.rs:436` (`kind_options: Vec<(String, String)> = vec![...]`) and the `Select` call site at `:931`.
- `bulk_map/mod.rs:44-57` (`pub(crate) fn build_source_options(...) -> Vec<(String, String)>`), `:213` (call), `:214` (inline `target_options` builder), `:230` (inline `mode_options` builder), `:585` (per-row call to `build_target_options`), `:977` (`fn build_target_options(...) -> Vec<(String, String)>`).

- [ ] **Step 3: Build to confirm green**

Run: `cargo build -p inputforge-gui-dx --tests`
Expected: zero errors.

- [ ] **Step 4: Run the GUI test suite**

Run: `cargo test -p inputforge-gui-dx`
Expected: all tests pass, including the new `select_renders_per_option_disabled_and_class` test from Task 8.

- [ ] **Step 5: Commit Tasks 8 and 9 together**

```bash
git add crates/inputforge-gui-dx/src/components/select.rs \
        crates/inputforge-gui-dx/src/components/mod.rs \
        crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_vjoy.rs
git commit -m "refactor(components): extend Select with per-option disabled and class"
```

---

## Part C: ChangeMode body and dispatcher rewire

### Task 10: Add the CSS for the change-mode body and pill row

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css:281-290` and append a new pill-row block

- [ ] **Step 1: Add `.if-stage__body-change-mode` to the existing 2-column grid rule**

In `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`, change lines 281-290 from:

```css
.if-stage__body-merge-axis,
.if-stage__body-vjoy,
.if-stage__body-keyboard {
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 16px;
    row-gap: 8px;
    align-items: center;
    justify-items: start;
}
```

to:

```css
.if-stage__body-merge-axis,
.if-stage__body-vjoy,
.if-stage__body-keyboard,
.if-stage__body-change-mode {
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 16px;
    row-gap: 8px;
    align-items: center;
    justify-items: start;
}
```

- [ ] **Step 2: Append the pill-row block**

After the `.if-stage__body-label { ... }` rule (which ends around line 300), append:

```css
/* F14 ChangeMode body. Two-pill segmented row used by the strategy
   field. Active pill carries the control-violet badge tint pattern;
   inactive pills are muted text on a hairline border. The row sits on
   bg-elevated; active-pill contrast tested per DESIGN.md §2.
   Promotion to a shared primitive is intentionally deferred until a
   third stage variant needs the same shape. */

.if-stage__body-strategy {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-full);
    background: var(--color-bg-elevated);
}

.if-stage__body-strategy-pill {
    display: inline-flex;
    align-items: center;
    height: 24px;
    padding: 0 12px;
    border-radius: var(--radius-full);
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-text-muted);
    font-family: var(--font-sans);
    font-size: 13px;
    cursor: pointer;
    transition: none; /* prefers-reduced-motion: instant swap */
}

.if-stage__body-strategy-pill:hover:not([aria-disabled="true"]) {
    color: var(--color-text);
}

.if-stage__body-strategy-pill[aria-pressed="true"] {
    color: var(--color-control-badge-text);
    background: var(--color-control-bg);
    border-color: var(--color-control);
}

.if-stage__body-strategy-pill[aria-disabled="true"] {
    opacity: 0.5;
    cursor: not-allowed;
}

/* Selected-but-disabled (Hold persisted then primary rebound to non-button).
   Per DESIGN.md §7 Tabs disabled-active rule: keep the tint but desaturate. */
.if-stage__body-strategy-pill[aria-pressed="true"][aria-disabled="true"] {
    color: var(--color-control-badge-text);
    background: var(--color-control-bg);
    border-color: var(--color-control);
    opacity: 0.5;
}

/* Orphaned mode option in the target Select dropdown (hint priority 2).
   Uses --color-error-hover (the brighter ramp step) to match the badge
   precedent at assets/components/badge.css:20; the canonical
   --color-error fails AA on the bg-elevated/UA-owned <option> surface. */
.if-select__option--orphan {
    color: var(--color-error-hover);
    font-style: italic;
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "style(mapping-editor): add change-mode body + strategy pill row"
```

### Task 11: Scaffold the `change_mode` body module and wire it into the dispatcher

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs:25, 115, 137-138`
- Delete: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/placeholders.rs`

- [ ] **Step 1: Create a minimal stub module**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs` with:

```rust
// Rust guideline compliant 2026-05-08

//! `ChangeMode` body. Renders a two-row form: strategy picker
//! (segmented Set/Hold pills) and target-mode `Select`. F14 owner.
//!
//! Hint priority (highest first):
//! 1. Empty target mode -> `"Choose a target mode"`.
//! 2. Target mode not in `MetaSnapshot.modes` -> orphan option + drift hint.
//! 3. Hold strategy with non-button primary -> selected-but-disabled Hold.
//! When (2) and (3) hold simultaneously the body emits a combined hint
//! so the user can recover both errors in one edit pass.

use dioxus::prelude::*;

use inputforge_core::action::{Action, ModeChangeStrategy};

use crate::components::{Select, SelectOption, Tooltip, TooltipPlacement};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::dispatch_stage_edit;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

/// Hint copy. Centralised so tests can grep these strings unchanged.
pub(crate) const HINT_TARGET_EMPTY: &str = "Choose a target mode";
pub(crate) const HINT_HOLD_NOT_BUTTON: &str =
    "Hold requires a button input. Pick a button or change the strategy.";
pub(crate) const TOOLTIP_HOLD_NOT_BUTTON: &str = "Hold requires a button input.";

/// Set / Hold pill activation gate. Returns `false` when the pill is
/// `aria-disabled` or already in the active state. Both onclick handlers
/// call this; standalone-testable so acceptance #15 (Enter on aria-disabled
/// is a no-op) can be unit-verified without DOM event simulation.
pub(crate) fn pill_activates(disabled: bool, was_active: bool) -> bool {
    !disabled && !was_active
}

#[component]
pub(crate) fn ChangeModeBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    /// Current strategy (destructured from `Action::ChangeMode { strategy }`
    /// in the dispatcher).
    strategy: ModeChangeStrategy,
    root_actions: Vec<Action>,
) -> Element {
    rsx! { div { class: "if-stage__body-change-mode", "F14 body wired" } }
}
```

- [ ] **Step 2: Wire the dispatcher**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`:

- Line 25, replace `mod placeholders;` with `mod change_mode;`.
- Line 115, replace the `Action::ChangeMode { .. }` arm with:

```rust
Action::ChangeMode { strategy } => rsx! {
    change_mode::ChangeModeBody {
        mapping_key: mapping_key.clone(),
        stage_id: stage_id.clone(),
        strategy: strategy.clone(),
        root_actions: root_actions.clone(),
    }
},
```

- Line 137-138, in `header_right_slot`, remove the `Action::ChangeMode { .. } => default_chevron(expanded),` arm so the `_ => default_chevron(expanded)` catch-all handles it. Update the `#[allow(clippy::match_same_arms, ...)]` reason text to drop the F14 mention if it still references "F14".

Sequencing note: the engine cut (Tasks 1-3) lands first so the dispatcher's `strategy` destructure binds only `SwitchTo` / `Temporary`. Task 4's formatter trim then becomes safe (no Pop / Cycle arms can flow through), and finally Task 11 mounts the new body. Every commit boundary keeps `cargo build -p inputforge-core --tests` and `cargo build -p inputforge-gui-dx --tests` green.

- [ ] **Step 3: Delete the placeholder module**

Run: `git rm crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/placeholders.rs`

(Use `git rm` directly so the deletion is staged in one shot. The Step 5 commit block then references only the two modified files.)

- [ ] **Step 4: Build to confirm the stub mounts**

Run: `cargo build -p inputforge-gui-dx --tests`
Expected: green. The stub renders a single `div` with the new class; functionality lands in subsequent tasks.

- [ ] **Step 5: Commit the scaffold**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs
git commit -m "feat(mapping-editor): scaffold ChangeModeBody and remove placeholder"
```

(`placeholders.rs` is already staged for removal by the `git rm` in Step 3.)

### Task 12: Render the strategy pill row + target Select (no edit dispatch yet)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs`

- [ ] **Step 1: Write the failing tests**

Append a `#[cfg(test)] mod tests` block to `change_mode.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    use crate::frame::mapping_editor::pipeline::tests::render_change_mode_body_for_test;

    #[test]
    fn renders_strategy_pills_and_target_select_for_switch_to() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "Combat".to_owned(),
        };
        let (html, _hints) =
            render_change_mode_body_for_test(strategy, "btn0", &["Default", "Combat"]);

        assert!(html.contains("if-stage__body-change-mode"));
        assert!(html.contains("if-stage__body-strategy"), "pill row missing: {html}");
        assert!(
            html.contains("data-strategy=\"set\""),
            "Set pill missing: {html}"
        );
        assert!(
            html.contains("data-strategy=\"hold\""),
            "Hold pill missing: {html}"
        );
        assert!(
            html.contains("aria-pressed=\"true\""),
            "active pill must carry aria-pressed=true: {html}"
        );
    }

    #[test]
    fn renders_temporary_with_hold_pill_pressed() {
        let strategy = ModeChangeStrategy::Temporary {
            mode: "Combat".to_owned(),
        };
        let (html, _hints) =
            render_change_mode_body_for_test(strategy, "btn0", &["Default", "Combat"]);
        // Find the hold pill region and assert aria-pressed=true is on it.
        let hold_idx = html
            .find(r#"data-strategy="hold""#)
            .expect("hold pill must render");
        let after = &html[hold_idx..hold_idx + 200];
        assert!(
            after.contains("aria-pressed=\"true\""),
            "hold pill must be pressed when strategy is Temporary, fragment: {after}"
        );
    }
}
```

The helper `render_change_mode_body_for_test` must be added to `pipeline/tests.rs` (next step).

- [ ] **Step 2: Extend `HarnessProps` and `HarnessComponent` with a `meta_modes` field**

The existing harness at `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs:340-466` hard-codes `MetaSnapshot.modes = vec!["Default".to_owned()]`. F14 tests need to seed multiple modes. Extend `HarnessProps` with a defaulted field so existing call sites do not need to be touched:

```rust
#[props(default = vec!["Default".to_owned()])]
meta_modes: Vec<String>,
```

In `HarnessComponent`, replace the `let meta = use_signal(...)` block with:

```rust
let meta_modes_value = props.meta_modes.clone();
let meta = use_signal(|| MetaSnapshot {
    engine_status: EngineStatus::Running,
    profile_name: Some("P".to_owned()),
    modes: meta_modes_value,
    startup_mode: Some("Default".to_owned()),
    current_mode: "Default".to_owned(),
    ..MetaSnapshot::default()
});
```

(Capture the field value before the destructuring so the closure can move it. Adjust the `PartialEq` impl in the same step if `HarnessProps` derives it manually.)

- [ ] **Step 3: Add the F14 test helpers**

Append to `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs`:

```rust
/// Construct an `InputAddress` from the F14 test sigil:
///   - "btn{N}"   -> Bound + Button { index: N }
///   - "axis{N}"  -> Bound + Axis { index: N }
///   - "hat{N}"   -> Bound + Hat { index: N }
///   - "unbound"  -> InputAddress::Unbound
fn parse_primary_for_test(spec: &str) -> InputAddress {
    use inputforge_core::types::{DeviceId, InputId};
    match spec {
        "unbound" => InputAddress::Unbound,
        s if s.starts_with("btn") => InputAddress::Bound {
            device: DeviceId("d".to_owned()),
            input: InputId::Button {
                index: s[3..].parse().unwrap_or(0),
            },
        },
        s if s.starts_with("axis") => InputAddress::Bound {
            device: DeviceId("d".to_owned()),
            input: InputId::Axis {
                index: s[4..].parse().unwrap_or(0),
            },
        },
        s if s.starts_with("hat") => InputAddress::Bound {
            device: DeviceId("d".to_owned()),
            input: InputId::Hat {
                index: s[3..].parse().unwrap_or(0),
            },
        },
        _ => panic!("unknown primary spec: {spec}"),
    }
}

/// SSR-render a single-stage `ChangeMode` mapping. Stage 0 is pre-expanded
/// and the harness runs the settled-render path so the body's render-phase
/// hint write propagates back to the parent before HTML capture. Returns
/// (rendered HTML, malformed-hints snapshot) so tests can observe both
/// without a thread-local exporter (which races under parallel `cargo
/// test` worker reuse).
pub(crate) fn render_change_mode_body_for_test(
    strategy: inputforge_core::action::ModeChangeStrategy,
    primary: &str,
    modes: &[&str],
) -> (String, HashMap<StageId, String>) {
    let addr = parse_primary_for_test(primary);
    let modes_owned: Vec<String> = modes.iter().map(|s| (*s).to_owned()).collect();
    let actions = vec![Action::ChangeMode {
        strategy,
    }];
    let (state, _addr_back) = build_state_with_mapping(actions, addr.clone());

    let hints_capture: Arc<RwLock<HashMap<StageId, String>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let mut vdom = VirtualDom::new_with_props(
        HarnessComponent,
        HarnessProps {
            state: Arc::new(RwLock::new(state)),
            addr,
            pre_expanded_stages: vec![StageId(vec![StageIdSegment::Index(0)])],
            virtual_devices: vec![],
            pre_stage_menu: None,
            pre_malformed_hints: HashMap::new(),
            meta_modes: modes_owned,
            hints_capture: Some(hints_capture.clone()),
        },
    );
    vdom.rebuild_in_place();
    vdom.render_immediate(&mut dioxus::core::NoOpMutations);
    let html = render(&vdom);
    let hints = hints_capture.read().expect("hints lock").clone();
    (html, hints)
}
```

`HarnessProps` gains a corresponding optional field so non-F14 callers ignore the capture:

```rust
#[props(default)]
hints_capture: Option<Arc<RwLock<HashMap<StageId, String>>>>,
```

In `HarnessComponent`, after the body has had a chance to write hints (place this immediately before the final `rsx! { MappingEditor {} }`), forward `editor.malformed_hints.read().clone()` into the capture if one was provided:

```rust
if let Some(capture) = props.hints_capture.as_ref() {
    *capture.write().expect("hints lock") = editor.malformed_hints.read().clone();
}
```

Tests then read the snapshot directly from the second tuple element returned by `render_change_mode_body_for_test`. No thread-local; no race under parallel test execution.

`build_state_with_mapping` is a small variant of the existing `build_state` that accepts a custom `InputAddress` instead of always using `axis(0)`. Locate `build_state` (search `fn build_state` in the same file) and clone it into a parameterised version that takes `(actions, addr)`.

(Helpers `simulate_dispatch_strategy_change` and `simulate_dispatch_target_change` follow in Task 13 and Task 14; both call the closure-extracted free functions directly with constructed args, no DOM event simulation.)

- [ ] **Step 4: Run the failing tests**

Run: `cargo test -p inputforge-gui-dx change_mode`
Expected: FAIL on the missing pill markup; the helpers themselves should compile.

- [ ] **Step 5: Implement the body markup**

Replace the `ChangeModeBody` body in `change_mode.rs` with:

```rust
let mode = match &strategy {
    ModeChangeStrategy::SwitchTo { mode } | ModeChangeStrategy::Temporary { mode } => mode.clone(),
};

let is_hold = matches!(strategy, ModeChangeStrategy::Temporary { .. });
let is_set = !is_hold;

let primary_is_button_shaped = mapping_key.1.is_button_shaped();
let hold_disabled = !primary_is_button_shaped;

let set_aria_pressed = if is_set { "true" } else { "false" };
let hold_aria_pressed = if is_hold { "true" } else { "false" };
let hold_aria_disabled = if hold_disabled { "true" } else { "false" };

let ctx = use_context::<AppContext>();
let cfg = ctx.config.read();
let modes: Vec<String> = ctx.meta.read().modes.clone();

let target_options: Vec<SelectOption> = modes
    .iter()
    .map(|m| SelectOption {
        value: m.clone(),
        label: m.clone(),
        disabled: false,
        class: None,
    })
    .collect();

// Mode value Signal (sync prop -> signal each render so the dropdown
// follows snapshot echoes; same pattern as MapToVJoyBody).
let mode_for_signal = mode.clone();
let mut target_value: Signal<String> = use_signal(|| mode_for_signal.clone());
if *target_value.peek() != mode_for_signal {
    target_value.set(mode_for_signal.clone());
}

let hold_pill = rsx! {
    button {
        r#type: "button",
        class: "if-stage__body-strategy-pill",
        "data-strategy": "hold",
        "aria-pressed": "{hold_aria_pressed}",
        "aria-disabled": "{hold_aria_disabled}",
        // onclick wired in Task 13
        "Hold"
    }
};

rsx! {
    div { class: "if-stage__body-change-mode",
        div { class: "if-stage__body-field",
            label { class: "if-stage__body-label", "Strategy" }
            // Toggle-button-group pattern (role="group" + child aria-pressed).
            // Matches existing aria-pressed usage on <button> at
            // top_bar/engine_pill/mod.rs:77, panel_slot/device_panel.rs:92,
            // frame/bulk_map/mod.rs:636. Avoids the radiogroup/aria-pressed
            // mismatch screen readers degrade unpredictably on.
            div {
                class: "if-stage__body-strategy",
                role: "group",
                "aria-label": "Mode change strategy",
                button {
                    r#type: "button",
                    class: "if-stage__body-strategy-pill",
                    "data-strategy": "set",
                    "aria-pressed": "{set_aria_pressed}",
                    "Set"
                }
                if hold_disabled {
                    Tooltip {
                        content: TOOLTIP_HOLD_NOT_BUTTON.to_owned(),
                        placement: TooltipPlacement::Top,
                        {hold_pill}
                    }
                } else {
                    {hold_pill}
                }
            }
        }
        div { class: "if-stage__body-field",
            label { class: "if-stage__body-label", "Target mode" }
            Select {
                value: target_value,
                options: target_options,
                onchange: move |_evt: FormEvent| {},
            }
        }
    }
}
```

(`AppContext::meta` is the `Signal<MetaSnapshot>` that holds the modes list; see `bulk_map/mod.rs:119` for the canonical `ctx.meta.read().modes.clone()` access.)

The Tooltip wrapping happens only when `hold_disabled`, otherwise the pill renders bare so a screen reader does not announce a tooltip target on the enabled pill.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p inputforge-gui-dx change_mode`
Expected: PASS. Iterate on harness setup until the helper renders successfully.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git commit -m "feat(change-mode): render strategy pills and target Select"
```

### Task 13: Wire the Set/Hold strategy edit through `dispatch_stage_edit`

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs`

- [ ] **Step 1: Write the failing test**

Append to the `mod tests` block in `change_mode.rs`:

```rust
#[test]
fn dispatches_strategy_switch_with_target_preserved() {
    use inputforge_core::action::Action;
    use inputforge_core::engine::EngineCommand;

    let strategy_before = ModeChangeStrategy::SwitchTo {
        mode: "Combat".to_owned(),
    };

    // Direct call into the closure-extracted helper. No DOM event simulation.
    let (commands, undo_label) =
        crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_strategy_change(
            strategy_before,
            "btn0",
            &["Default", "Combat"],
            StrategyTarget::Hold,
        );

    let first = commands.into_iter().next().expect("expected SetMapping");
    match first {
        EngineCommand::SetMapping { actions, .. } => {
            assert!(matches!(
                actions.first(),
                Some(Action::ChangeMode {
                    strategy: ModeChangeStrategy::Temporary { mode }
                }) if mode == "Combat"
            ), "target must be preserved across strategy switch: {actions:?}");
        }
        other => panic!("expected SetMapping, got {other:?}"),
    }
    assert_eq!(undo_label.as_deref(), Some("Change mode: strategy Set -> Hold"));
}
```

`simulate_dispatch_strategy_change` is a thin wrapper in `pipeline/tests.rs` that builds the args (mpsc channel for `cmd_tx`, fresh `Signal<UndoLog>`, modes seed), calls `dispatch_strategy_change` directly, and returns `(commands_drained_from_rx, undo_log_label_or_none)`. No `VirtualDom`, no event injection.

- [ ] **Step 2: Run the failing test**

Run: `cargo test -p inputforge-gui-dx dispatches_strategy_switch_with_target_preserved`
Expected: FAIL (`dispatch_strategy_change` and `simulate_dispatch_strategy_change` do not exist yet).

- [ ] **Step 3: Extract the dispatch into a free function and write thin closures**

Add to `change_mode.rs`:

```rust
/// Which pill the user clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StrategyTarget {
    Set,
    Hold,
}

/// Dispatch a Set/Hold strategy change. Caller passes the same `current_mode`
/// across both targets, so target preservation across Set <-> Hold falls out
/// for free. Returns the formatted undo label that was committed (`None`
/// when the dispatch was skipped, e.g. same-state click).
pub(crate) fn dispatch_strategy_change(
    target: StrategyTarget,
    current_mode: &str,
    is_currently_hold: bool,
    hold_disabled: bool,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    mapping_names: &HashMap<InputAddress, String>,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<UndoLog>,
) -> Option<String> {
    let was_active = match target {
        StrategyTarget::Set => !is_currently_hold,
        StrategyTarget::Hold => is_currently_hold,
    };
    let target_disabled = matches!(target, StrategyTarget::Hold) && hold_disabled;
    if !pill_activates(target_disabled, was_active) {
        return None;
    }

    let new_strategy = match target {
        StrategyTarget::Set => ModeChangeStrategy::SwitchTo { mode: current_mode.to_owned() },
        StrategyTarget::Hold => ModeChangeStrategy::Temporary { mode: current_mode.to_owned() },
    };
    let new_action = Action::ChangeMode { strategy: new_strategy };
    let (before, after) = match target {
        StrategyTarget::Set => ("Hold", "Set"),
        StrategyTarget::Hold => ("Set", "Hold"),
    };
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Change mode"),
            field: Some("strategy"),
            before_after: Some((before, after)),
            ..LabelArgs::default()
        },
    );
    let name = mapping_names.get(&mapping_key.1).cloned();
    dispatch_stage_edit(
        root_actions,
        stage_id,
        new_action,
        mapping_key,
        name,
        cmd_tx,
        undo_log,
        label.clone(),
    );
    Some(label)
}
```

The two onclick closures collapse to thin wrappers:

```rust
let editor = use_context::<EditorState>();
let cmd_tx = ctx.commands.clone();
let cfg_signal = ctx.config;
let mut undo_log = editor.undo_log;

let mapping_key_set = mapping_key.clone();
let stage_id_set = stage_id.clone();
let root_actions_set = root_actions.clone();
let mode_set = mode.clone();
let on_set_click = move |_evt: MouseEvent| {
    let cfg = cfg_signal.read();
    let names = cfg.mapping_names.clone();
    drop(cfg);
    let _ = dispatch_strategy_change(
        StrategyTarget::Set,
        &mode_set,
        is_hold,
        false, // Set pill is never gated by the button-shape rule
        &mapping_key_set,
        &stage_id_set,
        &root_actions_set,
        &names,
        &cmd_tx,
        &mut undo_log,
    );
};

let mapping_key_hold = mapping_key.clone();
let stage_id_hold = stage_id.clone();
let root_actions_hold = root_actions.clone();
let mode_hold = mode.clone();
let cmd_tx_hold = ctx.commands.clone();
let mut undo_log_hold = editor.undo_log;
let cfg_signal_hold = ctx.config;
let on_hold_click = move |_evt: MouseEvent| {
    let cfg = cfg_signal_hold.read();
    let names = cfg.mapping_names.clone();
    drop(cfg);
    let _ = dispatch_strategy_change(
        StrategyTarget::Hold,
        &mode_hold,
        is_hold,
        hold_disabled,
        &mapping_key_hold,
        &stage_id_hold,
        &root_actions_hold,
        &names,
        &cmd_tx_hold,
        &mut undo_log_hold,
    );
};
```

Bind `onclick: on_set_click` and `onclick: on_hold_click` on the respective pills.

The migration-from-Hold-to-Set path documented in spec acceptance #11 falls out of the same code path: when the user clicks Set while `is_hold == true`, `was_active` is `false` for the Set target, `pill_activates(false, false)` returns `true`, and dispatch fires with the target preserved.

- [ ] **Step 4: Add `simulate_dispatch_strategy_change` to `pipeline/tests.rs`**

```rust
pub(crate) fn simulate_dispatch_strategy_change(
    strategy_before: ModeChangeStrategy,
    primary: &str,
    modes: &[&str],
    target: StrategyTarget,
) -> (Vec<EngineCommand>, Option<String>) {
    use std::sync::mpsc;
    let addr = parse_primary_for_test(primary);
    let mode_before = match &strategy_before {
        ModeChangeStrategy::SwitchTo { mode } | ModeChangeStrategy::Temporary { mode } => mode.clone(),
    };
    let is_hold = matches!(strategy_before, ModeChangeStrategy::Temporary { .. });
    let hold_disabled = !addr.is_button_shaped();
    let mapping_key: MappingKey = ("test".to_owned(), addr.clone());
    let stage_id = StageId(vec![StageIdSegment::Index(0)]);
    let root_actions = vec![Action::ChangeMode { strategy: strategy_before }];
    let names: HashMap<InputAddress, String> = HashMap::new();
    let _ = modes; // currently unused; kept in signature for symmetry with render helper

    let (tx, rx) = mpsc::channel::<EngineCommand>();
    let mut undo_log = use_signal_for_test(UndoLog::default()); // existing test helper
    let label = dispatch_strategy_change(
        target,
        &mode_before,
        is_hold,
        hold_disabled,
        &mapping_key,
        &stage_id,
        &root_actions,
        &names,
        &tx,
        &mut undo_log,
    );
    drop(tx);
    let commands: Vec<EngineCommand> = rx.try_iter().collect();
    (commands, label)
}
```

If `use_signal_for_test` does not already exist in `pipeline/tests.rs`, add a thin shim that constructs a `Signal` outside a Dioxus scope using `Signal::new` (or whatever Dioxus 0.7 exposes for non-component Signal construction).

- [ ] **Step 5: Run the tests**

Run: `cargo test -p inputforge-gui-dx change_mode`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git commit -m "feat(change-mode): dispatch SetMapping on strategy pill toggle"
```

### Task 14: Wire the target-mode Select onchange and undo label

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs`

- [ ] **Step 1: Write the failing tests**

Append to `change_mode.rs` `mod tests`:

```rust
#[test]
fn dispatches_target_change_with_unset_before_label() {
    use inputforge_core::action::Action;
    use inputforge_core::engine::EngineCommand;

    let strategy_before = ModeChangeStrategy::SwitchTo {
        mode: String::new(), // empty -> "<unset>"
    };
    let (commands, undo_label) =
        crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_target_change(
            strategy_before,
            "btn0",
            &["Default", "Combat"],
            "Combat",
        );
    let first = commands.into_iter().next().expect("expected SetMapping");
    match first {
        EngineCommand::SetMapping { actions, .. } => {
            assert!(matches!(
                actions.first(),
                Some(Action::ChangeMode {
                    strategy: ModeChangeStrategy::SwitchTo { mode }
                }) if mode == "Combat"
            ));
        }
        other => panic!("expected SetMapping, got {other:?}"),
    }
    assert_eq!(undo_label.as_deref(), Some("Change mode: target <unset> -> Combat"));
}

#[test]
fn dispatches_target_change_with_explicit_before_label() {
    let strategy_before = ModeChangeStrategy::SwitchTo {
        mode: "Default".to_owned(),
    };
    let (_commands, undo_label) =
        crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_target_change(
            strategy_before,
            "btn0",
            &["Default", "Combat"],
            "Combat",
        );
    assert_eq!(undo_label.as_deref(), Some("Change mode: target Default -> Combat"));
}
```

- [ ] **Step 2: Run the failing tests**

Run: `cargo test -p inputforge-gui-dx change_mode::tests::dispatches_target_change`
Expected: FAIL.

- [ ] **Step 3: Extract `dispatch_target_change` and write a thin closure**

Add to `change_mode.rs`:

```rust
/// Dispatch a target-mode change. Returns the formatted undo label that
/// was committed (`None` when the new mode equals the current mode).
pub(crate) fn dispatch_target_change(
    new_mode: &str,
    current_strategy: &ModeChangeStrategy,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    mapping_names: &HashMap<InputAddress, String>,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<UndoLog>,
) -> Option<String> {
    let mode_before = match current_strategy {
        ModeChangeStrategy::SwitchTo { mode } | ModeChangeStrategy::Temporary { mode } => mode.as_str(),
    };
    if new_mode == mode_before {
        return None;
    }
    let new_strategy = match current_strategy {
        ModeChangeStrategy::SwitchTo { .. } => ModeChangeStrategy::SwitchTo { mode: new_mode.to_owned() },
        ModeChangeStrategy::Temporary { .. } => ModeChangeStrategy::Temporary { mode: new_mode.to_owned() },
    };
    let new_action = Action::ChangeMode { strategy: new_strategy };
    let before_label = if mode_before.is_empty() { "<unset>" } else { mode_before };
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Change mode"),
            field: Some("target"),
            before_after: Some((before_label, new_mode)),
            ..LabelArgs::default()
        },
    );
    let name = mapping_names.get(&mapping_key.1).cloned();
    dispatch_stage_edit(
        root_actions,
        stage_id,
        new_action,
        mapping_key,
        name,
        cmd_tx,
        undo_log,
        label.clone(),
    );
    Some(label)
}
```

The Select onchange becomes a thin wrapper that pulls `evt.value()` and forwards:

```rust
let mapping_key_t = mapping_key.clone();
let stage_id_t = stage_id.clone();
let root_actions_t = root_actions.clone();
let strategy_for_t = strategy.clone();
let cmd_tx_t = ctx.commands.clone();
let mut undo_log_t = editor.undo_log;
let cfg_for_t = ctx.config;
let on_target_change = move |evt: FormEvent| {
    let cfg = cfg_for_t.read();
    let names = cfg.mapping_names.clone();
    drop(cfg);
    let _ = dispatch_target_change(
        &evt.value(),
        &strategy_for_t,
        &mapping_key_t,
        &stage_id_t,
        &root_actions_t,
        &names,
        &cmd_tx_t,
        &mut undo_log_t,
    );
};
```

Pass `onchange: on_target_change` to the `Select`.

- [ ] **Step 4: Add `simulate_dispatch_target_change` to `pipeline/tests.rs`**

```rust
pub(crate) fn simulate_dispatch_target_change(
    strategy_before: ModeChangeStrategy,
    primary: &str,
    modes: &[&str],
    new_mode: &str,
) -> (Vec<EngineCommand>, Option<String>) {
    use std::sync::mpsc;
    let addr = parse_primary_for_test(primary);
    let mapping_key: MappingKey = ("test".to_owned(), addr.clone());
    let stage_id = StageId(vec![StageIdSegment::Index(0)]);
    let root_actions = vec![Action::ChangeMode { strategy: strategy_before.clone() }];
    let names: HashMap<InputAddress, String> = HashMap::new();
    let _ = modes; // accepted for symmetry with render helper

    let (tx, rx) = mpsc::channel::<EngineCommand>();
    let mut undo_log = use_signal_for_test(UndoLog::default());
    let label = dispatch_target_change(
        new_mode,
        &strategy_before,
        &mapping_key,
        &stage_id,
        &root_actions,
        &names,
        &tx,
        &mut undo_log,
    );
    drop(tx);
    let commands: Vec<EngineCommand> = rx.try_iter().collect();
    (commands, label)
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p inputforge-gui-dx change_mode`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git commit -m "feat(change-mode): dispatch SetMapping on target-mode pick"
```

### Task 15: Implement the three malformed-hint priorities and the orphan option

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `mod tests` block:

```rust
fn hint_for_stage_zero(hints: &HashMap<StageId, String>) -> Option<&str> {
    hints.get(&StageId(vec![StageIdSegment::Index(0)])).map(String::as_str)
}

#[test]
fn priority_1_hint_when_target_empty() {
    let strategy = ModeChangeStrategy::SwitchTo { mode: String::new() };
    let (_html, hints) =
        crate::frame::mapping_editor::pipeline::tests::render_change_mode_body_for_test(
            strategy,
            "btn0",
            &["Default", "Combat"],
        );
    assert_eq!(hint_for_stage_zero(&hints), Some(HINT_TARGET_EMPTY));
}

#[test]
fn priority_2_hint_when_target_not_in_modes() {
    let strategy = ModeChangeStrategy::SwitchTo {
        mode: "Ghost".to_owned(),
    };
    let (html, hints) =
        crate::frame::mapping_editor::pipeline::tests::render_change_mode_body_for_test(
            strategy,
            "btn0",
            &["Default", "Combat"],
        );
    assert_eq!(
        hint_for_stage_zero(&hints),
        Some(r#"Mode "Ghost" is not in this profile. Pick a current mode."#)
    );
    let ghost_idx = html
        .find(r#"<option value="Ghost""#)
        .expect("orphan option must render");
    let ghost_slice = &html[ghost_idx..ghost_idx + 200];
    assert!(
        ghost_slice.contains("disabled=true"),
        "orphan option must carry disabled=true: {ghost_slice}"
    );
    assert!(
        ghost_slice.contains("if-select__option--orphan"),
        "orphan option must carry the orphan class: {ghost_slice}"
    );
}

#[test]
fn priority_3_hint_when_hold_on_non_button() {
    let strategy = ModeChangeStrategy::Temporary {
        mode: "Combat".to_owned(),
    };
    let (html, hints) =
        crate::frame::mapping_editor::pipeline::tests::render_change_mode_body_for_test(
            strategy,
            "axis0",
            &["Default", "Combat"],
        );
    assert_eq!(hint_for_stage_zero(&hints), Some(HINT_HOLD_NOT_BUTTON));
    let hold_idx = html
        .find(r#"data-strategy="hold""#)
        .expect("hold pill must render");
    let after = &html[hold_idx..hold_idx + 200];
    assert!(after.contains(r#"aria-pressed="true""#));
    assert!(after.contains(r#"aria-disabled="true""#));
}

#[test]
fn combined_hint_when_orphan_and_hold_disabled() {
    let strategy = ModeChangeStrategy::Temporary { mode: "Ghost".to_owned() };
    let (_html, hints) =
        crate::frame::mapping_editor::pipeline::tests::render_change_mode_body_for_test(
            strategy,
            "axis0",
            &["Default", "Combat"],
        );
    let hint = hint_for_stage_zero(&hints).expect("hint must surface");
    assert!(
        hint.contains("not in this profile") && hint.contains("button input"),
        "combined hint must mention both error conditions: {hint}"
    );
}

#[test]
fn priority_3_disabled_hold_pill_click_is_noop() {
    // The Hold pill click handler must short-circuit when the primary input
    // is non-button-shaped (acceptance item #9: aria-disabled pill must not
    // commit on click). The `pill_activates` gate carries the contract;
    // this test exercises it through `dispatch_strategy_change` so a future
    // refactor that removes the gate fails loudly.
    let strategy = ModeChangeStrategy::SwitchTo {
        mode: "Combat".to_owned(),
    };
    let (commands, _label) =
        crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_strategy_change(
            strategy,
            "axis0",
            &["Default", "Combat"],
            StrategyTarget::Hold,
        );
    assert!(
        commands.is_empty(),
        "click on aria-disabled Hold pill must not commit, got {commands:?}"
    );
}

#[test]
fn pill_activates_gate_blocks_disabled_and_active() {
    // Direct unit test of the keyboard-and-mouse activation gate. Acceptance
    // item #15 (Enter on aria-disabled is a no-op) is satisfied because both
    // the click and Enter code paths route through `pill_activates`.
    assert!(pill_activates(false, false), "enabled inactive pill must activate");
    assert!(!pill_activates(true, false), "disabled pill must not activate");
    assert!(!pill_activates(false, true), "already-active pill must not re-activate");
    assert!(!pill_activates(true, true), "selected-but-disabled pill must not activate");
}

#[test]
fn priority_3_set_pill_migration_preserves_target() {
    use inputforge_core::action::Action;
    use inputforge_core::engine::EngineCommand;
    let strategy = ModeChangeStrategy::Temporary {
        mode: "Combat".to_owned(),
    };
    let (commands, _label) =
        crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_strategy_change(
            strategy,
            "axis0",
            &["Default", "Combat"],
            StrategyTarget::Set,
        );
    let first = commands.into_iter().next().expect("expected SetMapping");
    match first {
        EngineCommand::SetMapping { actions, .. } => {
            assert!(matches!(
                actions.first(),
                Some(Action::ChangeMode {
                    strategy: ModeChangeStrategy::SwitchTo { mode }
                }) if mode == "Combat"
            ), "Set click on selected-but-disabled Hold must migrate to SwitchTo with target preserved");
        }
        other => panic!("expected SetMapping, got {other:?}"),
    }
}
```

- [ ] **Step 2: Run the failing tests**

Run: `cargo test -p inputforge-gui-dx change_mode::tests::priority`
Expected: FAIL.

- [ ] **Step 3: Add hint computation and orphan option**

Insert before the rsx! block in `ChangeModeBody`:

```rust
// Hint priority computation (render-phase write, same convention as
// MergeAxisBody / MapToVJoyBody so SSR observes the hint the same frame).
let target_in_modes = !mode.is_empty() && modes.iter().any(|m| m == &mode);
let target_orphaned = !mode.is_empty() && !target_in_modes;
let combined = target_orphaned && is_hold && hold_disabled;

// One owned String per render. Combines the orphan + hold-disabled
// failure modes so the user can recover both errors in a single edit
// pass instead of fixing one and bouncing back to the next.
let dynamic_hint: Option<String> = if mode.is_empty() {
    Some(HINT_TARGET_EMPTY.to_owned())
} else if combined {
    Some(format!(
        r#"Mode "{mode}" is not in this profile, and Hold requires a button input. Pick a button-shaped input, then a current mode."#
    ))
} else if target_orphaned {
    Some(format!(
        r#"Mode "{mode}" is not in this profile. Pick a current mode."#
    ))
} else if is_hold && hold_disabled {
    Some(HINT_HOLD_NOT_BUTTON.to_owned())
} else {
    None
};

{
    let mut malformed = editor.malformed_hints;
    match dynamic_hint.as_ref() {
        Some(s) => {
            malformed.write().insert(stage_id.clone(), s.clone());
        }
        None => {
            malformed.write().remove(&stage_id);
        }
    }
}

// Orphan option support: when the persisted target is non-empty and not
// in the modes list, prepend a disabled error-tinted option so the user
// sees the stale value. The orphan option is NOT rewritten back into the
// action; it disappears once the user picks a current mode.
let mut target_options: Vec<SelectOption> = modes
    .iter()
    .map(|m| SelectOption {
        value: m.clone(),
        label: m.clone(),
        disabled: false,
        class: None,
    })
    .collect();
if target_orphaned {
    target_options.insert(
        0,
        SelectOption {
            value: mode.clone(),
            label: mode.clone(),
            disabled: true,
            class: Some("if-select__option--orphan".into()),
        },
    );
}
```

(Replace the original `target_options` builder with this version. The simple `.map().collect()` call earlier in Task 12 goes away.)

- [ ] **Step 4: Run the tests**

Run: `cargo test -p inputforge-gui-dx change_mode`
Expected: PASS, including the priority 3 set-pill migration test (target preservation falls out of `dispatch_strategy_change` passing `current_mode` unchanged across both `Set` and `Hold` targets).

- [ ] **Step 5: Verify the picked mode replaces the orphan in the persisted action**

Replace the prior cross-VDom render-twice approach (which proved nothing about clearing because each render was a fresh harness with fresh signals) with a dispatcher-level assertion: when the user picks a real mode, the dispatched `EngineCommand::SetMapping` carries only the picked mode in the action graph, never the orphan name. The render layer drops the orphan option implicitly on the next snapshot echo.

```rust
#[test]
fn orphan_pick_dispatches_action_without_orphan_mode() {
    use inputforge_core::action::Action;
    use inputforge_core::engine::EngineCommand;
    let strategy = ModeChangeStrategy::SwitchTo { mode: "Ghost".to_owned() };
    let (commands, _label) =
        crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_target_change(
            strategy,
            "btn0",
            &["Default", "Combat"],
            "Combat",
        );
    let first = commands.into_iter().next().expect("expected SetMapping");
    match first {
        EngineCommand::SetMapping { actions, .. } => {
            assert!(matches!(
                actions.first(),
                Some(Action::ChangeMode {
                    strategy: ModeChangeStrategy::SwitchTo { mode }
                }) if mode == "Combat"
            ), "persisted action must use the picked mode, not the orphan: {actions:?}");
        }
        other => panic!("expected SetMapping, got {other:?}"),
    }
}
```

Run: `cargo test -p inputforge-gui-dx change_mode::tests::orphan_pick`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/change_mode.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git commit -m "feat(change-mode): three-priority malformed hints + orphan option"
```

### Task 16: Verify the collapsed header preempts summary with the hint

**Files:** None (verification-only; the existing `pipeline/stage.rs:155-159` already implements the preempt). The change is just confirming a new test asserts the integration end-to-end.

- [ ] **Step 1: Write the failing test**

Append to `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs`:

```rust
#[test]
fn collapsed_change_mode_header_shows_summary_when_no_hint() {
    let actions = vec![Action::ChangeMode {
        strategy: ModeChangeStrategy::SwitchTo { mode: "Combat".to_owned() },
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![], &["Default", "Combat"]); // collapsed
    assert!(html.contains("Set Combat"), "expected summary 'Set Combat': {html}");
}

#[test]
fn change_mode_collapsed_header_renders_chevron_not_thumbnail() {
    // Acceptance #13: header_right_slot for Action::ChangeMode renders the
    // default chevron. After Task 11 deleted the F14 placeholder arm, the
    // catch-all in `header_right_slot` must produce the chevron icon.
    let actions = vec![Action::ChangeMode {
        strategy: ModeChangeStrategy::SwitchTo { mode: "Combat".to_owned() },
    }];
    let (state, addr) = build_state(actions);
    let html = render_with_expanded(state, addr, vec![], &["Default", "Combat"]);
    assert!(
        html.contains("if-stage__chevron"),
        "ChangeMode header must include the chevron class: {html}"
    );
    // F10 / F11 thumbnail markers must not appear.
    assert!(!html.contains("if-curve-thumb"), "F10 thumbnail leaked: {html}");
    assert!(!html.contains("if-deadzone-thumb"), "F11 thumbnail leaked: {html}");
}

#[test]
fn collapsed_change_mode_header_shows_hint_when_target_empty() {
    let actions = vec![Action::ChangeMode {
        strategy: ModeChangeStrategy::SwitchTo { mode: String::new() },
    }];
    let (state, addr) = build_state(actions);
    let _expanded =
        render_with_expanded(state.clone(), addr.clone(), vec![StageId(vec![StageIdSegment::Index(0)])], &["Default"]);
    // The body must run once expanded to write the hint; the collapse render
    // sees the hint via the shared editor state.
    let html_collapsed = render_with_expanded(state, addr, vec![], &["Default"]);
    assert!(
        html_collapsed.contains("Choose a target mode"),
        "expected hint preempt: {html_collapsed}"
    );
}
```

Both `render_with_expanded` and `render_with_expanded_settled` gain a `modes: &[&str]` parameter so tests can seed the `MetaSnapshot.modes` list without mutating shared state. Existing call sites that do not care about modes pass `&["Default"]`.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p inputforge-gui-dx collapsed_change_mode_header`
Expected: PASS (the preempt was already wired in F9; this is integration coverage).

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git commit -m "test(change-mode): collapsed header summary and hint preempt"
```

### Task 17: SSR test for the F7 "Set as default" dispatch

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs` (extend the existing `#[cfg(test)] mod tests`)

Acceptance #14 reads "F14 adds one SSR test asserting the menu item renders and clicking it sends `EngineCommand::SetDefaultMode`". The contract resolves into two halves: (1) the helper that dispatches the command is correct, and (2) the rendered menu actually includes the item that wires through to the helper. The plan covers both with a closure-extracted helper plus a thin SSR markup-presence test, mirroring the closure-extraction pattern Tasks 13 and 14 use.

- [ ] **Step 1: Refactor the click closure into a helper**

In `context_menu.rs`, extract the `set_default_onclick` body (around line 108) into a free function next to the existing helpers:

```rust
fn dispatch_set_default(
    commands: &std::sync::mpsc::Sender<inputforge_core::engine::EngineCommand>,
    name: &str,
) {
    let _ = commands.send(inputforge_core::engine::EngineCommand::SetDefaultMode {
        name: name.to_owned(),
    });
}
```

Replace the inline closure body with a call to `dispatch_set_default(&cmd_default, &default_name);`.

- [ ] **Step 2: Add the helper unit test**

In `context_menu.rs`, append to `mod tests`:

```rust
#[test]
fn dispatch_set_default_sends_set_default_mode_command() {
    use std::sync::mpsc;
    use inputforge_core::engine::EngineCommand;

    let (tx, rx) = mpsc::channel::<EngineCommand>();
    super::dispatch_set_default(&tx, "Combat");
    match rx.try_recv() {
        Ok(EngineCommand::SetDefaultMode { name }) => assert_eq!(name, "Combat"),
        other => panic!("expected SetDefaultMode {{ name: \"Combat\" }}, got {other:?}"),
    }
}
```

- [ ] **Step 3: Add the SSR markup-presence test**

```rust
#[test]
fn set_as_default_item_renders_when_flag_is_enabled() {
    use dioxus::prelude::*;
    use dioxus_ssr::render;

    // Mount ModeTabContextMenu with flags.set_default_disabled = false.
    // The menu must include a "Set as default" <li> with the canonical
    // data-action attribute that wires onto dispatch_set_default.
    #[expect(non_snake_case, reason = "Dioxus components are PascalCase by convention")]
    fn Harness() -> Element {
        rsx! {
            ModeTabContextMenu {
                target_mode: "Combat".to_owned(),
                flags: ContextMenuFlags {
                    set_default_disabled: false,
                    ..ContextMenuFlags::default()
                },
                // Other required props use defaults that the existing
                // flags_for tests already exercise.
            }
        }
    }

    let mut vdom = VirtualDom::new(Harness);
    vdom.rebuild_in_place();
    let html = render(&vdom);

    assert!(
        html.contains("Set as default"),
        "menu must surface the 'Set as default' item: {html}"
    );
    assert!(
        !html.contains(r#"aria-disabled="true""#)
            || !html.contains("Set as default"),
        "the Set as default item must be enabled when flag is false: {html}"
    );
}
```

If `ModeTabContextMenu` requires additional context (e.g. an `AppContext` provider), wrap the harness with `use_context_provider` against a minimal default. The test does not assert dispatch (Step 2 covers that); it only asserts the rendered item is present and unblocked.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p inputforge-gui-dx mode_tabs::context_menu`
Expected: PASS for both `dispatch_set_default_sends_set_default_mode_command` and `set_as_default_item_renders_when_flag_is_enabled`.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs
git commit -m "test(mode-tabs): cover SetDefaultMode dispatch from context menu"
```

### Task 18: Final acceptance pass

**Files:** None (verification-only)

- [ ] **Step 1: Confirm the placeholder module is fully removed**

Run: `rg 'placeholders::' crates/`
Expected: zero matches. The `git rm` in Task 11 removes the file; this gate catches any forgotten import or `mod placeholders` declaration before the broader sweep.

- [ ] **Step 2: Run the engine grep gate**

```bash
rg "ModeChangeStrategy::Previous" crates/
rg "ModeChangeStrategy::Cycle" crates/
rg "CycleModes" crates/
rg "go_previous" crates/
rg "fn cycle\b" crates/inputforge-core/src/mode/state.rs
rg 'strategy = "previous"' .
rg 'strategy = "cycle"' .
rg "F14 owns this body" crates/
rg "ChangeModePlaceholder" crates/
```

Expected: every command returns zero matches except inside spec/plan markdown files. If anything else hits, return to the relevant task and finish.

- [ ] **Step 3: Run the full workspace**

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: all green, zero clippy warnings.

- [ ] **Step 4: Walk the GUI manually**

Run: `dx run -p inputforge-app`

Manual verification (each is one acceptance item):

- Add a Change mode stage from the palette. The collapsed header shows `"Choose a target mode"` instead of `"Set "`.
- Expand the body. Two pills (Set / Hold) plus a `Target mode` Select are visible.
- Pick `Combat` from the Select. One undo entry is added with label `Change mode: target <unset> -> Combat`. The collapsed header now shows `"Set Combat"`.
- Click `Hold`. One undo entry: `Change mode: strategy Set -> Hold`. The target Select still shows `Combat`. Collapsed header: `"Hold Combat"`.
- Click `Set`. One undo entry: `Change mode: strategy Hold -> Set`. Target preserved.
- Rebind the mapping primary input from a button to an axis (use the F8 rebind affordance). Switch back to the Change mode stage. The Hold pill renders selected-but-disabled (if the strategy was Hold) or just-disabled (if Set), and hovering or focusing it surfaces the tooltip `"Hold requires a button input."` Hint priority 3 is visible in the collapsed header. Clicking Set commits a one-step migration to `SwitchTo { mode }` preserving the target.
- Tab to the disabled Hold pill (focus ring should be visible per DESIGN.md). Press Enter. Observe no commit (no toast, no undo entry, target Select unchanged). The `pill_activates` unit test carries the bulk of the contract; this manual confirms the rendered button does not bypass the gate.
- Open the Default profile, manually edit the mapping's TOML (or use a snapshot rollback) to reference a non-existent mode. Reload. The orphaned name renders as an italic error-tinted disabled `<option>` at the top of the dropdown; hint priority 2 is visible. Pick a current mode; the orphan option disappears on the next render.
- Author Hold against an axis-bound mapping AND a non-existent target mode. The combined hint surfaces ("`Mode "Ghost" is not in this profile, and Hold requires a button input. ...`") so the user can recover both errors in one pass.
- Right-click a mode tab in the chrome and pick "Set as default". The startup-mode flag updates without F14 having added wiring (verified by F7).

- [ ] **Step 5: Mark the master plan F14 entry resolved**

Update `docs/superpowers/specs/2026-04-24-egui-to-dioxus-rewrite-design.md` at the F14 entry to mark the feature shipped.

- [ ] **Step 6: Commit the master-plan update**

```bash
git add docs/superpowers/specs/2026-04-24-egui-to-dioxus-rewrite-design.md
git commit -m "docs(master-plan): mark F14 mode editing shipped"
```

---

## Notes for the implementer

- **Sibling reference for the body shape:** read `merge_axis.rs` (operation Select + secondary input row) AND `map_to_vjoy.rs` (two Selects, render-phase malformed-hint write). `change_mode.rs` is closer to `merge_axis.rs` in size and to `map_to_vjoy.rs` in malformed-hint pattern. The newer `dispatch_stage_edit` helper (used by F10 and F11) is the canonical dispatch path; `merge_axis.rs` and `map_to_vjoy.rs` predate it and inline `cmd_tx.send` directly. F14 follows the F10/F11 pattern.
- **No middle-dot in undo labels:** `format_undo_label` for `UndoKind::StageEdit` produces `"{name}: {field} {b} -> {a}"` (ASCII arrow, single space on each side). Do not introduce U+00B7 middle-dot characters in any new label.
- **Tooltip wrapping:** wrap only the disabled Hold pill in `Tooltip`. The enabled pill stays bare so the tooltip never announces a stale "Hold requires a button input" message on a working pill.
- **No effort estimates:** every step here describes a code change, not a duration. Skip if a step turns out to be a no-op (e.g., `rename_mode_refs` does not actually become infallible).
- **Do not amend commits.** Each task ships in one fresh commit. The conventional-commits skill enforces the `type(scope): subject` format.
- **Smoke vs manual.** Steps that say `cargo test` or `cargo build` are smoke tests. Steps that say `dx run` are manual verification (Task 18 only).
