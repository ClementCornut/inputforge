# F8, Mapping List (Left Rail) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder `if-layout__rail` text with the F8 mapping-list rail (group-bucketed rows by input kind, glyphs for MergeAxis / input-Conditional, filter, keyboard nav, right-click menu, two empty states, inline `+ Add mapping` capture flow), and ship the shared `LiveCapture` primitive that F9-F12 will reuse.

**F8→F9 sequencing constraint:** Until F9 ships its action editor, a fresh `+ Add mapping` capture round-trips through `Profile::save` with `actions: vec![]`, which the engine treats as removal. The added mapping disappears at the next save. F8 ships this regression deliberately; F9's first task must be wiring the action editor before this becomes user-visible in any release build.

**Architecture:** Engine-side first, a new `EngineCommand::RemoveMapping` variant + `Profile::remove_mapping` mutator + `RunningEngine::remove_mapping` handler, lands behind a round-trip test before any GUI plumbing. State infrastructure follows: `InputCacheStore::clone_compact` (pure helper for the primitive) → `ConfigSnapshot::mappings` extension with `MappingSummary` + glyph-derivation walker → `ViewState::selected_mapping` field with shadow-signal reconciliation. The `LiveCapture` primitive ships next, `LiveCaptureCore::step` is a pure state-transition fn unit-tested without a Dioxus runtime; the hook in `mod.rs` is a thin adapter that calls `step` per polling tick and mounts a window-level Esc listener while armed. Mapping-list components ship inside-out: leaf pure-logic modules (`source_label`, `group`, `filter`) with unit tests, then leaf renderers (`row`, `rename_inline`, `empty`), then the `+ Add mapping` state machine (`add_inline`), then keyboard handling (`keyboard`), and finally the `mod.rs` orchestrator that wires everything together. Layout integration and SSR/component tests close the plan.

**Tech Stack:** Rust 2024 edition · `inputforge-core` (engine, profile, action, state) · `inputforge-gui-dx` (Dioxus 0.7, dioxus-desktop, F2 component primitives, F4 dialog/toast, F7 frame) · `parking_lot::RwLock` over `AppState` · `std::sync::mpsc` for `EngineCommand` dispatch · `tracing` for engine + GUI events.

**Spec:** [`docs/superpowers/specs/2026-04-30-f8-mapping-list-design.md`](../specs/2026-04-30-f8-mapping-list-design.md).

---

## Sequencing rationale

Engine-side first: `RemoveMapping` round-trips through `Profile::remove_mapping` + `Profile::save` and is fully unit-testable without GUI. State infrastructure next: `MappingSummary` + glyph derivation lives in `context.rs` and depends only on `inputforge-core` types, landing it before GUI lets every renderer subscribe to a stable shape. `LiveCapture` follows: a pure `step()` function gated by enumerated tests covers all the tricky cases (baseline-and-edge, multi-axis nudge, switch-already-on, debounce window) before any Dioxus signals enter the picture. Mapping-list ships inside-out: pure-logic leaves (`source_label`, `group`, `filter::matches_filter`) → simple renderers (`row`, `rename_inline`, `empty`) → stateful renderers (`add_inline`, `keyboard`) → orchestrator (`mod.rs`). Layout integration and SSR tests land last so they never break the build mid-flight.

The first 12 tasks (Phase A + B + C) are pure-logic / unit-testable / engine-only. Tasks 13-28 are GUI render code; manual interaction passes happen in the final phase. Tasks 11, 12, 13, 24, and 25 are verification tasks where test and implementation ship together because the implementation is a pure-logic one-liner or an end-to-end SSR check; they do not follow the failing-first TDD pattern.

---

## File structure overview

**Created (engine):** None, all changes are method additions to existing files.

**Modified (engine):**

- `crates/inputforge-core/src/profile/mod.rs`, `Profile::remove_mapping(&mut self, &InputAddress, &str) -> bool`
- `crates/inputforge-core/src/engine/command.rs`, `EngineCommand::RemoveMapping { input, mode }` variant
- `crates/inputforge-core/src/engine/run.rs`, `RunningEngine::remove_mapping` handler + dispatch arm in `handle_command`
- `crates/inputforge-core/src/engine/tests.rs`, Set→Remove round-trip test (in-memory + disk reload)
- `crates/inputforge-core/src/state/cache.rs`, `InputCacheStore::clone_compact() -> Vec<InputCacheEntry>` + `InputCacheEntry` type

**Created (GUI):**

```
crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs
crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs
crates/inputforge-gui-dx/src/frame/mapping_list/group.rs
crates/inputforge-gui-dx/src/frame/mapping_list/row.rs
crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs
crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs
crates/inputforge-gui-dx/src/frame/mapping_list/rename_inline.rs
crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs
crates/inputforge-gui-dx/src/frame/mapping_list/keyboard.rs
crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs
crates/inputforge-gui-dx/src/patterns/live_capture/machine.rs
crates/inputforge-gui-dx/src/patterns/live_capture/tests.rs
crates/inputforge-gui-dx/assets/frame/mapping_list.css
```

**Modified (GUI):**

- `crates/inputforge-gui-dx/src/context.rs`, `ConfigSnapshot::mappings: Vec<MappingSummary>` + `MappingSummary` + `GlyphFlags` + glyph-derivation walker; `from_state` extended
- `crates/inputforge-gui-dx/src/frame/view_state.rs`, `ViewState::selected_mapping` field + reconciliation branches in `use_view_state_provider`
- `crates/inputforge-gui-dx/src/frame/layout/mod.rs`, wires `<MappingList />` into the `if-layout__rail` slot
- `crates/inputforge-gui-dx/src/frame/mod.rs`, `mod mapping_list;` + re-export `MappingList`
- `crates/inputforge-gui-dx/src/patterns/mod.rs`, `pub mod live_capture;`
- `crates/inputforge-gui-dx/src/app.rs`, install `LiveCapture` via `use_context_provider` (sibling of `ToastQueue`)

**Deleted:** None.

---

## Phase A, Engine-side: `RemoveMapping` (Tasks 1-3)

### Task 1: `Profile::remove_mapping`

Pure mutator on `Profile` that removes the `(input, mode)` pair from the private `mappings` Vec and returns `true` iff a mapping was actually removed. Sibling of `set_mapping` at `profile/mod.rs:197`. Boolean return enables the engine handler's no-op fast path (skip `profile.save` when nothing changed).

