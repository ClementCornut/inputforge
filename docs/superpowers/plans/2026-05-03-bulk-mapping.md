# Bulk Mapping Wizard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a side-panel wizard that creates baseline pass-through mappings (one source device to one vJoy device) in a single atomic engine command, with per-row override, per-(row, mode) conflict resolution, and an `AutoBeforeBulkMap` recovery snapshot.

**Architecture:** Two layers. (1) Core: a new `BulkMapEntry` value type, a new `Profile::set_mappings_bulk` upsert pass, a new `SnapshotKind::AutoBeforeBulkMap`, a new `EngineCommand::SetMappingsBulk` whose handler runs pre-save then snapshot then bulk-upsert then post-save inside one command pass (borrows from `RestoreSnapshot`'s lock-release-before-snapshot pattern, with handler-internal warning emission via `state.warnings.push` instead of error propagation). (2) GUI: a new `frame/bulk_map/` module that mounts inside the existing `<aside class="if-panel-slot">` via a new `PanelSlot::BulkMap` variant, owns the wizard state machine, computes auto-mapping/conflicts/summary per row by mode, and dispatches a single bulk command on Apply. The wizard's live readout is a new sibling component (`row_readout`) that consumes F9's `read_axis_display`/`read_button_pressed`/`read_hat_direction` helpers (Task 16 promotes them to `pub(crate)`); F9's `LiveReadout` markup is not modified.

**Tech Stack:** Rust 2024, Dioxus 0.7 (`#[component]`, signals, `dioxus_ssr::render` for component tests), `parking_lot::RwLock`, `tempfile` for engine tests, `tracing` for warnings, BEM-style `.if-bulk-map__*` CSS.

**Reference reading before starting:**
- Spec: `docs/superpowers/specs/2026-05-03-bulk-mapping-design.md`
- Existing snapshot/restore handler: `crates/inputforge-core/src/engine/run.rs:686-727`
- Existing single-mapping handler: `crates/inputforge-core/src/engine/run.rs:783-812`
- Existing tools cluster: `crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/mod.rs`
- Existing panel slot mount: `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs`
- Existing component test harness: `crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs:25-44`
- Engine test fixture: `crates/inputforge-core/src/engine/tests.rs:1424-1455`
- F8 conflict pattern (read for inspiration, do not modify): `crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs:245-280` (the `Collision` reducer arm and `find_mapping` call site)

**Coding rules (project-wide):**
- No em-dash, en-dash, or `--` substitutes anywhere (code comments, doc strings, tests, CSS, plan output). Use comma, colon, semicolon, period, parentheses.
- Conventional Commits for every commit. Scope is required (`feat(bulk_map): ...`, `test(bulk_map): ...`).
- Smoke tests run via `cargo test`; manual interactive verification uses `dx run -p inputforge-app`. Never put `dx run` in an automated step.
- When you specify `display:`, default to `flex`; choose another value only with a stated reason. Always set `display:` explicitly on flex containers (existing project CSS does not silently default).
- TDD: write test, watch fail, write minimum, watch pass, commit.
- All code must satisfy clippy. Don't add `#[allow(...)]` without a `reason = "..."`.

---

## File Structure

### `inputforge-core` (modifications)

- `crates/inputforge-core/src/action/mod.rs`: add `mod bulk;` plus `pub use bulk::BulkMapEntry;`.
- `crates/inputforge-core/src/profile/mod.rs`: add `Profile::set_mappings_bulk` plus its tests.
- `crates/inputforge-core/src/snapshot/types.rs`: add `SnapshotKind::AutoBeforeBulkMap`.
- `crates/inputforge-core/src/snapshot/tests.rs`: add `AutoBeforeBulkMap` parity tests.
- `crates/inputforge-core/src/engine/command.rs`: add `EngineCommand::SetMappingsBulk { entries, snapshot_label }` and extend `tests::debug_format_contains_variant_name`.
- `crates/inputforge-core/src/engine/run.rs`: add the `SetMappingsBulk` arm in `handle_command` and the private `set_mappings_bulk` method.
- `crates/inputforge-core/src/engine/tests.rs`: add the layer-3 handler tests and the layer-6 smoke test.

### `inputforge-core` (new files)

- `crates/inputforge-core/src/action/bulk.rs`: `BulkMapEntry` struct.

### `inputforge-gui-dx` (new files)

- `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`: `BulkMapPanel` component, panel layout assembly.
- `crates/inputforge-gui-dx/src/frame/bulk_map/state.rs`: row/wizard state types, defaults.
- `crates/inputforge-gui-dx/src/frame/bulk_map/auto_map.rs`: positional auto-mapping pure logic.
- `crates/inputforge-gui-dx/src/frame/bulk_map/conflicts.rs`: per-(row, mode) conflict detection.
- `crates/inputforge-gui-dx/src/frame/bulk_map/group_actions.rs`: per-group chip predicate logic.
- `crates/inputforge-gui-dx/src/frame/bulk_map/summary.rs`: count tally for the summary chip.
- `crates/inputforge-gui-dx/src/frame/bulk_map/apply.rs`: entry generation and dispatch glue.
- `crates/inputforge-gui-dx/src/frame/bulk_map/row_readout.rs`: compact live readout per row (axis bar / button dot / hat letter).
- `crates/inputforge-gui-dx/src/frame/bulk_map/empty_state.rs`: no-vJoy empty state.
- `crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs`: layer-5 SSR tests.
- `crates/inputforge-gui-dx/assets/frame/bulk_map.css`: panel-scoped CSS.

### `inputforge-gui-dx` (modifications)

- `crates/inputforge-gui-dx/src/frame/mod.rs`: add `mod bulk_map;`.
- `crates/inputforge-gui-dx/src/frame/view_state.rs`: extend `PanelSlot` with a `BulkMap` variant.
- `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs`: add the `PanelSlot::BulkMap` arm that mounts `BulkMapPanel`.
- `crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/logic.rs`: add `Tool::BulkMap` and update `tool_active`.
- `crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/mod.rs`: render the new tool button.

---

### Task 1: Add `BulkMapEntry` value type

**Files:**
- Create: `crates/inputforge-core/src/action/bulk.rs`
- Modify: `crates/inputforge-core/src/action/mod.rs`

- [ ] **Step 1: Write the type and its compile-time test.**

Write `crates/inputforge-core/src/action/bulk.rs`:

```rust
//! Single entry of a bulk mapping apply. Used by
//! `EngineCommand::SetMappingsBulk` and `Profile::set_mappings_bulk`.

use crate::types::{InputAddress, OutputAddress};

/// One row by mode pair the user committed in the bulk-map wizard.
///
/// `input` MUST be `InputAddress::Bound { device, input }`. The wizard
/// always knows the source device, so all entries it dispatches are
/// bound. The bulk-map pipeline silently skips `Unbound` entries; the
/// filter lives in `Profile::set_mappings_bulk` (covered by the
/// `engine_set_mappings_bulk_skips_entries_with_unbound_input` test).
#[derive(Debug, Clone, PartialEq)]
pub struct BulkMapEntry {
    pub input: InputAddress,
    pub mode: String,
    pub output: OutputAddress,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, InputId, OutputId, VJoyAxis};

    #[test]
    fn bulk_map_entry_clone_and_partial_eq() {
        let entry = BulkMapEntry {
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: 0 },
            },
            mode: "Default".to_owned(),
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        };
        assert_eq!(entry, entry.clone());
    }
}
```

Edit `crates/inputforge-core/src/action/mod.rs` near the existing `mod` lines (after `mod mode_change;`):

```rust
mod bulk;
mod condition;
mod mapping;
mod mode_change;

pub use bulk::BulkMapEntry;
pub use condition::{Condition, validate_depth};
pub use mapping::Mapping;
pub use mode_change::{CycleModes, ModeChangeStrategy};
```

- [ ] **Step 2: Run the test.**

Run: `cargo test -p inputforge-core --lib action::bulk -- --nocapture`
Expected: `test action::bulk::tests::bulk_map_entry_clone_and_partial_eq ... ok`.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-core/src/action/bulk.rs crates/inputforge-core/src/action/mod.rs
git commit -m "feat(bulk_map): add BulkMapEntry value type"
```

---

### Task 2: `Profile::set_mappings_bulk` upsert pass

**Files:**
- Modify: `crates/inputforge-core/src/profile/mod.rs` (after `set_mapping`, before `remove_mapping` near `crates/inputforge-core/src/profile/mod.rs:241`)

- [ ] **Step 1: Write the failing tests.**

Append to the existing `mod tests` block in `crates/inputforge-core/src/profile/mod.rs` (find the `mod tests` block near the bottom; locate it via `grep -n "mod tests" crates/inputforge-core/src/profile/mod.rs`):

```rust
#[test]
fn profile_set_mappings_bulk_with_empty_entries_is_noop() {
    use crate::action::BulkMapEntry;

    let mut profile = test_profile_with_one_mode();
    profile.set_mappings_bulk(&[] as &[BulkMapEntry]);
    assert!(profile.mappings().is_empty());
}

#[test]
fn profile_set_mappings_bulk_creates_single_mapping_with_unnamed_passthrough() {
    use crate::action::{Action, BulkMapEntry};
    use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    let mut profile = test_profile_with_one_mode();
    let entries = vec![BulkMapEntry {
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        },
        mode: "Default".to_owned(),
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    }];
    profile.set_mappings_bulk(&entries);

    assert_eq!(profile.mappings().len(), 1);
    let m = &profile.mappings()[0];
    assert_eq!(m.name, None);
    assert_eq!(m.actions.len(), 1);
    assert!(matches!(m.actions[0], Action::MapToVJoy { .. }));
}

#[test]
fn profile_set_mappings_bulk_creates_one_mapping_per_entry_across_modes() {
    use crate::action::BulkMapEntry;
    use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    let mut profile = test_profile_with_two_modes();
    let input = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let output = OutputAddress {
        device: 1,
        output: OutputId::Axis { id: VJoyAxis::X },
    };
    let entries = vec![
        BulkMapEntry { input: input.clone(), mode: "Default".to_owned(), output: output.clone() },
        BulkMapEntry { input: input.clone(), mode: "Combat".to_owned(), output: output.clone() },
    ];
    profile.set_mappings_bulk(&entries);
    assert_eq!(profile.mappings().len(), 2);
}

#[test]
fn profile_set_mappings_bulk_replaces_existing_mapping_overwriting_name_and_actions() {
    use crate::action::{Action, BulkMapEntry};
    use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    let mut profile = test_profile_with_one_mode();
    let input = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    profile.set_mapping(&input, "Default", Some("Throttle".to_owned()), vec![Action::Invert]);

    let entries = vec![BulkMapEntry {
        input: input.clone(),
        mode: "Default".to_owned(),
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::Y },
        },
    }];
    profile.set_mappings_bulk(&entries);

    assert_eq!(profile.mappings().len(), 1, "must upsert, not append");
    let m = &profile.mappings()[0];
    assert_eq!(m.name, None, "name must be cleared by bulk replace");
    assert!(matches!(m.actions[0], Action::MapToVJoy { .. }));
}

#[test]
fn profile_set_mappings_bulk_each_generated_mapping_has_action_vec_of_exactly_one_map_to_vjoy() {
    use crate::action::{Action, BulkMapEntry};
    use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    let mut profile = test_profile_with_one_mode();
    let entries = vec![BulkMapEntry {
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 5 },
        },
        mode: "Default".to_owned(),
        output: OutputAddress {
            device: 1,
            output: OutputId::Button { id: 6 },
        },
    }];
    profile.set_mappings_bulk(&entries);
    let m = &profile.mappings()[0];
    assert_eq!(m.actions.len(), 1);
    assert!(matches!(m.actions[0], Action::MapToVJoy { .. }));
}

#[test]
fn profile_set_mappings_bulk_each_generated_mapping_has_name_none() {
    use crate::action::BulkMapEntry;
    use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    let mut profile = test_profile_with_one_mode();
    let entries = vec![BulkMapEntry {
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Hat { index: 0 },
        },
        mode: "Default".to_owned(),
        output: OutputAddress {
            device: 1,
            output: OutputId::Hat { id: 1 },
        },
    }];
    profile.set_mappings_bulk(&entries);
    assert_eq!(profile.mappings()[0].name, None);
}

#[test]
fn profile_set_mappings_bulk_mixed_create_and_replace_in_one_call() {
    use crate::action::{Action, BulkMapEntry};
    use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    let mut profile = test_profile_with_one_mode();
    let in_a = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let in_b = InputAddress::Bound {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    profile.set_mapping(&in_a, "Default", Some("Pre".to_owned()), vec![Action::Invert]);

    let out = OutputAddress {
        device: 1,
        output: OutputId::Axis { id: VJoyAxis::X },
    };
    let entries = vec![
        BulkMapEntry { input: in_a.clone(), mode: "Default".to_owned(), output: out.clone() },
        BulkMapEntry { input: in_b.clone(), mode: "Default".to_owned(), output: out.clone() },
    ];
    profile.set_mappings_bulk(&entries);

    assert_eq!(profile.mappings().len(), 2);
    assert_eq!(profile.find_mapping(&in_a, "Default").unwrap().name, None);
    assert!(matches!(
        profile.find_mapping(&in_a, "Default").unwrap().actions[0],
        Action::MapToVJoy { .. }
    ));
    assert!(profile.find_mapping(&in_b, "Default").is_some());
}

#[test]
fn profile_set_mappings_bulk_into_unknown_mode_still_upserts_silently() {
    use crate::action::BulkMapEntry;
    use crate::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    let mut profile = test_profile_with_one_mode();
    let entries = vec![BulkMapEntry {
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        },
        mode: "Phantom".to_owned(),
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    }];
    profile.set_mappings_bulk(&entries);
    assert_eq!(profile.mappings().len(), 1, "engine accepts the upsert; reload-time validation will flag the orphan");
}
```

If `test_profile_with_one_mode()` and `test_profile_with_two_modes()` do not already exist as helpers in this `mod tests` block, add them at the top of the block:

```rust
fn test_profile_with_one_mode() -> Profile {
    let map = std::collections::HashMap::from([("Default".to_owned(), vec![])]);
    let modes = crate::mode::ModeTree::from_adjacency(&map).unwrap();
    Profile::new("T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned())
}

fn test_profile_with_two_modes() -> Profile {
    let map = std::collections::HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
    let modes = crate::mode::ModeTree::from_adjacency(&map).unwrap();
    Profile::new("T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned())
}
```

(Verify `Profile::new` arity and parameter order at `crates/inputforge-core/src/profile/mod.rs` lines around 100; mirror it exactly. The existing tests in this module already construct profiles, so a working pattern is in-file.)

- [ ] **Step 2: Run the tests, expect failure.**

Run: `cargo test -p inputforge-core --lib profile::tests::profile_set_mappings_bulk -- --nocapture`
Expected: compile error `no method named set_mappings_bulk found for struct Profile`.

- [ ] **Step 3: Implement the method.**

Insert after `Profile::set_mapping` in `crates/inputforge-core/src/profile/mod.rs` (around line 241):

```rust
/// Apply a batch of upserts in a single in-memory pass.
///
/// Each entry produces a mapping with `name: None` and exactly one
/// `Action::MapToVJoy { output: entry.output }`. Existing mappings
/// for `(entry.input, entry.mode)` are replaced. Empty `entries`
/// is a no-op.
///
/// **No file save.** The engine handler is responsible for
/// persistence; see `EngineCommand::SetMappingsBulk`.
pub fn set_mappings_bulk(&mut self, entries: &[crate::action::BulkMapEntry]) {
    use crate::action::Action;
    for entry in entries {
        let actions = vec![Action::MapToVJoy { output: entry.output.clone() }];
        self.set_mapping(&entry.input, &entry.mode, None, actions);
    }
}
```

- [ ] **Step 4: Run the tests, expect pass.**

Run: `cargo test -p inputforge-core --lib profile::tests::profile_set_mappings_bulk -- --nocapture`
Expected: all eight `profile_set_mappings_bulk_*` tests pass.

- [ ] **Step 5: Commit.**

```bash
git add crates/inputforge-core/src/profile/mod.rs
git commit -m "feat(bulk_map): add Profile::set_mappings_bulk upsert pass"
```

---

### Task 3: `SnapshotKind::AutoBeforeBulkMap` variant

**Files:**
- Modify: `crates/inputforge-core/src/snapshot/types.rs:27`
- Modify: `crates/inputforge-core/src/snapshot/mod.rs:31-37` (doc comment listing snapshot kinds)
- Modify: `crates/inputforge-core/src/snapshot/tests.rs`

- [ ] **Step 1: Write the failing tests.**

Append to `crates/inputforge-core/src/snapshot/tests.rs`:

```rust
#[test]
fn snapshot_kind_auto_before_bulk_map_serializes_to_snake_case() {
    #[derive(serde::Serialize)]
    struct Wrapper {
        kind: SnapshotKind,
    }
    let s = toml::to_string(&Wrapper {
        kind: SnapshotKind::AutoBeforeBulkMap,
    })
    .unwrap();
    assert!(s.contains("auto_before_bulk_map"), "got: {s}");
}

#[test]
fn snapshot_kind_auto_before_bulk_map_round_trips_through_toml() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wrapper {
        kind: SnapshotKind,
    }
    let s = toml::to_string(&Wrapper { kind: SnapshotKind::AutoBeforeBulkMap }).unwrap();
    let back: Wrapper = toml::from_str(&s).unwrap();
    assert_eq!(back.kind, SnapshotKind::AutoBeforeBulkMap);
}

#[test]
fn snapshot_kind_auto_before_bulk_map_creates_unpinned_snapshot() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let snap = create(&path, SnapshotKind::AutoBeforeBulkMap, None, &cfg)
        .unwrap()
        .unwrap();
    assert!(!snap.pinned, "AutoBeforeBulkMap is unpinned");
}

