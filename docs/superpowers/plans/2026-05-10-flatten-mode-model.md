# Flatten Mode Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace profile mode storage with a flat, ordered `Modes` list and update engine, GUI, profile, and snapshot tests to use that shape everywhere.

**Architecture:** `inputforge-core` owns the invariant boundary through a private `Modes(Vec<String>)` newtype. Runtime routing uses direct `(input, mode)` mapping lookup. GUI mode tabs continue to consume `MetaSnapshot.modes` as a flat `Vec<String>`.

**Tech Stack:** Rust, Cargo workspace tests, serde/TOML, Dioxus GUI crate, jcodemunch/jdocmunch-assisted repo navigation.

---

## Source Spec

- `docs/superpowers/specs/2026-05-10-flatten-mode-model-design.md`

## File Structure

- Modify: `crates/inputforge-core/src/mode/mod.rs`
  Defines `Modes`, serde behavior, invariants, and mode-list unit tests.
- Modify: `crates/inputforge-core/src/mode/state.rs`
  Changes validation inputs to `&Modes` while preserving stack behavior.
- Modify: `crates/inputforge-core/src/profile/mod.rs`
  Stores `Modes`, validates root-level flat TOML, and updates profile tests.
- Modify: `crates/inputforge-core/src/profile/manager.rs`
  Creates the default profile with `Modes::new(vec!["Default".to_owned()])`.
- Modify: `crates/inputforge-core/src/engine/command.rs`
  Keeps `EngineCommand::AddMode` to a single `name: String` field and updates comments/tests.
- Modify: `crates/inputforge-core/src/engine/run.rs`
  Appends new modes, deletes only the named mode, and preserves rename/default-mode behavior.
- Modify: `crates/inputforge-core/src/engine/dependencies.rs`
  Uses direct active-mode mapping lookup.
- Modify: `crates/inputforge-core/src/engine/output_handler.rs`
  Uses `Modes` and direct lookup during axis refresh.
- Modify: `crates/inputforge-core/src/engine/tests.rs`
  Rewrites fixtures and command assertions around flat mode lists.