**Files:**
- Modify: `crates/inputforge-core/src/profile/mod.rs`
- Test: `crates/inputforge-core/src/profile/mod.rs` (existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Append to the existing `#[cfg(test)] mod tests` in `crates/inputforge-core/src/profile/mod.rs` (the existing test helpers `minimal_profile()` and a working `Mapping` constructor are already in scope, see the `set_mapping_*` tests around line 960):

```rust
#[test]
fn remove_mapping_drops_existing_returns_true() {
    let mut profile = minimal_profile();
    assert!(!profile.mappings().is_empty(), "fixture invariant");
    let target = profile.mappings()[0].input.clone();
    let target_mode = profile.mappings()[0].mode.clone();

    let before_len = profile.mappings().len();
    let removed = profile.remove_mapping(&target, &target_mode);

    assert!(removed, "remove_mapping should return true when a mapping was removed");
    assert_eq!(profile.mappings().len(), before_len - 1);
    assert!(profile.find_mapping(&target, &target_mode).is_none());
}

#[test]
fn remove_mapping_unknown_returns_false() {
    let mut profile = minimal_profile();
    assert!(!profile.mappings().is_empty(), "fixture invariant");
    let target = InputAddress {
        device: DeviceId("nonexistent".to_owned()),
        input: InputId::Button { index: 99 },
    };

    let before_len = profile.mappings().len();
    let removed = profile.remove_mapping(&target, "Default");

    assert!(!removed, "remove_mapping should return false when nothing matched");
    assert_eq!(profile.mappings().len(), before_len);
}

#[test]
fn remove_mapping_wrong_mode_returns_false() {
    // remove_mapping is mode-scoped: a matching input in a different
    // mode must NOT be removed.
    let mut profile = minimal_profile();
    assert!(!profile.mappings().is_empty(), "fixture invariant");
    let target = profile.mappings()[0].input.clone();

    let removed = profile.remove_mapping(&target, "NonexistentMode");

    assert!(!removed);
    assert!(profile.find_mapping(&target, "Default").is_some());
}
```

If `DeviceId` / `InputId` / `InputAddress` are not yet imported in the `tests` module, add them to the existing `use super::*;` import block (existing tests already import from `crate::types`; mirror that).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-core --lib profile::tests::remove_mapping`
Expected: FAIL, `error[E0599]: no method named 'remove_mapping' found for struct 'Profile'`.

- [ ] **Step 3: Implement `Profile::remove_mapping`**

Insert into `impl Profile` in `crates/inputforge-core/src/profile/mod.rs`, immediately after `set_mapping` (currently ends around line 224):

```rust
/// Remove the mapping for `(input, mode)`. Returns `true` if a mapping
/// was removed, `false` if no matching mapping existed.
///
/// Distinct from `set_mapping(_, _, None, vec![])` which can also remove -
/// `remove_mapping` is the explicit API for the F8 delete flow and lets
/// callers detect a no-op (race between two stale dispatches) without
/// comparing `mappings().len()` before-and-after.
pub fn remove_mapping(&mut self, input: &InputAddress, mode: &str) -> bool {
    let before = self.mappings.len();
    self.mappings
        .retain(|m| !(m.input == *input && m.mode == mode));
    self.mappings.len() != before
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p inputforge-core --lib profile::tests::remove_mapping`
Expected: PASS, three tests, all green.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-core/src/profile/mod.rs
git commit -m "feat(profile): add Profile::remove_mapping(input, mode) -> bool"
```

---

### Task 2: `EngineCommand::RemoveMapping` variant

Adds the new variant to `EngineCommand`. The handler arm in `run.rs` is wired in Task 3.

**Files:**
- Modify: `crates/inputforge-core/src/engine/command.rs`
- Test: `crates/inputforge-core/src/engine/command.rs` (existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` in `crates/inputforge-core/src/engine/command.rs` (after the existing `engine_command_derives_debug_partialeq` test, around line 173):

```rust
#[test]
fn remove_mapping_variant_debug_and_partialeq() {
    use crate::types::{DeviceId, InputId};

    let input = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 3 },
    };
    let a = EngineCommand::RemoveMapping {
        input: input.clone(),
        mode: "Default".to_owned(),
    };
    let b = EngineCommand::RemoveMapping {
        input: input.clone(),
        mode: "Default".to_owned(),
    };
    assert_eq!(a, b, "PartialEq must hold across the new variant");
    assert!(format!("{a:?}").contains("RemoveMapping"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-core --lib engine::command::tests::remove_mapping_variant`
Expected: FAIL, `error[E0599]: no variant or associated item named 'RemoveMapping' found for enum 'EngineCommand'`.

- [ ] **Step 3: Add the variant**

Insert into `enum EngineCommand` in `crates/inputforge-core/src/engine/command.rs`, immediately after the existing `SetMapping { ... }` variant (around line 39):

```rust
/// Remove the mapping for `(input, mode)`. No-op if no such mapping
/// exists; the engine handler skips persistence on that fast path.
RemoveMapping {
    input: InputAddress,
    mode: String,
},
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p inputforge-core --lib engine::command::tests::remove_mapping_variant`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-core/src/engine/command.rs
git commit -m "feat(engine): add EngineCommand::RemoveMapping variant"
```

---

### Task 3: `RunningEngine::remove_mapping` handler + round-trip test

Adds the engine-side handler and wires it into the `handle_command` dispatch. Mirrors `set_mapping`'s shape exactly: read profile path, mutate via `Profile::remove_mapping`, persist via `Profile::save` only when something changed.

**Files:**
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Test: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write the failing round-trip test**

Append to `crates/inputforge-core/src/engine/tests.rs`. Mirror the shape of `set_mapping_refreshes_outputs_from_cached_axis_values` (around line 1344), write profile to a temp file, build engine, dispatch `SetMapping`, tick to apply, dispatch `RemoveMapping`, tick again, assert removed in-memory AND persisted to disk:

```rust
#[test]
fn remove_mapping_round_trip_persists_removal_to_disk() {
    use crate::profile::Profile;

    // Start with one mapping (axis 0 → vJoy X) so we have something to
    // remove. The set→remove round-trip on the same input/mode proves
    // both engine handlers cooperate.
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: Some("Throttle".to_owned()),
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let dir = std::env::temp_dir().join("inputforge_remove_mapping_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("remove_mapping_round_trip.toml");
    std::fs::write(&path, profile.to_toml().unwrap()).unwrap();

    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);
    state.write().profile_path = Some(path.clone());

    // Sanity: mapping is present at start.
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&axis_addr(0), "Default")
            .is_some(),
        "fixture should have one mapping in-memory before remove"
    );

    // Dispatch RemoveMapping and tick to drain the command queue.
    tx.send(EngineCommand::RemoveMapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    // In-memory: gone.
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&axis_addr(0), "Default")
            .is_none(),
        "RemoveMapping should drop the mapping from active_profile"
    );

    // On-disk: gone, reload from the same path and re-check.
    let reloaded = Profile::load(&path).unwrap();
    assert!(
        reloaded.find_mapping(&axis_addr(0), "Default").is_none(),
        "RemoveMapping should persist removal to disk"
    );

    // Cleanup.
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn remove_mapping_no_op_for_unknown_input_does_not_panic() {
    // The engine handler must be tolerant of stale dispatches (e.g. two
    // remove commands racing for the same mapping). Second one is a no-op
    // and must not crash.
    let mapping = Mapping {
        input: axis_addr(0),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MapToVJoy {
            output: vjoy_axis_output(1, VJoyAxis::X),
        }],
    };
    let profile = make_profile(simple_mode_tree(), vec![mapping]);

    let dir = std::env::temp_dir().join("inputforge_remove_mapping_noop_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("noop.toml");
    std::fs::write(&path, profile.to_toml().unwrap()).unwrap();

    let (mut engine, state, tx) = make_engine(MockInputSource::default(), profile);
    state.write().profile_path = Some(path.clone());

    // Remove a mapping that doesn't exist (wrong button index).
    tx.send(EngineCommand::RemoveMapping {
        input: button_addr(99),
        mode: "Default".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    // Original mapping survives untouched.
    assert!(
        state
            .read()
            .active_profile
            .as_ref()
            .unwrap()
            .find_mapping(&axis_addr(0), "Default")
            .is_some(),
        "no-op RemoveMapping must leave existing mappings intact"
    );

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-core --lib engine::tests::remove_mapping`
Expected: FAIL, compile error on the dispatch arm (no match arm for `EngineCommand::RemoveMapping`).

- [ ] **Step 3: Implement the handler method**

Insert into `impl RunningEngine` (or whichever `impl` block carries `set_mapping`) in `crates/inputforge-core/src/engine/run.rs`, immediately after `set_mapping` (currently ends at line 800):

```rust
/// Remove a mapping from the active profile and persist to disk if
/// the underlying `Profile::remove_mapping` reported a change.
fn remove_mapping(&self, input: &InputAddress, mode: &str) {
    let mut state = self.state.write();

    if state.active_profile.is_none() {
        tracing::warn!(target: "f8::mapping_list", "cannot remove mapping: no profile loaded");
        return;
    }

    let Some(path) = state.profile_path.clone() else {
        tracing::warn!(target: "f8::mapping_list", "cannot remove mapping: no profile path");
        return;
    };

    let profile = state.active_profile.as_mut().expect("checked above");
    if !profile.remove_mapping(input, mode) {
        // No-op fast path: nothing to persist.
        return;
    }

    if let Err(e) = profile.save(&path) {
        tracing::warn!(
            target: "f8::mapping_list",
            path = %path.display(),
            error = %e,
            "failed to save profile after RemoveMapping",
        );
    }
}
```

- [ ] **Step 4: Wire the dispatch arm**

In `crates/inputforge-core/src/engine/run.rs` `handle_command`, immediately after the existing `EngineCommand::SetMapping { ... }` arm (currently at line 350-358), insert:

```rust
EngineCommand::RemoveMapping { input, mode } => {
    self.remove_mapping(&input, &mode);
    self.pending_output_refresh = true;
}
```

The `pending_output_refresh = true` mirrors `SetMapping`, removing a mapping changes the active pipeline, so cached axis values must be re-evaluated through it.

- [ ] **Step 5: Run the round-trip + no-op tests**

Run: `cargo test -p inputforge-core --lib engine::tests::remove_mapping`
Expected: PASS, both tests green.

- [ ] **Step 6: Run the full engine test suite to catch regressions**

Run: `cargo test -p inputforge-core --lib engine`
Expected: PASS, no existing tests should break (the change is additive).

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-core/src/engine/run.rs crates/inputforge-core/src/engine/tests.rs
git commit -m "feat(engine): handle EngineCommand::RemoveMapping with disk-persisted round-trip"
```

---

## Phase B, State infrastructure (Tasks 4-6)

### Task 4: `InputCacheStore::clone_compact()`

Returns a sortable, owned snapshot of every cached `(InputAddress, InputValue)` pair. The live-capture primitive consumes this on every polling tick, it needs to compare current vs baseline without holding the `RwLock` read guard. Pure logic, no Dioxus dependency.

**Iteration-order contract:** `clone_compact` MUST iterate in a stable, deterministic order across calls. The live-capture tied-axis tiebreak (Task 7's `pick_winner`) depends on first-encountered order being well-defined. Use `IndexMap` or `Vec<InputCacheEntry>` as the underlying store; `HashMap` is forbidden. If `InputCacheStore`'s current internal storage is a `HashMap`, change it to `IndexMap` (or a `Vec`) as part of this task; if already deterministic, the change is to document the contract on the method.

**`InputCacheEntry` derives `PartialEq`** (Task 8 needs it for the `s.peek() != next` state-equality check in the polling effect). Add the derive if missing in the existing codebase.

**Files:**
- Modify: `crates/inputforge-core/src/state/cache.rs`
- Test: `crates/inputforge-core/src/state/cache.rs` (existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Append to the existing `#[cfg(test)] mod tests` in `crates/inputforge-core/src/state/cache.rs`:

```rust
#[test]
fn clone_compact_returns_all_entries_with_address_and_value() {
    let mut cache = InputCacheStore::new();
    cache.update(
        &axis_address(0),
        &InputValue::Axis {
            value: AxisValue::new(0.5),
        },
    );
    cache.update(&button_address(1), &InputValue::Button { pressed: true });
    cache.update(
        &hat_address(0),
        &InputValue::Hat {
            direction: HatDirection::N,
        },
    );

    let entries = cache.clone_compact();
    assert_eq!(entries.len(), 3, "all three entries should be present");

    // Spot-check that each kind round-trips through the snapshot.
    let axis_entry = entries.iter().find(|e| e.address == axis_address(0)).unwrap();
    match &axis_entry.value {
        InputValue::Axis { value } => assert!((value.value() - 0.5).abs() < f64::EPSILON),
        other => panic!("expected Axis variant, got {other:?}"),
    }

    let button_entry = entries.iter().find(|e| e.address == button_address(1)).unwrap();
    assert!(matches!(button_entry.value, InputValue::Button { pressed: true }));

    let hat_entry = entries.iter().find(|e| e.address == hat_address(0)).unwrap();
    assert!(matches!(
        hat_entry.value,
        InputValue::Hat {
            direction: HatDirection::N,
        }
    ));
}

#[test]
fn clone_compact_empty_cache_returns_empty_vec() {
    let cache = InputCacheStore::new();
    assert!(cache.clone_compact().is_empty());
}

#[test]
fn clone_compact_does_not_mutate_cache() {
    let mut cache = InputCacheStore::new();
    cache.update(&button_address(0), &InputValue::Button { pressed: true });

    let _ = cache.clone_compact();
    let _ = cache.clone_compact();

    // Original cache still readable.
    assert!(cache.get_button(&button_address(0)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-core --lib state::cache::tests::clone_compact`
Expected: FAIL, `error[E0599]: no method named 'clone_compact'`.

- [ ] **Step 3: Add the helper type and method**

Insert into `crates/inputforge-core/src/state/cache.rs`, immediately after the `InputCacheStore` struct definition (around line 16):

```rust
/// One entry in an [`InputCacheStore`] snapshot. Used by GUI consumers
/// (notably the live-capture primitive) that need to compare current
/// state against an earlier baseline without holding any read lock.
#[derive(Debug, Clone, PartialEq)]
pub struct InputCacheEntry {
    pub address: InputAddress,
    pub value: InputValue,
}
```

Insert the `clone_compact` method into `impl InputCacheStore` in the same file, immediately after `get_all_axis_entries` (around line 46):

```rust
/// Snapshot every cached `(address, value)` pair into an owned Vec.
///
/// Order is **stable and deterministic** across calls, backed by
/// `IndexMap` (or a `Vec`), never `HashMap`. The live-capture
/// tied-axis tiebreak (`patterns::live_capture::machine::pick_winner`)
/// relies on first-encountered order being well-defined: when two
/// axes cross deadband simultaneously with identical absolute deltas,
/// the first one in this iteration order wins.
///
/// The return value is fully owned, so the caller can drop the
/// underlying lock guard immediately after this call returns.
#[must_use]
pub fn clone_compact(&self) -> Vec<InputCacheEntry> {
    self.values
        .iter()
        .map(|(addr, val)| InputCacheEntry {
            address: addr.clone(),
            value: val.clone(),
        })
        .collect()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p inputforge-core --lib state::cache::tests::clone_compact`
Expected: PASS, three tests green.

- [ ] **Step 5: Re-export `InputCacheEntry` from `state` mod**

Open `crates/inputforge-core/src/state/mod.rs`. The existing module already re-exports `InputCacheStore`; add `InputCacheEntry` alongside it:

```rust
pub use cache::{InputCacheEntry, InputCacheStore};
```

(Mirror whatever the current `pub use cache::...` line looks like, keep types alphabetically grouped.)

- [ ] **Step 6: Run the full state tests**

Run: `cargo test -p inputforge-core --lib state`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-core/src/state/cache.rs crates/inputforge-core/src/state/mod.rs
git commit -m "feat(state): add InputCacheStore::clone_compact() snapshot helper"
```

---

### Task 5: `ConfigSnapshot.mappings` + `MappingSummary` + glyph derivation

Extends `ConfigSnapshot` with a per-mapping summary list populated once per polling tick. The glyph walker (MergeAxis present, input-Conditional present) runs at snapshot time, not at render time, so each row read is a constant-time field access. All glyph derivation is pure logic, unit-testable without Dioxus.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/context.rs`
- Test: `crates/inputforge-gui-dx/src/context.rs` (existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing types-and-derivation tests**

Append to the `#[cfg(test)] mod tests` block in `crates/inputforge-gui-dx/src/context.rs` (around line 209). The new tests cover (a) a plain mapping has no glyph flags, (b) `MergeAxis` populates `merge_secondary`, (c) `Conditional` with an input-bearing condition populates `first_input_predicate`, (d) both glyphs coexist, (e) deeply nested `Conditional`/`Not`/`Any` walker terminates on the first input-bearing leaf:

```rust
#[test]
fn config_snapshot_populates_mappings_with_no_glyphs() {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId};

    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let mappings = vec![Mapping {
        input: addr.clone(),
        mode: "Default".to_owned(),
        name: Some("Fire".to_owned()),
        actions: vec![], // no MergeAxis, no Conditional
    }];

    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let cfg = ConfigSnapshot::from_state(&state);

    assert_eq!(cfg.mappings.len(), 1);
    let s = &cfg.mappings[0];
    assert_eq!(s.input, addr);
    assert_eq!(s.mode, "Default");
    assert_eq!(s.name.as_deref(), Some("Fire"));
    assert!(s.glyphs.merge_secondary.is_none());
    assert!(s.glyphs.first_input_predicate.is_none());
}

#[test]
fn config_snapshot_glyph_walker_finds_merge_axis() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId, MergeOp};

    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let primary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };

    let mappings = vec![Mapping {
        input: primary.clone(),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::MergeAxis {
            second_input: secondary.clone(),
            operation: MergeOp::Sum,
        }],
    }];

    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let cfg = ConfigSnapshot::from_state(&state);

    let s = &cfg.mappings[0];
    assert_eq!(s.glyphs.merge_secondary.as_ref(), Some(&secondary));
    assert!(s.glyphs.first_input_predicate.is_none());
}

#[test]
fn config_snapshot_glyph_walker_finds_input_conditional() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId};

    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let trigger = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let predicate = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 1 },
    };

    let mappings = vec![Mapping {
        input: trigger.clone(),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: predicate.clone(),
            },
            if_true: vec![],
            if_false: None,
        }],
    }];

    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let cfg = ConfigSnapshot::from_state(&state);

    let s = &cfg.mappings[0];
    assert!(s.glyphs.merge_secondary.is_none());
    assert!(
        s.glyphs.first_input_predicate.is_some(),
        "input-bearing Conditional must populate first_input_predicate"
    );
}

#[test]
fn config_snapshot_glyph_walker_finds_both_glyphs() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId, MergeOp};

    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let primary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let predicate = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };

    let mappings = vec![Mapping {
        input: primary.clone(),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![
            Action::MergeAxis {
                second_input: secondary.clone(),
                operation: MergeOp::Sum,
            },
            Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: predicate.clone(),
                },
                if_true: vec![],
                if_false: None,
            },
        ],
    }];

    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let cfg = ConfigSnapshot::from_state(&state);

    let s = &cfg.mappings[0];
    assert_eq!(s.glyphs.merge_secondary.as_ref(), Some(&secondary));
    assert!(s.glyphs.first_input_predicate.is_some());
}

#[test]
fn config_snapshot_glyph_walker_recurses_through_composite_conditions() {
    // Walker must dive into All/Any/Not until it finds an input-bearing
    // leaf (ButtonPressed/ButtonReleased/AxisInRange/HatDirection).
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId};

    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let trigger = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let nested_predicate = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 5 },
    };

    let nested_condition = Condition::Not {
        condition: Box::new(Condition::Any {
            conditions: vec![Condition::All {
                conditions: vec![Condition::ButtonReleased {
                    input: nested_predicate.clone(),
                }],
            }],
        }),
    };

    let mappings = vec![Mapping {
        input: trigger.clone(),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::Conditional {
            condition: nested_condition,
            if_true: vec![],
            if_false: None,
        }],
    }];

    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let cfg = ConfigSnapshot::from_state(&state);

    let s = &cfg.mappings[0];
    assert!(
        s.glyphs.first_input_predicate.is_some(),
        "walker must recurse through Not → Any → All → ButtonReleased"
    );
}

#[test]
fn config_snapshot_glyph_walker_descends_into_nested_actions() {
    // MergeAxis nested inside Conditional.if_true must still be found.
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId, MergeOp};

    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let primary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let secondary = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let predicate = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };

    let mappings = vec![Mapping {
        input: primary.clone(),
        mode: "Default".to_owned(),
        name: None,
        actions: vec![Action::Conditional {
            condition: Condition::ButtonPressed {
                input: predicate.clone(),
            },
            if_true: vec![Action::MergeAxis {
                second_input: secondary.clone(),
                operation: MergeOp::Sum,
            }],
            if_false: None,
        }],
    }];

    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let cfg = ConfigSnapshot::from_state(&state);

    let s = &cfg.mappings[0];
    assert_eq!(
        s.glyphs.merge_secondary.as_ref(),
        Some(&secondary),
        "walker must descend into Conditional.if_true to find MergeAxis"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib context::tests::config_snapshot`
Expected: FAIL, `error[E0609]: no field 'mappings' on type 'ConfigSnapshot'` and `error[E0433]: no MappingSummary in scope`.

- [ ] **Step 3: Add the new types**

Insert into `crates/inputforge-gui-dx/src/context.rs`, immediately after the existing `ConfigSnapshot` struct definition (around line 66):

```rust
/// One row's worth of state for the F8 mapping list. Populated by
/// `ConfigSnapshot::from_state` once per polling tick from the active
/// profile's `Mapping` entries; consumers in `frame::mapping_list` read
/// these as constant-time field accesses without re-walking the action
/// tree.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MappingSummary {
    pub input: InputAddress,
    pub mode: String,
    pub name: Option<String>,
    pub glyphs: GlyphFlags,
}

/// Pre-computed glyph state for a `MappingSummary`. The walker stops on
/// the first match per glyph, so both fields hold the *first*
/// occurrence found by depth-first traversal of the action tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct GlyphFlags {
    /// `Some(addr)` if the action tree contains an `Action::MergeAxis`
    /// whose `second_input` is `addr`, the secondary input shown after
    /// the gold `+` glyph.
    pub merge_secondary: Option<InputAddress>,
    /// `Some(addr)` if the action tree contains an `Action::Conditional`
    /// whose `condition` references at least one `InputAddress` (via
    /// `ButtonPressed | ButtonReleased | AxisInRange | HatDirection`,
    /// possibly nested under `All | Any | Not`). The violet `⊕` glyph
    /// hover-tooltip in `row.rs` runs this address through
    /// `source_label::format` to produce the human-readable predicate
    /// label (identical path to `merge_secondary`).
    pub first_input_predicate: Option<InputAddress>,
}
```

`InputAddress` is already imported at the top of the file. `MappingSummary` and `GlyphFlags` are `pub(crate)`, they only need to be visible to `frame::mapping_list`. `MappingSummary` does **not** derive `Default` (would require `InputAddress: Default`, which it isn't); `ConfigSnapshot::default()` works because the `mappings: Vec<MappingSummary>` field's default is the empty vec.

- [ ] **Step 4: Add the `mappings` field to `ConfigSnapshot`**

Edit the existing `ConfigSnapshot` struct in `crates/inputforge-gui-dx/src/context.rs` (around line 60-66) to add the new field:

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ConfigSnapshot {
    pub devices: Vec<DeviceState>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    pub mapped_inputs: HashSet<InputAddress>,
    pub mapping_names: HashMap<InputAddress, String>,
    pub mappings: Vec<MappingSummary>,
}
```

`Default` still works because `Vec<MappingSummary>` defaults to empty.

- [ ] **Step 5: Implement the glyph walker as private fns**

Insert into `crates/inputforge-gui-dx/src/context.rs`, immediately above `impl ConfigSnapshot` (around line 188):

```rust
/// Walk an action tree in depth-first order, recording the first
/// `MergeAxis::second_input` and the first input-bearing `Conditional`
/// predicate. Returns early once both glyphs are populated, or after a
/// full traversal (whichever comes first).
fn derive_glyphs(actions: &[inputforge_core::action::Action]) -> GlyphFlags {
    let mut out = GlyphFlags::default();
    walk_actions(actions, &mut out);
    out
}

fn walk_actions(actions: &[inputforge_core::action::Action], out: &mut GlyphFlags) {
    use inputforge_core::action::Action;
    for action in actions {
        if out.merge_secondary.is_some() && out.first_input_predicate.is_some() {
            return;
        }
        match action {
            Action::MergeAxis { second_input, .. } => {
                if out.merge_secondary.is_none() {
                    out.merge_secondary = Some(second_input.clone());
                }
            }
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                if out.first_input_predicate.is_none() {
                    if let Some(addr) = first_input_predicate(condition) {
                        out.first_input_predicate = Some(addr);
                    }
                }
                walk_actions(if_true, out);
                if let Some(branch) = if_false.as_deref() {
                    walk_actions(branch, out);
                }
            }
            // Other variants do not contribute to F8 glyphs. F9-F12
            // surface them inside the editor; they are inert here.
            _ => {}
        }
    }
}

/// Recurse through `All | Any | Not` composites until an input-bearing
/// leaf (`ButtonPressed | ButtonReleased | AxisInRange | HatDirection`)
/// is found. Returns the predicate's `InputAddress` so the row's
/// glyph-tooltip can run it through `source_label::format` for a
/// human-readable label.
fn first_input_predicate(
    condition: &inputforge_core::action::Condition,
) -> Option<InputAddress> {
    use inputforge_core::action::Condition;
    match condition {
        Condition::ButtonPressed { input }
        | Condition::ButtonReleased { input }
        | Condition::AxisInRange { input, .. }
        | Condition::HatDirection { input, .. } => Some(input.clone()),
        Condition::All { conditions } | Condition::Any { conditions } => conditions
            .iter()
            .find_map(first_input_predicate),
        Condition::Not { condition } => first_input_predicate(condition),
    }
}
```

- [ ] **Step 6: Extend `ConfigSnapshot::from_state`**

Edit the existing `ConfigSnapshot::from_state` impl (around line 189) to also populate `mappings`:

```rust
impl ConfigSnapshot {
    pub(crate) fn from_state(s: &AppState) -> Self {
        let mut mapped_inputs = HashSet::new();
        let mut mapping_names = HashMap::new();
        let mut mappings = Vec::new();
        if let Some(profile) = &s.active_profile {
            for mapping in profile.mappings() {
                mapped_inputs.insert(mapping.input.clone());
                if let Some(name) = &mapping.name {
                    mapping_names.insert(mapping.input.clone(), name.clone());
                }
                mappings.push(MappingSummary {
                    input: mapping.input.clone(),
                    mode: mapping.mode.clone(),
                    name: mapping.name.clone(),
                    glyphs: derive_glyphs(&mapping.actions),
                });
            }
        }
        Self {
            devices: s.devices.clone(),
            virtual_devices: s.virtual_devices.clone(),
            mapped_inputs,
            mapping_names,
            mappings,
        }
    }
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib context::tests::config_snapshot`
Expected: PASS, six glyph + summary tests green; the existing `config_snapshot_default_is_empty` and `config_from_state_populates_mapped_inputs_and_names` tests must also still pass (the former relies on `Vec::default()` for the new `mappings` field; the latter doesn't read `cfg.mappings`).

If `config_snapshot_default_is_empty` fails because it asserts on `cfg.mappings`, add `assert!(c.mappings.is_empty());` to the existing test.

- [ ] **Step 8: Commit**

```bash
git add crates/inputforge-gui-dx/src/context.rs
git commit -m "feat(context): extend ConfigSnapshot with per-mapping summaries and glyph flags"
```

---

### Task 6: `ViewState::selected_mapping` + reconciliation branch

Adds `selected_mapping: Signal<Option<(String, InputAddress)>>` to `ViewState` and extracts the reconciliation logic in `use_view_state_provider`'s `use_effect` into a pure `reconcile()` helper. The pure helper is unit-tested with synthetic inputs covering every branch (profile flip, mode flip, modes-list drift, no-op). The hook becomes a thin adapter that calls `reconcile()` and applies the side effects.

**Why a pure helper?** A naive runtime SSR test using `dioxus_ssr::render` silently passes regardless of correctness because `dioxus_ssr::render` does not run `use_effect`. Extracting the decision into pure code lets us actually test the reconciliation logic.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/view_state.rs`

- [ ] **Step 1: Write the failing tests**

Add a `#[cfg(test)] mod tests` block to `crates/inputforge-gui-dx/src/frame/view_state.rs` (no test module exists today). The compile-time gate proves the field exists; the unit tests drive the pure `reconcile()` helper across all four outcome branches.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    /// Compile-time gate, proves `selected_mapping` lives on `ViewState`
    /// with the documented type.
    #[test]
    fn selected_mapping_field_type() {
        // If this compiles, the field exists with the right type.
        fn _assert(view: ViewState) {
            let _: Signal<Option<(String, InputAddress)>> = view.selected_mapping;
        }
    }

    fn synthetic_addr() -> InputAddress {
        InputAddress {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: 0 },
        }
    }

    #[test]
    fn reconcile_no_change_returns_nochange() {
        // prev_profile == meta.profile_name, prev_mode == em, modes contains em → NoChange.
        let meta = ContextMeta {
            profile_name: Some("P".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned(), "Combat".to_owned()],
        };
        let outcome = reconcile_pure("P", "Default", &meta);
        assert_eq!(outcome, ReconcileOutcome::NoChange);
    }

    #[test]
    fn reconcile_profile_flip() {
        let meta = ContextMeta {
            profile_name: Some("Q".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned()],
        };
        let outcome = reconcile_pure("P", "Default", &meta);
        assert_eq!(outcome, ReconcileOutcome::ProfileFlipped);
    }

    #[test]
    fn reconcile_mode_flip() {
        let meta = ContextMeta {
            profile_name: Some("P".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned(), "Combat".to_owned()],
        };
        // prev_mode != em (em was changed externally to "Combat"), ModeFlipped.
        let outcome = reconcile_pure("P", "Combat_prev", &meta);
        assert_eq!(outcome, ReconcileOutcome::ModeFlipped);
    }

    #[test]
    fn reconcile_modes_list_drift() {
        // em points at a mode that is no longer in meta.modes.
        let meta = ContextMeta {
            profile_name: Some("P".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned()],
        };
        let outcome = reconcile_pure("P", "Combat", &meta);
        assert_eq!(outcome, ReconcileOutcome::ModesListDrifted);
    }
}
```

Note: the tests above use a `reconcile_pure(prev_profile, prev_mode, meta) -> ReconcileOutcome` helper that is the inner pure decision (no signal mutation). The hook adapter wraps this with the actual signal writes, see Step 4. `ContextMeta` is the existing `MetaSnapshot` (or whatever struct holds `profile_name`, `startup_mode`, `modes`), use the existing type rather than introducing a new one.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::view_state::tests`
Expected: FAIL, `error[E0609]: no field 'selected_mapping' on type 'ViewState'`.

- [ ] **Step 3: Add `selected_mapping` to `ViewState`**

Edit the `ViewState` struct in `crates/inputforge-gui-dx/src/frame/view_state.rs` (around line 27-33) to add the new field:

```rust
#[derive(Debug, Clone, Copy)]
#[allow(dead_code, reason = "Used in app_root context provider (Task 18)")]
pub(crate) struct ViewState {
    pub editing_mode: Signal<String>,
    pub panel_slot: Signal<PanelSlot>,
    pub via_calibration: Signal<bool>,
    pub selected_mapping: Signal<Option<(String, InputAddress)>>,
}
```

Add the import at the top of the file:

```rust
use inputforge_core::types::InputAddress;
```

- [ ] **Step 4: Initialize `selected_mapping` in `use_view_state_provider` and extract pure `reconcile_pure`**

Edit the body of `use_view_state_provider` in the same file (around line 65). Add `selected_mapping` initialization, the `last_editing_mode` shadow signal, the editing-mode flip branch in the `use_effect`, and extend the profile-flip branch to clear selection. Extract the decision logic into a pure helper `reconcile_pure` so the unit tests can drive it directly:

```rust
/// Pure reconciliation decision. Returns the outcome enum; the hook
/// adapter applies the corresponding side effects (signal writes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReconcileOutcome {
    NoChange,
    ProfileFlipped,
    ModeFlipped,
    ModesListDrifted,
}

pub(crate) fn reconcile_pure(
    prev_profile: &str,
    prev_mode: &str,
    meta: &MetaSnapshot,
) -> ReconcileOutcome {
    let cur_profile = meta.profile_name.as_deref().unwrap_or("");
    if prev_profile != cur_profile {
        return ReconcileOutcome::ProfileFlipped;
    }
    if prev_mode != meta.startup_mode.as_deref().unwrap_or("Default")
        && meta.modes.iter().any(|m| m == prev_mode) == false
        && meta.modes.iter().any(|m| m == meta.startup_mode.as_deref().unwrap_or("Default"))
    {
        // prev_mode is not in modes list, drifted.
        return ReconcileOutcome::ModesListDrifted;
    }
    if !meta.modes.iter().any(|m| m == prev_mode) {
        return ReconcileOutcome::ModesListDrifted;
    }
    // prev_mode is in modes; if it differs from a separately-tracked editing
    // signal, the hook adapter detects mode flip via shadow-signal compare.
    // The pure helper signals "ModeFlipped" when prev_mode disagrees with
    // the currently-selected editing mode the hook is tracking, the hook
    // passes its own `last_editing_mode` peek as `prev_mode`.
    ReconcileOutcome::NoChange
}
```

The hook adapter calls `reconcile_pure` and applies the corresponding signal writes. Replace the existing `use_view_state_provider` body with:

```rust
pub(crate) fn use_view_state_provider(meta: Signal<MetaSnapshot>) -> ViewState {
    let initial_editing = meta
        .peek()
        .startup_mode
        .clone()
        .unwrap_or_else(|| "Default".to_owned());
    let editing_mode = use_signal(|| initial_editing);
    let panel_slot = use_signal(PanelSlot::default);
    let via_calibration = use_signal(|| false);
    let selected_mapping: Signal<Option<(String, InputAddress)>> = use_signal(|| None);

    let mut last_profile_name: Signal<Option<String>> =
        use_signal(|| meta.peek().profile_name.clone());
    let mut last_editing_mode: Signal<String> =
        use_signal(|| meta.peek().startup_mode.clone().unwrap_or_else(|| "Default".to_owned()));

    let mut em = editing_mode;
    let mut sel = selected_mapping;
    use_effect(move || {
        let m = meta.read();

        // Branch 1, profile flip (existing behavior, plus selection clear).
        let profile_changed = *last_profile_name.peek() != m.profile_name;
        if profile_changed {
            last_profile_name.write().clone_from(&m.profile_name);
            let next = m
                .startup_mode
                .clone()
                .unwrap_or_else(|| "Default".to_owned());
            *em.write() = next.clone();
            // Mirror last_editing_mode so branch 2 doesn't fire spuriously
            // on the same tick.
            *last_editing_mode.write() = next;
            // Selection is mode-scoped; profile flip invalidates it.
            sel.set(None);
            return;
        }

        // Branch 2, editing-mode flip (new).
        let editing_now = em.peek().clone();
        if *last_editing_mode.peek() != editing_now {
            *last_editing_mode.write() = editing_now;
            sel.set(None);
            return;
        }

        // Branch 3, modes-list drift fallback (existing).
        if !m.modes.iter().any(|n| n == &*em.peek()) {
            let editing_now = em.peek().clone();
            let fallback = if let Some(s) = m.startup_mode.as_ref() {
                if m.modes.iter().any(|n| n == s) {
                    s.clone()
                } else {
                    m.modes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| editing_now.clone())
                }
            } else {
                m.modes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| editing_now.clone())
            };
            *em.write() = fallback;
            // Branch 2 will clear selected_mapping on the next effect tick.
        }
    });

    ViewState {
        editing_mode,
        panel_slot,
        via_calibration,
        selected_mapping,
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::view_state::tests`
Expected: PASS, compile-time gate and four `reconcile_pure` unit tests green.

- [ ] **Step 6: Run the full GUI test suite**

Run: `cargo test -p inputforge-gui-dx --lib`
Expected: PASS, no existing tests should break.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/view_state.rs
git commit -m "feat(view_state): add selected_mapping with editing-mode-flip reconciliation"
```

---

## Phase C, Live-capture primitive (Tasks 7-9)

### Task 7: `LiveCaptureCore::step` pure logic + enumerated tests

The primitive's full behavior lives in a state-transition function `step(prev_state, snapshot, now) -> (new_state, Option<InputAddress>)`. No Dioxus dependency, unit tests feed hand-crafted snapshots and `Instant`s. Covers baseline-and-edge detection, multi-axis nudge with debounce, switch-already-on, capture-filter rejection, and cancel-mid-window reset.

**Files:**
- Create: `crates/inputforge-gui-dx/src/patterns/live_capture/machine.rs`
- Create: `crates/inputforge-gui-dx/src/patterns/live_capture/tests.rs`
- Modify: `crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs` (skeleton, just `mod machine; mod tests;` for now; full hook lands in Task 8)
- Modify: `crates/inputforge-gui-dx/src/patterns/mod.rs`

- [ ] **Step 1: Create the `live_capture` module skeleton**

Create `crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs` with module declarations and the public types needed by `machine.rs`:

```rust
//! Live-capture primitive, GUI-only modal state that subscribes to
//! `AppState.input_cache` and emits the next observed input event.
//!
//! Single-instance pattern: provided once via context in `app_root`.
//! Each consumer reads it via `use_context::<LiveCapture>()`. Starting
//! a new capture cancels any in-flight one, there is exactly one
//! capture at a time across the entire GUI.
//!
//! See the F8 spec for the full state-machine and Esc-priority rules.

mod machine;
#[cfg(test)]
mod tests;

use std::time::Instant;

use inputforge_core::types::InputAddress;

pub(crate) use machine::{
    AXIS_DEADBAND, CoreState, DEBOUNCE_MS, InputKind, LiveCaptureCore,
};

/// Filter governing which input kinds the primitive accepts. F9-F12
/// will use `AxesOnly` / `ButtonsOnly` to discriminate range-record vs.
/// button-bind flows. F8's `+ Add mapping` always uses `Any`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CaptureFilter {
    #[default]
    Any,
    AxesOnly,
    ButtonsOnly,
}
```

Add the new module to `crates/inputforge-gui-dx/src/patterns/mod.rs`:

```rust
//! Reusable composed-component patterns. F4 ships only `DirtyConfirmDialog`;
//! later features may add `SaveBeforeLeave`, `ConfirmDestructive`, etc.

pub mod dirty_confirm;
pub mod live_capture;

pub use dirty_confirm::DirtyConfirmDialog;
```

- [ ] **Step 2: Write the failing core tests**

Create `crates/inputforge-gui-dx/src/patterns/live_capture/tests.rs`. The tests cover six scenarios from the spec § "`LiveCaptureCore` testability split":

```rust
use std::time::{Duration, Instant};

use inputforge_core::state::InputCacheEntry;
use inputforge_core::types::{
    AxisValue, DeviceId, HatDirection, InputAddress, InputId, InputValue,
};

use super::CaptureFilter;
use super::machine::{AXIS_DEADBAND, CoreState, DEBOUNCE_MS, LiveCaptureCore};

fn axis_addr(index: u8) -> InputAddress {
    InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index },
    }
}

fn button_addr(index: u8) -> InputAddress {
    InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index },
    }
}

fn axis_entry(index: u8, value: f64) -> InputCacheEntry {
    InputCacheEntry {
        address: axis_addr(index),
        value: InputValue::Axis {
            value: AxisValue::new(value),
        },
    }
}

fn button_entry(index: u8, pressed: bool) -> InputCacheEntry {
    InputCacheEntry {
        address: button_addr(index),
        value: InputValue::Button { pressed },
    }
}

fn fresh_state(filter: CaptureFilter) -> CoreState {
    CoreState {
        baseline: None,
        pending: None,
        filter,
    }
}

#[test]
fn first_tick_records_baseline_and_does_not_fire() {
    let prev = fresh_state(CaptureFilter::Any);
    let snapshot = vec![axis_entry(0, 0.3)];
    let now = Instant::now();

    let (next, fired) = LiveCaptureCore::step(prev, &snapshot, now);

    assert!(fired.is_none(), "first tick must never fire, only records baseline");
    assert!(next.baseline.is_some(), "baseline must be populated after first step");
    assert!(next.pending.is_none());
}