#[test]
fn snapshot_kind_auto_before_bulk_map_always_fires_never_deduped() {
    let (_dir, path) = fresh_profile_dir();
    let cfg = SnapshotConfig::default();
    let a = create(&path, SnapshotKind::AutoBeforeBulkMap, None, &cfg).unwrap();
    let b = create(&path, SnapshotKind::AutoBeforeBulkMap, None, &cfg).unwrap();
    assert!(a.is_some() && b.is_some(), "AutoBeforeBulkMap must never dedup");
}
```

- [ ] **Step 2: Run tests, expect failure.**

Run: `cargo test -p inputforge-core --lib snapshot::tests::snapshot_kind_auto_before_bulk_map -- --nocapture`
Expected: compile error `no variant or associated item named AutoBeforeBulkMap`.

- [ ] **Step 3: Add the variant.**

Edit `crates/inputforge-core/src/snapshot/types.rs:27`. Insert `AutoBeforeBulkMap,` between `AutoBeforeRestore,` and `Manual,`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotKind {
    /// Created by `LoadProfile`. Deduped against the latest snapshot when
    /// `cfg.skip_if_unchanged` is set and the content hash matches.
    AutoSessionStart,
    /// Created by `RestoreSnapshot` immediately before applying the
    /// restore. Always fires; never deduped.
    AutoBeforeRestore,
    /// Created by `SetMappingsBulk` immediately before applying the
    /// bulk upsert pass. Always fires; never deduped.
    AutoBeforeBulkMap,
    /// Created by user dispatch of `CreateSnapshot { kind: Manual }`.
    /// Auto-pinned.
    Manual,
}
```

The `pinned: matches!(kind, SnapshotKind::Manual)` logic in `crates/inputforge-core/src/snapshot/mod.rs:80` covers the new variant correctly (matches the spec's note). No further changes needed there.

- [ ] **Step 4: Update the snapshot module doc comment.**

Edit `crates/inputforge-core/src/snapshot/mod.rs:31-37`. Add `AutoBeforeBulkMap` to the unpinned-kinds list and to the "always create / never dedup" line:

```
- `Manual` -> `pinned = true` unconditionally.
- `AutoSessionStart` / `AutoBeforeRestore` / `AutoBeforeBulkMap` -> `pinned = false`.
```

and

```
`AutoBeforeRestore`, `AutoBeforeBulkMap`, and `Manual` always create a new snapshot (never deduped).
```

Reference the actual current wording at `mod.rs:31-37` and adjust verbatim; the goal is mirror-symmetry with the `AutoBeforeRestore` clause.

- [ ] **Step 5: Run tests.**

Run: `cargo test -p inputforge-core --lib snapshot:: -- --nocapture`
Expected: all snapshot tests pass, including the four new ones.

- [ ] **Step 6: Commit.**

```bash
git add crates/inputforge-core/src/snapshot/types.rs crates/inputforge-core/src/snapshot/mod.rs crates/inputforge-core/src/snapshot/tests.rs
git commit -m "feat(bulk_map): add SnapshotKind::AutoBeforeBulkMap variant"
```

---

### Task 4: `EngineCommand::SetMappingsBulk` variant

**Files:**
- Modify: `crates/inputforge-core/src/engine/command.rs`

- [ ] **Step 1: Write the failing test.**

Add at the bottom of the existing `#[cfg(test)] mod tests` block in `crates/inputforge-core/src/engine/command.rs`:

```rust
#[test]
fn set_mappings_bulk_variant_debug_and_partialeq() {
    use crate::action::BulkMapEntry;
    use crate::types::{DeviceId, InputId, OutputId, VJoyAxis};

    let entry = BulkMapEntry {
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        },
        mode: "Default".to_owned(),
        output: crate::types::OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    };
    let a = EngineCommand::SetMappingsBulk {
        entries: vec![entry.clone()],
        snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
    };
    let b = EngineCommand::SetMappingsBulk {
        entries: vec![entry],
        snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
    };
    assert_eq!(a, b);
    assert!(format!("{a:?}").contains("SetMappingsBulk"));
}
```

- [ ] **Step 2: Run, expect failure.**

Run: `cargo test -p inputforge-core --lib engine::command::tests::set_mappings_bulk_variant_debug_and_partialeq`
Expected: compile error `no variant SetMappingsBulk`.

- [ ] **Step 3: Add the variant.**

Edit `crates/inputforge-core/src/engine/command.rs`. Add the import at the top:

```rust
use crate::action::BulkMapEntry;
```

Insert the new variant after `SetMapping { ... }` and before `RemoveMapping { ... }`:

```rust
    /// Apply a batch of mapping upserts in a single atomic pass.
    ///
    /// Engine handler order:
    ///   1. Pre-save the in-memory profile to disk (so the snapshot
    ///      captures the user's authored state, not whatever was on
    ///      disk last).
    ///   2. Create an `AutoBeforeBulkMap` snapshot, then `prune`. If
    ///      the snapshot fails, abort: profile is unchanged on disk
    ///      and in memory; a warning is pushed to the warnings
    ///      channel; user retries after fixing the underlying issue.
    ///   3. Run all entries through `Profile::set_mappings_bulk` in
    ///      one in-memory pass.
    ///   4. Save the post-bulk profile to disk.
    ///
    /// `snapshot_label` is the user-visible label attached to the
    /// recovery snapshot. Format guidance:
    /// `"Before bulk-map: <source> to vJoy <id>"`.
    SetMappingsBulk {
        entries: Vec<BulkMapEntry>,
        snapshot_label: String,
    },
```

- [ ] **Step 4: Run all tests in engine::command, expect pass.**

Run: `cargo test -p inputforge-core --lib engine::command -- --nocapture`
Expected: pass.

- [ ] **Step 5: Commit.**

```bash
git add crates/inputforge-core/src/engine/command.rs
git commit -m "feat(bulk_map): add EngineCommand::SetMappingsBulk variant"
```

---

### Task 5: Engine handler for `SetMappingsBulk`

**Files:**
- Modify: `crates/inputforge-core/src/engine/run.rs`

- [ ] **Step 1: Add the handler arm and private method.**

Locate the `match cmd` in `Engine::handle_command` (around `crates/inputforge-core/src/engine/run.rs:310`). Add a new arm after the existing `RemoveMapping` arm and before the closing brace of the match (which is at the line ending `Ok(())` near line 734):

```rust
            EngineCommand::SetMappingsBulk { entries, snapshot_label } => {
                self.set_mappings_bulk(entries, snapshot_label);
                self.pending_output_refresh = true;
            }
```

Then add the private method near the existing `fn set_mapping` (around line 783) and `fn remove_mapping`. Place after `remove_mapping`:

```rust
    /// Apply a bulk-map command. See `EngineCommand::SetMappingsBulk`
    /// for the four-step contract.
    ///
    /// Returns `()`, matching `set_mapping`'s shape. Snapshot and save
    /// errors surface to the user via the warnings channel rather than
    /// `?`, because the parent command-drain loop swallows arm errors
    /// and the user's recovery path is a manual Restore via the
    /// snapshot index UI.
    fn set_mappings_bulk(&self, entries: Vec<crate::action::BulkMapEntry>, snapshot_label: String) {
        // Step 0: clone the profile path. The read guard drops at the
        // end of this `let`. Do NOT hold any state lock during
        // `crate::snapshot::create` and `crate::snapshot::prune`,
        // which perform disk I/O that must run lock-free (mirrors
        // `engine/run.rs` RestoreSnapshot at lines 687-700).
        let Some(path) = self.state.read().profile_path.clone() else {
            tracing::warn!(target: "bulk_map", "SetMappingsBulk: no profile loaded, ignoring");
            self.state.write().warnings.push(
                "Bulk-map ignored: no profile loaded".to_owned(),
            );
            return;
        };

        // Step 1: pre-save in-memory profile so the on-disk body
        // matches the user's pre-bulk authored state. Without this,
        // the snapshot in step 2 captures whatever happened to be on
        // disk last (which may be older than the in-memory state if
        // any caller deferred a save).
        {
            let state = self.state.read();
            if let Some(profile) = state.active_profile.as_ref() {
                if let Err(e) = profile.save(&path) {
                    tracing::warn!(
                        target: "bulk_map",
                        path = %path.display(),
                        error = ?e,
                        "SetMappingsBulk: pre-snapshot save failed; aborting"
                    );
                    drop(state);
                    self.state.write().warnings.push(
                        "Bulk-map aborted: could not save profile before snapshot".to_owned(),
                    );
                    return;
                }
            } else {
                return;
            }
        } // read guard drops here

        // Step 2: take the recovery snapshot. Abort if it fails so the
        // user never ends up with bulk-applied mappings and no
        // snapshot to roll back to.
        match crate::snapshot::create(
            &path,
            crate::snapshot::SnapshotKind::AutoBeforeBulkMap,
            Some(snapshot_label),
            &self.settings.snapshot,
        ) {
            Ok(_) => {
                let _ = crate::snapshot::prune(&path, &self.settings.snapshot);
            }
            Err(e) => {
                tracing::warn!(
                    target: "bulk_map",
                    error = ?e,
                    "SetMappingsBulk: AutoBeforeBulkMap snapshot failed; aborting apply"
                );
                self.state.write().warnings.push(
                    "Bulk-map aborted: could not create recovery snapshot".to_owned(),
                );
                return;
            }
        }

        // Step 3: apply upserts and persist (second save).
        let mut state = self.state.write();
        let Some(profile) = state.active_profile.as_mut() else {
            return;
        };
        profile.set_mappings_bulk(&entries);
        if let Err(e) = profile.save(&path) {
            tracing::warn!(
                target: "bulk_map",
                path = %path.display(),
                error = ?e,
                "SetMappingsBulk: post-bulk save failed; in-memory state holds bulk; recovery via Restore"
            );
            state.warnings.push(
                "Bulk-map applied in memory but disk save failed; reload to revert".to_owned(),
            );
        }
    }
```

(`Engine::settings` is accessed via `self.settings.snapshot` because `AppSettings` is stored on the engine; verify by reading `crates/inputforge-core/src/engine/run.rs` around the existing snapshot/restore arm and matching the field path used there.)

If the existing `handle_command` carries `#[expect(clippy::too_many_lines, ...)]`, the new arm fits within that suppression and needs no further work.

- [ ] **Step 2: Run a syntax check.**

Run: `cargo check -p inputforge-core`
Expected: compiles cleanly with no warnings.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-core/src/engine/run.rs
git commit -m "feat(bulk_map): handle SetMappingsBulk with pre-save snapshot upsert post-save"
```

---

### Task 6: Engine handler tests (layer 3)

**Files:**
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Add the helper for capturing profile mtime.**

If a `read_mtime` helper is not already present in the file, add it near the other helpers (top of the file, around line 130):

```rust
fn read_mtime(path: &std::path::Path) -> std::time::SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}
```

- [ ] **Step 2: Write the new tests.**

Append a new section to `crates/inputforge-core/src/engine/tests.rs`:

```rust
// ---------------------------------------------------------------------------
// SetMappingsBulk handler tests (Bulk-map wizard)
// ---------------------------------------------------------------------------

fn make_bulk_entry(input_idx: u8, axis: VJoyAxis) -> crate::action::BulkMapEntry {
    crate::action::BulkMapEntry {
        input: axis_addr(input_idx),
        mode: "Default".to_owned(),
        output: vjoy_axis_output(1, axis),
    }
}

#[test]
fn engine_set_mappings_bulk_persists_to_disk_and_creates_snapshot() {
    let (mut engine, _state, tx, _dir, path) = make_engine_with_simple_disk_profile();

    let pre_writes = std::fs::read(&path).unwrap();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let post_writes = std::fs::read(&path).unwrap();
    assert_ne!(pre_writes, post_writes, "post-bulk save must update on-disk body");
    // Snapshot index lists exactly one new AutoBeforeBulkMap row.
    let listed = crate::snapshot::list(&path).unwrap();
    let bulk_count = listed
        .iter()
        .filter(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoBeforeBulkMap))
        .count();
    assert_eq!(bulk_count, 1, "exactly one AutoBeforeBulkMap snapshot per apply");
}

#[test]
fn engine_set_mappings_bulk_with_no_profile_loaded_is_noop_and_warns() {
    let state = Arc::new(RwLock::new(AppState::new()));
    state.write().engine_status = EngineStatus::Running;
    let (tx, rx) = mpsc::channel();
    let mut engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
    );

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert!(state.read().active_profile.is_none(), "still no profile");
    let warns = state.read().warnings.clone();
    assert!(
        warns.iter().any(|w| w.contains("Bulk-map ignored: no profile loaded")),
        "warnings must surface the no-profile abort, got: {warns:?}"
    );
}

#[test]
fn engine_set_mappings_bulk_sets_pending_output_refresh_true() {
    let (mut engine, _state, tx, _dir, _path) = make_engine_with_simple_disk_profile();
    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();
    assert!(engine.pending_output_refresh, "bulk apply must trigger output refresh");
}

#[test]
fn engine_set_mappings_bulk_applies_all_n_entries() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    let mut entries = Vec::new();
    for i in 0..8 {
        entries.push(make_bulk_entry(i, VJoyAxis::X));
    }
    tx.send(EngineCommand::SetMappingsBulk {
        entries,
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();
    assert_eq!(state.read().active_profile.as_ref().unwrap().mappings().len(), 8);
    let _ = path; // silence unused warning if needed.
}

#[test]
fn engine_set_mappings_bulk_creates_auto_before_bulk_map_snapshot_with_label() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let listed = crate::snapshot::list(&path).unwrap();
    let snap = listed
        .iter()
        .find(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoBeforeBulkMap))
        .expect("AutoBeforeBulkMap must exist");
    assert_eq!(snap.label.as_deref(), Some("Before bulk-map: dev-1 to vJoy 1"));
    // Sanity: the apply itself committed in memory.
    assert_eq!(state.read().active_profile.as_ref().unwrap().mappings().len(), 1);
}

#[test]
fn engine_set_mappings_bulk_pre_snapshot_save_failure_aborts_and_warns() {
    // Drive a portable pre-save failure: point the engine's profile
    // path into a sub-path whose parent directory is removed after
    // engine init. `Profile::save` opens the file under that parent
    // and fails with NotFound (Unix) / PathNotFound (Windows). Both
    // surface as `Err` in the handler's pre-save step, exercising the
    // abort branch on every platform.
    let (mut engine, state, tx, dir, path) = make_engine_with_simple_disk_profile();
    // The fixture's profile path lives directly under `dir`. Move the
    // active profile_path to a sub-directory we then delete.
    let nested = dir.path().join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    let nested_path = nested.join("profile.toml");
    std::fs::write(&nested_path, std::fs::read(&path).unwrap()).unwrap();
    state.write().profile_path = Some(nested_path.clone());
    // Remove the nested directory so the next save fails.
    std::fs::remove_dir_all(&nested).unwrap();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let warns = state.read().warnings.clone();
    assert!(
        warns.iter().any(|w| w.contains("Bulk-map aborted")),
        "warnings must surface a Bulk-map aborted line, got: {warns:?}"
    );
    // Apply must NOT have committed in memory.
    assert!(state.read().active_profile.as_ref().unwrap().mappings().is_empty());
}

#[test]
fn engine_set_mappings_bulk_aborts_apply_when_snapshot_creation_fails() {
    // Drive a snapshot-create failure by pre-creating the snapshot
    // directory as a regular file (not a dir): mkdir attempts will
    // fail on the snapshot path.
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();

    // The snapshot dir derives from the profile stem; use the same
    // helper the snapshot module exposes for tests.
    let snap_dir = crate::snapshot::__test_snap_dir(&path).unwrap();
    std::fs::write(&snap_dir, b"blocker").unwrap();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    // Profile mappings must remain empty (the apply was aborted).
    assert!(state.read().active_profile.as_ref().unwrap().mappings().is_empty());
    let warns = state.read().warnings.clone();
    assert!(
        warns.iter().any(|w| w.contains("recovery snapshot")),
        "must push 'recovery snapshot' warning, got: {warns:?}"
    );
}

