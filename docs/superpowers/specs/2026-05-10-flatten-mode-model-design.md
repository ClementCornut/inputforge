# Flatten the Mode Model: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-05-10
**Predecessor specs:**
- [`2026-05-08-f14-mode-editing-design.md`](./2026-05-08-f14-mode-editing-design.md), F14 already removed `Cycle`/`Previous` strategies that needed inheritance to be useful; this spec finishes the same train of thought by removing the tree itself.
- [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md), F5 scoped mode CRUD into the F7 chrome.

**Related (deferred):** "Duplicate Mode" (the original ask that triggered this brainstorm) is held for a follow-up spec authored against the flat model produced here.

---

## Context

The mode model in `crates/inputforge-core/src/mode/mod.rs` is a tree (`ModeTree` of `ModeNode { name, children }`) with parent-child inheritance: `resolve_mapping` at `crates/inputforge-core/src/mode/resolve.rs:16` walks `tree.ancestors(mode)` and returns the first matching mapping found in any ancestor. The TOML serialization is a flat adjacency map (`Default = ["Combat", "Landing"]`).

The GUI never exercises depth >= 2. `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/add_inline.rs:49` always dispatches `EngineCommand::AddMode { name, parent: None }`, so every user-created mode lands as a direct child of the profile root. The runtime tree shape matches what `MetaSnapshot.modes: Vec<String>` already exposes flat.

F14 removed `ModeChangeStrategy::Cycle` and `ModeChangeStrategy::Previous`, the two strategies that benefited materially from inheritance. With those gone, the tree's only remaining behavioral contribution is per-input fallback through ancestors, which the GUI cannot author. The data shape is paying carrying costs (custom `Serialize`/`Deserialize`, `from_adjacency`, `descendants_of`, `ancestors`, `with_added_child`, the F4 "subtree contains startup" rule, mode-tree resolve tests) for a feature that nothing currently surfaces.

This spec collapses `ModeTree` into a flat ordered list. The change is a deliberate breaking edit: the app is not yet distributed and the user has no profile files to migrate beyond their own (which they will convert by hand).

---

## Confirmed design choices

### Data model

**1. `ModeTree` and `ModeNode` go away. New type `Modes(Vec<String>)`.** A newtype wrapper, not a bare `Vec<String>` on `Profile`, so invariants stay enforced at one type boundary. `crates/inputforge-core/src/mode/mod.rs` is rewritten to expose only `Modes` plus the methods the rest of the codebase actually needs:

```rust
pub struct Modes(Vec<String>);

impl Modes {
    pub fn new(names: Vec<String>) -> Result<Self>;
    pub fn as_slice(&self) -> &[String];
    pub fn first(&self) -> &str;
    pub fn len(&self) -> usize;
    pub fn contains(&self, name: &str) -> bool;
    pub fn with_appended(&self, name: &str) -> Result<Self>;
    pub fn with_renamed(&self, from: &str, to: &str) -> Result<Self>;
    pub fn with_removed(&self, name: &str) -> Result<Self>;
}
```

The inner `Vec<String>` is private. `with_appended` returns `Err(InvalidConfig)` on duplicate; `with_renamed` returns `Err(ModeNotFound)` if `from` is absent and `Err(InvalidConfig)` if `to` collides with an existing different name; `with_removed` returns `Err(ModeNotFound)` when absent and `Err(InvalidConfig)` when removing the last element.

**2. Invariants enforced by `Modes::new`.**
- non-empty
- all names unique (case-sensitive, matches the existing `validate_mode_name` policy)

There is no separate "root" concept. `first()` is the implicit positional first-tab.

**3. `mode/resolve.rs` is deleted.** With ancestor inheritance gone, mapping resolution collapses to one direct lookup. Every former caller becomes:

```rust
mappings.iter().find(|m| m.input == *input && m.mode == *mode)
```

**4. `mode/state.rs` keeps the temporary-mode stack.** `ModeState::switch_to`, `push_temporary`, `pop_temporary`, and `EngineError::ModeCycleDetected` are runtime mechanics that are independent of the tree. The single change is parameter type: every `&ModeTree` arg becomes `&Modes`, and every `tree.contains(name)` becomes `modes.contains(name)`. Behavior, error variants, and the auto-release Hold lifecycle are all preserved. F14's Hold flow does not regress.