- Modify: `crates/inputforge-core/src/snapshot/tests.rs`
  Rewrites profile fixtures to root-level `modes = [...]`.
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/logic.rs`
  Simplifies delete disabling and makes duplicate validation ASCII-case-insensitive.
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs`
  Updates delete-disabling call sites.
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs`
  Simplifies context-menu flag tests.
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/delete_dialog.rs`
  Counts only the named mode and mappings directly scoped to it.
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/add_inline.rs`
  Dispatches `EngineCommand::AddMode { name: name.clone() }`.

## Existing Behavior To Preserve

- `ModeState::switch_to`, `push_temporary`, `pop_temporary`, `clear_stack_entries`, and `rename_in_place` behavior.
- `EngineError::ModeCycleDetected` behavior.
- F14 Hold/Temporary mode auto-release lifecycle.
- `RenameMode` cascades across mappings, action graphs, runtime current mode, `ModeState`, and `startup_mode`.
- `SetDefaultMode` still validates the target name against the profile modes.
- `MetaSnapshot.modes` remains `Vec<String>`.

## Commit Boundaries

Each task below must compile and pass its focused tests before committing.

```bash
git commit -m "test(mode): pin flat mode list invariants"
git commit -m "refactor(mode)!: replace mode storage with flat modes"
git commit -m "refactor(engine)!: use direct mode mapping lookup"
git commit -m "refactor(engine): flatten mode commands"
git commit -m "refactor(gui): simplify mode tab delete rules"
git commit -m "test(mode): prove flat profile fixtures"
```

---

### Task 1: Pin `Modes` Invariants And TOML Shape

**Files:**
- Modify: `crates/inputforge-core/src/mode/mod.rs`

- [ ] **Step 1: Replace mode tests with failing flat-list tests**

In `crates/inputforge-core/src/mode/mod.rs`, replace the existing `#[cfg(test)] mod tests` body with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn modes(names: &[&str]) -> Modes {
        Modes::new(names.iter().map(|name| (*name).to_owned()).collect()).unwrap()
    }

    #[test]
    fn new_accepts_non_empty_unique_names() {
        let modes = modes(&["Default", "Combat", "Landing"]);

        assert_eq!(modes.as_slice(), ["Default", "Combat", "Landing"]);
        assert_eq!(modes.first(), "Default");
        assert_eq!(modes.len(), 3);
        assert!(modes.contains("Combat"));
        assert!(!modes.contains("Missing"));
    }

    #[test]
    fn new_rejects_empty_list() {
        let err = Modes::new(Vec::new()).unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(err.to_string(), "invalid configuration: modes cannot be empty");
    }

    #[test]
    fn new_rejects_duplicate_names_case_insensitively() {
        let err = Modes::new(vec!["Combat".to_owned(), "combat".to_owned()]).unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(
            err.to_string(),
            "invalid configuration: duplicate mode name: combat"
        );
    }

    #[test]
    fn with_appended_places_new_name_at_tail() {
        let modes = modes(&["Default", "Combat"]);

        let modes = modes.with_appended("Landing").unwrap();

        assert_eq!(modes.as_slice(), ["Default", "Combat", "Landing"]);
    }

    #[test]
    fn with_appended_rejects_duplicate_name_case_insensitively() {
        let modes = modes(&["Default", "Combat"]);

        let err = modes.with_appended("combat").unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(
            err.to_string(),
            "invalid configuration: duplicate mode name: combat"
        );
    }

    #[test]
    fn with_renamed_rewrites_one_name_in_place() {
        let modes = modes(&["Default", "Combat", "Landing"]);

        let modes = modes.with_renamed("Combat", "Cruise").unwrap();

        assert_eq!(modes.as_slice(), ["Default", "Cruise", "Landing"]);
    }

    #[test]
    fn with_renamed_keeps_no_op_rename_valid() {
        let modes = modes(&["Default", "Combat"]);

        let renamed = modes.with_renamed("Combat", "Combat").unwrap();

        assert_eq!(renamed.as_slice(), ["Default", "Combat"]);
    }

    #[test]
    fn with_renamed_rejects_unknown_source() {
        let modes = modes(&["Default", "Combat"]);

        let err = modes.with_renamed("Missing", "Cruise").unwrap_err();

        assert!(matches!(err, EngineError::ModeNotFound { .. }));
    }

    #[test]
    fn with_renamed_rejects_collision_case_insensitively() {
        let modes = modes(&["Default", "Combat", "Landing"]);

        let err = modes.with_renamed("Landing", "combat").unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(
            err.to_string(),
            "invalid configuration: duplicate mode name: combat"
        );
    }

    #[test]
    fn with_removed_drops_one_name() {
        let modes = modes(&["Default", "Combat", "Landing"]);

        let modes = modes.with_removed("Combat").unwrap();

        assert_eq!(modes.as_slice(), ["Default", "Landing"]);
    }

    #[test]
    fn with_removed_rejects_unknown_name() {
        let modes = modes(&["Default", "Combat"]);

        let err = modes.with_removed("Missing").unwrap_err();

        assert!(matches!(err, EngineError::ModeNotFound { .. }));
    }

    #[test]
    fn with_removed_rejects_last_mode() {
        let modes = modes(&["Default"]);

        let err = modes.with_removed("Default").unwrap_err();

        assert!(matches!(err, EngineError::InvalidConfig { .. }));
        assert_eq!(
            err.to_string(),
            "invalid configuration: cannot remove the last mode"
        );
    }

    #[test]
    fn serde_toml_roundtrip_uses_flat_list() {
        #[derive(Debug, Deserialize, PartialEq, Serialize)]
        struct Wrapper {
            modes: Modes,
        }

        let wrapper = Wrapper {
            modes: modes(&["Default", "Combat", "Landing"]),
        };

        let toml = toml::to_string(&wrapper).unwrap();
        assert_eq!(toml, "modes = [\"Default\", \"Combat\", \"Landing\"]\n");

        let parsed: Wrapper = toml::from_str(&toml).unwrap();
        assert_eq!(parsed, wrapper);
    }

    #[test]
    fn deserialize_rejects_non_list_value() {
        #[derive(Debug, Deserialize)]
        struct Wrapper {
            modes: Modes,
        }

        let err = toml::from_str::<Wrapper>("modes = 42\n").unwrap_err();

        assert!(err.to_string().contains("modes must be a flat list of strings"));
    }

    #[test]
    fn deserialize_rejects_non_string_entry() {
        #[derive(Debug, Deserialize)]
        struct Wrapper {
            modes: Modes,
        }

        let err = toml::from_str::<Wrapper>("modes = [\"Default\", 42]\n").unwrap_err();

        assert!(err.to_string().contains("mode names must be strings"));
    }
}
```

- [ ] **Step 2: Run the failing tests**

Run:

```bash
cargo test -p inputforge-core mode::tests -- --nocapture
```

Expected: compile fails because `Modes` does not exist yet.

- [ ] **Step 3: Rewrite `mode/mod.rs` around `Modes`**

Keep `mod state;` and `pub use state::ModeState;`. Replace the production mode container with:

```rust
use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};