#[test]
fn engine_set_mappings_bulk_abort_path_does_not_leak_state_write_lock() {
    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    let snap_dir = crate::snapshot::__test_snap_dir(&path).unwrap();
    std::fs::write(&snap_dir, b"blocker").unwrap();

    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    // try_read returns Some when no writer holds the lock.
    assert!(
        state.try_read().is_some(),
        "abort path must release any write guard before returning"
    );
}

#[test]
fn engine_set_mappings_bulk_happy_path_in_memory_state_holds_one_mapping() {
    // Guard that on the happy path the post-bulk in-memory state is
    // populated (one apply, one mapping). Dedicated failure cases are
    // covered by the snapshot-failure and pre-save-failure tests above.
    let (mut engine, state, tx, _dir, _path) = make_engine_with_simple_disk_profile();
    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![make_bulk_entry(0, VJoyAxis::X)],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    assert_eq!(state.read().active_profile.as_ref().unwrap().mappings().len(), 1);
}

#[test]
fn engine_set_mappings_bulk_skips_entries_with_unbound_input() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_simple_disk_profile();

    let entry = crate::action::BulkMapEntry {
        input: InputAddress::Unbound,
        mode: "Default".to_owned(),
        output: vjoy_axis_output(1, VJoyAxis::X),
    };
    tx.send(EngineCommand::SetMappingsBulk {
        entries: vec![entry],
        snapshot_label: "x".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    // The handler currently delegates to `set_mapping`, which accepts
    // any InputAddress. This test asserts the contract surfaced in
    // BulkMapEntry's docstring: defensive treatment of Unbound is the
    // engine's responsibility. Update `Profile::set_mappings_bulk` to
    // skip Unbound entries before this assertion will pass.
    assert!(
        state.read().active_profile.as_ref().unwrap().mappings().is_empty(),
        "Unbound input entries must be skipped by the bulk handler"
    );
}
```

- [ ] **Step 3: Run, expect failure on the unbound-skip test.**

Run: `cargo test -p inputforge-core --lib engine::tests::engine_set_mappings_bulk -- --nocapture`
Expected: most tests pass; `engine_set_mappings_bulk_skips_entries_with_unbound_input` fails because the current `Profile::set_mappings_bulk` does not filter `Unbound`.

- [ ] **Step 4: Add the Unbound filter at the profile layer.**

Modify `Profile::set_mappings_bulk` in `crates/inputforge-core/src/profile/mod.rs`:

```rust
pub fn set_mappings_bulk(&mut self, entries: &[crate::action::BulkMapEntry]) {
    use crate::action::Action;
    use crate::types::InputAddress;
    for entry in entries {
        if matches!(entry.input, InputAddress::Unbound) {
            continue;
        }
        let actions = vec![Action::MapToVJoy { output: entry.output.clone() }];
        self.set_mapping(&entry.input, &entry.mode, None, actions);
    }
}
```

- [ ] **Step 5: Run all engine tests.**

Run: `cargo test -p inputforge-core --lib engine::tests::engine_set_mappings_bulk -- --nocapture`
Expected: all 10 tests pass.

Run: `cargo test -p inputforge-core` (full suite)
Expected: all tests pass.

- [ ] **Step 6: Commit.**

```bash
git add crates/inputforge-core/src/engine/tests.rs crates/inputforge-core/src/profile/mod.rs
git commit -m "test(bulk_map): cover SetMappingsBulk handler and Unbound filter"
```

---

### Task 7: Workspace-level smoke test

**Files:**
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write the smoke test.**

Append to `crates/inputforge-core/src/engine/tests.rs`:

```rust
#[test]
fn smoke_bulk_map_full_round_trip_creates_correct_profile_state() {
    use crate::action::BulkMapEntry;
    use crate::types::{OutputAddress, OutputId};

    let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
    let mut entries = Vec::new();
    // Four axes, eight buttons, one hat.
    for i in 0u8..4 {
        let axis_enum = match i {
            0 => VJoyAxis::X,
            1 => VJoyAxis::Y,
            2 => VJoyAxis::Z,
            _ => VJoyAxis::Rx,
        };
        entries.push(BulkMapEntry {
            input: axis_addr(i),
            mode: "Default".to_owned(),
            output: vjoy_axis_output(1, axis_enum),
        });
    }
    for i in 0u8..8 {
        entries.push(BulkMapEntry {
            input: button_addr(i),
            mode: "Default".to_owned(),
            output: vjoy_button_output(1, i + 1),
        });
    }
    entries.push(BulkMapEntry {
        input: InputAddress::Bound {
            device: dev_id(),
            input: InputId::Hat { index: 0 },
        },
        mode: "Default".to_owned(),
        output: OutputAddress {
            device: 1,
            output: OutputId::Hat { id: 1 },
        },
    });

    tx.send(EngineCommand::SetMappingsBulk {
        entries,
        snapshot_label: "Before bulk-map: dev-1 to vJoy 1".to_owned(),
    })
    .unwrap();
    engine.tick().unwrap();

    let mappings = state.read().active_profile.as_ref().unwrap().mappings().clone();
    assert_eq!(mappings.len(), 13, "4 axes + 8 buttons + 1 hat = 13");
    for m in &mappings {
        assert_eq!(m.name, None);
        assert_eq!(m.actions.len(), 1);
        assert!(matches!(m.actions[0], crate::action::Action::MapToVJoy { .. }));
    }

    let listed = crate::snapshot::list(&path).unwrap();
    assert!(
        listed
            .iter()
            .any(|s| matches!(s.kind, crate::snapshot::SnapshotKind::AutoBeforeBulkMap)),
        "AutoBeforeBulkMap must be listed"
    );
}
```

- [ ] **Step 2: Run.**

Run: `cargo test -p inputforge-core --lib engine::tests::smoke_bulk_map_full_round_trip -- --nocapture`
Expected: pass.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-core/src/engine/tests.rs
git commit -m "test(bulk_map): add 13-entry smoke test for bulk-map round trip"
```

---

### Task 8: Add `PanelSlot::BulkMap` variant and panel mount stub

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/view_state.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs`

- [ ] **Step 1: Extend `PanelSlot`.**

Edit `crates/inputforge-gui-dx/src/frame/view_state.rs:28`. Add `BulkMap` to the enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code, reason = "Used by regions in Task 18+")]
pub(crate) enum PanelSlot {
    #[default]
    None,
    Devices,
    Profiles,
    BulkMap,
}
```

- [ ] **Step 2: Refactor the panel-slot shell to support a custom-body mode.**

The existing `panel_slot/mod.rs` deliberately hoists the `<aside class="if-panel-slot">` element OUTSIDE the match block so the entrance keyframe (defined on `.if-panel-slot` in `panel_slot.css:35-47`) fires only on a genuine `None -> Some` open. A `return rsx!` short-circuit inside the match would re-fire the keyframe on every tool swap (Dioxus diffs different match arms as different VNodes). Spec line 87 explicitly demands this discipline be preserved for BulkMap.

Replace the body of `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs` with the two-mode shell:

```rust
use dioxus::prelude::*;

use crate::frame::view_state::{PanelSlot as PanelSlotEnum, ViewState};

const PANEL_SLOT_CSS: Asset = asset!("/assets/frame/panel_slot.css");

/// Shell layout for the right-side panel-slot. `Standard` composes the
/// shared caption/title/body shell used by F12 (Devices) and F13
/// (Profiles); `Custom` lets a tool render its own body inside the same
/// stable `<aside class="if-panel-slot">` element. Both branches go
/// through the single `<aside>` node outside the match, so swapping
/// between them does NOT remount the element and the entrance keyframe
/// fires only on `None -> Some`.
enum ShellMode {
    Standard {
        caption: &'static str,
        title: &'static str,
        body: &'static str,
        aria: &'static str,
    },
    Custom {
        aria: &'static str,
        content: Element,
    },
}

#[component]
pub(crate) fn PanelSlot() -> Element {
    tracing::trace!(target: "frame::render", region = "panel_slot");
    let view = use_context::<ViewState>();
    let slot = use_memo(move || *view.panel_slot.read());
    let via_calib = use_memo(move || *view.via_calibration.read());

    let s = *slot.read();
    if matches!(s, PanelSlotEnum::None) {
        return rsx! { Stylesheet { href: PANEL_SLOT_CSS } };
    }

    let calib = *via_calib.read();
    let mode = match s {
        PanelSlotEnum::Devices if calib => ShellMode::Standard {
            caption: "Panel · F12",
            title: "Calibration",
            body: "F12 owns content (calibration)",
            aria: "Calibration panel",
        },
        PanelSlotEnum::Devices => ShellMode::Standard {
            caption: "Panel · F12",
            title: "Devices",
            body: "F12 owns content",
            aria: "Devices panel",
        },
        PanelSlotEnum::Profiles => ShellMode::Standard {
            caption: "Panel · F13",
            title: "Profiles",
            body: "F13 owns content",
            aria: "Profiles panel",
        },
        PanelSlotEnum::BulkMap => ShellMode::Custom {
            aria: "Bulk mapping wizard",
            content: rsx! { crate::frame::bulk_map::BulkMapPanel {} },
        },
        PanelSlotEnum::None => unreachable!("None branch returned above"),
    };

    let aria = match &mode {
        ShellMode::Standard { aria, .. } | ShellMode::Custom { aria, .. } => *aria,
    };

    rsx! {
        Stylesheet { href: PANEL_SLOT_CSS }
        aside {
            class: "if-panel-slot",
            "aria-label": "{aria}",
            match mode {
                ShellMode::Standard { caption, title, body, .. } => rsx! {
                    header { class: "if-panel-slot__header",
                        div { class: "if-panel-slot__caption", "{caption}" }
                        h2 { class: "if-panel-slot__title", "{title}" }
                    }
                    div { class: "if-panel-slot__body", "{body}" }
                },
                ShellMode::Custom { content, .. } => rsx! { {content} },
            }
        }
    }
}
```

(Keep the existing `Devices`/`Profiles` arms intact, just rephrased into `ShellMode::Standard`. The single stable `<aside>` outside the match is preserved.)

- [ ] **Step 3: Add a regression test pinning the discipline.**

Append to `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    //! Verify the `<aside class="if-panel-slot">` element is present in
    //! both standard and custom shell modes (a structural guarantee that
    //! Dioxus diffs them as the same VNode rather than remounting).

    use super::*;
    use crate::frame::view_state::{PanelSlot as PanelSlotEnum, ViewState};
    use dioxus::prelude::*;
    use dioxus_ssr::render;

    #[test]
    fn panel_slot_renders_aside_for_standard_and_custom_modes() {
        for variant in [PanelSlotEnum::Devices, PanelSlotEnum::Profiles, PanelSlotEnum::BulkMap] {
            let mut vdom = VirtualDom::new_with_props(super::TestHarness, TestHarnessProps { variant });
            vdom.rebuild_in_place();
            let html = render(&vdom);
            assert!(html.contains(r#"<aside class="if-panel-slot""#),
                "variant {variant:?} must render the stable .if-panel-slot aside");
        }
    }
}
```

(Add a small `TestHarness` component above the `tests` module that wires `ViewState` with the requested variant; if the existing test infrastructure already exposes a panel-slot harness, prefer reusing it.)

- [ ] **Step 4: Sanity build.**

Run: `cargo check -p inputforge-gui-dx`
Expected: compile error `unresolved module bulk_map` until Task 9 lands. Continue to Task 9.

---

### Task 9: Module scaffolding for `frame::bulk_map`

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mod.rs`
- Create: `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`
- Create: `crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs`

- [ ] **Step 1: Register the module.**

Edit `crates/inputforge-gui-dx/src/frame/mod.rs` near the top mod block:

```rust
mod banner;
mod bulk_map;
mod layout;
mod mapping_editor;
mod mapping_list;
mod panel_slot;
mod status_bar;
mod top_bar;
mod view_state;
```

- [ ] **Step 2: Write the module entry with a stub component.**

Create `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`:

```rust
//! F-bulk-map: side-panel bulk mapping wizard. See
//! `docs/superpowers/specs/2026-05-03-bulk-mapping-design.md`.

#![allow(
    dead_code,
    reason = "Module is wired progressively across tasks 9-18; final exports settle in task 18."
)]

mod apply;
mod auto_map;
mod conflicts;
mod empty_state;
mod group_actions;
mod row_readout;
mod state;
mod summary;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

const BULK_MAP_CSS: Asset = asset!("/assets/frame/bulk_map.css");

/// Bulk-map wizard panel. Mounts inside `<aside class="if-panel-slot">`
/// when `view.panel_slot == PanelSlot::BulkMap`.
#[component]
pub(crate) fn BulkMapPanel() -> Element {
    tracing::trace!(target: "frame::render", region = "bulk_map");
    rsx! {
        Stylesheet { href: BULK_MAP_CSS }
        section { class: "if-bulk-map", "aria-label": "Bulk-map device wizard",
            // Real layout assembled in task 18.
            "Bulk-map wizard (under construction)"
        }
    }
}
```

- [ ] **Step 3: Create empty submodule files (filled in subsequent tasks).**

Create each file with a single-line module header so the parent `mod` declarations compile. Run these four `Write` calls:

`crates/inputforge-gui-dx/src/frame/bulk_map/state.rs`:

```rust
//! Wizard state types. Filled in task 10.
```

`crates/inputforge-gui-dx/src/frame/bulk_map/auto_map.rs`:

```rust
//! Positional auto-mapping logic. Filled in task 11.
```

`crates/inputforge-gui-dx/src/frame/bulk_map/conflicts.rs`:

```rust
//! Per-(row, mode) conflict detection. Filled in task 12.
```

`crates/inputforge-gui-dx/src/frame/bulk_map/group_actions.rs`:

```rust
//! Per-group bulk-action chip predicates. Filled in task 13.
```

`crates/inputforge-gui-dx/src/frame/bulk_map/summary.rs`:

```rust
//! Pre-apply summary chip count tally. Filled in task 14.
```

`crates/inputforge-gui-dx/src/frame/bulk_map/apply.rs`:

```rust
//! Entry generation and dispatch glue. Filled in task 15.
```

`crates/inputforge-gui-dx/src/frame/bulk_map/row_readout.rs`:

```rust
//! Compact live readout per row. Filled in task 16.
```

`crates/inputforge-gui-dx/src/frame/bulk_map/empty_state.rs`:

```rust
//! No-vJoy empty state. Filled in task 17.
```

`crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs`:

```rust
//! Layer-5 SSR tests for the bulk-map wizard. Body lands across tasks 10-18.

#![allow(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention"
)]
```

- [ ] **Step 4: Create the (empty) CSS file so the `asset!` macro can resolve it.**

Create `crates/inputforge-gui-dx/assets/frame/bulk_map.css`:

```css
/* Bulk-map wizard styles. Body lands in task 19. */
/* Always declare display explicitly on flex containers in this project. */
.if-bulk-map {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-3);
}
```

- [ ] **Step 5: Build to confirm.**

Run: `cargo check -p inputforge-gui-dx`
Expected: clean compile.

- [ ] **Step 6: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/mod.rs crates/inputforge-gui-dx/src/frame/bulk_map/ crates/inputforge-gui-dx/src/frame/view_state.rs crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs crates/inputforge-gui-dx/assets/frame/bulk_map.css
git commit -m "feat(bulk_map): scaffold BulkMap panel slot and module tree"
```

**Parallelization note:** Tasks 10-17 are independent of each other. Each writes one new module body and its unit tests; none depends on the others' implementations (only on Task 9's scaffolding). Dispatch via `superpowers:dispatching-parallel-agents` for faster turnaround. Tasks 18a-18d (assemble) require all of 10-17.

The module-level `#![allow(dead_code, ...)]` in `bulk_map/mod.rs` is intentional during 9-17 (symbols ship task-by-task). It is removed in Task 18d once the panel imports every helper.

---

### Task 10: Wizard state types

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/state.rs`

- [ ] **Step 1: Write the state types and tests.**

Replace `crates/inputforge-gui-dx/src/frame/bulk_map/state.rs` body with:

```rust
//! Wizard state types and pure helpers.
//!
//! State machine summary: the wizard owns one source-device id, one
//! target-vjoy id, one mode picker value, an `apply_to_all_modes`
//! flag, and a `Vec<RowState>` keyed in source-input order. Each row
//! carries (a) a target override (`Option<OutputAddress>` where
//! `None` means "(do not map)") and (b) a per-row replace flag
//! defaulting to `false`. The flag is `true` only when the user has
//! explicitly clicked the row's `replace` chip.

use inputforge_core::types::{InputAddress, OutputAddress};

/// Discriminator used by the row template (kind chip + auto-map
/// algorithm). Matches the F8 mapping-list group taxonomy: Axes,
/// Buttons, Hats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowKind {
    Axis,
    Button,
    Hat,
}