**5. `Profile` storage flips.** `crates/inputforge-core/src/profile/mod.rs` carries `modes: ModeTree` in two places (`Profile` struct and the deserialize-helper struct around line 91), with `Profile::modes() -> &ModeTree` accessor at line 191 and `Profile::set_modes(&mut self, ModeTree)` setter at line 373. All four flip to `Modes`. The default profile that `profile/manager.rs` constructs becomes `Modes::new(vec!["Default".to_owned()])` (single-element).

### TOML schema, hard cut

**6. New on-disk shape.** Profiles emit `modes` as a top-level flat list of strings:

```toml
modes = ["Default", "Combat", "Landing"]
startup_mode = "Default"
```

`startup_mode` semantics are unchanged: separate `Option<String>` field, validated against `Modes::contains` at command time, independent of `Modes::first()`.

**7. Deserializer rejects the legacy adjacency-map shape.** `Modes`'s `Deserialize` impl reads only the flat list. Encountering the old shape (`[modes]` table with parent->children entries) fails fast. The error message is single-canonical:

> `Profile uses the legacy nested-modes schema, which is no longer supported. Convert the [modes] table to a flat list ('modes = ["Default", "Combat", ...]') and ensure 'startup_mode' is set.`

This message is referenced verbatim from both the serde error path and any wrapping error the engine returns when surfacing the failure to the warnings channel. A test pins the message text to catch future drift.

**8. No migration code. No version detection. No compatibility shim.** The user converts their local profile files by hand. The app is pre-distribution; verifying that no committed test fixtures or sample profiles in the repo carry the legacy shape is part of the implementation acceptance.

### Engine commands

**9. `EngineCommand::AddMode` drops `parent`.** New shape:

```rust
AddMode { name: String }
```

Engine handler at `crates/inputforge-core/src/engine/run.rs:684`: validate name (`validate_mode_name_for_engine` shared helper, unchanged), then `modes = modes.with_appended(&name)?` and persist. New modes always land at the tail of the list. The GUI's call site at `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/add_inline.rs:49` already passed `parent: None`, so the dispatch site only loses a struct field.

**10. `EngineCommand::DeleteMode` drops cascade semantics.** Engine handler at `crates/inputforge-core/src/engine/run.rs` (the `DeleteMode` arm around line 811) no longer calls `descendants_of`, no longer iterates a deleted set. It removes the named mode from `Modes` (via `with_removed`) and drops every mapping whose `mode` field equals that single name. Validation rules:
- Reject deletion of `modes.first()` (the first tab in the list).
- Reject deletion of `name == startup_mode`.

The third historical rule, "subtree contains startup," is removed. The "first tab" rule is preserved purely as a positional invariant: the GUI assumes `modes.first()` exists, and `Modes::new` enforces non-empty, so deleting the only mode would violate that invariant.

**11. `EngineCommand::RenameMode { from, to }` and `SetDefaultMode { name }` keep their external contracts.** Internal validation switches from `ModeTree` to `Modes`, which is `contains(&str)` either way. `RenameMode`'s cascade across mappings, action graphs, and `startup_mode` is unchanged.

### GUI

**12. `delete_disabled_for_tab` simplifies.** `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/logic.rs::delete_disabled_for_tab` loses its `descendants: &[String]` parameter. New signature: `delete_disabled_for_tab(name: &str, modes: &[String], startup: Option<&str>) -> bool`. New rule: disabled iff `name == modes[0]` OR `Some(name) == startup`. The unit tests for "subtree contains startup" are deleted.

**13. Both call sites in `mode_tabs/mod.rs` drop their descendants lookups.** Two blocks resolve `descendants` today: the keyboard-Delete arm around line 195 and the context-menu flags computation around line 285. Both blocks are removed in full. The two surrounding `Option<String>` reads (`startup` from `meta`) and the `modes_now`/`modes_for_flags` snapshot reads stay; only the descendants computation goes. The keyboard-Delete arm around line 195 still gates on `delete_disabled_for_tab`, which now takes 3 args instead of 4; both call sites update accordingly.

**14. `ContextMenuFlags` keeps its 4 fields.** `delete_disabled` is still a `bool` on `ContextMenuFlags`; only the parent computation that fills it changes. `flags_for` in the test module at `mode_tabs/context_menu.rs` loses its `descendants_of_name_contains_startup: bool` parameter. The `delete_disabled_when_subtree_contains_startup` test is deleted.