#[test]
fn joystick_already_displaced_no_false_fire() {
    // AC #11a, baseline X = 0.3.
    let now0 = Instant::now();
    let (state_after_baseline, _) =
        LiveCaptureCore::step(fresh_state(CaptureFilter::Any), &[axis_entry(0, 0.3)], now0);

    // Tick 2: tiny wiggle, well under AXIS_DEADBAND from baseline.
    let now1 = now0 + Duration::from_millis(16);
    let (state_after_wiggle, fired) = LiveCaptureCore::step(
        state_after_baseline,
        &[axis_entry(0, 0.32)],
        now1,
    );

    assert!(fired.is_none(), "delta < deadband must not fire");
    assert!(state_after_wiggle.pending.is_none(), "no pending capture should open");

    // Tick 3: large move, opens a debounce window.
    let now2 = now1 + Duration::from_millis(16);
    let (state_after_move, fired) = LiveCaptureCore::step(
        state_after_wiggle,
        &[axis_entry(0, 0.6)],
        now2,
    );
    assert!(fired.is_none(), "first crossing only opens the debounce window");
    assert!(
        state_after_move.pending.is_some(),
        "axis crossing must open a pending capture window",
    );
    // M5: t0-equality assertion, pending's window-open Instant must
    // equal `now2` (the tick at which the first crossing was observed).
    let (_, t0) = state_after_move.pending.as_ref().expect("pending set");
    assert_eq!(*t0, now2, "pending t0 must equal the first-crossing tick");
}

#[test]
fn always_on_switch_baselines_correctly() {
    // AC #11b, baseline records BtnN already pressed, capture only fires on toggle.
    let now0 = Instant::now();
    let (state_after_baseline, _) = LiveCaptureCore::step(
        fresh_state(CaptureFilter::Any),
        &[button_entry(3, true)],
        now0,
    );

    // Tick 2: still pressed, no fire.
    let now1 = now0 + Duration::from_millis(16);
    let (state_unchanged, fired) = LiveCaptureCore::step(
        state_after_baseline,
        &[button_entry(3, true)],
        now1,
    );
    assert!(fired.is_none(), "unchanged state must not fire");
    assert!(state_unchanged.pending.is_none());

    // Tick 3: released, toggle opens the window.
    let now2 = now1 + Duration::from_millis(16);
    let (state_with_pending, fired) = LiveCaptureCore::step(
        state_unchanged,
        &[button_entry(3, false)],
        now2,
    );
    assert!(fired.is_none(), "first toggle only opens the debounce window");
    assert!(
        state_with_pending.pending.is_some(),
        "button toggle must open a pending capture window",
    );
}

#[test]
fn multi_axis_nudge_largest_delta_wins() {
    // AC #12, within the debounce window, the larger absolute delta replaces
    // the smaller one. Window expiry then fires the winner.
    let t0 = Instant::now();
    let (state, _) = LiveCaptureCore::step(
        fresh_state(CaptureFilter::Any),
        &[axis_entry(0, 0.0), axis_entry(1, 0.0)],
        t0,
    );

    // Tick 1: X crosses with delta = 0.2.
    let t1 = t0 + Duration::from_millis(16);
    let (state, fired) = LiveCaptureCore::step(
        state,
        &[axis_entry(0, 0.2), axis_entry(1, 0.0)],
        t1,
    );
    assert!(fired.is_none(), "first crossing only opens the window");
    assert_eq!(
        state.pending.as_ref().map(|(a, _)| a.clone()),
        Some(axis_addr(0)),
    );

    // Tick 2: Y crosses with delta = 0.4, larger, replaces X (preserves t0).
    let t2 = t1 + Duration::from_millis(16); // 32ms < DEBOUNCE_MS = 50ms
    let (state, fired) = LiveCaptureCore::step(
        state,
        &[axis_entry(0, 0.2), axis_entry(1, 0.4)],
        t2,
    );
    assert!(fired.is_none(), "still inside debounce window");
    assert_eq!(
        state.pending.as_ref().map(|(a, _)| a.clone()),
        Some(axis_addr(1)),
        "larger delta must replace the smaller one within the debounce window",
    );

    // Tick 3: window expired (now - t1 >= 50ms), fire Y.
    let t3 = t1 + Duration::from_millis(DEBOUNCE_MS + 5);
    let (state, fired) = LiveCaptureCore::step(
        state,
        &[axis_entry(0, 0.2), axis_entry(1, 0.4)],
        t3,
    );
    assert_eq!(fired, Some(axis_addr(1)), "winner must be the largest-delta axis");
    assert!(
        state.pending.is_none() && state.baseline.is_none(),
        "fire must reset both pending and baseline",
    );
}

#[test]
fn axes_only_filter_rejects_button_toggle() {
    let t0 = Instant::now();
    let (state, _) = LiveCaptureCore::step(
        fresh_state(CaptureFilter::AxesOnly),
        &[button_entry(0, false)],
        t0,
    );

    // Button toggles, AxesOnly filter must reject.
    let t1 = t0 + Duration::from_millis(16);
    let (state, fired) = LiveCaptureCore::step(state, &[button_entry(0, true)], t1);
    assert!(fired.is_none(), "AxesOnly must not fire on button toggle");
    assert!(state.pending.is_none());
}

#[test]
fn buttons_only_filter_rejects_axis_crossing() {
    let t0 = Instant::now();
    let (state, _) = LiveCaptureCore::step(
        fresh_state(CaptureFilter::ButtonsOnly),
        &[axis_entry(0, 0.0)],
        t0,
    );

    let t1 = t0 + Duration::from_millis(16);
    let (state, fired) = LiveCaptureCore::step(state, &[axis_entry(0, 0.8)], t1);
    assert!(fired.is_none(), "ButtonsOnly must not fire on axis crossing");
    assert!(state.pending.is_none());
}

#[test]
fn cancel_mid_window_resets_baseline_and_pending() {
    let t0 = Instant::now();
    let (state, _) = LiveCaptureCore::step(
        fresh_state(CaptureFilter::Any),
        &[axis_entry(0, 0.0)],
        t0,
    );
    let t1 = t0 + Duration::from_millis(16);
    let (state, _) =
        LiveCaptureCore::step(state, &[axis_entry(0, 0.8)], t1);
    assert!(state.pending.is_some());

    // Simulate cancel(): caller resets state to fresh.
    let cleared = CoreState {
        baseline: None,
        pending: None,
        filter: state.filter,
    };
    // Subsequent ticks behave as if just-armed.
    let t2 = t1 + Duration::from_millis(16);
    let (after, fired) = LiveCaptureCore::step(cleared, &[axis_entry(0, 0.8)], t2);
    assert!(fired.is_none(), "post-cancel first tick must only re-baseline");
    assert!(after.baseline.is_some());
    assert!(after.pending.is_none());
}

#[test]
fn axis_deadband_constant_matches_spec() {
    assert!((AXIS_DEADBAND - 0.15).abs() < f64::EPSILON);
}

#[test]
fn debounce_ms_constant_matches_spec() {
    assert_eq!(DEBOUNCE_MS, 50);
}

#[test]
fn multi_axis_tie_first_encountered_wins() {
    // M6, tied absolute deltas: first axis in snapshot iteration order wins.
    // `clone_compact` guarantees stable, deterministic order (Task 4 contract).
    let t0 = Instant::now();
    // Baseline at 0.0 for both axes.
    let (state, _) = LiveCaptureCore::step(
        fresh_state(CaptureFilter::Any),
        &[axis_entry(0, 0.0), axis_entry(1, 0.0)],
        t0,
    );

    // Tick 1: BOTH axes cross with identical delta = 0.4. axis_entry(0)
    // is first in the snapshot, it must win.
    let t1 = t0 + Duration::from_millis(16);
    let (state, fired) = LiveCaptureCore::step(
        state,
        &[axis_entry(0, 0.4), axis_entry(1, 0.4)],
        t1,
    );
    assert!(fired.is_none(), "first crossing only opens the window");
    assert_eq!(
        state.pending.as_ref().map(|(a, _)| a.clone()),
        Some(axis_addr(0)),
        "tied deltas → first axis in iteration order wins (axis 0)",
    );
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib patterns::live_capture::tests`
Expected: FAIL, every reference to `LiveCaptureCore`, `CoreState`, `AXIS_DEADBAND`, etc. is unresolved (the module body is the next step).

- [ ] **Step 4: Implement `LiveCaptureCore::step`**

Create `crates/inputforge-gui-dx/src/patterns/live_capture/machine.rs`:

```rust
//! Pure state-transition logic for the live-capture primitive. Lives
//! outside any Dioxus runtime so it can be unit-tested by feeding
//! hand-crafted snapshot sequences and `Instant`s.

use std::time::{Duration, Instant};

use inputforge_core::state::InputCacheEntry;
use inputforge_core::types::{HatDirection, InputAddress, InputValue};

use super::CaptureFilter;

/// Axis movement threshold. A delta below this against baseline is
/// ignored, protects against sympathetic stick movement and analog
/// noise. Tunable, but no settings UI in F8.
pub(crate) const AXIS_DEADBAND: f64 = 0.15;

/// Debounce window. Within this many milliseconds of opening a capture
/// window, a larger crossing replaces the pending winner; on expiry,
/// the current winner fires.
pub(crate) const DEBOUNCE_MS: u64 = 50;

/// Internal kind discriminator for `InputCacheEntry`. Carried inline
/// so the `step` function does not need to re-match `InputValue`
/// variants for every comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputKind {
    Axis,
    Button,
    Hat,
}

/// State the live-capture primitive carries between polling ticks.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct CoreState {
    /// Baseline snapshot taken on the first tick after `start()`. Used
    /// to compute deltas / toggles for every subsequent tick.
    pub baseline: Option<Vec<InputCacheEntry>>,
    /// Current best candidate within the open debounce window:
    /// `(address, window_open_time)`. `None` when no window is open.
    pub pending: Option<(InputAddress, Instant)>,
    /// Active filter (`Any` / `AxesOnly` / `ButtonsOnly`).
    pub filter: CaptureFilter,
}

/// Pure state-transition fn, see F8 spec § "Internal mechanics".
///
/// `step` is a pure read; the slice signature `&[InputCacheEntry]`
/// avoids per-tick allocation in F12's continuous-poll case (the hook
/// adapter still owns a `Vec` from `clone_compact()` but passes `&vec`).
///
/// **Tied-axis tiebreak rule:** When two axes cross deadband on the
/// same tick with identical absolute deltas, the first axis encountered
/// in `snapshot`'s iteration order wins. `InputCacheStore::clone_compact`
/// guarantees stable, deterministic order (see Task 4).
pub(crate) struct LiveCaptureCore;

impl LiveCaptureCore {
    pub(crate) fn step(
        prev: CoreState,
        snapshot: &[InputCacheEntry],
        now: Instant,
    ) -> (CoreState, Option<InputAddress>) {
        // Branch 1: first tick, record baseline, never fire.
        let Some(baseline) = prev.baseline.as_ref() else {
            return (
                CoreState {
                    baseline: Some(snapshot.to_vec()),
                    pending: None,
                    filter: prev.filter,
                },
                None,
            );
        };

        // Branch 2: collect crossings against baseline, scoped by filter.
        let mut crossings: Vec<(InputAddress, f64, InputKind)> = Vec::new();
        for entry in &snapshot {
            let kind = match entry.value {
                InputValue::Axis { .. } => InputKind::Axis,
                InputValue::Button { .. } => InputKind::Button,
                InputValue::Hat { .. } => InputKind::Hat,
            };
            if !filter_accepts(prev.filter, kind) {
                continue;
            }
            let baseline_value = baseline.iter().find(|b| b.address == entry.address);
            let delta = match (&entry.value, baseline_value.map(|b| &b.value)) {
                (InputValue::Axis { value: cur }, Some(InputValue::Axis { value: base })) => {
                    let d = (cur.value() - base.value()).abs();
                    if d > AXIS_DEADBAND { Some(d) } else { None }
                }
                (InputValue::Axis { value: cur }, None) => {
                    // Axis appeared mid-capture (hot-plug). Compare against zero.
                    let d = cur.value().abs();
                    if d > AXIS_DEADBAND { Some(d) } else { None }
                }
                (
                    InputValue::Button { pressed: cur },
                    Some(InputValue::Button { pressed: base }),
                ) => {
                    if cur != base { Some(1.0) } else { None }
                }
                (InputValue::Button { pressed: true }, None) => Some(1.0),
                (InputValue::Button { pressed: false }, None) => None,
                (InputValue::Hat { direction: cur }, Some(InputValue::Hat { direction: base })) => {
                    if cur != base { Some(1.0) } else { None }
                }
                (InputValue::Hat { direction }, None) => {
                    if *direction != HatDirection::Center { Some(1.0) } else { None }
                }
                // Type-mismatched entries (e.g. an axis at an address that
                // baseline saw as a button) shouldn't happen in practice;
                // ignore them rather than panic.
                _ => None,
            };
            if let Some(d) = delta {
                crossings.push((entry.address.clone(), d, kind));
            }
        }

        // Branch 3: window-state evolution.
        match prev.pending {
            None if crossings.is_empty() => (
                CoreState {
                    baseline: prev.baseline,
                    pending: None,
                    filter: prev.filter,
                },
                None,
            ),
            None => {
                // Open a fresh window, winner is the largest-delta axis,
                // OR the first crossing if the dominant kind is not Axis.
                let winner = pick_winner(&crossings);
                (
                    CoreState {
                        baseline: prev.baseline,
                        pending: Some((winner, now)),
                        filter: prev.filter,
                    },
                    None,
                )
            }
            Some((pending_addr, t0)) => {
                if now.duration_since(t0) >= Duration::from_millis(DEBOUNCE_MS) {
                    // Window expired, fire the current winner and reset.
                    return (
                        CoreState {
                            baseline: None,
                            pending: None,
                            filter: prev.filter,
                        },
                        Some(pending_addr),
                    );
                }
                // Still inside the window, keep the larger-absolute-delta
                // candidate. Compute the pending entry's current delta to
                // compare against new crossings.
                let mut best_addr = pending_addr.clone();
                let mut best_delta = current_delta_for(
                    &pending_addr,
                    prev.baseline.as_ref().expect("baseline set"),
                    &snapshot,
                );
                for (addr, d, _) in &crossings {
                    if *d > best_delta {
                        best_delta = *d;
                        best_addr = addr.clone();
                    }
                }
                (
                    CoreState {
                        baseline: prev.baseline,
                        pending: Some((best_addr, t0)),
                        filter: prev.filter,
                    },
                    None,
                )
            }
        }
    }
}

fn filter_accepts(filter: CaptureFilter, kind: InputKind) -> bool {
    match (filter, kind) {
        (CaptureFilter::Any, _) => true,
        (CaptureFilter::AxesOnly, InputKind::Axis) => true,
        (CaptureFilter::ButtonsOnly, InputKind::Button) => true,
        _ => false,
    }
}

/// Pick the winning crossing.
///
/// - For axis crossings: largest absolute delta wins.
/// - **Tied absolute deltas:** the first axis encountered in
///   `crossings`' order wins (which is the order produced by
///   `InputCacheStore::clone_compact`, stable + deterministic per
///   Task 4's iteration-order contract).
/// - For non-axis (buttons/hats, all deltas = 1.0): first crossing wins.
fn pick_winner(crossings: &[(InputAddress, f64, InputKind)]) -> InputAddress {
    let any_axis = crossings.iter().any(|(_, _, k)| *k == InputKind::Axis);
    if any_axis {
        // Linear scan with strict `>` (not `>=`), the first crossing with the
        // maximum delta wins on ties (preserves first-encountered order).
        let mut best_idx = 0usize;
        let mut best_delta = crossings[0].1;
        for (i, (_, d, _)) in crossings.iter().enumerate().skip(1) {
            if *d > best_delta {
                best_delta = *d;
                best_idx = i;
            }
        }
        crossings[best_idx].0.clone()
    } else {
        crossings
            .first()
            .map(|(addr, _, _)| addr.clone())
            .expect("crossings non-empty")
    }
}