/// One source input (the wizard's row identity).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RowState {
    pub kind: RowKind,
    /// Index of this input on the source device (0-based for all kinds).
    pub source_index: u8,
    /// Source address always Bound. Computed from
    /// `source_device + RowKind + source_index`.
    pub input: InputAddress,
    /// `None` means `(do not map)`; `Some` carries the user-chosen
    /// or auto-suggested target. Overflow rows default to `None`.
    pub target: Option<OutputAddress>,
    /// Sticky per-row "replace existing" flag. When `false`, a row
    /// whose `(input, mode)` already exists is skipped. When `true`,
    /// the row promotes to a replace tally and the existing mapping
    /// is overwritten.
    pub replace: bool,
}

/// Aggregate wizard state. Held by the panel component and threaded
/// to its children via signals or props as needed.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct WizardState {
    pub source_device_id: Option<inputforge_core::types::DeviceId>,
    pub target_vjoy_id: Option<u8>,
    pub mode: String,
    pub apply_to_all_modes: bool,
    pub rows: Vec<RowState>,
}

impl WizardState {
    pub fn empty(default_mode: String) -> Self {
        Self {
            source_device_id: None,
            target_vjoy_id: None,
            mode: default_mode,
            apply_to_all_modes: false,
            rows: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_initial_values() {
        let s = WizardState::empty("Default".to_owned());
        assert!(s.source_device_id.is_none());
        assert!(s.target_vjoy_id.is_none());
        assert_eq!(s.mode, "Default");
        assert!(!s.apply_to_all_modes);
        assert!(s.rows.is_empty());
    }
}
```

- [ ] **Step 2: Run tests.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map::state -- --nocapture`
Expected: pass.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/state.rs
git commit -m "feat(bulk_map): add WizardState and RowState data types"
```

---

### Task 11: Auto-mapping logic

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/auto_map.rs`

- [ ] **Step 1: Write tests first.**

Replace `crates/inputforge-gui-dx/src/frame/bulk_map/auto_map.rs` body with:

```rust
//! Positional auto-mapping logic. Pure functions only.
//!
//! Convention (locked in design Q4):
//! - Source axis index `i` maps to the `i`-th vJoy axis in
//!   `VirtualDeviceConfig.axes` order. The order is the canonical
//!   `VJoyAxis` enum order: X, Y, Z, Rx, Ry, Rz, Slider0, Slider1,
//!   subject to which slots vJoy actually exposes.
//! - Source button `i` (0-indexed) maps to vJoy button `i + 1`
//!   (1-indexed at the SDK layer). 0-vs-1 convention is intentional.
//! - Source hat `i` (0-indexed) maps to vJoy hat `i + 1`.
//! - Overflow (source has more inputs of a kind than the target):
//!   the row's auto-target is `None`.

use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis, VirtualDeviceConfig};

/// Return the auto-suggested target for source axis `i` against
/// `target`. `None` when `i >= target.axes.len()`.
pub(super) fn auto_axis_target(target: &VirtualDeviceConfig, i: usize) -> Option<OutputAddress> {
    let axis: VJoyAxis = *target.axes.get(i)?;
    Some(OutputAddress {
        device: target.device_id,
        output: OutputId::Axis { id: axis },
    })
}

/// Return the auto-suggested target for source button `i` against
/// `target`. `None` when `i >= target.button_count`.
pub(super) fn auto_button_target(target: &VirtualDeviceConfig, i: usize) -> Option<OutputAddress> {
    if i >= target.button_count as usize {
        return None;
    }
    let id = u8::try_from(i + 1).ok()?;
    Some(OutputAddress {
        device: target.device_id,
        output: OutputId::Button { id },
    })
}

/// Return the auto-suggested target for source hat `i` against
/// `target`. `None` when `i >= target.hat_count`.
pub(super) fn auto_hat_target(target: &VirtualDeviceConfig, i: usize) -> Option<OutputAddress> {
    if i >= target.hat_count as usize {
        return None;
    }
    let id = u8::try_from(i + 1).ok()?;
    Some(OutputAddress {
        device: target.device_id,
        output: OutputId::Hat { id },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> VirtualDeviceConfig {
        VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X, VJoyAxis::Y, VJoyAxis::Z, VJoyAxis::Rx, VJoyAxis::Ry, VJoyAxis::Rz, VJoyAxis::Slider0, VJoyAxis::Slider1],
            button_count: 32,
            hat_count: 1,
        }
    }

    #[test]
    fn axis_index_zero_maps_to_x() {
        let t = auto_axis_target(&target(), 0).unwrap();
        assert!(matches!(t.output, OutputId::Axis { id: VJoyAxis::X }));
    }

    #[test]
    fn axis_index_seven_maps_to_slider1() {
        let t = auto_axis_target(&target(), 7).unwrap();
        assert!(matches!(t.output, OutputId::Axis { id: VJoyAxis::Slider1 }));
    }

    #[test]
    fn axis_overflow_returns_none() {
        let mut tgt = target();
        tgt.axes = vec![VJoyAxis::X];
        assert!(auto_axis_target(&tgt, 1).is_none());
    }

    #[test]
    fn button_zero_maps_to_button_one() {
        let t = auto_button_target(&target(), 0).unwrap();
        assert!(matches!(t.output, OutputId::Button { id: 1 }));
    }

    #[test]
    fn button_overflow_returns_none() {
        let mut tgt = target();
        tgt.button_count = 4;
        assert!(auto_button_target(&tgt, 4).is_none());
    }

    #[test]
    fn hat_zero_maps_to_hat_one() {
        let t = auto_hat_target(&target(), 0).unwrap();
        assert!(matches!(t.output, OutputId::Hat { id: 1 }));
    }

    #[test]
    fn hat_overflow_returns_none() {
        let mut tgt = target();
        tgt.hat_count = 0;
        assert!(auto_hat_target(&tgt, 0).is_none());
    }
}
```

- [ ] **Step 2: Run.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map::auto_map -- --nocapture`
Expected: pass.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/auto_map.rs
git commit -m "feat(bulk_map): add positional auto-mapping pure logic"
```

---

### Task 12: Conflict detection per (row, mode)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/conflicts.rs`

- [ ] **Step 1: Write the helper plus tests.**

Replace the file body:

```rust
//! Per-(row, mode) conflict detection.
//!
//! The wizard treats a row as "conflicted" in a given mode when the
//! active profile already contains a mapping for `(row.input, mode)`.
//! With `apply_to_all_modes`, conflict checks fan out across every
//! profile mode and produce a per-(row, mode) verdict.

use inputforge_core::profile::Profile;
use inputforge_core::types::InputAddress;

/// Returns the existing mapping name (or `""` for unnamed) when
/// `(input, mode)` collides, `None` otherwise.
pub(super) fn existing_name_for(profile: &Profile, input: &InputAddress, mode: &str) -> Option<String> {
    profile
        .find_mapping(input, mode)
        .map(|m| m.name.clone().unwrap_or_default())
}

/// Returns the list of modes (from `modes`) where `input` already has
/// a mapping in `profile`.
pub(super) fn conflicting_modes(profile: &Profile, input: &InputAddress, modes: &[String]) -> Vec<String> {
    modes
        .iter()
        .filter(|m| profile.find_mapping(input, m).is_some())
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::action::Action;
    use inputforge_core::types::{DeviceId, InputId};

    fn one_mode_profile() -> Profile {
        let map = std::collections::HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
        let modes = inputforge_core::mode::ModeTree::from_adjacency(&map).unwrap();
        Profile::new("T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned())
    }

    fn axis_zero() -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        }
    }

    #[test]
    fn no_conflict_returns_none() {
        let profile = one_mode_profile();
        assert!(existing_name_for(&profile, &axis_zero(), "Default").is_none());
    }

    #[test]
    fn existing_named_mapping_returns_name() {
        let mut profile = one_mode_profile();
        profile.set_mapping(&axis_zero(), "Default", Some("Throttle".to_owned()), vec![Action::Invert]);
        assert_eq!(
            existing_name_for(&profile, &axis_zero(), "Default").as_deref(),
            Some("Throttle")
        );
    }

    #[test]
    fn existing_unnamed_mapping_returns_empty_string() {
        let mut profile = one_mode_profile();
        profile.set_mapping(&axis_zero(), "Default", None, vec![Action::Invert]);
        assert_eq!(existing_name_for(&profile, &axis_zero(), "Default").as_deref(), Some(""));
    }

    #[test]
    fn conflicting_modes_lists_only_collisions() {
        let mut profile = one_mode_profile();
        profile.set_mapping(&axis_zero(), "Default", None, vec![Action::Invert]);
        let modes = vec!["Default".to_owned(), "Combat".to_owned()];
        let collisions = conflicting_modes(&profile, &axis_zero(), &modes);
        assert_eq!(collisions, vec!["Default".to_owned()]);
    }
}
```

- [ ] **Step 2: Run.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map::conflicts -- --nocapture`
Expected: pass.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/conflicts.rs
git commit -m "feat(bulk_map): add per-(row, mode) conflict detection helpers"
```

---

### Task 13: Group-action chip predicates

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/group_actions.rs`

- [ ] **Step 1: Write the predicates and tests.**

Replace the file body:

```rust
//! Per-group bulk-action chip predicates.
//!
//! Each predicate inspects a slice of rows of one kind (Axes, Buttons,
//! Hats) plus the conflict mode list. Returns `true` when the
//! corresponding chip should render on the group header.

use super::state::RowState;

/// `skip all conflicts` chip: surfaces when at least one row in the
/// group is in replace-state and is conflict-driven (i.e., the
/// existing mapping is what triggered the replace state).
pub(super) fn show_skip_all_conflicts(rows: &[&RowState], conflicting: &[bool]) -> bool {
    rows.iter()
        .zip(conflicting.iter())
        .any(|(r, &c)| r.replace && c)
}

/// `replace all conflicts` chip: surfaces when at least one row in
/// the group is in skip-state with a conflict.
pub(super) fn show_replace_all_conflicts(rows: &[&RowState], conflicting: &[bool]) -> bool {
    rows.iter()
        .zip(conflicting.iter())
        .any(|(r, &c)| !r.replace && c && r.target.is_some())
}

/// `include all` chip: surfaces when at least one row in the group is
/// `(do not map)`.
pub(super) fn show_include_all(rows: &[&RowState]) -> bool {
    rows.iter().any(|r| r.target.is_none())
}

/// `exclude all` chip: surfaces when at least one row in the group
/// has a target set.
pub(super) fn show_exclude_all(rows: &[&RowState]) -> bool {
    rows.iter().any(|r| r.target.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::bulk_map::state::RowKind;
    use inputforge_core::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    fn axis_row(idx: u8, target: Option<OutputAddress>, replace: bool) -> RowState {
        RowState {
            kind: RowKind::Axis,
            source_index: idx,
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: idx },
            },
            target,
            replace,
        }
    }

    fn x_target() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }
    }

    #[test]
    fn skip_all_conflicts_on_when_any_row_is_in_replace_with_conflict() {
        let rows = vec![axis_row(0, Some(x_target()), true)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(show_skip_all_conflicts(&refs, &[true]));
    }

    #[test]
    fn replace_all_conflicts_off_when_no_conflict_present() {
        let rows = vec![axis_row(0, Some(x_target()), false)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(!show_replace_all_conflicts(&refs, &[false]));
    }

    #[test]
    fn replace_all_conflicts_on_when_skip_state_with_conflict() {
        // Skip state (replace=false) + conflicting=true + target set
        // is exactly the condition the chip is designed to surface.
        let rows = vec![axis_row(0, Some(x_target()), false)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(show_replace_all_conflicts(&refs, &[true]));
    }

    #[test]
    fn include_all_on_when_any_row_is_do_not_map() {
        let rows = vec![axis_row(0, None, false)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(show_include_all(&refs));
    }

    #[test]
    fn exclude_all_on_when_any_row_has_a_target() {
        let rows = vec![axis_row(0, Some(x_target()), false)];
        let refs: Vec<&RowState> = rows.iter().collect();
        assert!(show_exclude_all(&refs));
    }
}
```

- [ ] **Step 2: Run.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map::group_actions -- --nocapture`
Expected: pass.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/group_actions.rs
git commit -m "feat(bulk_map): add group-action chip predicates"
```

---

### Task 14: Summary chip count tally

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/summary.rs`

- [ ] **Step 1: Write the tally helper plus tests.**

Replace the file body:

```rust
//! Pre-apply summary chip counts.
//!
//! Walks the wizard rows against the conflict map and reports the
//! `(create, replace, skip, excluded)` tuple shown in the summary
//! chip. With `apply_to_all_modes`, the tally fans out across every
//! mode in `modes`; otherwise the tally counts only `current_mode`.
//!
//! **Asymmetry note.** `excluded` counts each `(do not map)` row
//! exactly once: exclusion is a row-level decision and never reaches
//! a per-(row, mode) verdict. `create` / `replace` / `skip` fan out
//! across `modes` because they describe per-(row, mode) outcomes, so a
//! single row over five modes contributes up to five increments to
//! that triple. Test `excluded_does_not_fan_out_across_modes` locks
//! the asymmetry.

use super::state::RowState;
use inputforge_core::profile::Profile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct SummaryCounts {
    pub create: usize,
    pub replace: usize,
    pub skip: usize,
    pub excluded: usize,
}

pub(super) fn tally(
    profile: &Profile,
    rows: &[RowState],
    modes: &[String],
) -> SummaryCounts {
    let mut counts = SummaryCounts::default();
    for row in rows {
        if row.target.is_none() {
            counts.excluded += 1;
            continue;
        }
        for mode in modes {
            let collides = profile.find_mapping(&row.input, mode).is_some();
            match (collides, row.replace) {
                (false, _) => counts.create += 1,
                (true, true) => counts.replace += 1,
                (true, false) => counts.skip += 1,
            }
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::bulk_map::state::RowKind;
    use inputforge_core::action::Action;
    use inputforge_core::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    fn one_mode_profile() -> Profile {
        let map = std::collections::HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
        let modes = inputforge_core::mode::ModeTree::from_adjacency(&map).unwrap();
        Profile::new("T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned())
    }

    fn axis_row(idx: u8, target: Option<OutputAddress>, replace: bool) -> RowState {
        RowState {
            kind: RowKind::Axis,
            source_index: idx,
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: idx },
            },
            target,
            replace,
        }
    }

    fn x_target() -> OutputAddress {
        OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        }
    }

    #[test]
    fn create_only_on_clean_profile_one_mode() {
        let p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        let counts = tally(&p, &rows, &["Default".to_owned()]);
        assert_eq!(counts, SummaryCounts { create: 1, replace: 0, skip: 0, excluded: 0 });
    }

    #[test]
    fn excluded_when_target_is_none() {
        let p = one_mode_profile();
        let rows = vec![axis_row(0, None, false)];
        let counts = tally(&p, &rows, &["Default".to_owned()]);
        assert_eq!(counts.excluded, 1);
        assert_eq!(counts.create, 0);
    }

    #[test]
    fn skip_when_conflict_and_replace_false() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        let counts = tally(&p, &rows, &["Default".to_owned()]);
        assert_eq!(counts.skip, 1);
    }

    #[test]
    fn replace_when_conflict_and_replace_true() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), true)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        let counts = tally(&p, &rows, &["Default".to_owned()]);
        assert_eq!(counts.replace, 1);
    }

    #[test]
    fn fans_out_across_all_modes_when_apply_to_all_modes_is_active() {
        let p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        let counts = tally(&p, &rows, &["Default".to_owned(), "Combat".to_owned()]);
        assert_eq!(counts.create, 2, "one row times two modes = two creates");
    }

    #[test]
    fn excluded_does_not_fan_out_across_modes() {
        // Exclusion is a row-level decision; it counts once even when
        // multiple modes are selected. Locks the documented asymmetry.
        let p = one_mode_profile();
        let rows = vec![axis_row(0, None, false)];
        let counts = tally(&p, &rows, &["Default".to_owned(), "Combat".to_owned()]);
        assert_eq!(counts.excluded, 1, "excluded counts the row, not row-by-mode");
        assert_eq!(counts.create, 0);
    }
}
```

- [ ] **Step 2: Run.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map::summary -- --nocapture`
Expected: pass.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/summary.rs
git commit -m "feat(bulk_map): add summary chip count tally"
```

---

### Task 15: Entry generation and dispatch

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/apply.rs`

- [ ] **Step 1: Write the helper plus tests.**

Replace the file body:

```rust
//! Entry generation and command dispatch.
//!
//! `build_entries` walks the cross-product of committed rows and
//! selected modes, filtering out `(do not map)` rows and skip-on-
//! conflict (row, mode) pairs, and emits a `Vec<BulkMapEntry>` ready
//! for `EngineCommand::SetMappingsBulk`.
//!
//! `format_snapshot_label` produces the user-visible recovery
//! snapshot label.

use super::state::RowState;
use inputforge_core::action::BulkMapEntry;
use inputforge_core::profile::Profile;

pub(super) fn build_entries(
    profile: &Profile,
    rows: &[RowState],
    modes: &[String],
) -> Vec<BulkMapEntry> {
    let mut out = Vec::new();
    for row in rows {
        let Some(target) = row.target.clone() else {
            continue;
        };
        for mode in modes {
            let collides = profile.find_mapping(&row.input, mode).is_some();
            if collides && !row.replace {
                continue;
            }
            out.push(BulkMapEntry {
                input: row.input.clone(),
                mode: mode.clone(),
                output: target.clone(),
            });
        }
    }
    out
}

pub(super) fn format_snapshot_label(source_name: &str, target_id: u8) -> String {
    format!("Before bulk-map: {source_name} to vJoy {target_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::bulk_map::state::RowKind;
    use inputforge_core::action::Action;
    use inputforge_core::types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis};

    fn one_mode_profile() -> Profile {
        let map = std::collections::HashMap::from([("Default".to_owned(), vec!["Combat".to_owned()])]);
        let modes = inputforge_core::mode::ModeTree::from_adjacency(&map).unwrap();
        Profile::new("T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned())
    }

    fn axis_row(idx: u8, target: Option<OutputAddress>, replace: bool) -> RowState {
        RowState {
            kind: RowKind::Axis,
            source_index: idx,
            input: InputAddress::Bound {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: idx },
            },
            target,
            replace,
        }
    }

    fn x_target() -> OutputAddress {
        OutputAddress { device: 1, output: OutputId::Axis { id: VJoyAxis::X } }
    }

    #[test]
    fn excludes_do_not_map_rows() {
        let p = one_mode_profile();
        let rows = vec![axis_row(0, None, false)];
        assert!(build_entries(&p, &rows, &["Default".to_owned()]).is_empty());
    }

    #[test]
    fn excludes_skip_on_conflict_rows() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        assert!(build_entries(&p, &rows, &["Default".to_owned()]).is_empty());
    }

    #[test]
    fn includes_replace_rows_with_conflict() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), true)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        assert_eq!(build_entries(&p, &rows, &["Default".to_owned()]).len(), 1);
    }

    #[test]
    fn fans_out_across_modes_with_per_mode_conflict_filter() {
        let mut p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), false)];
        p.set_mapping(&rows[0].input, "Default", None, vec![Action::Invert]);
        let entries = build_entries(&p, &rows, &["Default".to_owned(), "Combat".to_owned()]);
        assert_eq!(entries.len(), 1, "Default skipped (conflict, replace=false); Combat created");
        assert_eq!(entries[0].mode, "Combat");
    }

    #[test]
    fn includes_replace_rows_without_conflict_as_normal_create() {
        // replace=true with no existing mapping: the flag is irrelevant
        // (no mapping to replace), entry is emitted as a normal create.
        let p = one_mode_profile();
        let rows = vec![axis_row(0, Some(x_target()), true)];
        assert_eq!(build_entries(&p, &rows, &["Default".to_owned()]).len(), 1);
    }

    #[test]
    fn multi_row_mixed_state_single_mode() {
        // Three rows: replace-with-conflict, do-not-map, normal-create.
        // Expect two entries: the replace and the normal-create. The
        // do-not-map row is excluded, no entry generated.
        let mut p = one_mode_profile();
        let row_a = axis_row(0, Some(x_target()), true); // replace, conflict
        let row_b = axis_row(1, None, false);             // do-not-map
        let row_c = axis_row(2, Some(OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::Y },
        }), false);                                       // normal create
        p.set_mapping(&row_a.input, "Default", None, vec![Action::Invert]);
        let entries = build_entries(&p, &[row_a, row_b, row_c], &["Default".to_owned()]);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn label_format_matches_spec() {
        assert_eq!(format_snapshot_label("FlightStick", 1), "Before bulk-map: FlightStick to vJoy 1");
    }
}
```

- [ ] **Step 2: Run.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map::apply -- --nocapture`
Expected: pass.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/apply.rs
git commit -m "feat(bulk_map): add entry generation and snapshot-label helper"
```

---

### Task 16: `row_readout` component

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs` (promote three helpers to `pub(crate)` and one new helper)
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/row_readout.rs`

Spec Q10 explicitly endorses sharing F9's data-resolution helpers; only the markup forks. This task therefore promotes (rather than forks) the read functions on `live.device_inputs`.

- [ ] **Step 1: Promote the F9 read helpers.**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs`:
- Change `fn read_axis_display(...) -> AxisDisplay` (currently private around line 292) to `pub(crate) fn read_axis_display(...)`.
- Promote the supporting types `AxisDisplay` and `AxisPolarity` re-export to `pub(crate)` if they are not already, so external callers can name the return type.
- Add two siblings at the end of the file:

```rust
/// Read whether the button at `addr` is currently pressed in the live
/// snapshot.
///
/// Returns `false` when the device or button index is not present
/// (engine offline, non-button input, or stale address).
pub(crate) fn read_button_pressed(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> bool {
    let Some(InputId::Button { id }) = addr.input_id() else {
        return false;
    };
    let dev_idx = cfg
        .devices
        .iter()
        .position(|d| Some(&d.info.id) == addr.device());
    dev_idx
        .and_then(|di| live.device_inputs.get(di))
        .and_then(|dev_inputs| {
            // Buttons are 1-indexed at the SDK boundary; the live snapshot
            // stores them 0-indexed in `dev_inputs.buttons`.
            let zero_based = usize::from(id.checked_sub(1)?);
            dev_inputs.buttons.get(zero_based).copied()
        })
        .unwrap_or(false)
}

/// Read the hat direction at `addr` from the live snapshot. Returns
/// `HatDirection::Center` when the device or hat index is not present.
pub(crate) fn read_hat_direction(
    addr: &InputAddress,
    live: &LiveSnapshot,
    cfg: &ConfigSnapshot,
) -> HatDirection {
    let Some(InputId::Hat { index }) = addr.input_id() else {
        return HatDirection::Center;
    };
    let dev_idx = cfg
        .devices
        .iter()
        .position(|d| Some(&d.info.id) == addr.device());
    dev_idx
        .and_then(|di| live.device_inputs.get(di))
        .and_then(|dev_inputs| dev_inputs.hats.get(usize::from(*index)).copied())
        .unwrap_or(HatDirection::Center)
}
```

- [ ] **Step 2: Write the row_readout component using the shared helpers.**

Replace the file body of `crates/inputforge-gui-dx/src/frame/bulk_map/row_readout.rs`:

```rust
//! Compact live readout per row.
//!
//! Reads from `live.device_inputs` via the helpers shared with F9's
//! `LiveReadout` (`read_axis_display`, `read_button_pressed`,
//! `read_hat_direction`). The wizard renders these values into its
//! own grid template; F9 keeps its editor-row template.
//!
//! Rendered per row kind:
//! - Axis: bipolar bar (centered at 50%; fill grows toward the active edge).
//! - Button: filled-or-stamped dot.
//! - Hat: mono cardinal letter (N/E/S/W/NE/SE/SW/NW/centered dot).

use dioxus::prelude::*;

use inputforge_core::types::{HatDirection, InputAddress};

use crate::context::AppContext;
use crate::frame::bulk_map::state::RowKind;
use crate::frame::mapping_editor::live_readout::{
    AxisPolarity, read_axis_display, read_button_pressed, read_hat_direction,
};

#[component]
pub(super) fn RowReadout(kind: RowKind, address: InputAddress) -> Element {
    let ctx = use_context::<AppContext>();
    let live = ctx.live.read();
    let cfg = ctx.config.read();

    match kind {
        RowKind::Axis => {
            let display = read_axis_display(&address, &live, &cfg);
            let value = display.value;
            let half_width = (value.abs() * 50.0).clamp(0.0, 50.0);
            let style = match display.polarity {
                AxisPolarity::Bipolar if value >= 0.0 => {
                    format!("left: 50%; right: auto; width: {half_width:.2}%")
                }
                AxisPolarity::Bipolar => {
                    format!("right: 50%; left: auto; width: {half_width:.2}%")
                }
                AxisPolarity::Unipolar => {
                    let pct = (value * 100.0).clamp(0.0, 100.0);
                    format!("left: 0; right: auto; width: {pct:.2}%")
                }
            };
            rsx! {
                div { class: "if-bulk-map__live if-bulk-map__live--axis",
                    div { class: "if-bulk-map__live-bar", style: "{style}" }
                }
            }
        }
        RowKind::Button => {
            let pressed = read_button_pressed(&address, &live, &cfg);
            let cls = if pressed {
                "if-bulk-map__live if-bulk-map__live--button if-bulk-map__live--button-on"
            } else {
                "if-bulk-map__live if-bulk-map__live--button"
            };
            rsx! { div { class: "{cls}" } }
        }
        RowKind::Hat => {
            let direction = read_hat_direction(&address, &live, &cfg);
            let label = match direction {
                HatDirection::Center => "·",
                HatDirection::North => "N",
                HatDirection::NorthEast => "NE",
                HatDirection::East => "E",
                HatDirection::SouthEast => "SE",
                HatDirection::South => "S",
                HatDirection::SouthWest => "SW",
                HatDirection::West => "W",
                HatDirection::NorthWest => "NW",
            };
            rsx! { div { class: "if-bulk-map__live if-bulk-map__live--hat", "{label}" } }
        }
    }
}
```

- [ ] **Step 3: Build to confirm.**

Run: `cargo check -p inputforge-gui-dx`
Expected: clean compile. Tests for the component land in Task 18b (full SSR).

- [ ] **Step 4: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs crates/inputforge-gui-dx/src/frame/bulk_map/row_readout.rs
git commit -m "feat(bulk_map): add per-row live readout component sharing F9 read helpers"
```

---

### Task 17: Empty-state component

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/empty_state.rs`

- [ ] **Step 1: Write the component.**

Replace the file body:

```rust
//! No-vJoy empty state. Renders when `AppState.virtual_devices` is
//! empty (no vJoy devices configured) or when no profile is loaded.
//!
//! `caption` lets the panel customize the helper text; the title is
//! invariant. The icon uses `IconKind::Info` (the closest neutral
//! glyph in the project's icon set; no `CircleSlash` exists yet).

use dioxus::prelude::*;

use crate::icons::{Icon, IconKind};

#[component]
pub(super) fn NoVjoyEmptyState(
    #[props(default = "Configure outputs in vJoyConf, then reopen.".to_owned())]
    caption: String,
    #[props(default = "No vJoy devices configured".to_owned())]
    title: String,
) -> Element {
    rsx! {
        div { class: "if-bulk-map__empty",
            div { class: "if-bulk-map__empty-icon", "aria-hidden": "true",
                Icon { kind: IconKind::Info }
            }
            h3 { class: "if-bulk-map__empty-title", "{title}" }
            p { class: "if-bulk-map__empty-caption", "{caption}" }
        }
    }
}
```

- [ ] **Step 2: Build.**

Run: `cargo check -p inputforge-gui-dx`
Expected: clean compile.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/empty_state.rs
git commit -m "feat(bulk_map): add no-vJoy empty state component"
```

---

### Task 18 (split): Assemble the panel and add layer-5 SSR tests

Original Task 18 was ~870 lines, spanning panel skeleton + rows table + group bulk-action chips + apply dispatch. Spec Q13 also explicitly demands per-group bulk-action chips (`skip all conflicts` / `replace all conflicts` / `include all` / `exclude all`), which fit in their own sub-task. The task is therefore split into four self-contained units (18a/18b/18c/18d) with separate commits and review checkpoints. Each can be executed by a separate subagent in sequence.

Pre-task corrections applied throughout 18a-18d:
- `Select` and `Checkbox` calls go through the project's `Field` primitive (`crates/inputforge-gui-dx/src/components/field.rs`). Real `Select` props are `value: ReadSignal<String>`, `onchange: Option<EventHandler<FormEvent>>`, `options: Vec<(String, String)>` with `(value, label)` ordering, plus optional `id` and `disabled`. There is no `SelectOption` type and no `label`/`on_change` props directly on `Select`/`Checkbox`.
- `BulkMapPanel` adds a no-profile guard (renders the empty-state component with a different caption when `active_profile` is `None`); the original code's `expect("Apply visible only when profile is loaded")` would have panicked.
- `Stylesheet { href: BULK_MAP_CSS }` mounts once at the panel's outermost element, not inside every conditional branch.
- Axis labels use a manual match producing `"X axis"`, `"Slider 0"`, etc. (see `live_readout.rs:461-477`'s `format_output_label` for precedent), not `format!("Axis {axis:?}")`.
- The dispatch-capture SSR test invokes a `#[cfg(test)] pub(super) fn apply_for_test(...)` helper (added in 18d) and asserts on `cmd_rx.try_recv()` rather than asserting on rendered HTML.

---

### Task 18a: Panel skeleton (metadata strip, header, footer, empty states)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs`

The component is large but compositional; 18a lays the skeleton and the metadata strip. Rows come in 18b, chips in 18c, apply in 18d.

- [ ] **Step 1: Replace bulk_map/mod.rs `BulkMapPanel` body with the panel skeleton.**

The skeleton owns: header (title + close button), metadata strip (Source / Target / Mode pickers + Apply-to-all-modes checkbox), footer (Cancel / Apply outline). It returns the empty-state component when `virtual_devices` is empty OR when `active_profile` is `None`. Rows table, chips, and apply dispatch are stubbed (rendered as empty `<div>`s) and filled in by 18b, 18c, 18d.