**15. `add_inline.rs` dispatch site.** The struct literal on line 49 changes from `EngineCommand::AddMode { name: name.clone(), parent: None }` to `EngineCommand::AddMode { name: name.clone() }`. No other GUI logic changes in this file.

### Out of scope

- **Duplicate Mode.** The original ask that triggered this brainstorm. Held for a follow-up spec authored against the flat model produced here. Not implemented in this cycle.
- **Profile migration tooling.** Not building a converter, not building a `--migrate` flag, not building a one-shot helper script. The user converts files by hand.
- **`MetaSnapshot.modes` shape.** Already `Vec<String>` in `crates/inputforge-gui-dx/src/context.rs:93`. No change.
- **Per-mode mapping storage.** Mappings stay keyed by `(InputAddress, mode: String)` exactly as today. No change.
- **Mode-temporary-stack semantics.** `ModeState`'s push/pop, `ReleaseCallback::PopTemporaryMode`, `EngineError::ModeCycleDetected`, F14's Hold flow. All preserved unchanged.
- **Renaming `EngineCommand::AddMode` etc.** Variant names stay; only the `parent` field is removed.

---

## Touchpoint inventory

This list is authoritative for the implementation plan's removal/rewrite gates.

### Core (`crates/inputforge-core/`)

| File | Change |
| --- | --- |
| `src/mode/mod.rs` | Rewrite. Drop `ModeTree`, `ModeNode`, `from_adjacency`, `with_added_child`, `descendants_of`, `ancestors`, `find_mode`, `all_modes`, custom `Serialize`/`Deserialize`. Add `Modes` newtype with the API in choice 1. New `Deserialize` impl reads flat list only and emits the canonical error. |
| `src/mode/resolve.rs` | **Delete file.** Remove `mod resolve;` and `pub use resolve::resolve_mapping;` from `mode/mod.rs`. |
| `src/mode/state.rs` | Type swap: `&ModeTree` -> `&Modes`. Behavior preserved. |
| `src/engine/command.rs` | `AddMode { name, parent: Option<String> }` -> `AddMode { name: String }`. Doc comment updated. |
| `src/engine/run.rs` | `AddMode` arm: drop `parent` read, call `modes.with_appended(&name)`. `DeleteMode` arm: drop `descendants_of` call and the cascade loop, remove only the named mode and its mappings. `RenameMode` arm: switch tree validation to `Modes`. `SwitchMode` arm: pass `&Modes` to `ModeState::switch_to`. Any `Action::ChangeMode` output handler that hands `&ModeTree` to `ModeState`'s push/pop now hands `&Modes`. |
| `src/profile/mod.rs` | `Profile.modes` and the deserialize-helper field (line 71 and line 91), `Profile::modes()` accessor (line 191), `Profile::set_modes` setter (line 373): all flip from `ModeTree` to `Modes`. |
| `src/profile/manager.rs` | Default-profile constructor and any other `ModeTree::from_adjacency` site: switch to `Modes::new(vec![...])`. |

### GUI (`crates/inputforge-gui-dx/`)

| File | Change |
| --- | --- |
| `src/frame/top_bar/mode_tabs/logic.rs` | `delete_disabled_for_tab` loses `descendants` param. Tests for "subtree contains startup" deleted. |
| `src/frame/top_bar/mode_tabs/mod.rs` | Drop both `descendants_of` lookup blocks (keyboard-Delete arm ~line 195, context-menu flags ~line 285). Update `delete_disabled_for_tab` call sites. |
| `src/frame/top_bar/mode_tabs/context_menu.rs` | Test helper `flags_for` loses `descendants_of_name_contains_startup` param. `delete_disabled_when_subtree_contains_startup` test deleted. Production `ContextMenuFlags` shape unchanged. |
| `src/frame/top_bar/mode_tabs/add_inline.rs` | Line 49: drop `parent: None` from the `AddMode` struct literal. |

Module-level deletions: `mode/resolve.rs` only.

---

## Tests

### Deleted

- `crates/inputforge-core/src/mode/mod.rs::tests` – every test in `from_adjacency_*`, `descendants_of_*`, `ancestors_*`, `with_added_child_*`, the multiple-roots and unreachable-modes tests, the `test_tree` helper. Anything that asserted tree shape goes.
- `crates/inputforge-core/src/mode/resolve.rs::tests` – removed with the file. The "child falls through to parent's mapping" family disappears entirely.
- `crates/inputforge-core/src/engine/tests.rs::add_mode_under_named_parent` – the `parent` field is gone.
- `crates/inputforge-core/src/engine/tests.rs::add_mode_rejects_unknown_parent` – no parent to be unknown.
- `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs::tests::delete_disabled_when_subtree_contains_startup`.