/// Ordered flat list of profile modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Modes(Vec<String>);

impl Modes {
    /// Create a validated list of modes.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::InvalidConfig`] if the list is empty or if two
    /// names compare equal under ASCII case folding.
    pub fn new(names: Vec<String>) -> Result<Self> {
        if names.is_empty() {
            return Err(EngineError::InvalidConfig {
                reason: "modes cannot be empty".to_owned(),
            });
        }
        reject_duplicate_names(&names)?;
        Ok(Self(names))
    }

    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    #[must_use]
    pub fn first(&self) -> &str {
        &self.0[0]
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|candidate| candidate == name)
    }

    pub fn with_appended(&self, name: &str) -> Result<Self> {
        let mut names = self.0.clone();
        names.push(name.to_owned());
        Self::new(names)
    }

    pub fn with_renamed(&self, from: &str, to: &str) -> Result<Self> {
        let Some(index) = self.0.iter().position(|name| name == from) else {
            return Err(EngineError::ModeNotFound {
                name: from.to_owned(),
            });
        };

        let mut names = self.0.clone();
        names[index] = to.to_owned();
        Self::new(names)
    }

    pub fn with_removed(&self, name: &str) -> Result<Self> {
        let Some(index) = self.0.iter().position(|candidate| candidate == name) else {
            return Err(EngineError::ModeNotFound {
                name: name.to_owned(),
            });
        };
        if self.0.len() == 1 {
            return Err(EngineError::InvalidConfig {
                reason: "cannot remove the last mode".to_owned(),
            });
        }

        let mut names = self.0.clone();
        names.remove(index);
        Self::new(names)
    }
}

fn reject_duplicate_names(names: &[String]) -> Result<()> {
    for (index, name) in names.iter().enumerate() {
        if names[..index]
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(name))
        {
            return Err(EngineError::InvalidConfig {
                reason: format!("duplicate mode name: {name}"),
            });
        }
    }
    Ok(())
}

impl<'de> Deserialize<'de> for Modes {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        let toml::Value::Array(values) = value else {
            return Err(serde::de::Error::custom(
                "modes must be a flat list of strings",
            ));
        };

        let names = values
            .into_iter()
            .map(|value| match value {
                toml::Value::String(name) => Ok(name),
                other => Err(serde::de::Error::custom(format!(
                    "mode names must be strings, found {other:?}"
                ))),
            })
            .collect::<std::result::Result<Vec<_>, D::Error>>()?;