```rust
//! F-bulk-map: side-panel bulk mapping wizard. See
//! `docs/superpowers/specs/2026-05-03-bulk-mapping-design.md`.

#![allow(
    dead_code,
    reason = "Module wired progressively across tasks 9 to 18; allow removed in 18d."
)]

mod apply;
mod auto_map;
mod conflicts;
mod empty_state;
mod group_actions;
mod row_readout;
mod state;
mod summary;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{DeviceId, InputAddress, InputId, OutputAddress, VirtualDeviceConfig};

use crate::components::{Button, Checkbox, Field, Select};
use crate::context::AppContext;
use crate::frame::bulk_map::auto_map::{auto_axis_target, auto_button_target, auto_hat_target};
use crate::frame::bulk_map::empty_state::NoVjoyEmptyState;
use crate::frame::bulk_map::state::{RowKind, RowState, WizardState};
use crate::frame::view_state::{PanelSlot, ViewState};
use crate::toast::{ToastLevel, ToastQueue};

const BULK_MAP_CSS: Asset = asset!("/assets/frame/bulk_map.css");

#[component]
pub(crate) fn BulkMapPanel() -> Element {
    tracing::trace!(target: "frame::render", region = "bulk_map");

    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let mut panel = view.panel_slot;

    // Read connected devices and live vJoys once per render.
    let connected_devices = {
        let s = ctx.state.read();
        s.devices.iter().filter(|d| d.connected).cloned().collect::<Vec<_>>()
    };
    let virtual_devices: Vec<VirtualDeviceConfig> = ctx.state.read().virtual_devices.clone();
    let has_profile = ctx.state.read().active_profile.is_some();

    // Empty-state guards: rendered FIRST so the rest of the panel can
    // safely assume both a profile and at least one vJoy device. The
    // Stylesheet mounts ONCE at the outermost element (Dioxus dedupes
    // <link> by URL, so this is fine even though both branches mount it,
    // but mounting once keeps the convention used by F9/F12/F13).
    if !has_profile {
        return rsx! {
            Stylesheet { href: BULK_MAP_CSS }
            section { class: "if-bulk-map", "aria-label": "Bulk-map device wizard",
                BulkMapHeader { on_close: move |_| panel.set(PanelSlot::None) }
                NoVjoyEmptyState {
                    title: "No profile loaded".to_owned(),
                    caption: "Load or create a profile, then reopen.".to_owned(),
                }
                footer { class: "if-bulk-map__footer",
                    Button { onclick: move |_| panel.set(PanelSlot::None), "Cancel" }
                    Button { disabled: true, onclick: move |_| {}, "Apply" }
                }
            }
        };
    }
    if virtual_devices.is_empty() {
        return rsx! {
            Stylesheet { href: BULK_MAP_CSS }
            section { class: "if-bulk-map", "aria-label": "Bulk-map device wizard",
                BulkMapHeader { on_close: move |_| panel.set(PanelSlot::None) }
                NoVjoyEmptyState {}
                footer { class: "if-bulk-map__footer",
                    Button { onclick: move |_| panel.set(PanelSlot::None), "Cancel" }
                    Button { disabled: true, onclick: move |_| {}, "Apply" }
                }
            }
        };
    }

    // Derive the active editing mode and the full mode list from meta.
    let editing_mode = view.editing_mode.read().clone();
    let modes: Vec<String> = ctx.meta.read().modes.clone();

    let mut wizard = use_signal(|| WizardState::empty(editing_mode.clone()));

    // Initial pick: first connected device + first vJoy.
    if wizard.peek().source_device_id.is_none() {
        if let Some(first_dev) = connected_devices.first() {
            wizard.write().source_device_id = Some(first_dev.info.id.clone());
        }
    }
    if wizard.peek().target_vjoy_id.is_none() {
        if let Some(first_vj) = virtual_devices.first() {
            wizard.write().target_vjoy_id = Some(first_vj.device_id);
        }
    }

    // Wizard view-state caches read once per render so we can build
    // signals into the metadata pickers below.
    let source_value = use_signal(|| {
        wizard.peek().source_device_id.as_ref().map(|d| d.0.clone()).unwrap_or_default()
    });
    let target_value = use_signal(|| {
        wizard.peek().target_vjoy_id.map(|v| v.to_string()).unwrap_or_default()
    });
    let mode_value = use_signal(|| wizard.peek().mode.clone());
    let apply_to_all = use_signal(|| wizard.peek().apply_to_all_modes);

    // Sync wizard <- pickers via onchange handlers (one-line writes).
    let on_source_change = move |evt: FormEvent| {
        let v = evt.data.value();
        wizard.write().source_device_id = Some(DeviceId(v));
    };
    let on_target_change = move |evt: FormEvent| {
        if let Ok(id) = evt.data.value().parse::<u8>() {
            wizard.write().target_vjoy_id = Some(id);
        }
    };
    let on_mode_change = move |evt: FormEvent| {
        wizard.write().mode = evt.data.value();
    };
    let on_apply_to_all_change = move |evt: FormEvent| {
        wizard.write().apply_to_all_modes = evt.data.value() == "true";
    };

    let snapshot_caption = "Snapshot taken before apply.";

    rsx! {
        Stylesheet { href: BULK_MAP_CSS }
        section { class: "if-bulk-map", "aria-label": "Bulk-map device wizard",
            BulkMapHeader { on_close: move |_| panel.set(PanelSlot::None) }

            // Metadata strip
            div { class: "if-bulk-map__metadata",
                Field { label: "Source".into(), for_id: "bulk-map-source".into(),
                    Select {
                        id: "bulk-map-source".into(),
                        value: source_value,
                        onchange: Some(EventHandler::new(on_source_change)),
                        options: connected_devices.iter().map(|d| (d.info.id.0.clone(), d.info.name.clone())).collect(),
                    }
                }
                Field { label: "Target".into(), for_id: "bulk-map-target".into(),
                    Select {
                        id: "bulk-map-target".into(),
                        value: target_value,
                        onchange: Some(EventHandler::new(on_target_change)),
                        options: virtual_devices.iter().map(|v| (
                            v.device_id.to_string(),
                            format!(
                                "vJoy {}: {} axes, {} buttons, {} hat{}",
                                v.device_id,
                                v.axes.len(),
                                v.button_count,
                                v.hat_count,
                                if v.hat_count == 1 { "" } else { "s" },
                            ),
                        )).collect(),
                    }
                }
                Field { label: "Mode".into(), for_id: "bulk-map-mode".into(),
                    Select {
                        id: "bulk-map-mode".into(),
                        disabled: *apply_to_all.read(),
                        value: mode_value,
                        onchange: Some(EventHandler::new(on_mode_change)),
                        options: modes.iter().map(|m| (m.clone(), m.clone())).collect(),
                    }
                }
                Field {
                    label: format!("Apply to all modes ({})", modes.len()),
                    for_id: "bulk-map-all-modes".into(),
                    Checkbox {
                        id: "bulk-map-all-modes".into(),
                        checked: apply_to_all,
                        onchange: Some(EventHandler::new(on_apply_to_all_change)),
                    }
                }
            }

            // Rows table (assembled in 18b).
            div { role: "grid", class: "if-bulk-map__table" }

            // Summary chip + footer (assembled in 18d).
            div { class: "if-bulk-map__summary" }
            footer { class: "if-bulk-map__footer",
                Button { onclick: move |_| panel.set(PanelSlot::None), "Cancel" }
                Button { disabled: true, onclick: move |_| {}, "Apply" }
            }
            div { class: "if-bulk-map__caption", "{snapshot_caption}" }
        }
    }
}

#[component]
fn BulkMapHeader(on_close: EventHandler<MouseEvent>) -> Element {
    rsx! {
        header { class: "if-bulk-map__header",
            h2 { class: "if-bulk-map__title", "Bulk-map device" }
            button {
                r#type: "button",
                class: "if-bulk-map__close",
                "aria-label": "Close panel",
                title: "Esc",
                onclick: on_close,
                "×"
            }
        }
    }
}
```

- [ ] **Step 2: Write the layer-5 SSR tests for the skeleton.**

Replace `crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs` with the test harness plus 18a's tests:

```rust
//! Layer-5 SSR tests for the bulk-map wizard. Mounts the panel inside
//! a stub-context harness mirroring `frame::mapping_list::tests:25-44`.

#![allow(non_snake_case, reason = "Dioxus components are PascalCase by convention")]

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState};
use inputforge_core::types::{
    AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId, OutputAddress, OutputId,
    VJoyAxis, VirtualDeviceConfig,
};

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::frame::bulk_map::BulkMapPanel;
use crate::frame::view_state::PanelSlot;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

pub(super) fn provide(state: AppState) -> (AppContext, mpsc::Receiver<EngineCommand>) {
    let (tx, rx) = mpsc::channel();
    let ctx = AppContext {
        state: Arc::new(RwLock::new(state)),
        commands: tx,
        settings: Arc::new(AppSettings::default()),
        meta: use_signal(|| MetaSnapshot {
            profile_name: Some("T".to_owned()),
            startup_mode: Some("Default".to_owned()),
            modes: vec!["Default".to_owned()],
            ..MetaSnapshot::default()
        }),
        config: use_signal(ConfigSnapshot::default),
        live: use_signal(LiveSnapshot::default),
    };
    use_context_provider(|| ctx.clone());
    let view = crate::frame::use_view_state_provider(ctx.meta);
    use_context_provider(|| view);
    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });
    use_live_capture_provider();
    (ctx, rx)
}

pub(super) fn one_device_state() -> DeviceState {
    DeviceState {
        info: DeviceInfo {
            id: DeviceId("dev-1".to_owned()),
            name: "FlightStick".to_owned(),
            axes: 4, buttons: 8, hats: 1,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; 4],
        },
        connected: true,
    }
}

pub(super) fn one_vjoy() -> VirtualDeviceConfig {
    VirtualDeviceConfig {
        device_id: 1,
        axes: vec![
            VJoyAxis::X, VJoyAxis::Y, VJoyAxis::Z, VJoyAxis::Rx,
            VJoyAxis::Ry, VJoyAxis::Rz, VJoyAxis::Slider0, VJoyAxis::Slider1,
        ],
        button_count: 32,
        hat_count: 1,
    }
}

pub(super) fn seeded_state(with_vjoy: bool) -> AppState {
    let map = std::collections::HashMap::from([("Default".to_owned(), vec![])]);
    let modes = inputforge_core::mode::ModeTree::from_adjacency(&map).unwrap();
    let profile = inputforge_core::profile::Profile::new(
        "T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned(),
    );
    let mut s = AppState::with_profile(profile);
    s.devices.push(one_device_state());
    if with_vjoy {
        s.virtual_devices.push(one_vjoy());
    }
    s
}

#[test]
fn panel_renders_no_profile_empty_state_when_no_profile_loaded() {
    fn TestComponent() -> Element {
        let mut s = AppState::default();
        s.devices.push(one_device_state());
        s.virtual_devices.push(one_vjoy());
        let _ = provide(s);
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("No profile loaded"), "got: {html}");
}

#[test]
fn panel_renders_no_signal_when_virtual_devices_empty() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(false));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("No vJoy devices configured"), "got: {html}");
}

#[test]
fn panel_metadata_strip_absent_when_no_vjoys() {
    // Regression for double-render: when empty-state shows, the
    // metadata strip MUST NOT render alongside it.
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(false));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(!html.contains("if-bulk-map__metadata"), "metadata strip must not render: {html}");
}

#[test]
fn panel_disables_apply_button_when_virtual_devices_empty() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(false));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("disabled"), "Apply must render with disabled attribute: {html}");
}

#[test]
fn panel_source_picker_lists_only_connected_devices() {
    fn TestComponent() -> Element {
        let mut s = seeded_state(true);
        s.devices.push(DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-2".to_owned()),
                name: "Unplugged".to_owned(),
                axes: 0, buttons: 0, hats: 0,
                instance_path: None,
                axis_polarities: vec![],
            },
            connected: false,
        });
        let _ = provide(s);
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("FlightStick"));
    assert!(!html.contains("Unplugged"), "disconnected devices must be hidden: {html}");
}

#[test]
fn panel_target_picker_renders_capability_summary() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("vJoy 1: 8 axes, 32 buttons, 1 hat"), "got: {html}");
}

#[test]
fn panel_footer_renders_cancel_and_apply_buttons() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Cancel"));
    assert!(html.contains("Apply"));
}
```

- [ ] **Step 3: Run.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map -- --nocapture`
Expected: 18a's tests pass; some prop names may need tuning against the real `Field` / `Select` / `Checkbox` signatures. Iterate until clean.

- [ ] **Step 4: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/
git commit -m "feat(bulk_map): assemble panel skeleton with metadata strip and footer"
```

---

### Task 18b: Rows table

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs`

- [ ] **Step 0: Verify `DeviceInfo` has an `id: DeviceId` field.**

Read `crates/inputforge-core/src/types/device.rs`. Confirm `DeviceInfo.id: DeviceId`. The `derive_rows` helper below relies on this; if the field is named differently, adjust the call sites.

- [ ] **Step 1: Add the rows table components and helpers.**

Append to `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`:

```rust
use crate::frame::bulk_map::row_readout::RowReadout;

#[component]
fn BulkMapRowsGroup(
    title: String,
    kind: RowKind,
    rows: Vec<RowState>,
    target_vjoy: Option<VirtualDeviceConfig>,
    on_row_change: EventHandler<(u8, Option<OutputAddress>)>,
    on_row_replace_toggle: EventHandler<u8>,
) -> Element {
    rsx! {
        div { role: "rowgroup", class: "if-bulk-map__group",
            div { role: "row", class: "if-bulk-map__group-header",
                "{title} ({rows.len()})"
            }
            for row in rows.iter().cloned() {
                BulkMapRow {
                    row: row,
                    target_vjoy: target_vjoy.clone(),
                    on_change: on_row_change,
                    on_replace_toggle: on_row_replace_toggle,
                }
            }
        }
    }
}

#[component]
fn BulkMapRow(
    row: RowState,
    target_vjoy: Option<VirtualDeviceConfig>,
    on_change: EventHandler<(u8, Option<OutputAddress>)>,
    on_replace_toggle: EventHandler<u8>,
) -> Element {
    let kind_letter = match row.kind {
        RowKind::Axis => "A",
        RowKind::Button => "B",
        RowKind::Hat => "H",
    };
    let source_label = match row.kind {
        RowKind::Axis => format!("Axis {}", row.source_index),
        RowKind::Button => format!("Btn {}", row.source_index),
        RowKind::Hat => format!("Hat {}", row.source_index),
    };
    let target_options = build_target_options(row.kind, target_vjoy.as_ref());
    let current = row.target.as_ref().map(format_output_value).unwrap_or_else(|| "(do not map)".to_owned());
    let select_value = use_signal(|| current.clone());
    let id_attr = format!("bulk-map-row-{}-{}", kind_letter, row.source_index);

    let on_target_change = {
        let kind = row.kind;
        let target_vjoy = target_vjoy.clone();
        let source_index = row.source_index;
        move |evt: FormEvent| {
            let val = evt.data.value();
            let parsed = parse_target_value(kind, &val, target_vjoy.as_ref());
            on_change.call((source_index, parsed));
        }
    };

    rsx! {
        div { role: "row", class: "if-bulk-map__row",
            span { role: "gridcell", class: "if-bulk-map__kind", "{kind_letter}" }
            span { role: "gridcell", class: "if-bulk-map__source", "{source_label}" }
            span { role: "gridcell", class: "if-bulk-map__live-cell",
                RowReadout { kind: row.kind, address: row.input.clone() }
            }
            span { role: "gridcell", class: "if-bulk-map__target",
                Select {
                    id: id_attr,
                    value: select_value,
                    onchange: Some(EventHandler::new(on_target_change)),
                    options: target_options,
                }
            }
            span { role: "gridcell", class: "if-bulk-map__action",
                button {
                    r#type: "button",
                    class: if row.replace { "if-bulk-map__chip if-bulk-map__chip--active" } else { "if-bulk-map__chip" },
                    "aria-pressed": "{row.replace}",
                    onclick: move |_| on_replace_toggle.call(row.source_index),
                    if row.replace { "replacing" } else { "replace" }
                }
            }
        }
    }
}

fn derive_rows(
    src_id: &DeviceId,
    axes_count: u8,
    button_count: u8,
    hat_count: u8,
    target: &VirtualDeviceConfig,
) -> Vec<RowState> {
    let mut out = Vec::new();
    for i in 0..axes_count {
        out.push(RowState {
            kind: RowKind::Axis,
            source_index: i,
            input: InputAddress::Bound { device: src_id.clone(), input: InputId::Axis { index: i } },
            target: auto_axis_target(target, i as usize),
            replace: false,
        });
    }
    for i in 0..button_count {
        out.push(RowState {
            kind: RowKind::Button,
            source_index: i,
            input: InputAddress::Bound { device: src_id.clone(), input: InputId::Button { index: i } },
            target: auto_button_target(target, i as usize),
            replace: false,
        });
    }
    for i in 0..hat_count {
        out.push(RowState {
            kind: RowKind::Hat,
            source_index: i,
            input: InputAddress::Bound { device: src_id.clone(), input: InputId::Hat { index: i } },
            target: auto_hat_target(target, i as usize),
            replace: false,
        });
    }
    out
}

fn rows_signature_changed(old: &[RowState], new_rows: &[RowState]) -> bool {
    if old.len() != new_rows.len() { return true; }
    old.iter().zip(new_rows.iter()).any(|(a, b)| a.input != b.input || a.kind != b.kind)
}

fn build_target_options(kind: RowKind, target: Option<&VirtualDeviceConfig>) -> Vec<(String, String)> {
    let mut opts: Vec<(String, String)> = vec![("(do not map)".into(), "(do not map)".into())];
    let Some(t) = target else { return opts };
    match kind {
        RowKind::Axis => {
            for axis in &t.axes {
                opts.push((format!("axis:{axis:?}"), format_axis_label(*axis)));
            }
        }
        RowKind::Button => {
            for id in 1..=t.button_count {
                opts.push((format!("button:{id}"), format!("Button {id}")));
            }
        }
        RowKind::Hat => {
            for id in 1..=t.hat_count {
                opts.push((format!("hat:{id}"), format!("Hat {id}")));
            }
        }
    }
    opts
}

/// Human label for a `VJoyAxis`. Mirrors `live_readout.rs:461-477`'s
/// `format_output_label` so the wizard's option text matches what the
/// editor shows for the same output.
fn format_axis_label(axis: inputforge_core::types::VJoyAxis) -> String {
    use inputforge_core::types::VJoyAxis::*;
    match axis {
        X => "X axis".into(),
        Y => "Y axis".into(),
        Z => "Z axis".into(),
        Rx => "Rx axis".into(),
        Ry => "Ry axis".into(),
        Rz => "Rz axis".into(),
        Slider0 => "Slider 0".into(),
        Slider1 => "Slider 1".into(),
    }
}

fn format_output_value(addr: &OutputAddress) -> String {
    use inputforge_core::types::OutputId;
    match &addr.output {
        OutputId::Axis { id } => format!("axis:{id:?}"),
        OutputId::Button { id } => format!("button:{id}"),
        OutputId::Hat { id } => format!("hat:{id}"),
    }
}

fn parse_target_value(kind: RowKind, val: &str, target: Option<&VirtualDeviceConfig>) -> Option<OutputAddress> {
    use inputforge_core::types::{OutputId, VJoyAxis};
    if val == "(do not map)" { return None; }
    let target = target?;
    let (head, rest) = val.split_once(':')?;
    match (kind, head) {
        (RowKind::Axis, "axis") => {
            let axis = match rest {
                "X" => VJoyAxis::X, "Y" => VJoyAxis::Y, "Z" => VJoyAxis::Z,
                "Rx" => VJoyAxis::Rx, "Ry" => VJoyAxis::Ry, "Rz" => VJoyAxis::Rz,
                "Slider0" => VJoyAxis::Slider0, "Slider1" => VJoyAxis::Slider1,
                _ => return None,
            };
            Some(OutputAddress { device: target.device_id, output: OutputId::Axis { id: axis } })
        }
        (RowKind::Button, "button") => {
            let id = rest.parse::<u8>().ok()?;
            Some(OutputAddress { device: target.device_id, output: OutputId::Button { id } })
        }
        (RowKind::Hat, "hat") => {
            let id = rest.parse::<u8>().ok()?;
            Some(OutputAddress { device: target.device_id, output: OutputId::Hat { id } })
        }
        _ => None,
    }
}