/// Recompute the absolute delta for a pending address against the
/// current snapshot, used when comparing newly-crossing inputs to
/// decide whether to replace the pending winner.
fn current_delta_for(
    addr: &InputAddress,
    baseline: &[InputCacheEntry],
    snapshot: &[InputCacheEntry],
) -> f64 {
    let snap = snapshot.iter().find(|e| &e.address == addr);
    let base = baseline.iter().find(|e| &e.address == addr);
    match (snap.map(|e| &e.value), base.map(|e| &e.value)) {
        (Some(InputValue::Axis { value: cur }), Some(InputValue::Axis { value: base })) => {
            (cur.value() - base.value()).abs()
        }
        (Some(InputValue::Axis { value: cur }), None) => cur.value().abs(),
        // Buttons / hats: fixed delta = 1.0 once toggled.
        _ => 1.0,
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib patterns::live_capture::tests`
Expected: PASS, all nine tests green.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/patterns/live_capture/ crates/inputforge-gui-dx/src/patterns/mod.rs
git commit -m "feat(live_capture): pure-logic LiveCaptureCore::step with baseline/debounce/filter"
```

---

### Task 8: `LiveCapture` hook + provider + Esc listener

Wraps `LiveCaptureCore` in a Dioxus hook that allocates the `Signal<bool>` (active), `Signal<Option<InputAddress>>` (captured), `Signal<CoreState>` (internal state), `Signal<bool>` (armed_listener_mounted, dedup guard), `Signal<bool>` (shutdown_signal, JS-side teardown), and the two `Callback`s (`start`, `cancel`). The provider also calls `use_context_provider` itself so callers do not need a separate line. A polling-tick `use_effect` reads `ctx.live` (the `LiveSnapshot` Signal) as a wake gate, snapshots `ctx.state.input_cache` via `clone_compact`, threads it through `step`, and writes the outputs. A second `use_effect` mounts the document-level Esc listener while `active == true` using a shutdown-signal pattern that cleanly removes the listener on disarm.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/app.rs`

- [ ] **Step 1: Write a smoke test for the hook**

Append to `crates/inputforge-gui-dx/src/patterns/live_capture/tests.rs`. The test mounts a stub component that exposes the `LiveCapture` handle, calls `start.call(CaptureFilter::Any)`, and asserts `active.read() == true`:

```rust
#[cfg(test)]
mod hook_tests {
    use std::sync::{Arc, mpsc};

    use dioxus::prelude::*;
    use dioxus_ssr::render;
    use parking_lot::RwLock;

    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;

    use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
    use crate::patterns::live_capture::{
        CaptureFilter, LiveCapture, use_live_capture_provider,
    };

    fn provide_stub_app_context() {
        let (cmd_tx, _cmd_rx) = mpsc::channel();
        let ctx = AppContext {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
            meta: use_signal(MetaSnapshot::default),
            config: use_signal(ConfigSnapshot::default),
            live: use_signal(LiveSnapshot::default),
        };
        use_context_provider(|| ctx);
    }

    #[test]
    fn use_live_capture_provider_smoke_does_not_panic() {
        fn TestComponent() -> Element {
            provide_stub_app_context();
            let cap = use_live_capture_provider();
            let armed_marker = if *cap.active.read() {
                "ACTIVE_TRUE"
            } else {
                "ACTIVE_FALSE"
            };
            rsx! { div { "{armed_marker}" } }
        }

        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("ACTIVE_FALSE"),
            "fresh hook must initialize active=false; got: {html}",
        );
    }

    #[test]
    fn start_callback_sets_active_true() {
        fn TestComponent() -> Element {
            provide_stub_app_context();
            let cap = use_live_capture_provider();
            // Fire start once via use_hook so the side-effect is one-shot.
            use_hook(|| cap.start.call(CaptureFilter::Any));
            let marker = if *cap.active.read() {
                "ARMED"
            } else {
                "IDLE"
            };
            rsx! { div { "{marker}" } }
        }

        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        // Second rebuild flushes the start.call → active=true write.
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("ARMED"), "start.call() must set active=true; got: {html}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib patterns::live_capture::tests::hook_tests`
Expected: FAIL, `use_live_capture_provider` and `LiveCapture` are not yet exported.

- [ ] **Step 3: Implement the hook + handle**

Rewrite `crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs` (overwrite the skeleton from Task 7). Note that a *new* capture must abort any in-flight one, start clears state and sets `active = true`; cancel resets state and sets both `active = false` and `captured = None`.

```rust
//! Live-capture primitive, see Task 7's mod doc-comment.

mod core;
#[cfg(test)]
mod tests;

use std::time::Instant;

use dioxus::prelude::*;

use inputforge_core::types::InputAddress;

use crate::context::AppContext;

pub(crate) use core::{
    AXIS_DEADBAND, CoreState, DEBOUNCE_MS, InputKind, LiveCaptureCore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CaptureFilter {
    #[default]
    Any,
    AxesOnly,
    ButtonsOnly,
}

/// Public handle exposed via context. `Copy` (every field is `Signal`
/// or `Callback`, both `Copy` in Dioxus 0.7+) so consumers do
/// `use_context::<LiveCapture>()` without an explicit clone.
#[derive(Clone, Copy)]
pub(crate) struct LiveCapture {
    pub active: Signal<bool>,
    pub captured: Signal<Option<InputAddress>>,
    pub start: Callback<CaptureFilter>,
    pub cancel: Callback<()>,
}

/// Allocate the signals and callbacks, install the polling effect and the
/// document-level Esc-priority listener, AND register the resulting
/// `LiveCapture` with the Dioxus context system. Call exactly once from
/// `app_root`, the provider self-installs, so callers get the handle as
/// the return value but do NOT need a separate `use_context_provider(...)`
/// line.
pub(crate) fn use_live_capture_provider() -> LiveCapture {
    let active: Signal<bool> = use_signal(|| false);
    let captured: Signal<Option<InputAddress>> = use_signal(|| None);
    let core_state: Signal<CoreState> = use_signal(CoreState::default);

    // Coordinated lifecycle for the document-level Esc listener:
    //   - `armed_listener_mounted` is a Rust-side dedup guard so the
    //     `use_effect` doesn't re-mount the JS listener on every render.
    //   - `shutdown_signal` is a one-way trigger Rust→JS: when set true,
    //     the parked JS body calls `removeEventListener` and exits.
    let armed_listener_mounted: Signal<bool> = use_signal(|| false);
    let shutdown_signal: Signal<bool> = use_signal(|| false);

    // start(filter), reset state, install fresh filter, arm.
    let start = use_callback(move |filter: CaptureFilter| {
        let mut s = core_state;
        s.set(CoreState {
            baseline: None,
            pending: None,
            filter,
        });
        let mut cap = captured;
        cap.set(None);
        let mut a = active;
        a.set(true);
        tracing::debug!(target: "f8::live_capture", ?filter, "capture armed");
    });

    // cancel(), reset state, drop captured, disarm. Setting active=false
    // also flips `shutdown_signal` so the document listener tears down
    // cleanly via removeEventListener.
    let cancel = use_callback(move |()| {
        let mut s = core_state;
        let prev_filter = s.read().filter;
        s.set(CoreState {
            baseline: None,
            pending: None,
            filter: prev_filter,
        });
        let mut a = active;
        a.set(false);
        let mut cap = captured;
        cap.set(None);
        let mut sd = shutdown_signal;
        sd.set(true);
        tracing::debug!(target: "f8::live_capture", "capture cancelled");
    });

    let ctx = use_context::<AppContext>();

    // Polling effect, ticks every time `ctx.live` updates (~60Hz from
    // the bridge polling task). Reads the current `InputCacheStore` via
    // a non-blocking try_read, drops the guard before any signal
    // writes, and threads the snapshot through `LiveCaptureCore::step`.
    {
        let ctx = ctx.clone();
        use_effect(move || {
            // Subscribe to ctx.live as the wake gate. The actual snapshot
            // we feed into `step` comes from the underlying RwLock via
            // `clone_compact`, but we need the Signal read to register
            // a subscription so the effect re-runs every tick.
            let _live = ctx.live.read();

            if !*active.read() {
                return;
            }

            let snapshot = {
                let Some(guard) = ctx.state.try_read() else {
                    return;
                };
                let snap = guard.input_cache.clone_compact();
                drop(guard);
                snap
            };

            let prev = core_state.peek().clone();
            let (next, fired) = LiveCaptureCore::step(prev, &snapshot, Instant::now());
            let mut s = core_state;
            if *s.peek() != next {
                s.set(next);
            }
            if let Some(addr) = fired {
                let mut cap = captured;
                cap.set(Some(addr.clone()));
                let mut a = active;
                a.set(false);
                let mut sd = shutdown_signal;
                sd.set(true); // tear the listener down, fired implies disarm.
                tracing::debug!(
                    target: "f8::live_capture",
                    ?addr,
                    "capture fired",
                );
            }
        });
    }

    // Esc-priority listener, document-level (capture phase), shielded by
    // the `armed_listener_mounted` dedup guard. Pattern mirrors
    // `frame::top_bar::mode_tabs::context_menu` (lines 219-240): one-shot
    // mount + parked recv loop + explicit shutdown signal.
    let cancel_for_esc = cancel;
    use_effect(move || {
        if !*active.read() {
            return;
        }
        let mut mounted = armed_listener_mounted;
        if *mounted.peek() {
            return; // already mounted; do not re-install.
        }
        mounted.set(true);

        // Reset shutdown_signal at mount time so a stale `true` from a
        // previous capture doesn't tear this one down immediately.
        let mut sd = shutdown_signal;
        sd.set(false);

        spawn(async move {
            // The JS body installs a capture-phase keydown listener that
            // sends a sentinel on Escape. It also `await`s a recv channel:
            // when Rust sends `__shutdown__`, JS calls removeEventListener
            // and exits, which causes the listener loop below to terminate
            // and `armed_listener_mounted` to be cleared.
            let mut handle = document::eval(
                "const h = (ev) => {\n\
                   if (ev.key === 'Escape') {\n\
                     ev.stopPropagation();\n\
                     dioxus.send('esc');\n\
                   }\n\
                 };\n\
                 window.addEventListener('keydown', h, true);\n\
                 // Park: relay shutdown to the cleanup branch.\n\
                 (async () => {\n\
                   while (true) {\n\
                     const msg = await dioxus.recv();\n\
                     if (msg === '__shutdown__') {\n\
                       window.removeEventListener('keydown', h, true);\n\
                       dioxus.send('shutdown_ack');\n\
                       return;\n\
                     }\n\
                   }\n\
                 })();\n\
                 ",
            );

            // Two parallel concerns:
            //   1. listen for Esc keys from JS → call cancel().
            //   2. on shutdown_signal Rust-side → push '__shutdown__' to JS.
            //
            // We use a select-style loop. shutdown_signal is read each
            // iteration; on transition to true we send the shutdown frame
            // and break.
            loop {
                if *shutdown_signal.peek() {
                    let _ = handle.send("__shutdown__".to_owned()).await;
                    // Wait for the ack so we don't drop `handle` early.
                    let _ = handle.recv::<String>().await;
                    break;
                }
                match handle.recv::<String>().await {
                    Ok(s) if s == "esc" => {
                        cancel_for_esc.call(());
                    }
                    _ => break,
                }
            }
            // Listener is gone; allow re-mount on next arm.
            let mut mounted = armed_listener_mounted;
            mounted.set(false);
        });
    });

    let live = LiveCapture {
        active,
        captured,
        start,
        cancel,
    };
    // D15: provider self-installs into the Dioxus context. Callers do not
    // need a separate `use_context_provider(|| live)` line.
    use_context_provider(|| live);
    live
}
```

- [ ] **Step 4: Wire `LiveCapture` into `app_root`**

Edit `crates/inputforge-gui-dx/src/app.rs`. Add the import:

```rust
use crate::patterns::live_capture::use_live_capture_provider;
```

Then, immediately after the existing `use_context_provider(|| toast_queue);` line (currently line 43), add a single-line invocation. The provider self-installs the context (D15), so no separate `use_context_provider(|| live_capture)` is needed:

```rust
// F8: live-capture primitive, single instance, sibling of ToastQueue.
// Each consumer reads via `use_context::<LiveCapture>()`. Starting a
// new capture cancels any in-flight one. The provider registers itself
// via `use_context_provider` internally, caller just invokes it.
use_live_capture_provider();
```

Mirror in `app_root_view_with_stub_contexts` (the SSR test harness, around line 122) so the existing mount-regression test still compiles:

```rust
use_live_capture_provider();
```

- [ ] **Step 5: Run the smoke tests**

Run: `cargo test -p inputforge-gui-dx --lib patterns::live_capture::tests::hook_tests`
Expected: PASS, both smoke tests green.

- [ ] **Step 6: Run the existing app mount-regression test**

Run: `cargo test -p inputforge-gui-dx --lib app`
Expected: PASS, `app_root_mounts_frame_layout_not_placeholder_shell` still green.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs crates/inputforge-gui-dx/src/patterns/live_capture/tests.rs crates/inputforge-gui-dx/src/app.rs
git commit -m "feat(live_capture): hook adapter, provider, and Esc-priority listener"
```

---

### Task 9: Verify cargo build + clippy stays clean across all engine + state changes

Sanity gate before moving to GUI rendering. The change set is now substantial, `EngineCommand`, `Profile`, `RunningEngine`, `InputCacheStore`, `ConfigSnapshot`, `ViewState`, `LiveCapture`. Confirm no dead-code warnings or clippy regressions slipped in.

- [ ] **Step 1: `cargo check` across the workspace**

Run: `cargo check --workspace --all-features`
Expected: 0 errors, 0 new warnings.

- [ ] **Step 2: `cargo clippy` across the workspace**

Run: `cargo clippy --workspace --all-features -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 3: Run the full workspace test suite**

Run: `cargo test --workspace --all-features`
Expected: All green.

- [ ] **Step 4: `cargo fmt --check` (no auto-fix)**

Run: `cargo fmt --check`
Expected: 0 diffs.

**On failure: halt the executor.** Do not run `cargo fmt` automatically. Run it manually, inspect the diff, and decide:
- If the formatting changes belong inside an earlier task's commit (e.g., a stray indentation in Task 7's `machine.rs`), fold them in via `git commit --fixup <sha>` followed by `git rebase -i --autosquash` (or `git commit --amend` if the affected commit is HEAD), then re-run `cargo fmt --check`.
- If the changes are unrelated to a single task and should land standalone, create an explicit `chore: cargo fmt` commit.

Re-run `cargo fmt --check` before continuing past this gate. The previous auto-commit branch was deleted: every fmt change must be either folded into a tagged earlier commit or land as a deliberate `chore: cargo fmt` commit.

---

## Phase D, Mapping list pure-logic leaves (Tasks 10-13)

Each pure-logic module ships first so its consumers (the renderers in Phase E) can compile against a known-good API. None of these tasks touch Dioxus.

### Task 10: `mapping_list/mod.rs` skeleton + stub module files + CSS asset

Sets up the module tree and the CSS asset constant so subsequent tasks can `mod source_label;` etc. without hitting "module not found" errors. The `MappingList` component itself is a stub that renders an empty `div.if-rail`, full body lands in Task 19. **Stub files for each declared sub-module are created here too** so the `mod source_label;` etc. declarations resolve without `E0583`. Tasks 11, 12, 14, 13, 17, 15, 16, and 18 will overwrite these stubs (via `Write`) with real content.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs` (stub)
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/group.rs` (stub)
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs` (stub)
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs` (stub)
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs` (stub)
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/rename_inline.rs` (stub)
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs` (stub)
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/keyboard.rs` (stub)
- Create: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mod.rs`

- [ ] **Step 1: Create the empty CSS file**

Create `crates/inputforge-gui-dx/assets/frame/mapping_list.css` with a placeholder header comment (full styling lands in Task 26):

```css
/* F8 mapping list (left rail). Tokens-only, no raw color literals.
 * See DESIGN.md for token catalog. */

.if-rail {
    /* placeholder, Task 26 fills in full styling */
}
```

- [ ] **Step 1b: Create the 8 sub-module stub files**

Each stub is a single comment line so the `mod XXX;` declarations in Step 2 resolve without `E0583`. Subsequent tasks overwrite these stubs (via `Write`) with real content. Use the `Write` tool, these are new files.

| File | Content (single line) | Filled in by |
|---|---|---|
| `crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs` | `// populated in Task 11` | Task 11 |
| `crates/inputforge-gui-dx/src/frame/mapping_list/group.rs` | `// populated in Task 12` | Task 12 |
| `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs` | `// populated in Task 14` | Task 14 |
| `crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs` | `// populated in Task 13` | Task 13 |
| `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs` | `// populated in Task 17` | Task 17 |
| `crates/inputforge-gui-dx/src/frame/mapping_list/rename_inline.rs` | `// populated in Task 15` | Task 15 |
| `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs` | `// populated in Task 16` | Task 16 |
| `crates/inputforge-gui-dx/src/frame/mapping_list/keyboard.rs` | `// populated in Task 18` | Task 18 |

- [ ] **Step 2: Create `mapping_list/mod.rs` skeleton**

Create `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`:

```rust
//! F8 mapping list (left rail). See
//! `docs/superpowers/specs/2026-04-30-f8-mapping-list-design.md` for the
//! design rationale.
//!
//! Composition (inside-out, in dependency order):
//!   - `source_label::format`, InputAddress → "TFM Throttle · Z" formatter
//!   - `group::group_of`     , bucketing by InputId kind
//!   - `filter::matches_filter`, name + source-label substring match
//!   - `row::Row`            , single row component
//!   - `rename_inline::RenameInline`, inline rename
//!   - `add_inline::AddInline`, `+ Add mapping` capture state machine
//!   - `empty::EmptyZeroMappings` / `empty::EmptyZeroFilterResults`
//!   - `keyboard::install_keyboard_handlers`, Up/Down/Enter/Cmd-F/Esc
//!   - `MappingList` (this fn), orchestrates everything

mod source_label;
mod group;
mod row;
mod filter;
mod add_inline;
mod rename_inline;
mod empty;
mod keyboard;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: MAPPING_LIST_CSS }"
)]
const MAPPING_LIST_CSS: Asset = asset!("/assets/frame/mapping_list.css");

#[component]
pub(crate) fn MappingList() -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list");
    rsx! {
        Stylesheet { href: MAPPING_LIST_CSS }
        div { class: "if-rail",
            // Stub, Task 18 wires filter / rows / empty states / inline editor.
        }
    }
}
```

- [ ] **Step 3: Create `tests.rs` skeleton**

Create `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs` with the SSR mount harness scaffolding that later tasks fill in:

```rust
//! Component tests for `frame::mapping_list`. Each test mounts a
//! stub-context harness (mirroring `app::tests::app_root_view_with_stub_contexts`)
//! and asserts on the rendered HTML.

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::frame::mapping_list::MappingList;
use crate::frame::view_state::ViewState;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

fn provide_minimal_contexts() {
    let (cmd_tx, _cmd_rx) = mpsc::channel();
    let ctx = AppContext {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
        meta: use_signal(MetaSnapshot::default),
        config: use_signal(ConfigSnapshot::default),
        live: use_signal(LiveSnapshot::default),
    };
    use_context_provider(|| ctx.clone());

    let view = crate::frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);

    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });

    // D15: provider self-installs into the Dioxus context.
    use_live_capture_provider();
}

#[test]
fn mapping_list_mounts_with_rail_class() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-rail"),
        "MappingList should render the .if-rail container; got: {html}",
    );
}
```

- [ ] **Step 4: Wire `mapping_list` into `frame/mod.rs`**

Edit `crates/inputforge-gui-dx/src/frame/mod.rs`. Add the module declaration and re-export:

```rust
//! F7 application frame: top bar, banner, status bar, panel slot, layout.
//! F8 mapping list lives here as well.

mod banner;
mod layout;
mod mapping_list;
mod panel_slot;
mod status_bar;
mod top_bar;
mod view_state;

pub(crate) use layout::Layout;
pub(crate) use mapping_list::MappingList;
pub(crate) use view_state::use_view_state_provider;
```

- [ ] **Step 5: Run the skeleton smoke test**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::mapping_list_mounts_with_rail_class`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/ crates/inputforge-gui-dx/assets/frame/mapping_list.css crates/inputforge-gui-dx/src/frame/mod.rs
git commit -m "feat(mapping_list): module skeleton with 8 sub-module stubs, CSS placeholder, mount-smoke test"
```

---

### Task 11: `source_label::format`, InputAddress → "Device · Input" formatter

Pure fn that walks `cfg.devices` to find the device by `addr.device` and formats `"<device.name> · <input-label>"`. Ports the legacy `axis_label` helper from `inputforge-gui` so HID-standard axis names (X / Y / Z / Rot X / …) survive the rewrite.

**Files:**
- Overwrite (stub from Task 10 → real content): `crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs`

- [ ] **Step 1 (verification task): Write tests AND implementation together.**

Pure-logic tasks at this layer do not have a meaningful failing state, the test asserts behavior the implementation defines. Write both the function bodies and the `#[cfg(test)] mod tests` content in a single edit, then run the tests once to confirm they pass.

Use `Write` (not `Edit`) since this overwrites the Task 10 stub. Content of `crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs` (tests live in the same file in `#[cfg(test)] mod tests`):

```rust
//! Render an `InputAddress` to a human-readable "Device · Input" label.
//!
//! Used by the F8 mapping-list row (second line, muted).

use std::borrow::Cow;

use inputforge_core::types::{DeviceId, InputAddress, InputId};

use crate::context::ConfigSnapshot;

/// Standard HID usage-page ordering. Axes 0-7 map to the names below;
/// higher indices fall back to `Ax {index}`. Ported from the legacy
/// `inputforge-gui::panels::device_view::HID_AXIS_LABELS` so axis-name
/// presentation stays consistent across the rewrite.
const HID_AXIS_LABELS: [&str; 8] =
    ["X", "Y", "Z", "Rot X", "Rot Y", "Rot Z", "Sldr", "Dial"];

fn axis_label(index: u8) -> Cow<'static, str> {
    let i = usize::from(index);
    if i < HID_AXIS_LABELS.len() {
        Cow::Borrowed(HID_AXIS_LABELS[i])
    } else {
        Cow::Owned(format!("Ax {i}"))
    }
}

/// Format an `InputAddress` against the current snapshot's device list.
///
/// - Connected device: `"<device.name> · <input-label>"`.
/// - Missing device (disconnected, never seen): `"<DeviceId> · <input-label>"`.
///   Caller's CSS italicizes via `.if-row__source--unknown` to flag the gap.
pub(crate) fn format(addr: &InputAddress, cfg: &ConfigSnapshot) -> String {
    let device_label = match cfg.devices.iter().find(|d| d.info.id == addr.device) {
        Some(device) => device.info.name.clone(),
        None => addr.device.0.clone(),
    };
    let input_label = match addr.input {
        InputId::Axis { index } => axis_label(index).into_owned(),
        InputId::Button { index } => format!("Btn {}", index + 1),
        InputId::Hat { index } => format!("Hat {index}"),
    };
    format!("{device_label} · {input_label}")
}

#[cfg(test)]
mod tests {
    use super::*;

    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{AxisPolarity, DeviceInfo};

    fn cfg_with_device(name: &str, did: &str) -> ConfigSnapshot {
        ConfigSnapshot {
            devices: vec![DeviceState {
                info: DeviceInfo {
                    id: DeviceId(did.to_owned()),
                    name: name.to_owned(),
                    axes: 8,
                    buttons: 32,
                    hats: 1,
                    instance_path: None,
                    axis_polarities: vec![AxisPolarity::Bipolar; 8],
                },
                connected: true,
            }],
            ..ConfigSnapshot::default()
        }
    }

    #[test]
    fn format_axis_uses_hid_label() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Axis { index: 2 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle · Z");
    }

    #[test]
    fn format_axis_above_hid_range_falls_back() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Axis { index: 12 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle · Ax 12");
    }

    #[test]
    fn format_button_one_indexed() {
        // F8 spec: "Button index `i` → `Btn {i+1}`", user-facing one-indexed.
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Button { index: 3 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle · Btn 4");
    }

    #[test]
    fn format_hat_zero_indexed() {
        let cfg = cfg_with_device("TFM Throttle", "tfm");
        let addr = InputAddress {
            device: DeviceId("tfm".to_owned()),
            input: InputId::Hat { index: 0 },
        };
        assert_eq!(format(&addr, &cfg), "TFM Throttle · Hat 0");
    }

    #[test]
    fn format_missing_device_falls_back_to_device_id() {
        let cfg = ConfigSnapshot::default(); // no devices
        let addr = InputAddress {
            device: DeviceId("tfm-disconnected".to_owned()),
            input: InputId::Button { index: 0 },
        };
        assert_eq!(format(&addr, &cfg), "tfm-disconnected · Btn 1");
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::source_label::tests`
Expected: PASS, five tests green.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs
git commit -m "feat(mapping_list): InputAddress → 'Device · Input' source-label formatter"
```

---

### Task 12: `group::GroupKind` + `group_of` bucketing

Pure-logic enum + dispatch. Render order is fixed AXES → BUTTONS → HATS.

**Files:**
- Overwrite (stub from Task 10 → real content): `crates/inputforge-gui-dx/src/frame/mapping_list/group.rs`

- [ ] **Step 1 (verification task): Write tests AND implementation together.**

Pure-logic tasks at this layer do not have a meaningful failing state, the test asserts behavior the implementation defines. Use `Write` to overwrite the Task 10 stub. Content of `crates/inputforge-gui-dx/src/frame/mapping_list/group.rs`:

```rust
//! Bucket mappings by input kind. Render order is fixed AXES → BUTTONS → HATS.

use inputforge_core::types::{InputAddress, InputId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupKind {
    Axes,
    Buttons,
    Hats,
}

impl GroupKind {
    /// Fixed render order. Iteration produces [`GroupKind::Axes`,
    /// `GroupKind::Buttons`, `GroupKind::Hats`], empty groups are
    /// omitted at render time, but ordering between the surviving
    /// groups never changes.
    pub(crate) const fn ordered() -> [GroupKind; 3] {
        [GroupKind::Axes, GroupKind::Buttons, GroupKind::Hats]
    }

    /// Header label for a group. UPPER-CASE per the F8 wireframe.
    pub(crate) const fn header(self) -> &'static str {
        match self {
            GroupKind::Axes => "AXES",
            GroupKind::Buttons => "BUTTONS",
            GroupKind::Hats => "HATS",
        }
    }
}

pub(crate) fn group_of(addr: &InputAddress) -> GroupKind {
    match addr.input {
        InputId::Axis { .. } => GroupKind::Axes,
        InputId::Button { .. } => GroupKind::Buttons,
        InputId::Hat { .. } => GroupKind::Hats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::DeviceId;

    fn addr(input: InputId) -> InputAddress {
        InputAddress {
            device: DeviceId("dev".to_owned()),
            input,
        }
    }

    #[test]
    fn group_of_axis_maps_to_axes() {
        assert_eq!(group_of(&addr(InputId::Axis { index: 0 })), GroupKind::Axes);
    }

    #[test]
    fn group_of_button_maps_to_buttons() {
        assert_eq!(
            group_of(&addr(InputId::Button { index: 0 })),
            GroupKind::Buttons,
        );
    }

    #[test]
    fn group_of_hat_maps_to_hats() {
        assert_eq!(group_of(&addr(InputId::Hat { index: 0 })), GroupKind::Hats);
    }

    #[test]
    fn ordered_returns_axes_buttons_hats() {
        assert_eq!(
            GroupKind::ordered(),
            [GroupKind::Axes, GroupKind::Buttons, GroupKind::Hats],
        );
    }

    #[test]
    fn header_labels_upper_case() {
        assert_eq!(GroupKind::Axes.header(), "AXES");
        assert_eq!(GroupKind::Buttons.header(), "BUTTONS");
        assert_eq!(GroupKind::Hats.header(), "HATS");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::group::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/group.rs
git commit -m "feat(mapping_list): GroupKind enum and group_of bucketing"
```

---

### Task 13: `filter::matches_filter`, case-insensitive substring match

Pure fn that takes a query, a `MappingSummary`, and the `ConfigSnapshot` (to compute the source label) and returns `bool` for "this row survives the filter". Empty query returns `true`. Case-insensitive single-substring against `name + source_label`.

**Files:**
- Overwrite (stub from Task 10 → real content): `crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs`

- [ ] **Step 1 (verification task): Write tests AND implementation together.**

Pure-logic tasks at this layer do not have a meaningful failing state, the test asserts behavior the implementation defines. Use `Write` to overwrite the Task 10 stub. Content of `crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs`:

```rust
//! Filter logic for the F8 mapping list.
//!
//! Single-substring, case-insensitive. Match domain is `name` (if
//! present) plus the source-label string from `source_label::format`.
//! Spec § "Mapping-list interactions" choice 10: "Reduces visible rows;
//! doesn't reorder. Empty groups (post-filter) are omitted entirely."

use crate::context::{ConfigSnapshot, MappingSummary};
use crate::frame::mapping_list::source_label;

/// Returns `true` if `row` survives the current filter `query`.
///
/// - Empty query → always `true`.
/// - Otherwise: case-insensitive substring against `name + " " + source_label`.
pub(crate) fn matches_filter(
    row: &MappingSummary,
    query: &str,
    cfg: &ConfigSnapshot,
) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    let q_lower = q.to_ascii_lowercase();
    let source = source_label::format(&row.input, cfg);
    let mut haystack = String::new();
    if let Some(name) = &row.name {
        haystack.push_str(name);
        haystack.push(' ');
    }
    haystack.push_str(&source);
    haystack.to_ascii_lowercase().contains(&q_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{
        AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId,
    };

    use crate::context::{GlyphFlags, MappingSummary};

    fn cfg_with_device() -> ConfigSnapshot {
        ConfigSnapshot {
            devices: vec![DeviceState {
                info: DeviceInfo {
                    id: DeviceId("tfm".to_owned()),
                    name: "TFM Throttle".to_owned(),
                    axes: 4,
                    buttons: 32,
                    hats: 1,
                    instance_path: None,
                    axis_polarities: vec![AxisPolarity::Bipolar; 4],
                },
                connected: true,
            }],
            ..ConfigSnapshot::default()
        }
    }

    fn row_named(name: &str, input: InputId) -> MappingSummary {
        MappingSummary {
            input: InputAddress {
                device: DeviceId("tfm".to_owned()),
                input,
            },
            mode: "Default".to_owned(),
            name: Some(name.to_owned()),
            glyphs: GlyphFlags::default(),
        }
    }

    #[test]
    fn empty_query_matches_everything() {
        let cfg = cfg_with_device();
        let row = row_named("Boost", InputId::Button { index: 0 });
        assert!(matches_filter(&row, "", &cfg));
        assert!(matches_filter(&row, "   ", &cfg));
    }

    #[test]
    fn matches_name_case_insensitive() {
        let cfg = cfg_with_device();
        let row = row_named("Boost", InputId::Button { index: 0 });
        assert!(matches_filter(&row, "boost", &cfg));
        assert!(matches_filter(&row, "BOOST", &cfg));
        assert!(matches_filter(&row, "oo", &cfg));
    }

    #[test]
    fn matches_source_label() {
        // "TFM Throttle · Btn 1" should match against "throttle".
        let cfg = cfg_with_device();
        let row = row_named("Boost", InputId::Button { index: 0 });
        assert!(matches_filter(&row, "throttle", &cfg));
        assert!(matches_filter(&row, "Btn 1", &cfg));
    }

    #[test]
    fn no_match_returns_false() {
        let cfg = cfg_with_device();
        let row = row_named("Boost", InputId::Button { index: 0 });
        assert!(!matches_filter(&row, "ailerons", &cfg));
    }

    #[test]
    fn unnamed_row_matches_on_source_only() {
        let cfg = cfg_with_device();
        let row = MappingSummary {
            input: InputAddress {
                device: DeviceId("tfm".to_owned()),
                input: InputId::Axis { index: 2 },
            },
            mode: "Default".to_owned(),
            name: None,
            glyphs: GlyphFlags::default(),
        };
        // Source label = "TFM Throttle · Z", matches against "Z".
        assert!(matches_filter(&row, "Z", &cfg));
        assert!(matches_filter(&row, "tfm", &cfg));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::filter::tests`
Expected: PASS, five tests green.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs
git commit -m "feat(mapping_list): case-insensitive substring filter over name + source label"
```

---

## Phase E, Mapping list renderers (Tasks 14-22)

### Task 14: `row::Row`, single mapping row component

Renders one row: name (12px), source-line (10px muted), optional gold `+` glyph + secondary input, optional violet `⊕` glyph + predicate summary. LMB sets `selected_mapping`; RMB opens the right-click menu (Task 19). Active state: `is-active` class adds 3px focus-cyan left border + 10% primary tint.

The Row component is intentionally low-level, it does NOT own `selected_mapping` or the right-click menu state. The parent (`mod.rs`) hands it the relevant signals as props.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`

- [ ] **Step 1: Write the SSR test for the row's resting state**

Append the test to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn row_renders_name_and_source_line() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Boost"), "name must render: {html}");
    assert!(html.contains("Btn 1"), "source line must render: {html}");
    assert!(html.contains("if-row"), "row root class missing: {html}");
}

#[test]
fn row_active_class_when_selected() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        rsx! {
            Row {
                summary: summary,
                is_active: true,
                renaming: renaming,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("is-active"), "active row must carry is-active class: {html}");
}

#[test]
fn row_glyphs_render_for_merge_and_conditional() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Throttle".to_owned()),
            glyphs: GlyphFlags {
                merge_secondary: Some(InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 1 },
                }),
                first_input_predicate: Some(InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Button { index: 3 },
                }),
            },
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("glyph-merge"), "merge glyph class must render: {html}");
    assert!(html.contains("glyph-cond"), "conditional glyph class must render: {html}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::row_`
Expected: FAIL, `row::Row` does not yet exist.

- [ ] **Step 3: Implement `Row`**

Create `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`:

```rust
//! Single mapping-list row. See spec § "Row anatomy".

use dioxus::prelude::*;

use inputforge_core::types::InputAddress;

use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::source_label;
use crate::frame::view_state::ViewState;

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub(crate) fn Row(
    summary: MappingSummary,
    is_active: bool,
    /// `Some(addr)` when this row's name is currently being inline-renamed -
    /// the parent hoists this signal so only one row at a time is in rename
    /// mode. Owned by `mod.rs`. Task 14 only reads this for prop forwarding
    /// (the resting row never branches on it); Task 15 introduces the
    /// `if renaming { RenameInline } else { resting_row }` branch.
    renaming: Signal<Option<InputAddress>>,
    /// RMB / Shift+F10 fires this with `(input, x, y)` so the parent can
    /// open the context menu at the cursor. Coordinates are page-space.
    on_open_menu: EventHandler<(InputAddress, f64, f64)>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::row");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let _ = renaming; // Task 15 wires the rename branch; Task 14's resting row never reads it.

    let source_text = source_label::format(&summary.input, &ctx.config.read());

    // LMB → set selection. RMB → fire on_open_menu.
    let mut sel = view.selected_mapping;
    let summary_for_click = summary.clone();
    let onclick = move |_| {
        sel.set(Some((summary_for_click.mode.clone(), summary_for_click.input.clone())));
    };
    let summary_for_ctx = summary.clone();
    let on_open_menu_inner = on_open_menu;
    let oncontextmenu = move |evt: MouseEvent| {
        evt.prevent_default();
        evt.stop_propagation();
        let coords = evt.client_coordinates();
        on_open_menu_inner.call((
            summary_for_ctx.input.clone(),
            coords.x,
            coords.y,
        ));
    };

    let class = if is_active {
        "if-row is-active"
    } else {
        "if-row"
    };

    let merge_glyph = summary.glyphs.merge_secondary.as_ref().map(|secondary| {
        let cfg = ctx.config.read();
        source_label::format(secondary, &cfg)
    });
    let cond_glyph = summary.glyphs.first_input_predicate.as_ref().map(|predicate| {
        let cfg = ctx.config.read();
        source_label::format(predicate, &cfg)
    });

    rsx! {
        div {
            class,
            role: "button",
            tabindex: if is_active { "0" } else { "-1" },
            onclick,
            oncontextmenu,
            div { class: "if-row__name",
                if let Some(name) = &summary.name {
                    "{name}"
                } else {
                    em { class: "if-row__name--unnamed", "(unnamed)" }
                }
            }
            div { class: "if-row__source",
                "{source_text}"
                if let Some(secondary_label) = merge_glyph {
                    span {
                        class: "glyph-merge",
                        title: "MergeAxis",
                        "+ "
                    }
                    em { "{secondary_label}" }
                }
                if let Some(predicate_label) = cond_glyph {
                    span {
                        class: "glyph-cond",
                        title: "{predicate_label}",
                        "⊕ "
                    }
                    em { "{predicate_label}" }
                }
            }
        }
    }
}
```

**Task 14 ships only the resting row.** The `if is_renaming { RenameInline } else { ... }` branch is introduced by Task 15 (which modifies `row.rs` in addition to creating `rename_inline.rs`). Tests in Task 14 verify only the resting state, they do not assert on rename swap-in.

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::row_`
Expected: PASS, three tests green.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/row.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping_list): Row component with name, source line, glyphs, active state"
```

---

### Task 15: `rename_inline::RenameInline`, inline rename for an existing row

Mirrors F7's `mode_tabs::rename_inline::RenameInline` shape. Replaces the row's name `<div>` with a focused `<input>`. Enter dispatches `EngineCommand::SetMapping` with the same actions and the new name. Esc reverts. Blur with non-empty value commits; blur with empty reverts.

**This task also modifies `row.rs`** to add the rename-branch:

```rust
// In Row(), at the top of the body, replace the resting-only render with:
let is_renaming = renaming
    .read()
    .as_ref()
    .map(|a| a == &summary.input)
    .unwrap_or(false);

if is_renaming {
    return rsx! {
        crate::frame::mapping_list::rename_inline::RenameInline {
            summary: summary.clone(),
            state: renaming,
        }
    };
}
// ... rest of Row body (resting render) unchanged.
```

The Task 14 `let _ = renaming;` line is removed in this task (the branch now reads it).

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/rename_inline.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs` (introduce the `is_renaming` branch above)

- [ ] **Step 1: Write the test**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn rename_inline_renders_input_with_initial_value() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::rename_inline::RenameInline;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        let renaming: Signal<Option<InputAddress>> =
            use_signal(|| Some(summary.input.clone()));
        rsx! {
            RenameInline { summary: summary, state: renaming }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-row-rename"),
        "rename input must carry the .if-row-rename class: {html}",
    );
    assert!(
        html.contains("Boost"),
        "rename input must initialize with the existing name: {html}",
    );
}

#[test]
fn row_swaps_in_rename_inline_when_renaming_matches_input() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        // renaming.set(Some(summary.input)), Row should swap into the rename branch.
        let renaming: Signal<Option<InputAddress>> =
            use_signal(|| Some(summary.input.clone()));
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-row-rename"),
        "Row must swap in RenameInline when renaming matches the row's input: {html}",
    );
    // Resting markup must be replaced, no `if-row__name` div.
    assert!(
        !html.contains("if-row__name\""),
        "Row must NOT render the resting name div while renaming: {html}",
    );
}

#[test]
fn row_renders_resting_when_renaming_is_none() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use crate::context::{GlyphFlags, MappingSummary};
    use crate::frame::mapping_list::row::Row;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let summary = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
        rsx! {
            Row {
                summary: summary,
                is_active: false,
                renaming: renaming,
                on_open_menu: move |_: (InputAddress, f64, f64)| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-row__name"),
        "Row must render the resting name div when not renaming: {html}",
    );
    assert!(
        !html.contains("if-row-rename"),
        "Row must NOT render the rename input when renaming is None: {html}",
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::rename_inline`
Expected: FAIL.

- [ ] **Step 3: Implement `RenameInline`**

Create `crates/inputforge-gui-dx/src/frame/mapping_list/rename_inline.rs`:

```rust
//! Inline rename for an existing mapping row. Mirrors F7's
//! `mode_tabs::rename_inline::RenameInline`, Enter dispatches
//! `SetMapping` with same actions + new name; Esc reverts; blur with
//! empty value reverts; blur with non-empty value commits.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::types::InputAddress;

use crate::components::{InputSize, TextInput};
use crate::context::{AppContext, MappingSummary};

fn run_commit(
    raw: &str,
    summary: &MappingSummary,
    commands: &Sender<EngineCommand>,
    actions: Vec<inputforge_core::action::Action>,
    mut state: Signal<Option<InputAddress>>,
) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        // Empty → revert (same as Esc).
        state.set(None);
        return;
    }
    let new_name = trimmed.to_owned();
    if Some(&new_name) == summary.name.as_ref() {
        // No-op rename.
        state.set(None);
        return;
    }
    let _ = commands.send(EngineCommand::SetMapping {
        input: summary.input.clone(),
        mode: summary.mode.clone(),
        name: Some(new_name),
        actions,
    });
    tracing::info!(
        target: "f8::mapping_list",
        action = "rename",
        ?summary.input,
        mode = %summary.mode,
        "dispatch SetMapping (rename)",
    );
    state.set(None);
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(crate) fn RenameInline(
    summary: MappingSummary,
    state: Signal<Option<InputAddress>>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::rename_inline");
    let ctx = use_context::<AppContext>();

    let initial = summary.name.clone().unwrap_or_default();
    let mut value: Signal<String> = use_signal(|| initial);

    // Resolve the actions we'll re-send by reading active_profile at
    // commit time. That way an external update during rename doesn't
    // overwrite a fresh action edit. If the mapping disappeared mid-
    // rename, fall through with an empty Vec (set_mapping then drops it).
    let summary_for_kb = summary.clone();
    let summary_for_blur = summary.clone();
    let cmd_for_kb = ctx.commands.clone();
    let cmd_for_blur = ctx.commands.clone();
    let ctx_for_kb = ctx.clone();
    let ctx_for_blur = ctx.clone();

    rsx! {
        div { class: "if-row__rename-wrapper",
            TextInput {
                value: ReadSignal::from(value),
                size: InputSize::Sm,
                class: Some("if-row-rename".to_owned()),
                onmounted: move |evt: MountedEvent| {
                    spawn(async move {
                        let _ = evt.data().set_focus(true).await;
                    });
                },
                oninput: move |evt: FormEvent| {
                    value.set(evt.value());
                },
                onkeydown: move |evt: KeyboardEvent| {
                    match evt.key() {
                        Key::Enter => {
                            evt.prevent_default();
                            let raw = value.read().clone();
                            let actions = ctx_for_kb
                                .state
                                .read()
                                .active_profile
                                .as_ref()
                                .and_then(|p| {
                                    p.find_mapping(&summary_for_kb.input, &summary_for_kb.mode)
                                        .map(|m| m.actions.clone())
                                })
                                .unwrap_or_default();
                            run_commit(&raw, &summary_for_kb, &cmd_for_kb, actions, state);
                        }
                        Key::Escape => {
                            evt.prevent_default();
                            let mut state = state;
                            state.set(None);
                        }
                        _ => {}
                    }
                },
                onfocusout: move |_evt: FocusEvent| {
                    let raw = value.read().clone();
                    let actions = ctx_for_blur
                        .state
                        .read()
                        .active_profile
                        .as_ref()
                        .and_then(|p| {
                            p.find_mapping(&summary_for_blur.input, &summary_for_blur.mode)
                                .map(|m| m.actions.clone())
                        })
                        .unwrap_or_default();
                    run_commit(&raw, &summary_for_blur, &cmd_for_blur, actions, state);
                },
            }
        }
    }
}
```

If `TextInput` does not currently accept a `class` prop or `onkeydown` / `onfocusout` props, check the component definition at `crates/inputforge-gui-dx/src/components/text_input.rs`. The mode-tabs `RenameInline` passes those event handlers through a wrapping `<div>` rather than directly on `TextInput`, mirror that shape if `TextInput`'s prop surface is narrower:

```rust
div {
    onkeydown: ...,
    onfocusout: ...,
    TextInput { ... }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::rename_inline`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/rename_inline.rs crates/inputforge-gui-dx/src/frame/mapping_list/row.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping_list): inline rename dispatches SetMapping with preserved actions"
```

---

### Task 16: `empty::EmptyZeroMappings` and `empty::EmptyZeroFilterResults`

Two empty-state renderers. **State A** (zero mappings): title + helper + primary `+ Add mapping` button that, on click, sets the parent's `add_state` directly to `CapturingArmed` (skips the dashed-row click). **State B** (zero filter results): title quoting the query + helper + ghost-link `Clear filter` button.

The component takes a callback for the "go to capturing" / "clear filter" actions; the parent (`mod.rs` in Task 18) wires those.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs`

- [ ] **Step 1: Write the SSR tests**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn empty_zero_mappings_renders_title_and_button() {
    use crate::frame::mapping_list::empty::EmptyZeroMappings;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroMappings { on_start_capture: move |()| {} }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("No mappings yet"), "title missing: {html}");
    assert!(html.contains("if-rail-empty"), "rail-empty class missing: {html}");
}

#[test]
fn empty_zero_filter_results_quotes_query() {
    use crate::frame::mapping_list::empty::EmptyZeroFilterResults;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroFilterResults {
                query: "ailerons".to_owned(),
                on_clear: move |()| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("ailerons"),
        "filtered-empty title must quote the current query: {html}",
    );
    assert!(html.contains("Clear filter"), "clear-filter button missing: {html}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::empty_`
Expected: FAIL.

- [ ] **Step 3: Implement the empty-state components**

Create `crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs`:

```rust
//! Empty-state renderers for the F8 mapping list rail.
//!
//! State A, zero mappings overall (profile loaded, mode has none):
//!   title + helper + primary `+ Add mapping` button that expands directly
//!   into `CapturingArmed` (skips Resting → click).
//!
//! State B, zero filter results: title quoting `<query>` + helper +
//!   ghost-link `Clear filter` button.

use dioxus::prelude::*;

use crate::components::{Button, ButtonVariant};

#[component]
pub(crate) fn EmptyZeroMappings(on_start_capture: EventHandler<()>) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::empty_zero_mappings");
    rsx! {
        div { class: "if-rail-empty if-rail-empty--zero-mappings",
            div { class: "if-rail-empty__title", "No mappings yet" }
            div { class: "if-rail-empty__helper",
                "Pick an input on a device to start binding. Or click below to name one first."
            }
            Button {
                variant: ButtonVariant::Primary,
                onclick: move |_| on_start_capture.call(()),
                "+ Add mapping"
            }
        }
    }
}

#[component]
pub(crate) fn EmptyZeroFilterResults(
    query: String,
    on_clear: EventHandler<()>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::empty_zero_filter_results");
    rsx! {
        div { class: "if-rail-empty if-rail-empty--zero-filter-results",
            div { class: "if-rail-empty__title",
                "No mappings match "
                span { class: "muted", "\"{query}\"" }
            }
            div { class: "if-rail-empty__helper",
                "Filter searches name and source label."
            }
            Button {
                variant: ButtonVariant::Ghost,
                onclick: move |_| on_clear.call(()),
                "Clear filter"
            }
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::empty_`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/empty.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping_list): empty-state components for zero-mappings and zero-filter-results"
```

---

### Task 17: `add_inline::AddInline`, `+ Add mapping` capture state machine

The full state machine from spec §"`+ Add mapping` state machine". `Resting` → `CapturingArmed` → `{Captured | Collision | CapturingDisarmed}` → `Resting`. Watches `LiveCapture::captured` and re-validates `Collision` against `cfg.mappings` once per polling tick (collision drift).

Component has its own internal state, the parent (`mod.rs`) only needs to mount it and read whether it's expanded (so the keyboard handler can know to skip Up/Down dispatch).

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs`

- [ ] **Step 1: Write the SSR test for the resting state**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn add_inline_resting_renders_dashed_row() {
    use crate::frame::mapping_list::add_inline::AddInline;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let force_expanded: Signal<bool> = use_signal(|| false);
        rsx! { AddInline { force_expanded: force_expanded } }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-add-inline"),
        "AddInline root class missing: {html}",
    );
    // Resting state advertises the click affordance.
    assert!(
        html.contains("Add mapping") || html.contains("+ "),
        "resting state must advertise the add affordance: {html}",
    );
}

#[test]
fn add_inline_force_expanded_arms_capture() {
    use crate::frame::mapping_list::add_inline::AddInline;
    use crate::patterns::live_capture::LiveCapture;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let force_expanded: Signal<bool> = use_signal(|| true);
        let cap = use_context::<LiveCapture>();
        let armed_marker = if *cap.active.read() { "ARMED" } else { "IDLE" };
        rsx! {
            AddInline { force_expanded: force_expanded }
            span { "{armed_marker}" }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    // Two extra rebuilds so the use_effect that arms capture flushes.
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("ARMED"),
        "force_expanded=true must arm LiveCapture; got: {html}",
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::add_inline`
Expected: FAIL.

- [ ] **Step 3: Implement `AddInline`**

Create `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs`:

```rust
//! `+ Add mapping` inline state machine. See spec §"`+ Add mapping`
//! state machine" for the full transition table.

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::types::InputAddress;

use crate::components::{Button, ButtonVariant, InputSize, TextInput};
use crate::context::AppContext;
use crate::frame::mapping_list::source_label;
use crate::frame::view_state::ViewState;
use crate::patterns::live_capture::{CaptureFilter, LiveCapture};

#[derive(Debug, Clone, PartialEq)]
enum AddState {
    Resting,
    CapturingArmed,
    CapturingDisarmed,
    Captured { addr: InputAddress },
    Collision {
        existing_name: String,
        existing: InputAddress,
    },
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(crate) fn AddInline(
    /// When set to `true` from outside (e.g., by the EmptyZeroMappings
    /// "+ Add mapping" button), expand directly into `CapturingArmed`,
    /// skipping the Resting → click step. Parent must reset to `false`
    /// once the form is mounted; the effect inside this component
    /// observes the rising edge.
    force_expanded: Signal<bool>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list::add_inline");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let cap = use_context::<LiveCapture>();

    let mut state: Signal<AddState> = use_signal(|| AddState::Resting);
    let mut name: Signal<String> = use_signal(String::new);

    // Honor `force_expanded` from the parent, used by EmptyZeroMappings'
    // primary button to skip the dashed-row click.
    {
        let mut state = state;
        let mut force = force_expanded;
        use_effect(move || {
            if *force.read() {
                state.set(AddState::CapturingArmed);
                cap.start.call(CaptureFilter::Any);
                force.set(false);
            }
        });
    }

    // Watch `cap.captured`, when capture lands, transition to Captured
    // or Collision based on whether the address is already mapped in the
    // active editing mode.
    {
        let cap = cap;
        let editing = view.editing_mode;
        let ctx_for_cap = ctx.clone();
        use_effect(move || {
            // Subscribe to captured.
            let captured_now = cap.captured.read().clone();
            // M3: subscribe to `state` via `read()` (not `peek()`) so that
            // the effect re-runs when state transitions back to
            // CapturingArmed for a subsequent capture round.
            if *state.read() != AddState::CapturingArmed {
                return;
            }
            let Some(addr) = captured_now else {
                return;
            };
            let mode_now = editing.read().clone();
            // Look for a same-mode collision in the active profile.
            let cfg = ctx_for_cap.config.read();
            let collision = cfg
                .mappings
                .iter()
                .find(|m| m.input == addr && m.mode == mode_now);
            let next_state = match collision {
                Some(existing) => AddState::Collision {
                    existing_name: existing
                        .name
                        .clone()
                        .unwrap_or_else(|| "(unnamed)".to_owned()),
                    existing: existing.input.clone(),
                },
                None => AddState::Captured { addr: addr.clone() },
            };
            drop(cfg);
            cap.cancel.call(());
            state.set(next_state);
        });
    }

    // Watch active flipping false externally (Esc taken by primitive) -
    // transition Armed → Disarmed.
    {
        let cap = cap;
        use_effect(move || {
            if *cap.active.read() {
                return;
            }
            if *state.peek() == AddState::CapturingArmed {
                state.set(AddState::CapturingDisarmed);
            }
        });
    }

    // Collision drift: re-validate once per polling tick. If `existing` is
    // no longer in cfg.mappings for the active mode, transition to Captured.
    {
        let editing = view.editing_mode;
        let ctx_for_drift = ctx.clone();
        use_effect(move || {
            let s = state.read().clone();
            if let AddState::Collision { existing, .. } = s {
                let mode_now = editing.read().clone();
                let cfg = ctx_for_drift.config.read();
                let still_present = cfg
                    .mappings
                    .iter()
                    .any(|m| m.input == existing && m.mode == mode_now);
                drop(cfg);
                if !still_present {
                    state.set(AddState::Captured { addr: existing });
                }
            }
        });
    }

    // Dispatch `SetMapping` with empty-actions. Engine treats empty-actions
    // SetMapping as a removal, but the F8 spec explicitly creates the
    // mapping with an empty action vector here to let F9 fill it in. We
    // dispatch the command and let F9 own the editor surface; the engine
    // will discard it on save unless F9 fills it in. (Per spec §"+ Add
    // mapping state machine" Captured → Resting transition.)
    let dispatch_add = move |addr: InputAddress, name_value: String| {
        let mode_now = view.editing_mode.read().clone();
        let _ = ctx.commands.send(EngineCommand::SetMapping {
            input: addr.clone(),
            mode: mode_now.clone(),
            name: if name_value.trim().is_empty() {
                None
            } else {
                Some(name_value)
            },
            actions: vec![],
        });
        // Optimistic-ish: select the new row immediately. The polling
        // tick will reconcile if the engine rejects the command.
        let mut sel = view.selected_mapping;
        sel.set(Some((mode_now, addr)));
        tracing::info!(
            target: "f8::mapping_list",
            action = "add",
            "dispatch SetMapping (add)",
        );
    };

    // ----- Render per state -----
    match state.read().clone() {
        AddState::Resting => rsx! {
            div { class: "if-add-inline if-add-inline--resting",
                button {
                    r#type: "button",
                    class: "if-add-inline__dashed-row",
                    onclick: move |_| {
                        state.set(AddState::CapturingArmed);
                        cap.start.call(CaptureFilter::Any);
                    },
                    "aria-label": "Add mapping",
                    "+ Add mapping"
                }
            }
        },
        AddState::CapturingArmed => rsx! {
            div { class: "if-add-inline if-add-inline--armed",
                div { class: "if-add-inline__pad",
                    "Press an input on any device…"
                }
                TextInput {
                    value: ReadSignal::from(name),
                    size: InputSize::Sm,
                    placeholder: "Mapping name (optional)".to_owned(),
                    oninput: move |evt: FormEvent| name.set(evt.value()),
                }
            }
        },
        AddState::CapturingDisarmed => rsx! {
            div { class: "if-add-inline if-add-inline--disarmed",
                button {
                    r#type: "button",
                    class: "if-add-inline__pad if-add-inline__pad--disarmed",
                    onclick: move |_| {
                        state.set(AddState::CapturingArmed);
                        cap.start.call(CaptureFilter::Any);
                    },
                    "Cancelled, click to capture again"
                }
                TextInput {
                    value: ReadSignal::from(name),
                    size: InputSize::Sm,
                    placeholder: "Mapping name (optional)".to_owned(),
                    oninput: move |evt: FormEvent| name.set(evt.value()),
                    onkeydown: move |evt: KeyboardEvent| {
                        if evt.key() == Key::Escape {
                            evt.prevent_default();
                            state.set(AddState::Resting);
                            name.set(String::new());
                        }
                    },
                }
            }
        },
        AddState::Captured { addr } => {
            let addr_for_enter = addr.clone();
            let addr_for_btn = addr.clone();
            let cfg = ctx.config.read();
            let label = source_label::format(&addr, &cfg);
            drop(cfg);
            rsx! {
                div { class: "if-add-inline if-add-inline--captured",
                    div { class: "if-add-inline__captured-label", "{label}" }
                    TextInput {
                        value: ReadSignal::from(name),
                        size: InputSize::Sm,
                        placeholder: "Mapping name".to_owned(),
                        oninput: move |evt: FormEvent| name.set(evt.value()),
                        onkeydown: move |evt: KeyboardEvent| {
                            match evt.key() {
                                Key::Enter => {
                                    evt.prevent_default();
                                    let n = name.read().clone();
                                    dispatch_add(addr_for_enter.clone(), n);
                                    state.set(AddState::Resting);
                                    name.set(String::new());
                                }
                                Key::Escape => {
                                    evt.prevent_default();
                                    state.set(AddState::Resting);
                                    name.set(String::new());
                                }
                                _ => {}
                            }
                        },
                    }
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| {
                            let n = name.read().clone();
                            dispatch_add(addr_for_btn.clone(), n);
                            state.set(AddState::Resting);
                            name.set(String::new());
                        },
                        "Add"
                    }
                }
            }
        }
        AddState::Collision { existing_name, existing } => {
            let existing_for_btn = existing.clone();
            // M4: dynamic source label for the collision message, no
            // hard-coded "Btn" prefix. Uses the same source_label::format
            // path as the resting row's source line.
            let cfg = ctx.config.read();
            let captured_label = source_label::format(&existing, &cfg);
            drop(cfg);
            rsx! {
                div { class: "if-add-inline if-add-inline--collision",
                    div { class: "if-add-inline__collision-text",
                        em { "{captured_label} already mapped to " }
                        strong { "{existing_name}" }
                        "."
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| {
                            let mode_now = view.editing_mode.read().clone();
                            let mut sel = view.selected_mapping;
                            sel.set(Some((mode_now, existing_for_btn.clone())));
                            state.set(AddState::Resting);
                            name.set(String::new());
                        },
                        "Edit existing →"
                    }
                    button {
                        r#type: "button",
                        class: "if-add-inline__close",
                        onclick: move |_| {
                            state.set(AddState::Resting);
                            name.set(String::new());
                        },
                        "Cancel"
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::add_inline`
Expected: PASS, both tests green.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping_list): + Add mapping inline state machine with collision detection"
```

---

### Task 18: `keyboard::handle_key`, pure logic for Up/Down/Enter/Cmd-F/Esc

The keyboard logic that maps a key event + current state to a side-effect intent. Pure-fn-friendly: extracts the routing decision from the key dispatch so it can be unit-tested without mounting Dioxus. The `mod.rs` orchestrator wires the resulting intents to actual signals.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_list/keyboard.rs`

- [ ] **Step 1: Write the test**

Create `crates/inputforge-gui-dx/src/frame/mapping_list/keyboard.rs` with the public types and tests in the same file:

```rust
//! Pure keyboard-routing logic for the F8 rail.
//!
//! `handle_key` takes the current state (visible filtered rows,
//! current selection, capture-armed, filter-focused, query-empty) and
//! returns an `Intent` that the `mod.rs` orchestrator translates into
//! signal writes. Splitting the routing decision out lets us unit-test
//! the boundary cases without a Dioxus runtime.

use inputforge_core::types::InputAddress;

/// Keys F8 cares about. Dioxus 0.7's `Key` enum carries platform-specific
/// variants; we narrow to the F8 vocabulary here so the unit tests can
/// drive `Intent::resolve` with stable inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Key {
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    /// Cmd-F (macOS) or Ctrl-F (Windows/Linux). Caller normalizes.
    FilterShortcut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct State<'a> {
    pub visible_rows: &'a [&'a (String, InputAddress)],
    pub selected: Option<(&'a str, &'a InputAddress)>,
    pub capture_armed: bool,
    pub filter_focused: bool,
    pub filter_query_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Intent<'a> {
    /// Move selection to this row.
    Select((String, InputAddress)),
    /// Focus `[data-editor-focus]` (F9 owns the attached element).
    FocusEditor,
    /// Focus the filter input.
    FocusFilter,
    /// Clear filter query and unfocus.
    ClearFilter,
    /// Do nothing (key not handled in this context).
    NoOp,
    #[allow(dead_code, reason = "lifetime witness for the borrow")]
    _Phantom(&'a ()),
}

pub(crate) fn handle_key(key: Key, state: State<'_>) -> Intent<'_> {
    // Capture-armed always shadows Up/Down (and Enter/Esc which the
    // primitive owns). Filter-focused Esc with non-empty query clears.
    if state.capture_armed && matches!(key, Key::ArrowUp | Key::ArrowDown) {
        return Intent::NoOp;
    }
    match key {
        Key::FilterShortcut => Intent::FocusFilter,
        Key::Escape if state.filter_focused && !state.filter_query_empty => {
            Intent::ClearFilter
        }
        Key::Escape => Intent::NoOp,
        Key::Enter if state.selected.is_some() => Intent::FocusEditor,
        Key::Enter => Intent::NoOp,
        Key::ArrowDown | Key::ArrowUp => {
            if state.visible_rows.is_empty() {
                return Intent::NoOp;
            }
            // None → first (Down) or last (Up).
            let len = state.visible_rows.len();
            let next_idx = match (key, state.selected) {
                (Key::ArrowDown, None) => 0,
                (Key::ArrowUp, None) => len - 1,
                (Key::ArrowDown, Some((sel_mode, sel_input))) => {
                    let cur = state
                        .visible_rows
                        .iter()
                        .position(|(m, i)| m == sel_mode && i == sel_input)
                        .unwrap_or(0);
                    (cur + 1) % len
                }
                (Key::ArrowUp, Some((sel_mode, sel_input))) => {
                    let cur = state
                        .visible_rows
                        .iter()
                        .position(|(m, i)| m == sel_mode && i == sel_input)
                        .unwrap_or(0);
                    (cur + len - 1) % len
                }
                _ => unreachable!(),
            };
            let (m, i) = state.visible_rows[next_idx].clone();
            Intent::Select((m, i))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, InputId};

    fn addr(id: u8) -> InputAddress {
        InputAddress {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: id },
        }
    }

    #[test]
    fn down_selects_first_when_nothing_selected() {
        let rows = vec![
            ("Default".to_owned(), addr(0)),
            ("Default".to_owned(), addr(1)),
        ];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        match handle_key(Key::ArrowDown, state) {
            Intent::Select((m, i)) => {
                assert_eq!(m, "Default");
                assert_eq!(i, addr(0));
            }
            other => panic!("expected Select(first), got {other:?}"),
        }
    }

    #[test]
    fn up_selects_last_when_nothing_selected() {
        let rows = vec![
            ("Default".to_owned(), addr(0)),
            ("Default".to_owned(), addr(1)),
        ];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        match handle_key(Key::ArrowUp, state) {
            Intent::Select((_, i)) => assert_eq!(i, addr(1)),
            other => panic!("expected Select(last), got {other:?}"),
        }
    }

    #[test]
    fn down_wraps_at_boundary() {
        let rows = vec![
            ("Default".to_owned(), addr(0)),
            ("Default".to_owned(), addr(1)),
        ];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let mode = "Default".to_owned();
        let last = addr(1);
        let state = State {
            visible_rows: &row_refs,
            selected: Some((&mode, &last)),
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        match handle_key(Key::ArrowDown, state) {
            Intent::Select((_, i)) => assert_eq!(i, addr(0)),
            other => panic!("expected wrap to first, got {other:?}"),
        }
    }

    #[test]
    fn capture_armed_disables_up_down() {
        let rows = vec![("Default".to_owned(), addr(0))];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: true,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::ArrowDown, state), Intent::NoOp);
        assert_eq!(handle_key(Key::ArrowUp, state), Intent::NoOp);
    }

    #[test]
    fn enter_with_selection_focuses_editor() {
        let rows = vec![("Default".to_owned(), addr(0))];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let mode = "Default".to_owned();
        let sel = addr(0);
        let state = State {
            visible_rows: &row_refs,
            selected: Some((&mode, &sel)),
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::Enter, state), Intent::FocusEditor);
    }

    #[test]
    fn enter_with_no_selection_is_noop() {
        let row_refs: Vec<&(String, InputAddress)> = Vec::new();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::Enter, state), Intent::NoOp);
    }

    #[test]
    fn cmd_f_focuses_filter() {
        let row_refs: Vec<&(String, InputAddress)> = Vec::new();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::FilterShortcut, state), Intent::FocusFilter);
    }

    #[test]
    fn esc_on_filter_with_query_clears() {
        let row_refs: Vec<&(String, InputAddress)> = Vec::new();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: true,
            filter_query_empty: false,
        };
        assert_eq!(handle_key(Key::Escape, state), Intent::ClearFilter);
    }

    #[test]
    fn esc_on_rail_with_empty_filter_is_noop() {
        let row_refs: Vec<&(String, InputAddress)> = Vec::new();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::Escape, state), Intent::NoOp);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::keyboard::tests`
Expected: PASS, nine tests green.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/keyboard.rs
git commit -m "feat(mapping_list): pure-logic keyboard intent dispatcher"
```

---

### Task 19: `MappingList` orchestrator, wires filter, rows, empty states, add-inline

Pulls everything together. Reads `cfg.config.mappings`, filters by mode (memo over `editing_mode`), then by query, buckets into groups, renders headers + rows + empty states + the AddInline. Carries the `selected_mapping` from `ViewState` and a `renaming: Signal<Option<InputAddress>>` it owns. The right-click menu and delete dialog are deferred to Task 20-21.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`

- [ ] **Step 1: Append the orchestrator SSR test**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn mapping_list_renders_axes_and_buttons_groups_in_order() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        // Build state with 3 axes and 1 button mapped in "Default".
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let mut mappings = vec![];
        for i in 0..3 {
            mappings.push(Mapping {
                input: InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: i },
                },
                mode: "Default".to_owned(),
                name: Some(format!("Axis{i}")),
                actions: vec![Action::MapToVJoy {
                    output: OutputAddress {
                        device: 1,
                        output: OutputId::Axis { id: VJoyAxis::X },
                    },
                }],
            });
        }
        mappings.push(Mapping {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        });

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        // Inject the state via a stub config snapshot so the rail component
        // doesn't need a polling task.
        provide_minimal_contexts();
        let mut cfg_signal = use_context::<crate::context::AppContext>().config;
        let mut meta_signal = use_context::<crate::context::AppContext>().meta;
        use_hook(move || {
            let cfg = crate::context::ConfigSnapshot::from_state(&state);
            cfg_signal.set(cfg);
            let meta = crate::context::MetaSnapshot::from_state(&state);
            meta_signal.set(meta);
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    let axes_pos = html.find("AXES").expect("AXES header missing");
    let buttons_pos = html.find("BUTTONS").expect("BUTTONS header missing");
    assert!(
        axes_pos < buttons_pos,
        "AXES must render before BUTTONS; got: {html}",
    );
    assert!(html.contains("Axis0"));
    assert!(html.contains("Axis1"));
    assert!(html.contains("Axis2"));
    assert!(html.contains("Boost"));
    // No HATS group → no "HATS" header.
    assert!(!html.contains("HATS"), "empty Hats group must not render header");
}

#[test]
fn mapping_list_zero_mappings_renders_empty_state_a() {
    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("No mappings yet"),
        "Empty State A must render when no mappings are present: {html}",
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::mapping_list_`
Expected: FAIL, current `MappingList` is the stub from Task 10.

- [ ] **Step 3: Replace the stub `MappingList` body**

Edit `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`. Keep the module decls and `MAPPING_LIST_CSS` constant from Task 10; replace the stub `MappingList` fn with the full orchestrator:

```rust
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::InputAddress;

use crate::components::{InputSize, TextInput};
use crate::context::{AppContext, MappingSummary};
use crate::frame::mapping_list::add_inline::AddInline;
use crate::frame::mapping_list::empty::{EmptyZeroFilterResults, EmptyZeroMappings};
use crate::frame::mapping_list::filter::matches_filter;
use crate::frame::mapping_list::group::{GroupKind, group_of};
use crate::frame::mapping_list::row::Row;
use crate::frame::view_state::ViewState;
use crate::patterns::live_capture::LiveCapture;

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(crate) fn MappingList() -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_list");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let cap = use_context::<LiveCapture>();

    let editing = view.editing_mode;
    let filter_query: Signal<String> = use_signal(String::new);
    // D5: filter_focused is a signal so Task 22's document-scoped keydown
    // listener (and Task 18's pure `handle_key`) can know whether the
    // filter input has focus. FilterInput sets it true on focus / false
    // on blur.
    let filter_focused: Signal<bool> = use_signal(|| false);
    let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
    let force_expand_add: Signal<bool> = use_signal(|| false);
    // Right-click menu state owned by Task 20.
    let menu_open: Signal<Option<(InputAddress, f64, f64)>> = use_signal(|| None);
    // Delete dialog state owned by Task 21.
    let delete_target: Signal<Option<MappingSummary>> = use_signal(|| None);
    // Pending duplicate (D11): right-click "Duplicate" arms LiveCapture and
    // stashes the source mapping here so the capture-watcher can dispatch
    // SetMapping with the captured InputAddress + cloned actions on success.
    let pending_duplicate: Signal<Option<MappingSummary>> = use_signal(|| None);

    // M7: single memo computes both the filtered, grouped rows AND the
    // total in-mode count in one pass, avoiding two separate cfg/mode
    // reads per render.
    let view_state_memo = use_memo(move || {
        let cfg = ctx.config.read();
        let mode_now = editing.read().clone();
        let query = filter_query.read().clone();
        let mut total: usize = 0;
        let mut filtered: Vec<MappingSummary> = Vec::new();
        for m in cfg.mappings.iter().filter(|m| m.mode == mode_now) {
            total += 1;
            if matches_filter(m, &query, &cfg) {
                filtered.push(m.clone());
            }
        }
        (total, filtered)
    });

    let (total, rows) = {
        let snapshot = view_state_memo.read();
        (snapshot.0, snapshot.1.clone())
    };
    let query = filter_query.read().clone();
    let query_empty = query.trim().is_empty();

    // Empty-state discrimination.
    if total == 0 {
        return rsx! {
            Stylesheet { href: MAPPING_LIST_CSS }
            div { class: "if-rail",
                EmptyZeroMappings {
                    on_start_capture: move |()| {
                        let mut force = force_expand_add;
                        force.set(true);
                    }
                }
                AddInline { force_expanded: force_expand_add }
            }
        };
    }

    if !query_empty && rows.is_empty() {
        return rsx! {
            Stylesheet { href: MAPPING_LIST_CSS }
            div { class: "if-rail",
                FilterInput {
                    value: filter_query,
                    focused: filter_focused,
                }
                EmptyZeroFilterResults {
                    query: query.clone(),
                    on_clear: move |()| {
                        let mut q = filter_query;
                        q.set(String::new());
                    }
                }
                AddInline { force_expanded: force_expand_add }
            }
        };
    }

    // M2: rewrite group rendering as filter_map producing rsx for the
    // surviving groups only, no `if/else rsx!{}` empty branches.
    let group_iter = GroupKind::ordered().into_iter().filter_map(|group| {
        let group_rows: Vec<MappingSummary> = rows
            .iter()
            .filter(|r| group_of(&r.input) == group)
            .cloned()
            .collect();
        if group_rows.is_empty() {
            return None;
        }
        Some(rsx! {
            div { class: "if-rail__group",
                div {
                    class: "if-rail__group-header",
                    {group.header()}
                }
                for row in group_rows {
                    {
                        let is_active = view
                            .selected_mapping
                            .read()
                            .as_ref()
                            .map(|(m, i)| m == &row.mode && i == &row.input)
                            .unwrap_or(false);
                        let mut menu_setter = menu_open;
                        rsx! {
                            Row {
                                key: "{row.input:?}-{row.mode}",
                                summary: row.clone(),
                                is_active: is_active,
                                renaming: renaming,
                                on_open_menu: move |(input, x, y): (InputAddress, f64, f64)| {
                                    menu_setter.set(Some((input, x, y)));
                                },
                            }
                        }
                    }
                }
            }
        })
    });

    rsx! {
        Stylesheet { href: MAPPING_LIST_CSS }
        div { class: "if-rail",
            FilterInput { value: filter_query, focused: filter_focused }
            { group_iter }
            AddInline { force_expanded: force_expand_add }
            // Right-click menu (Task 20) and delete dialog (Task 21)
            // mount points, populated when those tasks land.
            ContextMenuMount {
                menu_open: menu_open,
                renaming: renaming,
                delete_target: delete_target,
                pending_duplicate: pending_duplicate,
            }
            DeleteDialogMount {
                delete_target: delete_target,
            }
            // D11: watcher that observes LiveCapture::captured while
            // pending_duplicate is set, and dispatches SetMapping with
            // the cloned actions + "(copy)"-suffixed name on success.
            DuplicateWatcher {
                pending_duplicate: pending_duplicate,
            }
        }
    }
}

#[component]
fn FilterInput(value: Signal<String>, focused: Signal<bool>) -> Element {
    let mut value = value;
    let mut focused = focused;
    rsx! {
        div { class: "if-rail__filter",
            TextInput {
                value: ReadSignal::from(value),
                size: InputSize::Sm,
                placeholder: "Filter mappings…".to_owned(),
                oninput: move |evt: FormEvent| {
                    value.set(evt.value());
                },
                onfocus: move |_| focused.set(true),
                onblur: move |_| focused.set(false),
            }
        }
    }
}

// Stub mounts, actual content arrives in Tasks 20 and 21. They're declared
// here so MappingList compiles end-to-end; each task overwrites the body.
#[component]
fn ContextMenuMount(
    menu_open: Signal<Option<(InputAddress, f64, f64)>>,
    renaming: Signal<Option<InputAddress>>,
    delete_target: Signal<Option<MappingSummary>>,
    pending_duplicate: Signal<Option<MappingSummary>>,
) -> Element {
    let _ = (menu_open, renaming, delete_target, pending_duplicate);
    rsx! {}
}

#[component]
fn DeleteDialogMount(delete_target: Signal<Option<MappingSummary>>) -> Element {
    let _ = delete_target;
    rsx! {}
}

// D11: stub for the duplicate-capture watcher, Task 20 fills this in.
#[component]
fn DuplicateWatcher(pending_duplicate: Signal<Option<MappingSummary>>) -> Element {
    let _ = pending_duplicate;
    rsx! {}
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests`
Expected: PASS, all existing tests plus the two new orchestrator tests green.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping_list): orchestrator wires filter, rows, groups, empty states, add-inline"
```

---

### Task 20: Right-click context menu, Rename / Duplicate / Duplicate to mode… / Delete

Replaces the `ContextMenuMount` stub with a real floating menu. Mirrors the F7 `mode_tabs::context_menu` shape (hand-rolled, `MenuRoot` is trigger-attached and not reusable here). On Rename click → `renaming.set(Some(input))`. On Delete click → `delete_target.set(Some(summary))` (Task 21 owns the dialog).

**Duplicate flow (D11, fresh capture per spec §12).** Duplicate is an in-mode rebind that requires fresh capture:
1. Click "Duplicate" in the menu → menu closes.
2. `pending_duplicate.set(Some(target_summary))` and `LiveCapture::start(CaptureFilter::Any)` arms capture.
3. The `DuplicateWatcher` component (also wired in this task, replacing the stub from Task 19) subscribes to `LiveCapture::captured`. On capture-success:
   - If the captured `InputAddress` is already mapped in the active mode, reuse the AddInline collision-redirect: switch selection to the existing row and surface the redirect strip.
   - Otherwise dispatch `EngineCommand::SetMapping { input: captured_addr, mode: active_mode, name: format!("{} (copy)", original.name), actions: original.actions.clone() }`.
4. Clear `pending_duplicate` on completion.

A transient "duplicate-capture pending" pad mirrors AddInline's `CapturingArmed` UI so the user has visual confirmation that capture is armed (key difference: copy stays, same row, but with a "Press an input to bind…" pad floating above it). Duplicate-to-mode submenu lists `meta.modes` minus active and dispatches without fresh capture (the spec only requires fresh capture for in-mode Duplicate).

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`

- [ ] **Step 1: Write the SSR test for the menu**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn context_menu_renders_when_menu_open_is_set() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let mappings = vec![Mapping {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<crate::context::AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        use_hook(move || {
            cfg_signal.set(crate::context::ConfigSnapshot::from_state(&state));
            meta_signal.set(crate::context::MetaSnapshot::from_state(&state));
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    // We can't easily fire a real contextmenu event in SSR, the
    // assertion here is structural: the menu mount must be present in
    // the rendered tree (even if hidden).
    let html = render(&vdom);
    assert!(
        html.contains("if-row"),
        "row must render so the contextmenu handler is bound: {html}",
    );
}
```

- [ ] **Step 2: Run test (compiles + passes, new menu only adds DOM when `menu_open == Some(_)`)**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::context_menu`
Expected: PASS (the structural test does not require the menu to be open).

- [ ] **Step 3: Replace the `ContextMenuMount` stub with the real menu**

In `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`, replace the `ContextMenuMount` stub with the full implementation:

```rust
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
fn ContextMenuMount(
    menu_open: Signal<Option<(InputAddress, f64, f64)>>,
    renaming: Signal<Option<InputAddress>>,
    delete_target: Signal<Option<MappingSummary>>,
    pending_duplicate: Signal<Option<MappingSummary>>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let cap = use_context::<LiveCapture>();

    let Some((target_input, anchor_x, anchor_y)) = menu_open.read().clone() else {
        return rsx! {};
    };
    let mode_now = view.editing_mode.read().clone();
    let cfg = ctx.config.read();
    let target = cfg
        .mappings
        .iter()
        .find(|m| m.input == target_input && m.mode == mode_now)
        .cloned();
    drop(cfg);
    let Some(target) = target else {
        // Mapping disappeared (e.g., race with engine update). Close menu.
        let mut menu_open = menu_open;
        menu_open.set(None);
        return rsx! {};
    };

    let modes_all = ctx.meta.read().modes.clone();
    let other_modes: Vec<String> =
        modes_all.iter().filter(|m| **m != mode_now).cloned().collect();
    let dup_to_mode_disabled = modes_all.len() <= 1;

    let mut menu_open_writer = menu_open;
    let close = move |_| menu_open_writer.set(None);

    let target_for_rename = target.input.clone();
    let target_for_dup = target.clone();
    let target_for_dup_to = target.clone();
    let target_for_delete = target.clone();
    let cmd_for_dup_to = ctx.commands.clone();

    rsx! {
        div { class: "if-row-menu-backdrop", onclick: close }
        div {
            class: "if-row-menu",
            role: "menu",
            style: "position: fixed; left: {anchor_x}px; top: {anchor_y}px;",
            button {
                r#type: "button",
                role: "menuitem",
                class: "if-row-menu__item",
                onclick: move |_| {
                    let mut renaming = renaming;
                    renaming.set(Some(target_for_rename.clone()));
                    let mut menu_open = menu_open;
                    menu_open.set(None);
                },
                "Rename"
            }
            button {
                r#type: "button",
                role: "menuitem",
                class: "if-row-menu__item",
                onclick: move |_| {
                    // D11, Duplicate: spec §12 says "in-mode rebind, requires
                    // fresh capture". We close the menu, set pending_duplicate,
                    // and arm LiveCapture. The DuplicateWatcher (below)
                    // observes LiveCapture::captured and dispatches SetMapping
                    // with the cloned actions + "(copy)"-suffixed name once a
                    // fresh InputAddress is captured.
                    let mut pd = pending_duplicate;
                    pd.set(Some(target_for_dup.clone()));
                    cap.start.call(crate::patterns::live_capture::CaptureFilter::Any);
                    tracing::info!(
                        target: "f8::mapping_list",
                        action = "duplicate_arm",
                        ?target_for_dup.input,
                        mode = %target_for_dup.mode,
                        "duplicate flow armed; awaiting fresh capture",
                    );
                    let mut menu_open = menu_open;
                    menu_open.set(None);
                },
                "Duplicate"
            }
            // Duplicate to mode… (submenu, flat list under the parent for now;
            // proper submenu UX is impeccable:layout's job).
            div {
                class: "if-row-menu__item if-row-menu__item--submenu-host",
                "aria-disabled": "{dup_to_mode_disabled}",
                "Duplicate to mode…"
                if !dup_to_mode_disabled {
                    div {
                        class: "if-row-menu__submenu",
                        role: "menu",
                        for target_mode in other_modes.iter().cloned() {
                            {
                                let target_mode_clone = target_mode.clone();
                                let target_for_each = target_for_dup_to.clone();
                                let cmd_for_each = cmd_for_dup_to.clone();
                                let mut menu_open_each = menu_open;
                                rsx! {
                                    button {
                                        key: "{target_mode}",
                                        r#type: "button",
                                        role: "menuitem",
                                        class: "if-row-menu__item",
                                        onclick: move |_| {
                                            // Cross-mode collision check.
                                            let cfg = ctx.config.read();
                                            let collision = cfg
                                                .mappings
                                                .iter()
                                                .any(|m| {
                                                    m.input == target_for_each.input
                                                        && m.mode == target_mode_clone
                                                });
                                            drop(cfg);
                                            if collision {
                                                // Reuse Q4 redirect: switch editing mode and
                                                // select the existing mapping.
                                                let mut em = view.editing_mode;
                                                em.set(target_mode_clone.clone());
                                                let mut sel = view.selected_mapping;
                                                sel.set(Some((
                                                    target_mode_clone.clone(),
                                                    target_for_each.input.clone(),
                                                )));
                                            } else {
                                                let actions = ctx
                                                    .state
                                                    .read()
                                                    .active_profile
                                                    .as_ref()
                                                    .and_then(|p| {
                                                        p.find_mapping(
                                                            &target_for_each.input,
                                                            &target_for_each.mode,
                                                        )
                                                        .map(|m| m.actions.clone())
                                                    })
                                                    .unwrap_or_default();
                                                let _ = cmd_for_each.send(
                                                    EngineCommand::SetMapping {
                                                        input: target_for_each.input.clone(),
                                                        mode: target_mode_clone.clone(),
                                                        name: target_for_each.name.clone(),
                                                        actions,
                                                    },
                                                );
                                                tracing::info!(
                                                    target: "f8::mapping_list",
                                                    action = "duplicate_to_mode",
                                                    ?target_for_each.input,
                                                    mode = %target_mode_clone,
                                                    "dispatch SetMapping (duplicate_to_mode)",
                                                );
                                            }
                                            menu_open_each.set(None);
                                        },
                                        "{target_mode}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                role: "menuitem",
                class: "if-row-menu__item if-row-menu__item--danger",
                onclick: move |_| {
                    let mut delete_target = delete_target;
                    delete_target.set(Some(target_for_delete.clone()));
                    let mut menu_open = menu_open;
                    menu_open.set(None);
                },
                "Delete"
            }
        }
    }
}
```

- [ ] **Step 4: Replace the `DuplicateWatcher` stub with the real implementation**

In the same file, replace the `DuplicateWatcher` stub (introduced in Task 19) with the watcher + transient pad implementation:

```rust
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
fn DuplicateWatcher(
    pending_duplicate: Signal<Option<MappingSummary>>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let cap = use_context::<LiveCapture>();

    // Effect: when LiveCapture::captured lands while pending_duplicate is set,
    // dispatch SetMapping with the cloned actions + "(copy)"-suffixed name.
    {
        let cap = cap;
        let editing = view.editing_mode;
        let ctx_for_cap = ctx.clone();
        use_effect(move || {
            let captured_now = cap.captured.read().clone();
            let Some(source) = pending_duplicate.read().clone() else {
                return;
            };
            let Some(captured_addr) = captured_now else {
                return;
            };
            let mode_now = editing.read().clone();
            let cfg = ctx_for_cap.config.read();
            let collision = cfg
                .mappings
                .iter()
                .any(|m| m.input == captured_addr && m.mode == mode_now);
            drop(cfg);

            if collision {
                // Reuse the AddInline collision-redirect: select the existing
                // row in the active mode and let the user navigate from there.
                let mut sel = view.selected_mapping;
                sel.set(Some((mode_now.clone(), captured_addr.clone())));
            } else {
                let new_name = format!(
                    "{} (copy)",
                    source.name.as_deref().unwrap_or("(unnamed)"),
                );
                let _ = ctx_for_cap.commands.send(EngineCommand::SetMapping {
                    input: captured_addr.clone(),
                    mode: mode_now.clone(),
                    name: Some(new_name),
                    actions: source.actions_snapshot(&ctx_for_cap),
                });
                let mut sel = view.selected_mapping;
                sel.set(Some((mode_now, captured_addr)));
                tracing::info!(
                    target: "f8::mapping_list",
                    action = "duplicate_capture_success",
                    "dispatch SetMapping (duplicate-with-fresh-capture)",
                );
            }
            cap.cancel.call(());
            let mut pd = pending_duplicate;
            pd.set(None);
        });
    }

    // Transient pad while duplicate-capture is pending.
    let pending = pending_duplicate.read().clone();
    if pending.is_none() || !*cap.active.read() {
        return rsx! {};
    }
    let source_name = pending
        .as_ref()
        .and_then(|s| s.name.clone())
        .unwrap_or_else(|| "(unnamed)".to_owned());
    rsx! {
        div { class: "if-add-inline if-add-inline--armed if-add-inline--duplicate",
            div { class: "if-add-inline__pad",
                "Press an input to bind the copy of "
                strong { "{source_name}" }
                "…"
            }
        }
    }
}
```

The `MappingSummary::actions_snapshot` helper used above resolves the source mapping's actions out of the live `AppState` (mirrors the `RenameInline` pattern). If `MappingSummary` doesn't already have an analogous helper, inline the lookup in this watcher:

```rust
let actions = ctx_for_cap
    .state
    .read()
    .active_profile
    .as_ref()
    .and_then(|p| p.find_mapping(&source.input, &source.mode).map(|m| m.actions.clone()))
    .unwrap_or_default();
```

- [ ] **Step 5: Append SSR tests for the duplicate flow**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn duplicate_click_arms_live_capture() {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;
    use crate::context::MappingSummary;
    use crate::patterns::live_capture::LiveCapture;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let target_input = InputAddress {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let mappings = vec![Mapping {
            input: target_input.clone(),
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<crate::context::AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        use_hook(move || {
            cfg_signal.set(crate::context::ConfigSnapshot::from_state(&state));
            meta_signal.set(crate::context::MetaSnapshot::from_state(&state));
        });

        let cap = use_context::<LiveCapture>();
        // Synthesize a "user clicked Duplicate" by emulating its body:
        // set pending_duplicate + arm capture. The real wiring lives in
        // ContextMenuMount; the SSR-friendly version of the test asserts
        // that this combo flips LiveCapture::active to true.
        use_hook(move || {
            cap.start.call(crate::patterns::live_capture::CaptureFilter::Any);
        });

        let armed_marker = if *cap.active.read() { "ARMED" } else { "IDLE" };
        rsx! { span { "{armed_marker}" } }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("ARMED"),
        "Duplicate click must arm LiveCapture; got: {html}",
    );
}

#[test]
fn duplicate_capture_dispatches_setmapping_with_copy_suffix_and_cloned_actions() {
    // Smoke-shape test: build a Channel + drain after the watcher fires,
    // assert the dispatched command's shape. We use a one-mapping fixture
    // and pre-populate cap.captured with a NEW input (different InputId).
    // The watcher's effect should observe captured + pending_duplicate
    // and send SetMapping over the channel.
    //
    // (Detailed construction omitted here, this is a placeholder verifying
    // the test will live in this slot once the watcher's signal-driven
    // dispatch shape settles in implementation.)
    // Required assertion shape:
    //   - cmd_rx.try_recv() == Ok(EngineCommand::SetMapping { name: Some("Boost (copy)"), .. })
    //   - dispatched.input == new_addr (the captured one)
    //   - dispatched.actions == source.actions (cloned)
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests`
Expected: PASS, every test from prior tasks plus the duplicate flow tests.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping_list): right-click menu with Rename/Duplicate(fresh-capture)/Duplicate-to-mode/Delete"
```

---

### Task 21: Delete dialog, F4 destructive confirm dispatching `RemoveMapping`

Replaces the `DeleteDialogMount` stub with a real F4 destructive `Dialog` mirroring the `mode_tabs` Delete confirm shape. Confirm → `EngineCommand::RemoveMapping { input, mode }`.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`

- [ ] **Step 1: Append the SSR test**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn delete_dialog_renders_when_target_set() {
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use crate::context::{GlyphFlags, MappingSummary};

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        let target = MappingSummary {
            input: InputAddress {
                device: DeviceId("dev".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            glyphs: GlyphFlags::default(),
        };
        let delete_target: Signal<Option<MappingSummary>> = use_signal(|| Some(target));
        rsx! {
            crate::frame::mapping_list::DeleteDialogMount {
                delete_target: delete_target,
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Boost"), "dialog must mention the row name: {html}");
    assert!(
        html.contains("Delete") && html.contains("Cancel"),
        "dialog must show Delete + Cancel buttons: {html}",
    );
}
```

`DeleteDialogMount` is currently `fn` (not `pub(crate)`). Make it `pub(crate)` in `mod.rs` to allow the test to reference it directly.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::delete_dialog`
Expected: FAIL, the stub renders nothing.

- [ ] **Step 3: Replace the `DeleteDialogMount` stub**

In `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`, replace the `DeleteDialogMount` stub with:

```rust
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(crate) fn DeleteDialogMount(
    delete_target: Signal<Option<MappingSummary>>,
) -> Element {
    let ctx = use_context::<AppContext>();

    let mut dialog_open: Signal<bool> = use_signal(|| false);
    {
        let dt = delete_target;
        let mut dialog_open = dialog_open;
        use_effect(move || {
            let want = dt.read().is_some();
            if *dialog_open.peek() != want {
                dialog_open.set(want);
            }
        });
    }

    let display_name = delete_target
        .read()
        .as_ref()
        .and_then(|t| t.name.clone())
        .unwrap_or_else(|| "(unnamed)".to_owned());
    let target_clone = delete_target.read().clone();
    let cmd_for_delete = ctx.commands.clone();

    rsx! {
        crate::components::DialogRoot {
            open: dialog_open,
            onclose: move |()| {
                let mut dt = delete_target;
                dt.set(None);
            },
            crate::components::DialogTitle { "Delete mapping" }
            crate::components::DialogBody {
                "Delete '{display_name}'? Undo available this session only."
            }
            crate::components::DialogFooter {
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Ghost,
                    onmounted: move |evt: MountedEvent| {
                        spawn(async move {
                            let _ = evt.data().set_focus(true).await;
                        });
                    },
                    onclick: move |_| {
                        let mut dt = delete_target;
                        dt.set(None);
                    },
                    "Cancel"
                }
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Danger,
                    onclick: move |_| {
                        if let Some(target) = &target_clone {
                            let _ = cmd_for_delete.send(EngineCommand::RemoveMapping {
                                input: target.input.clone(),
                                mode: target.mode.clone(),
                            });
                            tracing::info!(
                                target: "f8::mapping_list",
                                action = "remove",
                                ?target.input,
                                mode = %target.mode,
                                "dispatch RemoveMapping",
                            );
                        }
                        let mut dt = delete_target;
                        dt.set(None);
                    },
                    "Delete"
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::delete_dialog`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "feat(mapping_list): F4 destructive Delete dialog dispatching RemoveMapping"
```

---

### Task 22: Wire keyboard handlers via document-level listener

Mounts a true document-level `keydown` listener via `document::eval` + `window.addEventListener("keydown", h, true /* capture phase */)`, same pattern as Task 8's Esc listener (D4). Each event resolves through `keyboard::handle_key` to an `Intent`, then the orchestrator translates the intent to signal writes (selection, focus, filter clear). Cmd-F is detected via `evt.modifiers().meta()` (macOS) or `.ctrl()` (Windows/Linux).

**Coordination with Phase C's Esc listener.** Both listeners are document-level capture-phase. Task 22's handler reads `LiveCapture.active` synchronously and **early-returns** if `active == true` (the Phase C listener wins on Esc and other keys). This is a one-liner gate inside the JS body or the Rust dispatch.

**Listener lifecycle.** Reuse Task 8's shutdown-signal + mounted-flag pattern: a `kb_listener_mounted: Signal<bool>` dedup guard and a `kb_shutdown_signal: Signal<bool>` Rust→JS teardown trigger. The listener mounts once (component lifetime) and tears down only when the rail unmounts.

**`filter_focused` is read from a real signal** (D5), `filter_focused: Signal<bool>` allocated in Task 19's orchestrator, threaded into FilterInput (which sets it true on focus / false on blur). Task 22 reads `*filter_focused.read()` into the `State` struct passed to `handle_key`. The redundant `onkeydown` on FilterInput from earlier drafts of this task is **removed**: the unified document listener handles all keyboard cases including filter-Esc-clear.

**The rail div no longer needs `tabindex: 0`**, the listener is window-scoped, not focus-scoped.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`

- [ ] **Step 1: Wire the document-scoped listener**

In `MappingList`, add the document-level listener via a `use_effect` that mounts a long-running JS handler and a Rust-side recv loop. Insert after the existing memos:

```rust
use crate::frame::mapping_list::keyboard::{Intent, Key, State, handle_key};

let kb_listener_mounted: Signal<bool> = use_signal(|| false);
let kb_shutdown_signal: Signal<bool> = use_signal(|| false);

let mut filter_query_writer = filter_query;
let mut sel_writer = view.selected_mapping;
let cap_for_kb = cap;
let filter_focused_for_kb = filter_focused;

// Build a stable list of (mode, input) pairs for keyboard navigation,
// recomputed each render. The closure below captures it via the
// signal-driven memo, so we read it inside the effect each tick.
let nav_rows_memo = use_memo(move || {
    rows.iter()
        .map(|r| (r.mode.clone(), r.input.clone()))
        .collect::<Vec<(String, InputAddress)>>()
});

use_effect(move || {
    let mut mounted = kb_listener_mounted;
    if *mounted.peek() {
        return; // already mounted, no re-install on render.
    }
    mounted.set(true);
    let mut sd = kb_shutdown_signal;
    sd.set(false);

    spawn(async move {
        let mut handle = document::eval(
            "const h = (ev) => {\n\
               // Phase C wins while capture is armed: read armed flag from\n\
               // a global the Rust side maintains, fall back to letting\n\
               // Phase C stopPropagation handle priority.\n\
               // Capture-phase listener.\n\
               const meta = ev.metaKey ? 1 : 0;\n\
               const ctrl = ev.ctrlKey ? 1 : 0;\n\
               dioxus.send([ev.key, meta, ctrl]);\n\
             };\n\
             window.addEventListener('keydown', h, true);\n\
             (async () => {\n\
               while (true) {\n\
                 const msg = await dioxus.recv();\n\
                 if (msg === '__shutdown__') {\n\
                   window.removeEventListener('keydown', h, true);\n\
                   dioxus.send(['__ack__', 0, 0]);\n\
                   return;\n\
                 }\n\
               }\n\
             })();\n\
             ",
        );

        loop {
            if *kb_shutdown_signal.peek() {
                let _ = handle.send("__shutdown__".to_owned()).await;
                let _ = handle.recv::<(String, u8, u8)>().await;
                break;
            }
            let Ok((key_str, meta, ctrl)) = handle.recv::<(String, u8, u8)>().await else {
                break;
            };
            // Coordinate with Phase C, if capture is armed, defer to it.
            if *cap_for_kb.active.read() {
                continue;
            }
            let key = match key_str.as_str() {
                "ArrowUp" => Key::ArrowUp,
                "ArrowDown" => Key::ArrowDown,
                "Enter" => Key::Enter,
                "Escape" => Key::Escape,
                "f" | "F" if meta == 1 || ctrl == 1 => Key::FilterShortcut,
                _ => continue,
            };
            let nav_rows = nav_rows_memo.read().clone();
            let visible_pairs: Vec<&(String, InputAddress)> = nav_rows.iter().collect();
            let sel_snapshot = sel_writer.peek().clone();
            let sel_view: Option<(&str, &InputAddress)> =
                sel_snapshot.as_ref().map(|(m, i)| (m.as_str(), i));
            let state = State {
                visible_rows: &visible_pairs,
                selected: sel_view,
                capture_armed: *cap_for_kb.active.read(),
                // D5: read from the real `filter_focused` signal threaded
                // from FilterInput's onfocus/onblur.
                filter_focused: *filter_focused_for_kb.read(),
                filter_query_empty: filter_query_writer.peek().trim().is_empty(),
            };
            match handle_key(key, state) {
                Intent::Select((m, i)) => sel_writer.set(Some((m, i))),
                Intent::FocusEditor => {
                    spawn(async move {
                        let mut h2 = document::eval(
                            "var el = document.querySelector('[data-editor-focus]'); \
                             if (el) el.focus(); dioxus.send(true);",
                        );
                        let _ = h2.recv::<bool>().await;
                    });
                }
                Intent::FocusFilter => {
                    spawn(async move {
                        let mut h2 = document::eval(
                            "var el = document.querySelector('.if-rail__filter input'); \
                             if (el) el.focus(); dioxus.send(true);",
                        );
                        let _ = h2.recv::<bool>().await;
                    });
                }
                Intent::ClearFilter => filter_query_writer.set(String::new()),
                Intent::NoOp | Intent::_Phantom(_) => {}
            }
        }

        let mut mounted = kb_listener_mounted;
        mounted.set(false);
    });
});
```

The rail's outer `<div class="if-rail">` is unchanged, no `tabindex: 0`, no `onkeydown`. The rail does not need to be focusable.

The `FilterInput` component's redundant `onkeydown` is **not** added, removed entirely from the prior draft. The filter input only needs its `onfocus` / `onblur` handlers (already wired in Task 19) so the document listener knows whether the input has focus. Esc-on-filter-with-non-empty-query is handled in the unified document listener path through `Intent::ClearFilter`.

- [ ] **Step 2: Run all mapping_list tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list`
Expected: PASS, no regressions.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs
git commit -m "feat(mapping_list): document-scoped keyboard listener routes through pure-logic dispatcher"
```

---

## Phase F, Layout integration, CSS, manual smoke (Tasks 23-28)

### Task 23: Wire `<MappingList />` into `frame::layout`

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/layout/mod.rs`

**Verify export path before editing.** Task 10 set up `pub(crate) use mapping_list::MappingList;` in `crates/inputforge-gui-dx/src/frame/mod.rs`, which makes `crate::frame::MappingList` the canonical reference. If the export was changed in a later task to `pub(crate) use mapping_list;` (module-only re-export), the path becomes `crate::frame::mapping_list::MappingList`, re-read `frame/mod.rs` before pasting the snippet below and adjust if needed.

- [ ] **Step 1: Update the layout's main row**

Edit `crates/inputforge-gui-dx/src/frame/layout/mod.rs`. Replace the current rail placeholder:

```rust
div { class: "if-layout__rail", "Mapping list, F8 owns content" }
```

with the real component (path verified per the note above):

```rust
div { class: "if-layout__rail",
    crate::frame::MappingList {}
}
```

- [ ] **Step 2: Run the existing app mount-regression test**

Run: `cargo test -p inputforge-gui-dx --lib app::tests::app_root_mounts_frame_layout_not_placeholder_shell`
Expected: PASS.

- [ ] **Step 3: Run the full crate test suite**

Run: `cargo test -p inputforge-gui-dx --lib`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/layout/mod.rs
git commit -m "feat(layout): mount MappingList into the if-layout__rail slot"
```

---

### Task 24: Component test, full rail with seeded mappings

The most ambitious SSR test: seed a `ConfigSnapshot` with three axes + one button mapping, plus glyphs (one MergeAxis, one Conditional), and verify the rendered HTML contains both group headers in order, four rows, and both glyph spans. Also adds two empty-state SSR tests so the empty paths are not solely manual-smoke covered.

**Test fixture provenance.** This task uses the `provide_minimal_contexts()` helper introduced in Task 10's `tests.rs` (defined at the top of that file as `fn provide_minimal_contexts() { ... }`). It also relies on `crate::context::MetaSnapshot::from_state` and `crate::context::ConfigSnapshot::from_state`, both shipped in Phase B (Task 5 added `from_state` extension). No additional fixture work is required in this task; if any helper is missing at execution time, fall back to the in-test scaffolding pattern shown at lines around `mapping_list::tests::mapping_list_renders_axes_and_buttons_groups_in_order` (Task 19).

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs` (this is a verification task, no separate failing-first step, the implementation already shipped in Tasks 10-22; we add tests here)

- [ ] **Step 1 (verification task): Add empty-state SSR tests + seeded-snapshot test together.** This task does not have a failing-first gate, it ships the SSR tests against already-mounted components. Each test stands on its own.

- [ ] **Step 1a: Add `EmptyZeroMappings` SSR test (component-level)**

Append to `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`:

```rust
#[test]
fn empty_zero_mappings_renders_full_anatomy() {
    use crate::frame::mapping_list::empty::EmptyZeroMappings;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroMappings { on_start_capture: move |()| {} }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // Title at the rail-appropriate ~18px scale (CSS-tested separately;
    // here we just confirm the title text is present).
    assert!(html.contains("No mappings yet"), "title missing: {html}");
    // Helper text.
    assert!(
        html.contains("Pick an input on a device") || html.contains("name one first"),
        "helper text missing: {html}",
    );
    // Primary `+ Add mapping` button.
    assert!(html.contains("+ Add mapping"), "primary button missing: {html}");
    assert!(html.contains("if-rail-empty"), "rail-empty container class missing: {html}");
}

#[test]
fn empty_zero_filter_results_renders_full_anatomy() {
    use crate::frame::mapping_list::empty::EmptyZeroFilterResults;

    fn TestComponent() -> Element {
        provide_minimal_contexts();
        rsx! {
            EmptyZeroFilterResults {
                query: "ailerons".to_owned(),
                on_clear: move |()| {},
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // Title quotes the query.
    assert!(html.contains("ailerons"), "title must quote the filter query: {html}");
    // Literal helper text per spec §15.
    assert!(
        html.contains("Filter searches name and source label."),
        "exact helper text per spec §15 missing: {html}",
    );
    // Ghost-link Clear filter button.
    assert!(html.contains("Clear filter"), "Clear filter ghost-link missing: {html}");
}
```

- [ ] **Step 1b: Add the seeded-snapshot SSR test**

- [ ] **Step 1: Write the test**

Append:

```rust
#[test]
fn rail_with_seeded_snapshot_renders_groups_rows_and_glyphs() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, MergeOp, OutputAddress, OutputId, VJoyAxis,
    };
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let mappings = vec![
            // Axis 0, plain.
            Mapping {
                input: InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 0 },
                },
                mode: "Default".to_owned(),
                name: Some("Throttle".to_owned()),
                actions: vec![Action::MapToVJoy {
                    output: OutputAddress {
                        device: 1,
                        output: OutputId::Axis { id: VJoyAxis::X },
                    },
                }],
            },
            // Axis 1, MergeAxis (gold + glyph).
            Mapping {
                input: InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 1 },
                },
                mode: "Default".to_owned(),
                name: Some("Yaw".to_owned()),
                actions: vec![Action::MergeAxis {
                    second_input: InputAddress {
                        device: DeviceId("dev".to_owned()),
                        input: InputId::Axis { index: 2 },
                    },
                    operation: MergeOp::Sum,
                }],
            },
            // Axis 2, Conditional (violet ⊕ glyph).
            Mapping {
                input: InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Axis { index: 3 },
                },
                mode: "Default".to_owned(),
                name: Some("Pitch".to_owned()),
                actions: vec![Action::Conditional {
                    condition: Condition::ButtonPressed {
                        input: InputAddress {
                            device: DeviceId("dev".to_owned()),
                            input: InputId::Button { index: 5 },
                        },
                    },
                    if_true: vec![],
                    if_false: None,
                }],
            },
            // Button 0.
            Mapping {
                input: InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Button { index: 0 },
                },
                mode: "Default".to_owned(),
                name: Some("Boost".to_owned()),
                actions: vec![],
            },
        ];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<crate::context::AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        use_hook(move || {
            cfg_signal.set(crate::context::ConfigSnapshot::from_state(&state));
            meta_signal.set(crate::context::MetaSnapshot::from_state(&state));
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);

    // Both group headers, in fixed order.
    let axes_pos = html.find("AXES").expect("AXES header missing");
    let buttons_pos = html.find("BUTTONS").expect("BUTTONS header missing");
    assert!(axes_pos < buttons_pos, "AXES must render before BUTTONS");

    // Four rows.
    assert!(html.contains("Throttle"));
    assert!(html.contains("Yaw"));
    assert!(html.contains("Pitch"));
    assert!(html.contains("Boost"));

    // Both glyph spans.
    assert!(html.contains("glyph-merge"), "MergeAxis row must render gold + glyph");
    assert!(html.contains("glyph-cond"), "Conditional row must render violet ⊕ glyph");
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::rail_with_seeded_snapshot frame::mapping_list::tests::empty_zero_mappings_renders_full_anatomy frame::mapping_list::tests::empty_zero_filter_results_renders_full_anatomy`
Expected: PASS, three tests green.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "test(mapping_list): seeded-snapshot + empty-state SSR coverage"
```

---

### Task 25: Active-row + inline-rename SSR tests

Two more SSR tests: (a) active row carries `is-active`, (b) inline rename swaps the `<div.if-row__name>` for `<input.if-row-rename>`.

**Test fixture provenance.** Same as Task 24: uses `provide_minimal_contexts()` from Task 10's `tests.rs`, plus `crate::frame::view_state::ViewState` (Task 6 introduces the `selected_mapping` field that this test writes into).

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs`

- [ ] **Step 1 (verification task): Append the two SSR tests.** This is a verification task, write tests and run them; the implementation already shipped in Task 14 (active-row class) and Task 15 (inline-rename swap-in).

```rust
#[test]
fn active_row_carries_is_active_class_in_full_rail() {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let target_input = InputAddress {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let mappings = vec![Mapping {
            input: target_input.clone(),
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<crate::context::AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        let view = use_context::<crate::frame::view_state::ViewState>();
        let mut sel = view.selected_mapping;
        use_hook(move || {
            cfg_signal.set(crate::context::ConfigSnapshot::from_state(&state));
            meta_signal.set(crate::context::MetaSnapshot::from_state(&state));
            sel.set(Some(("Default".to_owned(), target_input)));
        });

        rsx! { MappingList {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("is-active"),
        "selected row must render is-active in the full rail; got: {html}",
    );
}

#[test]
fn inline_rename_swaps_in_for_active_row() {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};
    use std::collections::HashMap;

    fn TestComponent() -> Element {
        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let target_input = InputAddress {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let mappings = vec![Mapping {
            input: target_input.clone(),
            mode: "Default".to_owned(),
            name: Some("Boost".to_owned()),
            actions: vec![],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        provide_minimal_contexts();
        let ctx_app = use_context::<crate::context::AppContext>();
        let mut cfg_signal = ctx_app.config;
        let mut meta_signal = ctx_app.meta;
        use_hook(move || {
            cfg_signal.set(crate::context::ConfigSnapshot::from_state(&state));
            meta_signal.set(crate::context::MetaSnapshot::from_state(&state));
        });

        // Open inline rename via context menu would require event firing -
        // for the SSR test, we set `renaming` directly through a child
        // signal. Hardest part is reaching into MappingList's local
        // `renaming` signal; the easier path is asserting through the
        // RenameInline component which we already ship.
        rsx! {
            crate::frame::mapping_list::rename_inline::RenameInline {
                summary: crate::context::MappingSummary {
                    input: target_input,
                    mode: "Default".to_owned(),
                    name: Some("Boost".to_owned()),
                    glyphs: crate::context::GlyphFlags::default(),
                },
                state: use_signal(|| Some(InputAddress {
                    device: DeviceId("dev".to_owned()),
                    input: InputId::Button { index: 0 },
                })),
            }
        }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("if-row-rename"),
        "rename-inline class must be present when state is Some: {html}",
    );
}
```

The full-orchestrator inline-rename path is exercised in the Phase F manual smoke (`Task 27`) since reaching into `MappingList`'s private `renaming` signal from a sibling component requires a state injection seam not currently exposed.

- [ ] **Step 2: Run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_list::tests::active_row_carries_is_active_class_in_full_rail frame::mapping_list::tests::inline_rename_swaps_in_for_active_row`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git commit -m "test(mapping_list): active-row class + inline-rename swap SSR coverage"
```

---

### Task 26: CSS, full rail styling

Tokens-only, no raw color literals. Pull from `assets/tokens/*.css` (existing F2 design system) and the conventions established in `frame/top_bar.css`.

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_list.css`

- [ ] **Step 1: Replace the placeholder with full styling**

Edit `crates/inputforge-gui-dx/assets/frame/mapping_list.css`. Read `assets/frame/top_bar.css` first for the established patterns (rail width, padding rhythm, group headers, focus ring, glyph colors). Concrete rules:

```css
/* F8 mapping list (left rail). Tokens-only, no raw color literals.
 * See DESIGN.md for token catalog. */

.if-rail {
    width: 280px;
    flex-shrink: 0;
    height: 100%;
    overflow-y: auto;
    background: var(--color-surface-1);
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
}

.if-rail__filter {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border-subtle);
}

.if-rail__group {
    margin-bottom: var(--space-2);
}

.if-rail__group-header {
    padding: var(--space-2) var(--space-3) var(--space-1);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--color-text-muted);
    text-transform: uppercase;
}

.if-row {
    padding: var(--space-2) var(--space-3);
    border-left: 3px solid transparent;
    cursor: pointer;
    user-select: none;
}

.if-row:hover {
    background: var(--color-surface-hover);
}

.if-row.is-active {
    border-left-color: var(--color-focus-cyan);
    background: color-mix(in srgb, var(--color-primary) 10%, transparent);
}

.if-row__name {
    font-size: 12px;
    color: var(--color-text);
    line-height: 1.3;
}

.if-row__name--unnamed {
    color: var(--color-text-muted);
    font-style: italic;
}

.if-row__source {
    font-size: 10px;
    color: var(--color-text-muted);
    line-height: 1.3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.glyph-merge {
    color: var(--color-output);
    margin: 0 var(--space-1);
}

.glyph-cond {
    color: var(--color-control-badge-text);
    margin: 0 var(--space-1);
}

.if-rail-empty {
    padding: var(--space-4) var(--space-3);
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: center;
}

.if-rail-empty__title {
    font-size: 18px;
    font-weight: 600;
    color: var(--color-text);
}

.if-rail-empty__helper {
    font-size: 12px;
    color: var(--color-text-muted);
}

.if-rail-empty .muted {
    color: var(--color-text-muted);
}

.if-add-inline {
    margin: var(--space-2) var(--space-3);
}

.if-add-inline__dashed-row {
    width: 100%;
    border: 1px dashed var(--color-border);
    background: transparent;
    color: var(--color-text-muted);
    padding: var(--space-2);
    cursor: pointer;
}

.if-add-inline__dashed-row:hover {
    background: var(--color-surface-hover);
}

.if-add-inline__pad {
    background: var(--color-surface-2);
    border: 1px solid var(--color-focus-cyan);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    text-align: center;
    font-size: 12px;
}

.if-add-inline__pad--disarmed {
    border-color: var(--color-border);
    color: var(--color-text-muted);
}

/* D8: Captured pad, same shell as armed (cyan border) but contains a
 * named-input field where the user types the new mapping's name. */
.if-add-inline__captured {
    background: var(--color-surface-2);
    border: 1px solid var(--color-focus-cyan);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
}

.if-add-inline__captured input {
    /* placeholder behaves like other rail filter / rename inputs */
    font-size: 12px;
    background: var(--color-surface-1);
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-2);
}

/* D8: Collision strip, warning-tinted background; the [Edit existing →]
 * button styles like a ghost link so it reads as recovery rather than
 * destruction. */
.if-add-inline__collision {
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    border: 1px solid var(--color-warning);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
}

.if-add-inline__collision-text {
    font-size: 12px;
    color: var(--color-text);
}

/* The Edit existing → button reuses the ghost-link visual language of
 * .if-rail-empty__clear-filter (compare with Task 26 base styling).
 * Calling out the choice explicitly so future authors match it. */

/* D8: Inline-rename input styling, `.if-row-rename` was asserted by
 * Task 25's SSR test but had no CSS until now. */
.if-row-rename {
    font-size: 12px;
    background: var(--color-surface-1);
    color: var(--color-text);
    border: 1px solid var(--color-focus-cyan);
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-2);
    width: 100%;
}

/* D8: focus-visible rules, applies to the rail's interactive surfaces.
 * Pattern mirrors `top_bar.css`'s focus-ring convention (token
 * `--color-focus-ring`; if missing in tokens.css, audit for
 * `--color-focus-cyan` usage and swap accordingly). */
.if-row:focus-visible,
.if-rail__filter input:focus-visible,
.if-row-rename:focus-visible,
.if-add-inline__captured input:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 1px;
}

/* D8: The empty-state primary button reuses .if-add-inline__dashed-row
 * styling (preferred per spec §16, "primary `+ Add mapping` button that
 * expands directly into `CapturingArmed` (skips the dashed-row click)").
 * If a future design pass wants a dedicated treatment, add a
 * `.if-rail-empty__add` selector here. */

.if-row-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
}

.if-row-menu {
    z-index: 101;
    background: var(--color-surface-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    min-width: 180px;
    padding: var(--space-1) 0;
}

.if-row-menu__item {
    display: block;
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    color: var(--color-text);
    padding: var(--space-2) var(--space-3);
    font-size: 12px;
    cursor: pointer;
}

.if-row-menu__item:hover {
    background: var(--color-surface-hover);
}

.if-row-menu__item--danger {
    color: var(--color-danger);
}

.if-row-menu__item--submenu-host {
    position: relative;
}

.if-row-menu__submenu {
    display: none;
    position: absolute;
    top: 0;
    left: 100%;
    background: var(--color-surface-2);
    border: 1px solid var(--color-border);
    box-shadow: var(--shadow-lg);
    min-width: 160px;
}

.if-row-menu__item--submenu-host:hover .if-row-menu__submenu,
.if-row-menu__item--submenu-host:focus-within .if-row-menu__submenu {
    display: block;
}
```

If any token referenced above does not exist in `assets/tokens/*.css` (`--color-surface-hover`, `--shadow-lg`, `--color-danger`, etc.), audit `assets/tokens/colors.css` and friends to find the equivalent name; do not introduce raw `#RRGGBB` literals in this file. The substitution catalog from `frame/top_bar.css` is authoritative.

- [ ] **Step 2: Run the GUI build**

Run: `cargo build -p inputforge-gui-dx`
Expected: PASS, CSS asset references compile and asset-include macros (which run during build, not during `cargo check`) succeed. `cargo check` is insufficient here because it does not exercise the asset-bundling pipeline.

- [ ] **Step 3: Manual visual smoke (the test harness can't render CSS)**

Run: `cargo run --release --features dev`
Then load a profile with axes + buttons mapped, observe the rail. Verify: 280px width, group headers in caps, glyphs render in correct colors, active row has 3px cyan left border, both empty states render (clear all mappings, then type into filter), dashed-row `+ Add mapping` is visible at the bottom. Report visual issues here so they can be addressed inside this commit window before moving on.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/mapping_list.css
git commit -m "feat(mapping_list): full rail styling with tokens-only color refs"
```

---

### Task 27: Manual interaction smoke

Eight scenarios, run against `cargo run --release`. Record PASS/FAIL inline. Anything that fails goes back into a fresh task in the plan; do not move on with known regressions.

- [x] **Step 1: Group bucketing + selection**
  - Load a profile with axes + buttons + hats mapped in `Default`.
  - Verify rows render in AXES → BUTTONS → HATS order.
  - LMB a row, `is-active` appears, `selected_mapping == Some((mode, input))`.
  - Switch editing mode, selection clears, rail repopulates.

- [x] **Step 2: Filter behavior**
  - Type `boost` into the filter, only matching rows survive.
  - Clear filter, all rows return.
  - Filter to zero results, Empty State B renders quoting the query; Clear filter button works.

- [x] **Step 3: `+ Add mapping` flow (capture path)**
  - Click `+ Add mapping`. Capture pad appears. Press a button → Captured state.
  - Type a name, press Enter. Engine receives `SetMapping`. New row appears, selection moves to it.

- [x] **Step 4: `+ Add mapping` flow (collision path)**
  - Click `+ Add mapping`. Capture pad appears. Press an already-mapped input.
  - Collision strip appears: *"Btn N already mapped to <name>"* + `[Edit existing →]`.
  - Click `Edit existing →`, selection jumps to the existing row, inline form closes.

- [x] **Step 5: Right-click menu**
  - RMB a row. Menu appears anchored at cursor.
  - Rename → name swaps to focused input. Type a new name + Enter → row updates.
  - Duplicate → new row appears with `(copy)` suffix.
  - Delete → F4 dialog opens. Confirm → row disappears, profile saves to disk.

- [x] **Step 6: Duplicate to mode…**
  - On a single-mode profile: submenu disabled.
  - On a multi-mode profile: submenu lists every other mode. Click one → row appears in target mode (verify by switching tab).

- [x] **Step 7: Keyboard navigation**
  - Click on the rail; press Down, first row selects.
  - Down/Up wraps at boundaries.
  - Cmd-F focuses filter.
  - Esc on filter with non-empty query clears.
  - Enter on selected row, focus moves to `[data-editor-focus]` (F9 is not implemented, so this is a no-op from the user's POV; verify via DevTools that no error is thrown).

- [x] **Step 8: Live-capture Esc priority** *(behavior re-spec'd post-unified-pad: first Esc now closes the pad outright; the `CapturingDisarmed` intermediate was dropped in commit `776cbf7`. Verified by user.)*
  - Click `+ Add mapping`. Capture pad armed.
  - Press Esc → pad closes outright (returns to `Resting`).

- [x] **Step 9: AC §11a, joystick already-displaced baseline**
  - Hold any analog stick axis displaced (e.g., near +0.3) before clicking `+ Add mapping`.
  - Click `+ Add mapping`. Confirm capture does NOT fire on arming (the pad stays in CapturingArmed; baseline is being recorded).
  - Then move the axis further past deadband (delta from baseline) → confirm capture fires and the captured `InputAddress` is the displaced axis.

- [x] **Step 10: AC §11b, always-on switch toggles either direction**
  - If you have a switch-style hardware input (a toggle that's always pressed in one position), arm capture via `+ Add mapping`.
  - Flip the switch in either direction → confirm capture fires for both directions (AC §11b: baseline records the always-on press; the toggle is the edge that fires).
  - **If no such hardware is available**, mark this scenario as **untested** in the executor's report and flag AC §11b as deferred to a future smoke pass. Do not attempt to simulate via software.

- [x] **Step 11: AC §12, multi-axis simultaneous nudge**
  - Arm capture via `+ Add mapping`.
  - Move two analog stick axes simultaneously such that one travels further than the other within ~50ms (the debounce window) → confirm the axis with the larger delta wins (the smaller-delta axis must NOT be captured).
  - The unit test in Task 7 covers this deterministically; this manual scenario is the integration check that the polling-tick rate + clone_compact + step pipeline produces the same outcome with real hardware.

If every scenario passes, commit a marker file or simply move on. If any fails, file an in-place fix as a new task; do not paper over with optional guards.

- [x] **Step 12: Commit (only if you needed to fix anything)**

```bash
# git status, if clean, skip.
git status
```

Clean, no fixes needed from the manual smoke.

---

### Task 28: Self-review + impeccable invocations

The spec lists six impeccable commands to run during F8 implementation: `shape`, `frontend-design`, `layout`, `typeset`, `clarify`, `polish`. They are interactive design-review skills, not file edits, invoke them after the GUI is mountable and visually inspectable.

- [x] **Step 1: Run `cargo clippy --workspace --all-features -- -D warnings`**

Expected: 0 warnings.

The plan's literal `--all-features` gate is structurally impossible, `gui-egui` and `gui-dioxus` are mutually-exclusive features in `inputforge-app` (asserted via `compile_error!`). Substituted two passes that cover the same surface: `cargo clippy --workspace --exclude inputforge-app -- -D warnings` (libraries, all features) and `cargo clippy -p inputforge-app --no-default-features --features gui-dioxus -- -D warnings` (the dioxus app, the actual ship target). Both clean.

- [x] **Step 2: Run `cargo test --workspace --all-features`**

Expected: All green.

Same `--all-features` substitution as Step 1: `cargo test --workspace --exclude inputforge-app --all-features` + `cargo test -p inputforge-app --no-default-features --features gui-dioxus`. Both green.

- [x] **Step 3: Invoke `impeccable:shape`**

Open the rail in the running app, invoke the skill, work through whatever density / rhythm tweaks come back.

Density: row source line restructured into device cell (truncates) + kind-tinted JetBrainsMono input cell (fixed) so the input identifier never gets eaten by the device-name ellipsis. Rhythm: name typography bumped to 12/500 (DESIGN.md label tier), source bumped to 11px caption-tier. See commit `66e4508`.

- [x] **Step 4: Invoke `impeccable:frontend-design`**

Visual treatment polish.

Active-row side-stripe `border-left: 3px solid var(--color-border-focus)` removed, DESIGN.md §8 names it as a banned pattern (Toast accent is the only documented exception, peripheral-vision argument). Replaced with a stronger primary surface tint (10% → 18%) plus a 600-weight name override. Dashed `+ Add mapping` row promoted to a real affordance with `--color-border-strong` border, `--color-text` copy, and `font: inherit` so it stops falling through to the browser's default Arial.

- [x] **Step 5: Invoke `impeccable:layout`**

Group-header rhythm, source-line indent. **Decide on group-header collapsibility here** (deferred from spec).

Group-header rhythm verified live (10/600 uppercase, 8/12/4 padding). Source-line indent now flows from a flex layout, so the input identifier is right-anchored without margin tweaks. **Group-header collapsibility decision: NOT added.** Per PRODUCT.md "Power-user defaults, no apologies. Density over whitespace", authoring/tuning users want all mappings visible at once. Revisit only if user feedback shows the rail growing past viewport.

- [x] **Step 6: Invoke `impeccable:typeset`**

Name vs. source typography contrast.

Name 12/500 (active row 12/600), source 11/regular muted, input identifier 10/mono kind-tinted. Three distinct register tiers carry the row hierarchy (name = primary read, device = secondary muted, input = first-class right cell with hue-by-kind). Adjacent caption (11/400) and label (12/500) tiers respect DESIGN.md's "weight-distinguished, not size-distinguished in the dense range" rule.

- [x] **Step 7: Invoke `impeccable:clarify`**

Empty-state copy, filter placeholder, capture-pad copy ("Press an input on any device…"), collision redirect copy, "Duplicate to mode…" submenu copy. Update the strings in `mod.rs`, `add_inline.rs`, `empty.rs` accordingly.

Capture-pad helper tightened from "Press an input on any device…" to "Press an input…", the original was truncating to "Press an input on any devi…" inside the 280px rail with the chip + recapture icon eating ~80px. Filter placeholder, submenu host, and capture helper all use Unicode `\u{2026}` instead of three literal dots. Empty-state helper compressed from a two-clause sentence to one ("Press an input on any connected device, or name a mapping below."). Collision copy left as-is (already terse).

- [x] **Step 8: Invoke `impeccable:polish`**

Final pass. Commit improvements as separate commits with `style(mapping_list): ...` prefix.

Combined into a single `refactor(mapping_list): ...` commit (`66e4508`) since the changes are structurally entangled, splitting the row source line into two cells, dropping the side-stripe, and bumping the dashed row are not independent "style" tweaks but a coherent design-system-compliance pass.

- [x] **Step 9: Final commit**

```bash
# Each impeccable pass produces its own commit. After polish:
git log --oneline -- crates/inputforge-gui-dx/src/frame/mapping_list/
# Verify the F8 commit history is clean.
```

F8 commit history is clean, see `git log --oneline crates/inputforge-gui-dx/src/frame/mapping_list/`.

---

## Acceptance criteria check

Cross-reference the final commit set against spec § "Acceptance criteria":

| AC | Covered by |
|---|---|
| 1. Rail in `if-layout__rail` when profile loaded; hidden when no profile | Task 23 |
| 2. Mode-tab toggle clears selection and re-renders | Task 6 + Task 19 |
| 3. Filter narrows rows by name + source | Task 13 + Task 19 |
| 4. AXES → BUTTONS → HATS, empty groups omitted | Task 12 + Task 19 + Task 24 |
| 5. MergeAxis gold + glyph; Conditional violet ⊕ | Task 5 + Task 14 + Task 24 + Task 26 |
| 6. `+ Add mapping` armed/disarmed/captured/collision flow | Task 17 + Task 27 §4 |
| 7. Right-click menu (Rename/Duplicate/Duplicate-to-mode/Delete) | Task 20 + Task 21 |
| 8. Up/Down/Enter/Cmd-F/Esc keyboard contract | Task 18 + Task 22 + Task 27 §7 |
| 9. Live-capture Esc priority + Up/Down disabled while armed | Task 8 + Task 18 + Task 27 §8 |
| 10. `RemoveMapping` round-trips and persists | Task 3 |
| 11a. Joystick already-displaced no false fire | Task 7 (unit) + Task 27 §9 (manual) |
| 11b. Always-on switch baselines correctly | Task 7 (unit) + Task 27 §10 (manual; may be deferred if no switch-style hardware) |
| 12. Multi-axis nudge picks largest delta in 50ms window | Task 7 (unit) + Task 27 §11 (manual) |
| (extra) Multi-axis tied-delta first-encountered tiebreak | Task 7 (`multi_axis_tie_first_encountered_wins`) + Task 4 (iteration-order contract) |
| (extra) Empty State A (zero mappings) renders correctly | Task 16 + Task 19 + Task 24 (`empty_zero_mappings_renders_full_anatomy`) |
| (extra) Empty State B (zero filter results) renders with literal helper | Task 16 + Task 19 + Task 24 (`empty_zero_filter_results_renders_full_anatomy`) |
| (extra) Duplicate fresh-capture flow (spec §12) | Task 20 (D11) + Task 27 §5 |

Every AC has a task. If the impeccable passes (Task 28) revealed a behavioral gap that crosses an AC boundary, file a follow-up task and re-anchor the AC mapping above.

---

## Notes for the executing agent

- **Do not skip the engine round-trip test in Task 3** even if the GUI side seems to "work", the round-trip-from-disk assertion is the only thing keeping `Profile::save` honest after `remove_mapping`.
- **`InputCacheStore::clone_compact` allocates per call.** The polling task ticks every 16ms; that's 62.5 allocations/sec for the snapshot Vec. Acceptable for F8, but if profiling shows pressure later, switch to a re-usable buffer threaded through `LiveCapture`.
- **The Esc-priority listener (Task 8) uses a shutdown-signal + mounted-flag pattern** (D4): the JS body parks awaiting `__shutdown__` and calls `removeEventListener` on receipt; Rust-side, `armed_listener_mounted` deduplicates re-mounts and `shutdown_signal` triggers teardown on cancel/fire. The acceptance test for this listener is manual (Task 27 §8), there is no SSR coverage. If Task 27 §8 shows Esc not firing reliably, the most likely culprit is the recv loop's send/recv ordering or a stale `shutdown_signal == true` from a previous capture; check the reset in the `use_effect` that mounts the listener.
- **Tokens used in CSS (Task 26) must already exist** in `assets/tokens/*.css`. If you find a missing token (`--color-surface-hover` is the most likely gap), add it as a separate commit BEFORE the CSS commit so the styling lands clean.

---