        Self::new(names).map_err(serde::de::Error::custom)
    }
}
```

- [ ] **Step 4: Run the mode tests**

Run:

```bash
cargo test -p inputforge-core mode::tests -- --nocapture
```

Expected: all `mode::tests` pass.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-core/src/mode/mod.rs
git commit -m "test(mode): pin flat mode list invariants"
```

---

### Task 2: Swap Core Types To `Modes`

**Files:**
- Modify: `crates/inputforge-core/src/mode/state.rs`
- Modify: `crates/inputforge-core/src/profile/mod.rs`
- Modify: `crates/inputforge-core/src/profile/manager.rs`

- [ ] **Step 1: Update `ModeState` tests first**

In `crates/inputforge-core/src/mode/state.rs`, use this flat fixture:

```rust
fn test_modes() -> Modes {
    Modes::new(vec![
        "Default".to_owned(),
        "Combat".to_owned(),
        "Landing".to_owned(),
        "Missiles".to_owned(),
        "Guns".to_owned(),
    ])
    .unwrap()
}
```

Use `modes` for local fixture variables, and pass `&modes` to `switch_to` and `push_temporary`.

- [ ] **Step 2: Update `ModeState` signatures**

Change both validation parameters to `&Modes`:

```rust
pub fn switch_to(&mut self, name: &str, modes: &Modes) -> Result<()>
pub fn push_temporary(&mut self, name: &str, modes: &Modes) -> Result<()>
```

Each method should keep the same `contains` check, error variants, stack updates, and cycle detection.

- [ ] **Step 3: Update `Profile` storage**

In `crates/inputforge-core/src/profile/mod.rs`, replace mode storage, raw TOML storage, constructor parameter, accessor, and setter types with `Modes`.

Keep validation in `from_raw`:

```rust
if !raw.modes.contains(&raw.profile.startup_mode) {
    return Err(EngineError::InvalidConfig {
        reason: format!("startup_mode '{}' not found in modes", raw.profile.startup_mode),
    });
}
```

Mapping validation stays direct:

```rust
if !raw.modes.contains(&mapping.mode) {
    return Err(EngineError::InvalidConfig {
        reason: format!("mapping references unknown mode '{}'", mapping.mode),
    });
}
```

- [ ] **Step 4: Update root-level TOML fixtures**

Profile TOML tests must put `modes = [...]` before `[profile]`:

```toml
modes = ["Default", "Combat"]

[profile]
id = "550e8400-e29b-41d4-a716-446655440000"
name = "Test"
startup_mode = "Default"
```

Use this ordering in every profile fixture touched by this task.

- [ ] **Step 5: Update default profile creation**

In `crates/inputforge-core/src/profile/manager.rs`, create default modes with:

```rust
Modes::new(vec!["Default".to_owned()]).unwrap()
```

- [ ] **Step 6: Run focused core tests**

Run:

```bash
cargo test -p inputforge-core mode::state::tests -- --nocapture
cargo test -p inputforge-core profile::tests -- --nocapture
cargo test -p inputforge-core profile::manager -- --nocapture
```