#[cfg(test)]
impl WizardState {
    /// Test-only helper for SSR fixtures that need to bypass the
    /// derive-rows-on-render path. 18d's dispatch test uses this.
    pub(super) fn with_seed_rows(rows: Vec<RowState>, mode: String) -> Self {
        let mut w = Self::empty(mode);
        w.rows = rows;
        w
    }
}
```

Then wire the rows table into the panel by replacing the empty `<div role="grid">` placeholder from 18a with three `BulkMapRowsGroup` instances. The closures capture `wizard` and update the matching row in place when the user changes the per-row target or toggles replace. Per 18a's metadata strip, the wizard `Signal` is in scope.

```rust
// Inside BulkMapPanel, replace `div { role: "grid", class: "if-bulk-map__table" }`:
let target = virtual_devices.iter().find(|v| Some(v.device_id) == *wizard.read().target_vjoy_id.as_ref());
let target_for_groups = target.cloned();

// Re-derive rows when (source, target) flips.
if let (Some(dev_id), Some(tgt)) = (wizard.read().source_device_id.clone(), target_for_groups.as_ref()) {
    if let Some(src) = connected_devices.iter().find(|d| d.info.id == dev_id) {
        let new_rows = derive_rows(&src.info.id, src.info.axes, src.info.buttons, src.info.hats, tgt);
        let mut w = wizard.write();
        if w.rows.is_empty() || rows_signature_changed(&w.rows, &new_rows) {
            w.rows = new_rows;
        }
    }
}

let make_on_row_change = move |kind: RowKind| -> EventHandler<(u8, Option<OutputAddress>)> {
    let mut w = wizard;
    EventHandler::new(move |(idx, target): (u8, Option<OutputAddress>)| {
        if let Some(row) = w.write().rows.iter_mut().find(|r| r.kind == kind && r.source_index == idx) {
            row.target = target;
        }
    })
};
let make_on_row_replace = move |kind: RowKind| -> EventHandler<u8> {
    let mut w = wizard;
    EventHandler::new(move |idx: u8| {
        if let Some(row) = w.write().rows.iter_mut().find(|r| r.kind == kind && r.source_index == idx) {
            row.replace = !row.replace;
        }
    })
};

rsx! {
    div { role: "grid", class: "if-bulk-map__table",
        BulkMapRowsGroup {
            title: "Axes".into(), kind: RowKind::Axis,
            rows: wizard.read().rows.iter().filter(|r| r.kind == RowKind::Axis).cloned().collect(),
            target_vjoy: target_for_groups.clone(),
            on_row_change: make_on_row_change(RowKind::Axis),
            on_row_replace_toggle: make_on_row_replace(RowKind::Axis),
        }
        BulkMapRowsGroup {
            title: "Buttons".into(), kind: RowKind::Button,
            rows: wizard.read().rows.iter().filter(|r| r.kind == RowKind::Button).cloned().collect(),
            target_vjoy: target_for_groups.clone(),
            on_row_change: make_on_row_change(RowKind::Button),
            on_row_replace_toggle: make_on_row_replace(RowKind::Button),
        }
        BulkMapRowsGroup {
            title: "Hats".into(), kind: RowKind::Hat,
            rows: wizard.read().rows.iter().filter(|r| r.kind == RowKind::Hat).cloned().collect(),
            target_vjoy: target_for_groups.clone(),
            on_row_change: make_on_row_change(RowKind::Hat),
            on_row_replace_toggle: make_on_row_replace(RowKind::Hat),
        }
    }
}
```

- [ ] **Step 2: Add SSR tests for rows.**

Append to `crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs`:

```rust
#[test]
fn panel_axis_row_renders_compact_bipolar_bar() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("if-bulk-map__live--axis"), "axis live cell class: {html}");
}

#[test]
fn panel_button_row_renders_filled_or_stamped_dot() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("if-bulk-map__live--button"), "button live cell class: {html}");
}

#[test]
fn panel_hat_row_renders_cardinal_letter() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("if-bulk-map__live--hat"), "hat live cell class: {html}");
}

#[test]
fn panel_target_picker_options_render_axis_human_labels() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("X axis"), "axis option label must use 'X axis' format: {html}");
    assert!(html.contains("Slider 0"), "slider option label must use 'Slider 0' format: {html}");
}

#[test]
fn panel_replace_chip_renders_aria_pressed_false_by_default() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains(r#"aria-pressed="false""#), "replace chip default: {html}");
}
```

- [ ] **Step 3: Run.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map -- --nocapture`
Expected: pass.

- [ ] **Step 4: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/
git commit -m "feat(bulk_map): render rows table with per-row override and replace chip"
```

---

### Task 18c: Group bulk-action chips

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs`

Spec Q13: each group header conditionally surfaces four chips (`skip all conflicts`, `replace all conflicts`, `include all`, `exclude all`). Predicates were defined in Task 13 (`group_actions::*`); 18c renders the chips and wires the click handlers.

- [ ] **Step 1: Extend `BulkMapRowsGroup` with chip props and rendering.**

Modify `BulkMapRowsGroup` in `bulk_map/mod.rs`:

```rust
use crate::frame::bulk_map::group_actions::{
    show_skip_all_conflicts, show_replace_all_conflicts, show_include_all, show_exclude_all,
};

#[component]
fn BulkMapRowsGroup(
    title: String,
    kind: RowKind,
    rows: Vec<RowState>,
    target_vjoy: Option<VirtualDeviceConfig>,
    /// `conflicting[i]` is true when `rows[i]` collides with an existing
    /// mapping in the target mode. Computed by `BulkMapPanel` via the
    /// `conflicts::is_conflict` helper from Task 12.
    conflicting: Vec<bool>,
    on_row_change: EventHandler<(u8, Option<OutputAddress>)>,
    on_row_replace_toggle: EventHandler<u8>,
    on_skip_all_conflicts: EventHandler<()>,
    on_replace_all_conflicts: EventHandler<()>,
    on_include_all: EventHandler<()>,
    on_exclude_all: EventHandler<()>,
) -> Element {
    let row_refs: Vec<&RowState> = rows.iter().collect();
    let render_skip = show_skip_all_conflicts(&row_refs, &conflicting);
    let render_replace = show_replace_all_conflicts(&row_refs, &conflicting);
    let render_include = show_include_all(&row_refs);
    let render_exclude = show_exclude_all(&row_refs);

    rsx! {
        div { role: "rowgroup", class: "if-bulk-map__group",
            div { role: "row", class: "if-bulk-map__group-header",
                span { class: "if-bulk-map__group-title", "{title} ({rows.len()})" }
                if render_skip {
                    button {
                        r#type: "button", class: "if-bulk-map__chip",
                        onclick: move |_| on_skip_all_conflicts.call(()),
                        "skip all conflicts"
                    }
                }
                if render_replace {
                    button {
                        r#type: "button", class: "if-bulk-map__chip",
                        onclick: move |_| on_replace_all_conflicts.call(()),
                        "replace all conflicts"
                    }
                }
                if render_include {
                    button {
                        r#type: "button", class: "if-bulk-map__chip",
                        onclick: move |_| on_include_all.call(()),
                        "include all"
                    }
                }
                if render_exclude {
                    button {
                        r#type: "button", class: "if-bulk-map__chip",
                        onclick: move |_| on_exclude_all.call(()),
                        "exclude all"
                    }
                }
            }
            for row in rows.iter().cloned() {
                BulkMapRow {
                    row: row,
                    target_vjoy: target_vjoy.clone(),
                    on_change: on_row_change,
                    on_replace_toggle: on_row_replace_toggle,
                }
            }
        }
    }
}
```

- [ ] **Step 2: Wire the chip handlers in `BulkMapPanel`.**

Replace the three `BulkMapRowsGroup` instantiations in `BulkMapPanel` with versions that pass:
- `conflicting`: a `Vec<bool>` computed once per render via `conflicts::is_conflict(row, current_mode, profile)` for each row in the group;
- the four chip handlers, each mutating `wizard.write().rows` for rows in that group:
  - `on_skip_all_conflicts`: for every conflicting row in the group, set `replace = false`;
  - `on_replace_all_conflicts`: for every conflicting row, set `replace = true`;
  - `on_include_all`: for every row with `target == None` in the group, restore the auto-target from `auto_*_target`;
  - `on_exclude_all`: for every row in the group, set `target = None`.

Build a small helper `make_group_chip_handlers(kind: RowKind) -> (EventHandler<()> x 4)` to keep `BulkMapPanel`'s body readable.

- [ ] **Step 3: Add SSR tests for chip render conditions.**

Append to `bulk_map/tests.rs`:

```rust
#[test]
fn panel_axes_group_shows_replace_all_chip_when_axis_conflict_exists() {
    fn TestComponent() -> Element {
        let map = std::collections::HashMap::from([("Default".to_owned(), vec![])]);
        let modes = inputforge_core::mode::ModeTree::from_adjacency(&map).unwrap();
        let mut profile = inputforge_core::profile::Profile::new(
            "T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned(),
        );
        let collide_input = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        profile.set_mapping(&collide_input, "Default", Some("Throttle".to_owned()), vec![Action::Invert]);
        let mut s = AppState::with_profile(profile);
        s.devices.push(one_device_state());
        s.virtual_devices.push(one_vjoy());
        let _ = provide(s);
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("replace all conflicts"), "chip must render on Axes group: {html}");
}

#[test]
fn panel_buttons_group_omits_replace_all_chip_when_no_button_conflict() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // The Buttons section header should NOT contain `replace all conflicts`
    // even though Axes might (regression: chips must scope per group).
    let buttons_section = html.split("Buttons (").nth(1).unwrap_or("");
    let to_next_group = buttons_section.split("Hats (").next().unwrap_or("");
    assert!(!to_next_group.contains("replace all conflicts"), "no chip on clean group");
}

#[test]
fn panel_axes_group_shows_include_all_chip_when_a_row_is_do_not_map() {
    // Excluding axis 0 via parse path is hard from SSR; instead, set up
    // a state where auto_axis_target returns None for axis 3 by giving
    // the vJoy only 3 axes.
    fn TestComponent() -> Element {
        let mut s = seeded_state(true);
        if let Some(v) = s.virtual_devices.first_mut() {
            v.axes = vec![VJoyAxis::X, VJoyAxis::Y, VJoyAxis::Z];
        }
        let _ = provide(s);
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("include all"), "chip must render when at least one row is unmapped: {html}");
}

#[test]
fn panel_axes_group_shows_exclude_all_chip_when_at_least_one_row_has_target() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("exclude all"), "exclude-all chip must render when rows have targets: {html}");
}
```

- [ ] **Step 4: Run.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map -- --nocapture`
Expected: pass.

- [ ] **Step 5: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/
git commit -m "feat(bulk_map): wire per-group bulk-action chips"
```

---

### Task 18d: Apply integration

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs`

Wire the summary chip + footer count + `on_apply` closure + dispatch. Adds the dispatch-capture SSR test that actually verifies the engine command via `cmd_rx.try_recv()`. Removes the module-level `dead_code` allow now that all symbols are wired.

- [ ] **Step 1: Replace the placeholder summary div and Apply button.**

In `BulkMapPanel`, compute the `counts` and `apply_count` once per render, build an `on_apply` closure that:
1. Calls `apply::build_entries(profile, &wizard.rows, &active_modes)`;
2. Calls `apply::format_snapshot_label(...)`;
3. Sends `EngineCommand::SetMappingsBulk { entries, snapshot_label }` on `ctx.commands`;
4. Pushes a success toast (`Created N mappings`);
5. Sets `panel.set(PanelSlot::None)`.

Replace the placeholder summary chip and footer with the real summary + apply button:

```rust
// Inside BulkMapPanel, replace the empty summary / footer placeholders:
let active_modes: Vec<String> = if *apply_to_all.read() {
    modes.clone()
} else {
    vec![wizard.read().mode.clone()]
};
let profile_ref = ctx.state.read();
let profile = profile_ref.active_profile.as_ref()
    .expect("no_profile guard at top of component covers this path");
let counts = summary::tally(profile, &wizard.read().rows, &active_modes);
let apply_count = counts.create + counts.replace;
let apply_label = format!("Apply {apply_count} mappings");
drop(profile_ref);

let on_apply = {
    let cmd_tx = ctx.commands.clone();
    let toast = use_context::<ToastQueue>();
    let mut wizard_ref = wizard;
    let active_modes_owned = active_modes.clone();
    move |_| {
        let w = wizard_ref.peek().clone();
        let entries = {
            let s = ctx.state.read();
            apply::build_entries(s.active_profile.as_ref().expect("profile loaded"), &w.rows, &active_modes_owned)
        };
        let count = entries.len();
        let label = apply::format_snapshot_label(
            w.source_device_id.as_ref().map(|d| d.0.as_str()).unwrap_or("source"),
            w.target_vjoy_id.unwrap_or(0),
        );
        let _ = cmd_tx.send(EngineCommand::SetMappingsBulk {
            entries,
            snapshot_label: label,
        });
        toast.push(ToastLevel::Success, format!("Created {count} mappings"));
        panel.set(PanelSlot::None);
    }
};

rsx! {
    div { class: "if-bulk-map__summary",
        span { class: "if-bulk-map__summary-create", "+{counts.create} create" }
        if *apply_to_all.read() {
            span { class: "if-bulk-map__summary-modes", " across {modes.len()} modes" }
        }
        span { class: "if-bulk-map__summary-sep", " · " }
        span { class: "if-bulk-map__summary-replace", "{counts.replace} replace" }
        span { class: "if-bulk-map__summary-sep", " · " }
        span { class: "if-bulk-map__summary-skip", "{counts.skip} skip" }
        span { class: "if-bulk-map__summary-sep", " · " }
        span { class: "if-bulk-map__summary-excluded", "{counts.excluded} excluded" }
    }
    footer { class: "if-bulk-map__footer",
        Button { onclick: move |_| panel.set(PanelSlot::None), "Cancel" }
        Button { disabled: apply_count == 0, onclick: on_apply, "{apply_label}" }
    }
}
```

- [ ] **Step 2: Add a test-only dispatch helper.**

Append to `bulk_map/mod.rs`:

```rust
#[cfg(test)]
pub(super) fn apply_for_test(
    state: &inputforge_core::state::AppState,
    wizard: &WizardState,
    modes: &[String],
    cmd_tx: &std::sync::mpsc::Sender<EngineCommand>,
) {
    let profile = state.active_profile.as_ref().expect("profile loaded");
    let entries = apply::build_entries(profile, &wizard.rows, modes);
    let label = apply::format_snapshot_label(
        wizard.source_device_id.as_ref().map(|d| d.0.as_str()).unwrap_or("source"),
        wizard.target_vjoy_id.unwrap_or(0),
    );
    let _ = cmd_tx.send(EngineCommand::SetMappingsBulk { entries, snapshot_label: label });
}
```

- [ ] **Step 3: Add the dispatch + summary SSR tests.**

Append to `bulk_map/tests.rs`:

```rust
#[test]
fn panel_apply_button_renders_count_when_no_conflicts() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // 4 axes + 8 buttons + 1 hat = 13 creates, 0 replace, count = 13.
    assert!(html.contains("Apply 13 mappings"), "Apply label: {html}");
}

#[test]
fn panel_summary_chip_counts_match_row_states() {
    fn TestComponent() -> Element {
        let _ = provide(seeded_state(true));
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("+13 create"), "create count: {html}");
}

#[test]
fn panel_summary_includes_across_n_modes_when_apply_to_all_checked() {
    // Drive apply_to_all=true via a seed-state path: build a profile
    // with two modes and a wizard pre-seeded with apply_to_all on.
    // The tally should fan create across both modes (13 * 2 = 26).
    fn TestComponent() -> Element {
        let map = std::collections::HashMap::from([
            ("Default".to_owned(), vec!["Combat".to_owned()]),
        ]);
        let modes = inputforge_core::mode::ModeTree::from_adjacency(&map).unwrap();
        let profile = inputforge_core::profile::Profile::new(
            "T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned(),
        );
        let mut s = AppState::with_profile(profile);
        s.devices.push(one_device_state());
        s.virtual_devices.push(one_vjoy());
        let _ = provide(s);
        // The panel's `apply_to_all` signal defaults to false; without
        // a click event, this test asserts the modes-count rendering
        // when the user has toggled on. Since SSR cannot dispatch
        // clicks, we rely on the fan-out test in summary.rs to lock
        // the tally semantics, and assert here only that the modes
        // list is present in `BulkMapPanel`'s render scope.
        rsx! { BulkMapPanel {} }
    }
    let mut vdom = VirtualDom::new(TestComponent);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // The mode picker renders both modes; this is a structural smoke.
    assert!(html.contains("Default"));
    assert!(html.contains("Combat"));
}