### Rewritten

- `crates/inputforge-core/src/mode/state.rs::tests` – same coverage; type swap from `ModeTree` fixtures to `Modes::new(vec![...])` fixtures.
- `crates/inputforge-core/src/engine/tests.rs::add_mode_appends_under_root_by_default` – semantically renamed to `add_mode_appends_to_modes_list`. The "under root" framing has no meaning with no tree; the assertion is "new mode lands at tail."
- `crates/inputforge-core/src/engine/tests.rs` `DeleteMode` family – drop "deletes whole subtree" assertions; keep "rejects deleting first mode," "rejects deleting startup mode," "drops mappings keyed to that mode."
- Every GUI test that built fixtures via `ModeTree::from_adjacency(...)`: switch to `Modes::new(vec![...])`.

### Added

- `Modes::new` invariants: empty list rejected, duplicate names rejected.
- `Modes` round-trip: a valid flat list TOML serializes and parses back identically.
- `Modes` deserializer rejects the legacy adjacency-map shape with the canonical error message. The assertion pins the message text verbatim so future drift is caught.
- Engine integration: load a profile fixture written in the legacy shape, assert load fails with the migration error surfaced through the warnings channel.

---

## Acceptance gates

The implementation plan converts these into per-step checkboxes.

1. **Grep gate, removal proof.** No occurrence of `ModeTree`, `ModeNode`, `from_adjacency`, `descendants_of`, `ancestors`, `with_added_child`, `find_mode`, `all_modes`, or `resolve_mapping` survives in production code (test fixtures included). The single legitimate string match for `descendants_of` is the deletion entry in this spec or its successor plan.
2. **Single-file removal.** `crates/inputforge-core/src/mode/resolve.rs` no longer exists.
3. **`AddMode` shape.** `EngineCommand::AddMode` has exactly one field, `name: String`. No `parent` anywhere in the workspace.
4. **TOML round-trip.** A profile authored with `modes = ["A", "B", "C"]` and `startup_mode = "A"` saves and loads byte-identically (modulo `toml`'s deterministic key ordering for the rest of the profile).
5. **Legacy-shape rejection.** A profile with the old `[modes]\nDefault = ["Combat"]` shape fails to load with the canonical error message. The message is asserted verbatim in a test.
6. **F14 Hold preserved.** The Hold/Set strategy editor and the auto-release lifecycle are unchanged. The `mode_change` action editor's existing tests pass without modification.
7. **F4 delete dialog.** Deleting the first mode is still rejected. Deleting the startup mode is still rejected. The "subtree contains startup" rule is no longer reachable in any path; the corresponding test is removed.
8. **GUI smoke.** Adding a mode appends it to the tab strip. Renaming and deleting modes keep working. The right-click context menu still mounts; Activate / Rename / Delete / Set as default all dispatch their existing engine commands. Verified via `cargo test -p inputforge-gui-dx -p inputforge-core`.

---

## Risks and rejected alternatives

**Inheritance loss is real but unused.** `resolve_mapping`'s ancestor walk is the only feature this spec strictly removes. The GUI cannot author depth-2 trees today, F14 already removed the two `ModeChangeStrategy` variants that depended on inheritance, and the user confirmed they have no profile files exercising it. The behavioral change is detectable only by hand-edited TOML, which the user will convert anyway.

**Rejected: keep `ModeTree` and just hide the depth-2 UI.** Carrying a tree the GUI cannot author is exactly the situation that drove this spec. Every CRUD path pays the tree's complexity cost without surfacing the benefit.

**Rejected: tolerant deserializer (read both shapes, write the new shape).** Considered as the "soft cut" option. Rejected because (a) the user explicitly chose the hard cut, and (b) keeping the legacy reader as a "remove later" task is a known source of code rot. A canonical error message is a clearer signal than a silent migration.

**Rejected: bundle Duplicate Mode into this spec.** Combined scope was offered and declined. Two clean specs in series (this one, then Duplicate against the flat model) is materially less risky to review than one large rewrite.

**Rejected: rename `ModeTree` -> `ModeList`.** `Modes` is the chosen name; `ModeList` adds noise without disambiguation, and the rest of the codebase already uses `modes` as a plural noun (`MetaSnapshot.modes`, `Profile.modes`).