Expected: all three commands pass.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-core/src/mode/state.rs crates/inputforge-core/src/profile/mod.rs crates/inputforge-core/src/profile/manager.rs
git commit -m "refactor(mode)!: replace mode storage with flat modes"
```

---

### Task 3: Use Direct Mode Mapping Lookup

**Files:**
- Modify: `crates/inputforge-core/src/mode/mod.rs`
- Delete: `crates/inputforge-core/src/mode/resolve.rs`
- Modify: `crates/inputforge-core/src/engine/dependencies.rs`
- Modify: `crates/inputforge-core/src/engine/output_handler.rs`
- Modify: `crates/inputforge-core/src/engine/run.rs`

- [ ] **Step 1: Add a local direct lookup helper where needed**

In `crates/inputforge-core/src/engine/dependencies.rs`, use:

```rust
fn direct_mapping_for<'a>(
    mappings: &'a [Mapping],
    input: &InputAddress,
    mode: &str,
) -> Option<&'a Mapping> {
    mappings
        .iter()
        .find(|mapping| mapping.input == *input && mapping.mode == *mode)
}
```

Call this helper for the primary source and for derived input checks.

- [ ] **Step 2: Update output refresh lookup**

In `crates/inputforge-core/src/engine/output_handler.rs`, change `Modes` parameters and use direct matching:

```rust
if let Some(mapping) = mappings
    .iter()
    .find(|mapping| mapping.input == address && mapping.mode == *mode)
{
    // keep the existing refresh body
}
```

- [ ] **Step 3: Update engine run signatures**

Any helper in `crates/inputforge-core/src/engine/run.rs` that receives the active modes should accept:

```rust
mode_list: &crate::mode::Modes
```

Keep the existing call order and error propagation.

- [ ] **Step 4: Remove the separate lookup module**

Remove the module declaration/export from `crates/inputforge-core/src/mode/mod.rs`, then delete:

```text
crates/inputforge-core/src/mode/resolve.rs
```

- [ ] **Step 5: Run focused routing tests**

Run:

```bash
cargo test -p inputforge-core engine::dependencies -- --nocapture
cargo test -p inputforge-core engine::output_handler -- --nocapture
```

Expected: both commands pass.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-core/src/mode/mod.rs crates/inputforge-core/src/engine/dependencies.rs crates/inputforge-core/src/engine/output_handler.rs crates/inputforge-core/src/engine/run.rs
git rm crates/inputforge-core/src/mode/resolve.rs
git commit -m "refactor(engine)!: use direct mode mapping lookup"
```

---

### Task 4: Flatten Engine Commands And Delete Rules

**Files:**
- Modify: `crates/inputforge-core/src/engine/command.rs`
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Update `AddMode` command shape**

In `crates/inputforge-core/src/engine/command.rs`, use:

```rust
AddMode { name: String },
```

Update command comments to say new modes are appended to the mode list.

- [ ] **Step 2: Rewrite the `AddMode` handler**

In `crates/inputforge-core/src/engine/run.rs`, match:

```rust
EngineCommand::AddMode { name } => {
```

Append and persist:

```rust
let modes = profile.modes().with_appended(&name)?;
profile.set_modes(modes);
```

Trace only the mode name:

```rust
tracing::info!(target: "engine", mode = %name, "AddMode applied");
```

- [ ] **Step 3: Keep `RenameMode` cascade order**

Preserve the validation-first order:

```rust
let new_modes = profile.modes().with_renamed(&from, &to)?;
let touched = profile.rename_mode_refs(&from, &to);
profile.set_modes(new_modes);
```

Then keep runtime current-mode and `ModeState` rename updates.

- [ ] **Step 4: Rewrite the `DeleteMode` handler**

Use these validation and mutation rules:

```rust
if profile.modes().first() == name {
    return Err(crate::error::EngineError::InvalidConfig {
        reason: "cannot delete first mode".to_owned(),
    });
}
if !profile.modes().contains(&name) {
    return Err(crate::error::EngineError::ModeNotFound { name: name.clone() });
}

let startup = profile.settings().startup_mode().to_owned();
if startup == name {
    return Err(crate::error::EngineError::InvalidConfig {
        reason: format!("cannot delete startup mode '{startup}'"),
    });
}

let new_modes = profile.modes().with_removed(&name)?;
profile.set_modes(new_modes);
let mappings_dropped = profile.remove_mappings_for_mode(&name);
```

Runtime state cleanup should compare directly with `name`:

```rust
if state.current_mode == name {
    startup.clone_into(&mut state.current_mode);
}
```

For `ModeState`:

```rust
if self.mode_state.current() == name {
    let modes = self
        .state
        .read()
        .active_profile
        .as_ref()
        .map(|profile| profile.modes().clone());
    if let Some(modes) = modes {
        self.mode_state.switch_to(&startup, &modes)?;
    }
}
self.mode_state.clear_stack_entries(std::slice::from_ref(&name));
```

- [ ] **Step 5: Update engine test fixtures**

Use flat helpers:

```rust
fn simple_modes() -> Modes {
    Modes::new(vec!["Default".to_owned()]).unwrap()
}

fn two_modes() -> Modes {
    Modes::new(vec!["Default".to_owned(), "Combat".to_owned()]).unwrap()
}

fn shift_modes() -> Modes {
    Modes::new(vec!["Default".to_owned(), "Shift".to_owned()]).unwrap()
}

fn three_modes() -> Modes {
    Modes::new(vec![
        "Default".to_owned(),
        "Combat".to_owned(),
        "Landing".to_owned(),
    ])
    .unwrap()
}
```

Change `make_profile` to accept `Modes`.

- [ ] **Step 6: Update command tests**

Keep and update tests for:

```rust
add_mode_appends_to_modes_list
delete_mode_rejects_first_mode
delete_mode_rejects_startup_mode
delete_mode_drops_mappings_for_deleted_mode
rename_mode_renames_modes_and_persists
```

Every `AddMode` literal should be:

```rust
EngineCommand::AddMode { name: mode_name }
```

- [ ] **Step 7: Run focused engine command tests**

Run:

```bash
cargo test -p inputforge-core add_mode -- --nocapture
cargo test -p inputforge-core delete_mode -- --nocapture
cargo test -p inputforge-core rename_mode -- --nocapture
cargo test -p inputforge-core set_default_mode -- --nocapture
```

Expected: all four commands pass.

- [ ] **Step 8: Commit**

```bash
git add crates/inputforge-core/src/engine/command.rs crates/inputforge-core/src/engine/run.rs crates/inputforge-core/src/engine/tests.rs
git commit -m "refactor(engine): flatten mode commands"
```

---

### Task 5: Simplify GUI Mode Tab Rules

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/logic.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/delete_dialog.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/add_inline.rs`

- [ ] **Step 1: Update logic tests first**

Add this duplicate-name test:

```rust
#[test]
fn validate_duplicate_is_ascii_case_insensitive() {
    assert_eq!(
        validate_mode_name("combat", &modes(), None),
        NameValidation::Duplicate {
            name: "combat".to_owned()
        }
    );
}
```

Update delete-disabled tests to call:

```rust
assert!(delete_disabled_for_tab("Default", &modes(), Some("Combat")));
assert!(delete_disabled_for_tab("Combat", &modes(), Some("Combat")));
assert!(!delete_disabled_for_tab("Landing", &modes(), Some("Combat")));
assert!(!delete_disabled_for_tab("Landing", &modes(), None));
```

- [ ] **Step 2: Simplify `delete_disabled_for_tab`**

Use:

```rust
pub(crate) fn delete_disabled_for_tab(
    name: &str,
    modes: &[String],
    startup: Option<&str>,
) -> bool {
    let is_first = modes.first().is_some_and(|first| first == name);
    let is_startup = startup.is_some_and(|startup| startup == name);
    is_first || is_startup
}
```

In `validate_mode_name`, duplicate detection must use:

```rust
.any(|n| n.eq_ignore_ascii_case(trimmed));
```

- [ ] **Step 3: Update mode tab call sites**

In `mode_tabs/mod.rs`, pass only the tab name, `modes_snapshot`, and `startup.as_deref()` into `delete_disabled_for_tab`.

For context-menu flags:

```rust
delete_disabled: logic::delete_disabled_for_tab(
    &open_name,
    &modes_for_flags,
    startup.as_deref(),
),
```

- [ ] **Step 4: Simplify context menu tests**

Use this helper signature:

```rust
fn flags_for(
    name: &str,
    modes: &[String],
    startup: Option<&str>,
    current: &str,
    has_profile: bool,
) -> ContextMenuFlags
```

Keep tests for first tab, startup tab, no profile rename disabling, and current-mode Activate disabling.

- [ ] **Step 5: Simplify delete dialog counting**

In `delete_dialog.rs`, count the selected mode and its direct mappings:

```rust
let modes_count = 1usize;
let mappings_count = profile
    .mappings()
    .iter()
    .filter(|mapping| mapping.mode == *name)
    .count();