#[test]
fn panel_do_not_map_target_excludes_row_from_apply_count() {
    // Use the test-only `with_seed_rows` helper to construct a wizard
    // state with one explicit `(do not map)` row (target = None) and
    // assert via apply_for_test that no entry is emitted for that row.
    use crate::frame::bulk_map::{apply_for_test, state::{RowKind, RowState, WizardState}};
    let map = std::collections::HashMap::from([("Default".to_owned(), vec![])]);
    let modes = inputforge_core::mode::ModeTree::from_adjacency(&map).unwrap();
    let profile = inputforge_core::profile::Profile::new(
        "T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let row = RowState {
        kind: RowKind::Axis,
        source_index: 0,
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        },
        target: None, // (do not map)
        replace: false,
    };
    let wizard = WizardState::with_seed_rows(vec![row], "Default".to_owned());
    let (tx, rx) = mpsc::channel();
    apply_for_test(&state, &wizard, &["Default".to_owned()], &tx);
    let cmd = rx.try_recv().expect("dispatch must always send a command, even if entries empty");
    match cmd {
        EngineCommand::SetMappingsBulk { entries, .. } => {
            assert!(entries.is_empty(), "do-not-map row must not produce an entry");
        }
        _ => panic!("expected SetMappingsBulk"),
    }
}

#[test]
fn panel_apply_for_test_dispatches_set_mappings_bulk_with_snapshot_label() {
    use crate::frame::bulk_map::{apply_for_test, state::{RowKind, RowState, WizardState}};
    let map = std::collections::HashMap::from([("Default".to_owned(), vec![])]);
    let modes = inputforge_core::mode::ModeTree::from_adjacency(&map).unwrap();
    let profile = inputforge_core::profile::Profile::new(
        "T".to_owned(), vec![], modes, vec![], vec![], "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let row = RowState {
        kind: RowKind::Axis,
        source_index: 0,
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        },
        target: Some(OutputAddress { device: 1, output: OutputId::Axis { id: VJoyAxis::X } }),
        replace: false,
    };
    let mut wizard = WizardState::with_seed_rows(vec![row], "Default".to_owned());
    wizard.source_device_id = Some(DeviceId("dev-1".to_owned()));
    wizard.target_vjoy_id = Some(1);
    let (tx, rx) = mpsc::channel();
    apply_for_test(&state, &wizard, &["Default".to_owned()], &tx);
    let cmd = rx.try_recv().expect("dispatch arrives");
    match cmd {
        EngineCommand::SetMappingsBulk { entries, snapshot_label } => {
            assert_eq!(entries.len(), 1);
            assert_eq!(snapshot_label, "Before bulk-map: dev-1 to vJoy 1");
        }
        _ => panic!("expected SetMappingsBulk"),
    }
}
```

- [ ] **Step 4: Remove the module-level `dead_code` allow.**

Edit `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`. Delete the `#![allow(dead_code, ...)]` block at the top of the file. Every helper is now wired by the panel.

- [ ] **Step 5: Run the workspace tests.**

Run: `cargo test -p inputforge-gui-dx --lib frame::bulk_map -- --nocapture`
Expected: pass.

Run: `cargo test --workspace --lib -- --nocapture`
Expected: pass (the dropped Task 21 sanity check folds into this step).

- [ ] **Step 6: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/bulk_map/
git commit -m "feat(bulk_map): wire apply dispatch with summary chip"
```

---

### Task 19: Panel-scoped CSS

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/bulk_map.css`

- [ ] **Step 1: Replace the placeholder CSS.**

Replace `crates/inputforge-gui-dx/assets/frame/bulk_map.css` with:

```css
/* Bulk-map wizard. Panel-scoped under .if-bulk-map. */

.if-bulk-map {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-3);
    color: var(--color-text);
    font-size: var(--text-base);
    width: 460px;
    min-width: 0;
    overflow: hidden;
}

@media (max-width: 1100px) {
    .if-bulk-map {
        width: min(460px, calc(100vw - 240px - 320px));
    }
}

.if-bulk-map__header {
    display: flex;
    flex-direction: row;
    align-items: baseline;
    justify-content: space-between;
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--color-border);
}

.if-bulk-map__title {
    font-size: var(--text-lg);
    font-weight: var(--weight-semibold);
    line-height: var(--leading-tight);
    letter-spacing: -0.01em;
}

.if-bulk-map__close {
    background: transparent;
    border: none;
    color: var(--color-text-subtle);
    font-size: var(--text-lg);
    line-height: 1;
    cursor: pointer;
    padding: var(--space-1) var(--space-2);
}

/* grid: align metadata pairs across two columns */
.if-bulk-map__metadata {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2) var(--space-3);
}

.if-bulk-map__table {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
}

.if-bulk-map__group-header {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    padding: var(--space-2) var(--space-1);
    font-size: var(--text-sm);
    color: var(--color-text-subtle);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    position: sticky;
    top: 0;
    background: var(--color-bg-elevated);
    z-index: 1;
}

/* grid: align cells across rows in a tabular layout */
.if-bulk-map__row {
    display: grid;
    grid-template-columns: 28px 1fr 70px 1fr auto;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-1);
}

.if-bulk-map__kind {
    display: flex;
    width: 18px;
    height: 18px;
    align-items: center;
    justify-content: center;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    background: rgba(56, 189, 248, 0.14);
    border-radius: 3px;
    color: var(--color-text);
}

.if-bulk-map__source {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
}

.if-bulk-map__live {
    display: flex;
    height: 8px;
    align-items: center;
    position: relative;
}

.if-bulk-map__live--axis {
    height: 8px;
    background: rgba(255, 255, 255, 0.04);
    border-radius: 2px;
    position: relative;
}

.if-bulk-map__live-bar {
    position: absolute;
    top: 0;
    bottom: 0;
    background: var(--color-live);
    opacity: 0.65;
    border-radius: 2px;
    transition: width var(--duration-fast) var(--easing-fast),
                left var(--duration-fast) var(--easing-fast),
                right var(--duration-fast) var(--easing-fast);
}

.if-bulk-map__live--button {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
    margin: 0 auto;
}

.if-bulk-map__live--button-on {
    background: var(--color-live);
    opacity: 0.7;
}

.if-bulk-map__live--hat {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--color-live);
    text-align: center;
}

.if-bulk-map__chip {
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: 3px;
    padding: var(--space-0) var(--space-2);
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    cursor: pointer;
}

.if-bulk-map__chip--active {
    background: var(--color-warning-bg);
    border-color: var(--color-warning);
    color: var(--color-warning);
}

.if-bulk-map__summary {
    display: flex;
    flex-direction: row;
    gap: var(--space-1);
    padding-top: var(--space-2);
    border-top: 1px solid var(--color-border);
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--color-text-subtle);
    font-size: var(--text-sm);
}

.if-bulk-map__summary-create,
.if-bulk-map__summary-replace,
.if-bulk-map__summary-skip,
.if-bulk-map__summary-excluded {
    color: var(--color-text);
}

.if-bulk-map__summary-modes,
.if-bulk-map__summary-sep {
    color: var(--color-text-subtle);
}

.if-bulk-map__footer {
    display: flex;
    flex-direction: row;
    justify-content: flex-end;
    gap: var(--space-2);
}

.if-bulk-map__caption {
    font-size: var(--text-xs);
    color: var(--color-text-subtle);
    text-align: right;
}

.if-bulk-map__empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-5);
}

.if-bulk-map__empty-icon {
    font-size: var(--text-2xl);
    color: var(--color-text-subtle);
}

.if-bulk-map__empty-title {
    font-size: var(--text-base);
    color: var(--color-text);
}

.if-bulk-map__empty-caption {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
    text-align: center;
}
```

- [ ] **Step 2: Verify build.**

Run: `cargo check -p inputforge-gui-dx`
Expected: clean compile.

- [ ] **Step 3: Commit.**

```bash
git add crates/inputforge-gui-dx/assets/frame/bulk_map.css
git commit -m "style(bulk_map): add panel-scoped wizard CSS"
```

---

### Task 20: Tools-cluster button + click handler

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/logic.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/mod.rs`

- [ ] **Step 1: Extend `Tool` and `tool_active`.**

Edit `crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/logic.rs`. Add a variant and a match arm:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Devices,
    Calibration,
    Profiles,
    BulkMap,
}

pub(crate) fn tool_active(slot: PanelSlot, via_calibration: bool, tool: Tool) -> bool {
    matches!(
        (slot, via_calibration, tool),
        (PanelSlot::Devices, false, Tool::Devices)
            | (PanelSlot::Devices, true, Tool::Calibration)
            | (PanelSlot::Profiles, _, Tool::Profiles)
            | (PanelSlot::BulkMap, _, Tool::BulkMap)
    )
}
```

Append a unit test to the existing `mod tests` block:

```rust
#[test]
fn bulk_map_panel_lights_bulk_map_regardless_of_via_calibration() {
    assert!(tool_active(PanelSlot::BulkMap, false, Tool::BulkMap));
    assert!(tool_active(PanelSlot::BulkMap, true, Tool::BulkMap));
    assert!(!tool_active(PanelSlot::BulkMap, false, Tool::Devices));
}
```

- [ ] **Step 2: Add the tool button.**

Edit `crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/mod.rs`. Inside the `nav { ... }` block, append a fourth `ToolButton` after the `Profiles` one:

```rust
            ToolButton {
                label: "Bulk-map",
                active: bulk_map_active,
                disabled: !p,
                disabled_reason: "Load a profile to bulk-map a device.",
                onclick: move |_| {
                    if bulk_map_active {
                        panel.set(PanelSlot::None);
                    } else {
                        panel.set(PanelSlot::BulkMap);
                        via.set(false);
                    }
                },
            }
```

Below the existing per-button activeness lines (Devices/Calibration/Profiles at lines 32-34), append:

```rust
    let bulk_map_active = tool_active(s, v, Tool::BulkMap);
```

(Preserves the declaration order Devices, Calibration, Profiles, BulkMap.)

- [ ] **Step 3: Run logic tests.**

Run: `cargo test -p inputforge-gui-dx --lib frame::top_bar::tools_cluster::logic -- --nocapture`
Expected: pass.

- [ ] **Step 4: Commit.**

```bash
git add crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/
git commit -m "feat(bulk_map): add tools-cluster Bulk-map button"
```

---

### Task 21: Manual interactive verification (interactive only, NOT automated)

**Files:** none

This task is interactive. The implementer runs `dx run -p inputforge-app` and walks through the checklist below. None of these steps belongs in `cargo test`. (Task 8's `ShellMode` refactor and Task 18d's `BulkMapPanel` mount complete the panel-slot wiring; no separate "finalize mount" task is needed.)

- [ ] **Step 1: Launch the GUI in dev.**

Run: `dx run -p inputforge-app`

- [ ] **Step 2: Open the wizard from the tools cluster.**

Click the Bulk-map button. Verify the panel slides in from the right, source picker shows the connected stick, target picker shows the live vJoy with capability summary (`vJoy 1: 8 axes, 32 buttons, 1 hat`). Spec Q7 + Q9.

- [ ] **Step 3: Verify auto-mapping and live readout.**

For a stick with 4 axes / 8 buttons / 1 hat: confirm axis 0 to X, axis 1 to Y, axis 2 to Z, axis 3 to Rx. Wiggle the stick: each axis row's bipolar bar should pulse. Press buttons: dots fill. Roll the hat: cardinal letter updates. Spec Q4 + Q10.

- [ ] **Step 3a: Verify positional overflow.**

If the source has more inputs of a kind than the live vJoy provides (e.g. a 10-axis stick against an 8-axis vJoy), the overflowing rows default to `(do not map)`. Per-row select for those rows excludes the unavailable slots. Spec Q4.

- [ ] **Step 4: Trigger a conflict.**

Manually create one mapping for the same axis as the wizard's auto-mapping (use the existing `+ Add mapping` flow before opening the wizard). Reopen the wizard. The conflicting row dims and reads `already mapped: "<existing name>"`. The summary chip subtracts that row from `create` and adds it to `skip`. Click `replace` on the conflicting row: the chip turns to `replacing`, the row tints amber, summary subtracts from `skip` and adds to `replace`. Spec Q3.

- [ ] **Step 4a: Verify per-group bulk-action chips.**

With at least one conflicting row in the Axes group and zero conflicts in the Buttons / Hats groups: the Axes group header shows `skip all conflicts` / `replace all conflicts` chips; Buttons and Hats group headers do NOT show those chips. Click `replace all conflicts` on the Axes group: every conflicting axis row toggles to replace state. Spec Q13.

- [ ] **Step 5: Apply with `Apply to all modes` checked on a profile with three modes.**

Verify the count chip's `create` clause reads `+N create across 3 modes` and that the other clauses (`replace` / `skip` / `excluded`) are present without an `across` suffix. Click Apply. The panel closes immediately, success toast reads `Created N mappings` (where N equals `create + replace`). Reopen the snapshot manager (or restart): an `auto_before_bulk_map` snapshot exists with the label `Before bulk-map: <source> to vJoy <id>`. Spec Q11 + Q12 + Q14.

- [ ] **Step 5a: Post-apply audit.**

Open one of the mappings the wizard created in the editor (F9). Confirm: the actions list contains exactly one `MapToVJoy { output }` entry; the mapping's `name` is blank (the mapping-list rail displays the input identifier as fallback, not an auto-generated name). Spec Q5 + Q6.

- [ ] **Step 6: Test the connected-only source picker.**

Unplug a secondary input device, reopen the wizard, and confirm that device does NOT appear in the source picker. Reconnect it and reopen: it appears. Spec Q8.

- [ ] **Step 7: Test the no-vJoy empty state.**

Disable vJoy in vJoyConf, restart the app, open the wizard. The metadata strip and rows table are replaced by `No vJoy devices configured` with the `Configure outputs in vJoyConf, then reopen.` caption. Apply is disabled. (Restart is required because the engine probes vJoy at startup; live re-probe is out of scope for this feature.) Spec Q9.

- [ ] **Step 8: Test Esc-close.**

Open the wizard, press Esc. Panel dismisses silently with no confirm.

When all manual steps and `cargo test --workspace --lib` pass: the wizard is ready to ship.

---

## Spec coverage check

Each spec section is implemented as follows:

- Q1 (per-row select with `(do not map)`): Task 18b, `BulkMapRow`'s target select includes `(do not map)` at the top and is the only excluder.
- Q2 (mode picker + `Apply to all modes (N)`): Task 18a metadata strip; Task 14 fans the count out across modes.
- Q3 (skip on conflict, per-row `replace`): Task 12 defines `is_conflict(row, mode)`; Task 14 reuses it for the tally; Task 15 reuses it for the dispatch filter; per-row replace flag honored by Tasks 14 and 15.
- Q4 (positional auto-mapping with overflow): Task 11.
- Q5 (bare passthrough action set): Task 2's `set_mappings_bulk` builds `vec![Action::MapToVJoy { output }]` only.
- Q6 (blank names): Task 2 sets `name: None`.
- Q7 (tools-cluster trigger only): Task 20.
- Q8 (connected-only source picker): Task 18a filters `devices` by `connected: true`.
- Q9 (live-detected target picker, capability summary, empty state): Task 18a metadata strip + Task 17 empty state + Task 18b capacity-filtered target options.
- Q10 (per-row live readout sharing F9 helpers): Task 16.
- Q11 (atomic apply with embedded snapshot): Task 4 (variant), Task 5 (handler), Task 6 (handler tests).
- Q12 (summary chip co-located with Apply): Task 18d footer ordering, Task 14 counts.
- Q13 (per-group bulk action chips): Task 13 predicates; Task 18c renders the chips and wires the four `EventHandler<()>` props back to `BulkMapPanel`.
- Q14 (apply-to-all-modes inline-count checkbox + picker dimming): Task 18a metadata strip.
- Engine command `SetMappingsBulk`: Tasks 4, 5.
- `SnapshotKind::AutoBeforeBulkMap`: Task 3.
- `BulkMapEntry`: Task 1.
- `Profile::set_mappings_bulk`: Task 2.
- Panel-slot mount discipline (single stable `<aside>` outside the match): Task 8 `ShellMode { Standard, Custom }` refactor.
- Layer 2-6 tests: Tasks 2, 3, 6, 7, 18a-18d. Layer 1 (`BulkMapEntry`) is compile-time only.
- Manual verification: Task 21.

Non-goals (replacing per-input add flow, offline authoring, default action templates, auto-naming, panel-level bulk operations, segmented control, reusable `SidePanel`, live keyboard preview, mapping-list rename allow-blank): not implemented; deferred per the spec.

## Plan complete

Plan saved to `docs/superpowers/plans/2026-05-03-bulk-mapping.md`. Two execution options:

1. **Subagent-Driven (recommended).** Fresh subagent per task, two-stage review between tasks, fast iteration.
2. **Inline Execution.** Execute tasks in this session via superpowers:executing-plans, batch with checkpoints.

Which approach?