```

- [ ] **Step 6: Update the AddMode dispatch**

In `add_inline.rs`, dispatch:

```rust
EngineCommand::AddMode { name: name.clone() }
```

- [ ] **Step 7: Run GUI mode-tab tests**

Run:

```bash
cargo test -p inputforge-gui-dx mode_tabs -- --nocapture
```

Expected: mode-tab tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/logic.rs crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/context_menu.rs crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/delete_dialog.rs crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/add_inline.rs
git commit -m "refactor(gui): simplify mode tab delete rules"
```

---

### Task 6: Prove Flat Profile And Snapshot Fixtures

**Files:**
- Modify: `crates/inputforge-core/src/profile/mod.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`
- Modify: `crates/inputforge-core/src/snapshot/tests.rs`

- [ ] **Step 1: Add profile-level flat TOML round-trip test**

In `crates/inputforge-core/src/profile/mod.rs`, add:

```rust
#[test]
fn flat_modes_toml_roundtrip() {
    let input = r#"
modes = ["Default", "Combat", "Landing"]

[profile]
id = "01J00000000000000000000000"
name = "Flight"
startup_mode = "Default"
"#;

    let profile = Profile::from_toml(input).unwrap();
    assert_eq!(
        profile.modes().as_slice(),
        ["Default", "Combat", "Landing"]
    );

    let saved = profile.to_toml().unwrap();
    assert!(saved.contains("modes = [\"Default\", \"Combat\", \"Landing\"]"));

    let reparsed = Profile::from_toml(&saved).unwrap();
    assert_eq!(reparsed, profile);
}
```

- [ ] **Step 2: Add profile-level invalid-shape tests**

In the same test module, add:

```rust
#[test]
fn non_list_modes_value_is_rejected_with_neutral_message() {
    let input = r#"
modes = 42

[profile]
id = "01J00000000000000000000000"
name = "Flight"
startup_mode = "Default"
"#;

    let err = Profile::from_toml(input).unwrap_err();

    assert!(err.to_string().contains("modes must be a flat list of strings"));
}

#[test]
fn non_string_modes_entry_is_rejected() {
    let input = r#"
modes = ["Default", 42]

[profile]
id = "01J00000000000000000000000"
name = "Flight"
startup_mode = "Default"
"#;

    let err = Profile::from_toml(input).unwrap_err();

    assert!(err.to_string().contains("mode names must be strings"));
}
```

- [ ] **Step 3: Update engine profile fixtures**

In `crates/inputforge-core/src/engine/tests.rs`, every embedded profile TOML fixture should use:

```toml
modes = ["Default"]

[profile]
id = "550e8400-e29b-41d4-a716-446655440000"
name = "Example"
startup_mode = "Default"
```

For multi-mode tests, add names to the same root-level list.

- [ ] **Step 4: Update snapshot profile fixtures**

In `crates/inputforge-core/src/snapshot/tests.rs`, every embedded profile TOML fixture should use:

```toml
modes = ["Default"]

[profile]
id = "550e8400-e29b-41d4-a716-446655440000"
name = "Example"
startup_mode = "Default"
```

Keep the existing metadata setup and assertions unchanged unless they directly inspect the profile text.

- [ ] **Step 5: Run focused fixture tests**

Run:

```bash
cargo test -p inputforge-core flat_modes -- --nocapture
cargo test -p inputforge-core non_list_modes -- --nocapture
cargo test -p inputforge-core non_string_modes -- --nocapture
cargo test -p inputforge-core snapshot::tests -- --nocapture
```

Expected: all four commands pass.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-core/src/profile/mod.rs crates/inputforge-core/src/engine/tests.rs crates/inputforge-core/src/snapshot/tests.rs
git commit -m "test(mode): prove flat profile fixtures"
```

---

### Task 7: Final Cleanup And Workspace Sweep

**Files:**
- Modify as needed based on search output from `crates/inputforge-core/`, `crates/inputforge-gui-dx/`, and this plan/spec pair.

- [ ] **Step 1: Search for removed mode-model symbols and wording**

Use a deny-list search over tracked files in `crates/`, `docs/superpowers/plans/2026-05-10-flatten-mode-model.md`, and `docs/superpowers/specs/2026-05-10-flatten-mode-model-design.md`. The deny-list should include removed mode-model identifiers and unsupported profile-shape examples from this feature area.

Expected: zero hits in mode-related code, tests, and touched docs. Comments outside this feature are not part of this sweep.

- [ ] **Step 2: Prove `AddMode` has only `name`**

Search `crates/` for `EngineCommand::AddMode` literals and patterns.

Expected: every result uses only `name`.

- [ ] **Step 3: Run core and GUI test suites**

Run:

```bash
cargo test -p inputforge-core -p inputforge-gui-dx
```

Expected: all tests pass.

- [ ] **Step 4: Format**

Run:

```bash
cargo fmt --all --check
```

Expected: pass. If it fails, run:

```bash
cargo fmt --all
```

Then rerun:

```bash
cargo fmt --all --check
```

- [ ] **Step 5: Optional Dioxus smoke**

Run:

```bash
dx run -p inputforge-app
```

Expected: the GUI launches. In the app, manually verify:

- Adding a mode appends it to the tab strip.
- Renaming a mode updates the tab and mappings.
- Deleting the first mode is unavailable/rejected.
- Deleting the startup mode is unavailable/rejected.
- Right-click context menu mounts and Activate/Rename/Delete/Set as default dispatch.
- Hold/Temporary mode action editor behavior is unchanged.

Stop the dev process after the smoke check.

- [ ] **Step 6: Commit final cleanup if needed**

Only commit if this task produced edits beyond formatting:

```bash
git add crates docs/superpowers/plans/2026-05-10-flatten-mode-model.md docs/superpowers/specs/2026-05-10-flatten-mode-model-design.md
git commit -m "refactor(mode): remove leftover flat-mode cleanup"
```

---

## Acceptance Checklist

- [ ] Mode storage is a private `Modes(Vec<String>)` newtype.
- [ ] Root-level `modes = ["Default", "Combat", "Landing"]` serializes and parses as the canonical TOML shape.
- [ ] Unsupported non-list `modes` values fail with neutral flat-list wording.
- [ ] `EngineCommand::AddMode` has exactly one field: `name: String`.
- [ ] Delete mode removes only the selected mode and mappings directly scoped to it.
- [ ] F14 Hold/Temporary mode tests pass unchanged except for fixture type swaps.
- [ ] F4 delete dialog counts only the selected mode and its directly scoped mappings.
- [ ] Snapshot tests use root-level flat `modes = [...]` fixtures.
- [ ] Removed mode-model identifiers and unsupported profile-shape wording are absent from mode-related code, tests, and touched docs.
- [ ] `cargo test -p inputforge-core -p inputforge-gui-dx` passes.
- [ ] `cargo fmt --all --check` passes.

## Self-Review Notes

- Spec coverage: data model, root-level TOML, engine commands, GUI rules, snapshot fixtures, tests, deleted module, and acceptance gates are covered by Tasks 1-7.
- Placeholder scan: the plan contains no unresolved placeholder instructions.
- Type consistency: the new type is consistently `Modes`, and command examples use the final flat API.
