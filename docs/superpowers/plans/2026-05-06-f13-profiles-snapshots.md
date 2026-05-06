# F13 Profiles + Snapshots Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder Profiles panel with a right-side profile library, active-profile snapshot drawer, no-profile state, and engine-backed profile/snapshot lifecycle actions.

**Architecture:** Core remains the authority for durable profile and snapshot state. The engine owns profile/snapshot projection state in `AppState`, refreshes it after lifecycle commands, and exposes it to the GUI through `AppContext`. Snapshot storage uses two namespaces: library profiles continue to use the existing sibling `<profile_stem>.snapshots` directories computed by `snapshot::fs::snapshots_dir_for(profile_path)`; external Load-once profiles use an app-managed namespace at `AppSettings::config_dir()/external_snapshots/<hash>/`, where `<hash>` is a deterministic lowercase hex of the SHA-256 of the external file's canonical path. Reloading the same external path resolves to the same namespace. Add to library copies only the profile file and does not migrate external snapshot history. Pending-delete payloads live in a `.pending/` subdirectory inside each namespace.

**Tech Stack:** Rust workspace, `inputforge-core`, `inputforge-gui-dx`, Dioxus desktop/SSR tests, existing CSS token system, existing F4 toast/dialog/menu primitives.

---

## File Structure

- Create `crates/inputforge-core/src/profile/library.rs`: profile-library operations that wrap existing manager/profile APIs and keep profile file state plus sibling snapshot directories consistent.
- Modify `crates/inputforge-core/src/profile/mod.rs`: expose library operation types and helpers.
- Modify `crates/inputforge-core/src/profile/manager.rs`: expose only the small helpers needed by `library.rs`, including sanitized destination path calculation if needed.
- Create `crates/inputforge-core/src/snapshot/pending_delete.rs`: persistent pending-delete manifests, staged file restore, expiry purge, and visible-list filtering helpers.
- Modify `crates/inputforge-core/src/snapshot/mod.rs`: route visible deletion through pending-delete helpers and hide staged rows from engine projections.
- Modify `crates/inputforge-core/src/snapshot/types.rs`: add persistent pending-delete metadata type.
- Modify `crates/inputforge-core/src/state/mod.rs`: add `ProfileOrigin`, engine-owned profile library rows, active snapshot rows, and active profile origin.
- Modify `crates/inputforge-core/src/engine/command.rs`: add profile lifecycle commands and `UndoSnapshotDelete`.
- Modify `crates/inputforge-core/src/engine/run.rs`: implement new command handling inside the existing `handle_command` match and refresh projections after profile/snapshot lifecycle commands.
- Modify `crates/inputforge-core/src/engine/tests.rs`: command-level coverage for profile lifecycle, external load, no-profile state, projected rows, and snapshot pending-delete undo.
- Modify `crates/inputforge-gui-dx/src/context.rs`: surface projected profile rows, active snapshot rows, and no-profile status from `AppState`; dispatch durable mutations through `EngineCommand`.
- Modify `crates/inputforge-gui-dx/src/frame/view_state.rs`: add Profiles panel presentation state: filter, sub-mode, row menu id, inline rename drafts, drawer state, and snapshot delete toast identity.
- Create `crates/inputforge-gui-dx/src/frame/profiles/mod.rs`: F13 panel root and composition.
- Create `crates/inputforge-gui-dx/src/frame/profiles/projection.rs`: pure filter/sort helpers for already-engine-projected rows.
- Create `crates/inputforge-gui-dx/src/frame/profiles/actions.rs`: GUI event-to-`EngineCommand` mapping plus confirmation/toast descriptors.
- Create `crates/inputforge-gui-dx/src/frame/profiles/library.rs`: profile library header, filter, rows, overlay actions, inline rename, filtered empty state.
- Create `crates/inputforge-gui-dx/src/frame/profiles/new_profile.rs`: panel-scoped New Profile sub-mode.
- Create `crates/inputforge-gui-dx/src/frame/profiles/snapshot_drawer.rs`: bottom-anchored drawer header, sibling toggle/action controls, ledger rows, restore/delete/pin actions.
- Create `crates/inputforge-gui-dx/src/frame/profiles/no_profile.rs`: compact no-profile panel actions and disabled snapshot bar.
- Create `crates/inputforge-gui-dx/src/frame/profiles/tests.rs`: projection, SSR/component, interaction, keyboard, and command dispatch tests.
- Modify `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs`: replace the Profiles placeholder with `ProfilesPanel`.
- Modify `crates/inputforge-gui-dx/src/frame/layout/mod.rs`: keep center workspace stable and show no-profile explanation/actions when no profile is loaded.
- Modify `crates/inputforge-gui-dx/src/frame/top_bar/tools_cluster/logic.rs`: keep Profiles panel behavior consistent with existing right-panel activation.
- Create `crates/inputforge-gui-dx/assets/frame/profiles.css`: compact rows, overlay menus, drawer states, no-profile bar, focus and reduced-motion styling.
- The CSS is loaded by declaring `const PROFILES_CSS: Asset = asset!("/assets/frame/profiles.css");` inside `frame/profiles/mod.rs` and emitting `Stylesheet { href: PROFILES_CSS }` from the panel root, mirroring the existing `theme/mod.rs:22,76` and `panel_slot/mod.rs:7,23` pattern. Do not edit `global.css`.

---

## Task 1: Core Profile Library Operations

**Files:**
- Create: `crates/inputforge-core/src/profile/library.rs`
- Modify: `crates/inputforge-core/src/profile/mod.rs`
- Modify: `crates/inputforge-core/src/profile/manager.rs`
- Modify: `crates/inputforge-core/Cargo.toml` (add `opener` dep)
- Test: `crates/inputforge-core/src/profile/library.rs`

- [ ] **Step 1: Write failing library-operation tests**

Add tests proving the corrected storage contract:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::manager::{create_profile_in, list_profiles_in};
    use crate::snapshot::fs::snapshots_dir_for;
    use crate::Profile;

    #[test]
    fn rename_profile_updates_internal_name_and_moves_sibling_snapshot_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        let original_path = create_profile_in("Alpha", &profiles_dir).unwrap();
        let old_snapshots = snapshots_dir_for(&original_path).unwrap();
        std::fs::create_dir_all(&old_snapshots).unwrap();

        let renamed = rename_library_profile(&original_path, "Bravo").unwrap();

        let new_snapshots = snapshots_dir_for(&renamed.path).unwrap();
        assert_eq!(renamed.name, "Bravo");
        assert!(!original_path.exists());
        assert!(renamed.path.exists());
        assert_eq!(Profile::load(&renamed.path).unwrap().name(), "Bravo");
        assert!(!old_snapshots.exists());
        assert!(new_snapshots.exists());
    }

    #[test]
    fn duplicate_profile_rewrites_internal_name_without_copying_snapshots() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        let original_path = create_profile_in("Alpha", &profiles_dir).unwrap();
        std::fs::create_dir_all(snapshots_dir_for(&original_path).unwrap()).unwrap();

        let duplicated = duplicate_library_profile(&original_path, "Alpha Copy", &profiles_dir).unwrap();

        assert_eq!(duplicated.name, "Alpha Copy");
        assert_eq!(Profile::load(&duplicated.path).unwrap().name(), "Alpha Copy");
        assert!(!snapshots_dir_for(&duplicated.path).unwrap().exists());
    }

    #[test]
    fn add_external_to_library_rewrites_internal_name_without_copying_external_snapshots() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        let source_path = create_profile_in("Source", &profiles_dir).unwrap();
        let external_path = tmp.path().join("external.toml");
        std::fs::copy(&source_path, &external_path).unwrap();
        std::fs::create_dir_all(snapshots_dir_for(&external_path).unwrap()).unwrap();

        let imported = add_external_profile_to_library(&external_path, "Imported", &profiles_dir).unwrap();

        assert_eq!(imported.name, "Imported");
        assert_eq!(Profile::load(&imported.path).unwrap().name(), "Imported");
        assert!(!snapshots_dir_for(&imported.path).unwrap().exists());
    }

    #[test]
    fn duplicate_name_uses_invalid_config_error() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        let original_path = create_profile_in("Alpha", &profiles_dir).unwrap();
        create_profile_in("Alpha Copy", &profiles_dir).unwrap();

        let err = duplicate_library_profile(&original_path, "Alpha Copy", &profiles_dir).unwrap_err();
        assert!(err.to_string().contains("invalid config"));
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn list_library_rows_sorts_alphabetically() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles_dir = tmp.path().join("profiles");
        create_profile_in("Zulu", &profiles_dir).unwrap();
        create_profile_in("Alpha", &profiles_dir).unwrap();

        let profiles = list_profiles_in(&profiles_dir).unwrap();
        assert_eq!(profiles[0].name, "Alpha");
        assert_eq!(profiles[1].name, "Zulu");
    }

    #[test]
    fn reveal_profile_does_not_panic_for_missing_path() {
        // opener::reveal may return Ok or a controlled Err on a non-existent path;
        // the contract is that it never panics.
        let result = reveal_profile_in_explorer(std::path::Path::new("E:/does/not/exist.toml"));
        let _ = result;
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-core profile::library -- --nocapture`

Expected: FAIL because `profile::library`, `rename_library_profile`, `duplicate_library_profile`, and `add_external_profile_to_library` do not exist yet.

- [ ] **Step 3: Implement library operations**

Create `crates/inputforge-core/src/profile/library.rs` with these public shapes:

```rust
use std::path::{Path, PathBuf};

use crate::error::{EngineError, Result};
use crate::profile::manager::rename_profile;
use crate::snapshot::fs::snapshots_dir_for;
use crate::Profile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryProfile {
    pub name: String,
    pub path: PathBuf,
}

pub fn rename_library_profile(path: &Path, new_name: &str) -> Result<LibraryProfile> {
    let old_snapshot_dir = snapshots_dir_for(path)?;
    let old_snapshot_exists = old_snapshot_dir.exists();
    let new_path = destination_path_for_name(path, new_name)?;
    let new_snapshot_dir = snapshots_dir_for(&new_path)?;
    if old_snapshot_exists && new_snapshot_dir.exists() && new_snapshot_dir != old_snapshot_dir {
        return Err(EngineError::InvalidConfig {
            reason: format!("snapshot directory already exists for profile '{new_name}'"),
        });
    }

    let renamed_path = rename_profile(path, new_name)?;
    if old_snapshot_exists {
        std::fs::rename(&old_snapshot_dir, &new_snapshot_dir)?;
    }

    Ok(LibraryProfile { name: new_name.to_owned(), path: renamed_path })
}

pub fn duplicate_library_profile(source_path: &Path, new_name: &str, library_dir: &Path) -> Result<LibraryProfile> {
    save_profile_copy_with_name(source_path, new_name, library_dir)
}

pub fn add_external_profile_to_library(external_path: &Path, name: &str, library_dir: &Path) -> Result<LibraryProfile> {
    save_profile_copy_with_name(external_path, name, library_dir)
}

pub fn reveal_profile_in_explorer(path: &Path) -> Result<()> {
    opener::reveal(path).map_err(|e| EngineError::Io { source: std::io::Error::other(e) })
}
```

Implement `destination_path_for_name` using the same sanitization policy as `create_profile_in` and duplicate-name errors as `EngineError::InvalidConfig { reason: format!("a profile named '{name}' already exists") }`. Implement `save_profile_copy_with_name` by loading `Profile::load(source_path)`, calling `profile.set_name(new_name.to_owned())`, and saving to the sanitized destination. Do not copy snapshot directories. `reveal_profile_in_explorer` delegates to the cross-platform `opener` crate (Windows: selects the file in Explorer; macOS/Linux: best-effort reveal that may open the parent directory). Add `opener = "0.7"` (or current stable) to `crates/inputforge-core/Cargo.toml` dependencies.

- [ ] **Step 4: Export module**

In `profile/mod.rs`:

```rust
pub mod library;

pub use library::{
    add_external_profile_to_library, duplicate_library_profile, rename_library_profile,
    reveal_profile_in_explorer, LibraryProfile,
};
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p inputforge-core profile::library -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-core/src/profile/library.rs crates/inputforge-core/src/profile/mod.rs crates/inputforge-core/src/profile/manager.rs crates/inputforge-core/Cargo.toml Cargo.lock
git commit -m "feat(core): add profile library operations"
```

---

## Task 2: Engine Commands And Projection State

**Files:**
- Modify: `crates/inputforge-core/src/state/mod.rs`
- Modify: `crates/inputforge-core/src/engine/command.rs`
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Modify: `crates/inputforge-core/src/snapshot/fs.rs` (add `external_snapshots_dir_for`)
- Test: `crates/inputforge-core/src/engine/tests.rs`

**Pre-flight check:** confirm `EngineCommand::RenameSnapshot`, `PinSnapshot`, `UnpinSnapshot` already exist in `engine/command.rs` (introduced by F6/F7). The plan reuses them implicitly. If any are missing, insert a Task 2.5 to introduce them before Task 2 Step 4.

- [ ] **Step 1: Write failing engine projection tests**

Add command-level tests that assert durable state and projections. Reuse the existing `make_engine()` helper (`engine/tests.rs:127`), augmented by a small `make_engine_with_profiles_dir()` free function that points the engine at a `tempfile::tempdir()`-backed `profiles_dir` for isolation, and `write_external_profile(dir, name)` that writes a minimal profile TOML at `dir/<name>.toml` and returns the path.

```rust
#[test]
fn load_external_profile_once_marks_origin_external_and_does_not_add_library_row() {
    let (mut engine, _rx, tmp) = make_engine_with_profiles_dir();
    let external = write_external_profile(tmp.path().join("external"), "External");

    engine.handle_command(EngineCommand::LoadExternalProfileOnce(external.clone())).unwrap();

    let state = engine.state();
    assert_eq!(state.profile_path.as_ref(), Some(&external));
    assert_eq!(state.active_profile_origin, Some(ProfileOrigin::External));
    assert!(state.profile_library_rows.iter().all(|row| row.path != external));
    assert_eq!(state.engine_status, EngineStatus::Stopped);
}

#[test]
fn delete_active_library_profile_enters_no_profile_state_and_refreshes_rows() {
    let (mut engine, _rx, _tmp) = make_engine_with_profiles_dir();
    engine.handle_command(EngineCommand::CreateProfile { name: "Alpha".to_owned() }).unwrap();
    let alpha_path = engine.state().profile_library_rows.iter().find(|r| r.name == "Alpha").unwrap().path.clone();
    engine.handle_command(EngineCommand::LoadProfile(alpha_path)).unwrap();

    engine.handle_command(EngineCommand::DeleteProfile { name: "Alpha".to_owned() }).unwrap();

    let state = engine.state();
    assert!(state.active_profile.is_none());
    assert!(state.profile_path.is_none());
    assert!(state.active_profile_origin.is_none());
    assert!(state.active_snapshot_rows.is_empty());
    assert!(state.profile_library_rows.iter().all(|row| row.name != "Alpha"));
    assert_eq!(state.engine_status, EngineStatus::Stopped);
}

#[test]
fn profile_lifecycle_commands_refresh_projected_library_rows() {
    let (mut engine, _rx, _tmp) = make_engine_with_profiles_dir();
    engine.handle_command(EngineCommand::CreateProfile { name: "Alpha".to_owned() }).unwrap();
    let alpha_path = engine.state().profile_library_rows.iter().find(|r| r.name == "Alpha").unwrap().path.clone();
    engine.handle_command(EngineCommand::DuplicateProfile {
        source_path: alpha_path,
        name: "Bravo".to_owned(),
    }).unwrap();

    let names = engine.state().profile_library_rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>();
    assert_eq!(names, vec!["Alpha", "Bravo"]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-core engine::tests -- --nocapture`

Expected: FAIL because the new commands and `AppState` projection fields do not exist yet.

- [ ] **Step 3: Add core projection types**

In `state/mod.rs`, add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileOrigin {
    Library,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileLibraryRow {
    pub name: String,
    pub path: PathBuf,
    pub origin: ProfileOrigin,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSnapshotRow {
    pub id: crate::snapshot::SnapshotId,
    pub kind: crate::snapshot::SnapshotKind,
    pub label: Option<String>,
    pub taken_at: chrono::DateTime<chrono::Utc>,
    pub pinned: bool,
}
```

Add these fields to `AppState`:

```rust
pub active_profile_origin: Option<ProfileOrigin>,
pub profile_library_rows: Vec<ProfileLibraryRow>,
pub active_snapshot_rows: Vec<ActiveSnapshotRow>,
```

- [ ] **Step 4: Add engine command variants**

Extend `EngineCommand` without replacing existing variants:

```rust
CreateProfile { name: String },
LoadExternalProfileOnce(PathBuf),
AddExternalProfileToLibrary { path: PathBuf, name: String },
RenameProfile { old_name: String, new_name: String },
DuplicateProfile { source_path: PathBuf, name: String },
DeleteProfile { name: String },
RevealProfile { path: PathBuf },
UndoSnapshotDelete { id: SnapshotId },
```

- [ ] **Step 5: Implement command handling in the existing `handle_command` match**

Use the existing `reload_profile_from_disk(&path)` helper for load flows. After each profile lifecycle command, call a new `refresh_profile_library_rows()` helper. After each snapshot lifecycle command, call a new `refresh_active_snapshot_rows()` helper.

Add a new helper to `crates/inputforge-core/src/snapshot/fs.rs`:

```rust
pub(crate) fn external_snapshots_dir_for(canonical_path: &Path) -> Result<PathBuf> {
    use sha2::{Digest, Sha256};
    let bytes = canonical_path.as_os_str().to_string_lossy().as_bytes().to_vec();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());
    Ok(crate::settings::AppSettings::config_dir().join("external_snapshots").join(hash))
}
```

Library profiles continue to use `snapshots_dir_for(profile_path)`. External profiles resolve through `external_snapshots_dir_for(canonical_path)`. Engine snapshot commands (`CreateSnapshot`, `RestoreSnapshot`, `DeleteSnapshot`, `RenameSnapshot`, `PinSnapshot`, `UnpinSnapshot`) consult `AppState.active_profile_origin` to pick the correct namespace.

Policy:
- `LoadProfile(path)`: assumes `path` is a library file under `AppSettings::profiles_dir()`. Sets `active_profile_origin` to `Some(ProfileOrigin::Library)`. If `path` is outside the library, return `EngineError::InvalidConfig` with reason `"use LoadExternalProfileOnce for external paths"`. Existing F6 LoadProfile assertions are preserved; only origin assignment is added.
- `LoadExternalProfileOnce(path)`: canonicalizes `path` (falls back to the input on canonicalize error), calls `reload_profile_from_disk(&path)`, sets origin to `External`, sets `engine_status` to `Stopped`, clears `mode_force`, creates/prunes `AutoSessionStart`, refreshes rows. The canonical path is the key into `external_snapshots_dir_for` for subsequent snapshot ops.
- `CreateProfile { name }`: creates in `AppSettings::profiles_dir()`, loads it as active library profile, and refreshes rows.
- `AddExternalProfileToLibrary { path, name }`: imports via `add_external_profile_to_library`, loads imported library profile, origin `Library`, refreshes rows. Does not migrate external snapshot history.
- `RenameProfile`: renames the library profile and, if it was active, reloads from the renamed path. Library snapshot directory is moved by `rename_library_profile`.
- `DuplicateProfile`: duplicates without changing active profile and without copying snapshots.
- `DeleteProfile`: deletes the library file and its sibling `<stem>.snapshots` directory; if active, clears `active_profile`, `profile_path`, `active_profile_origin`, `mode_force`, `active_snapshot_rows`, and sets `engine_status` to `Stopped`.
- `RevealProfile { path }`: dispatches to `crate::profile::reveal_profile_in_explorer(&path)` (cross-platform via `opener` crate); logs warnings on failure; durable state unchanged.

- [ ] **Step 6: Run tests**

Run: `cargo test -p inputforge-core engine::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-core/src/state/mod.rs crates/inputforge-core/src/engine/command.rs crates/inputforge-core/src/engine/run.rs crates/inputforge-core/src/engine/tests.rs crates/inputforge-core/src/snapshot/fs.rs
git commit -m "feat(core): route profile lifecycle through engine state"
```

---

## Task 3: Persistent Snapshot Pending Delete

**Files:**
- Create: `crates/inputforge-core/src/snapshot/pending_delete.rs`
- Modify: `crates/inputforge-core/src/snapshot/mod.rs`
- Modify: `crates/inputforge-core/src/snapshot/types.rs`
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Test: `crates/inputforge-core/src/snapshot/tests.rs`
- Test: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write failing pending-delete tests**

Add tests covering manifest persistence. Tests construct profile + snapshot fixtures using the existing `Profile::new()` + `snapshot::create()` flow already exercised in `crates/inputforge-core/src/snapshot/tests.rs` (no harness type required). The pending-delete dir is `<namespace_dir>/.pending/`.

```rust
#[test]
fn pending_delete_hides_row_until_undo_restores_it() {
    let tmp = tempfile::tempdir().unwrap();
    let profile_path = tmp.path().join("profile.toml");
    Profile::new("Profile").save(&profile_path).unwrap();
    let namespace = snapshots_dir_for(&profile_path).unwrap();
    let snapshot = snapshot::create(&profile_path, "before trim", SnapshotKind::Manual).unwrap();

    let staged = stage_delete(&namespace, &snapshot.id).unwrap();
    assert!(list_visible(&namespace).unwrap().iter().all(|row| row.id != snapshot.id));

    undo_delete_by_id(&namespace, &snapshot.id).unwrap();
    assert!(list_visible(&namespace).unwrap().iter().any(|row| row.id == snapshot.id));
    assert!(!staged.manifest_path.exists());
}

#[test]
fn expired_pending_delete_purges_on_startup_cleanup() {
    let tmp = tempfile::tempdir().unwrap();
    let profile_path = tmp.path().join("profile.toml");
    Profile::new("Profile").save(&profile_path).unwrap();
    let namespace = snapshots_dir_for(&profile_path).unwrap();
    let snapshot = snapshot::create(&profile_path, "delete me", SnapshotKind::Manual).unwrap();

    stage_delete(&namespace, &snapshot.id).unwrap();
    purge_expired_pending_deletes(&namespace, chrono::Duration::zero()).unwrap();

    assert!(list_visible(&namespace).unwrap().iter().all(|row| row.id != snapshot.id));
    assert!(!pending_manifest_path(&namespace, &snapshot.id).exists());
}

#[test]
fn external_load_once_uses_canonical_path_hash_namespace() {
    let canonical = std::path::Path::new("E:/Profiles/external.toml");
    let dir1 = external_snapshots_dir_for(canonical).unwrap();
    let dir2 = external_snapshots_dir_for(canonical).unwrap();
    assert!(dir1.starts_with(crate::settings::AppSettings::config_dir().join("external_snapshots")));
    assert_eq!(dir1, dir2, "namespace must be deterministic for the same canonical path");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-core snapshot::tests -- --nocapture`

Expected: FAIL because pending-delete helpers do not exist.

- [ ] **Step 3: Implement persistent metadata**

In `snapshot/types.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSnapshotDelete {
    pub id: SnapshotId,
    pub profile_path: std::path::PathBuf,
    pub original_path: std::path::PathBuf,
    pub staged_path: std::path::PathBuf,
    pub deleted_at: DateTime<Utc>,
}
```

- [ ] **Step 4: Implement pending-delete helpers**

Create `snapshot/pending_delete.rs` with these functions. All helpers operate on a `namespace_dir`, which is either the library sibling `<profile_stem>.snapshots` (resolved via `snapshots_dir_for`) or the external Load-once dir `<config_dir>/external_snapshots/<hash>/` (resolved via `external_snapshots_dir_for`).

```rust
pub fn stage_delete(namespace_dir: &Path, id: &SnapshotId) -> Result<PendingSnapshotDelete>;
pub fn undo_delete_by_id(namespace_dir: &Path, id: &SnapshotId) -> Result<()>;
pub fn purge_expired_pending_deletes(namespace_dir: &Path, max_age: chrono::Duration) -> Result<()>;
pub fn list_visible(namespace_dir: &Path) -> Result<Vec<Snapshot>>;
pub fn pending_manifest_path(namespace_dir: &Path, id: &SnapshotId) -> PathBuf;
pub(crate) fn pending_dir(namespace_dir: &Path) -> PathBuf;
pub(crate) fn resolve_snapshot_namespace(state: &crate::state::AppState) -> Result<PathBuf>;
```

Rules:
- The pending dir for a namespace is `<namespace_dir>/.pending/`.
- The staged snapshot file path is `<namespace_dir>/.pending/<id>.toml`.
- The manifest path is `<namespace_dir>/.pending/<id>.pending.toml`.
- `resolve_snapshot_namespace(state)` returns the active profile's namespace dir based on `state.active_profile_origin`: library origin returns `snapshots_dir_for(state.profile_path)`; external origin returns `external_snapshots_dir_for(canonicalize(state.profile_path))`. If no profile is loaded, returns `EngineError::InvalidConfig`.
- `stage_delete(namespace_dir, id)` reads and validates the snapshot file exists, ensures `<namespace_dir>/.pending/` exists, writes the manifest, and moves the snapshot file into the pending dir. It does **not** rewrite the snapshot index; visibility is enforced by `list_visible` filtering pending manifests.
- `undo_delete_by_id(namespace_dir, id)` reads the manifest, restores the staged file to its `original_path`, and removes the manifest. The visible list returns to its prior state on the next call.
- `purge_expired_pending_deletes(namespace_dir, max_age)` deletes staged files and manifests whose `deleted_at` is older than `Utc::now() - max_age`. With `max_age == Duration::zero()`, all pending entries purge immediately.
- `list_visible(namespace_dir)` calls existing `snapshot::list_for_namespace(namespace_dir)` (introduce a thin wrapper if `snapshot::list` only accepts a profile path) and removes ids whose `<id>.pending.toml` exists in `<namespace_dir>/.pending/`.

- [ ] **Step 4b: Wire startup purge**

In `Engine::new` (or the closest existing init path in `engine/run.rs`), after settings load and before the first command is processed, sweep every known snapshot namespace and call `purge_expired_pending_deletes(namespace_dir, settings.snapshot.pending_delete_ttl)`. Library namespaces are discoverable by listing `AppSettings::profiles_dir()` and computing `snapshots_dir_for` for each profile file. External namespaces are discoverable by listing `<config_dir>/external_snapshots/`. Failures purging a single namespace must be logged and skipped, not propagated.

Add a corresponding test asserting that an aged manifest disappears after `Engine::new` runs.

- [ ] **Step 5: Wire engine delete/undo**

Update `EngineCommand::DeleteSnapshot` in `handle_command` to resolve the namespace via `resolve_snapshot_namespace(&self.state)` and call `stage_delete(&namespace, &id)` instead of immediate `snapshot::delete`, then refresh `active_snapshot_rows`.

Add `EngineCommand::UndoSnapshotDelete { id }` handling:
- If no profile is loaded, log a warning and return `Ok(())`.
- If a profile is loaded, resolve the namespace, call `undo_delete_by_id(&namespace, &id)`, then refresh `active_snapshot_rows`.

Snapshot-creating, restore, rename, and pin commands likewise consult `resolve_snapshot_namespace` so external Load-once profiles operate on their canonical-path-hash dir.

- [ ] **Step 6: Run tests**

Run: `cargo test -p inputforge-core snapshot::tests -- --nocapture`

Run: `cargo test -p inputforge-core engine::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-core/src/snapshot/pending_delete.rs crates/inputforge-core/src/snapshot/mod.rs crates/inputforge-core/src/snapshot/types.rs crates/inputforge-core/src/engine/command.rs crates/inputforge-core/src/engine/run.rs crates/inputforge-core/src/snapshot/tests.rs crates/inputforge-core/src/engine/tests.rs
git commit -m "feat(core): add persistent snapshot delete undo"
```

---

## Task 4: GUI Projection Types And Panel State

**Files:**
- Modify: `crates/inputforge-gui-dx/src/context.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/view_state.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mod.rs` (add `pub mod profiles;`)
- Create: `crates/inputforge-gui-dx/src/frame/profiles/mod.rs` (empty shell so submodules compile)
- Create: `crates/inputforge-gui-dx/src/frame/profiles/projection.rs`
- Create: `crates/inputforge-gui-dx/src/frame/profiles/tests.rs`

- [ ] **Step 1: Write failing projection tests**

```rust
#[test]
fn projection_pins_active_and_sorts_inactive_alphabetically() {
    let rows = sample_profile_rows("Bravo", &["Zulu", "Alpha", "Bravo"]);

    let projected = project_profile_rows(&rows, "Bravo", "");

    assert_eq!(projected.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(), vec!["Bravo", "Alpha", "Zulu"]);
    assert!(projected[0].is_active);
}

#[test]
fn active_profile_stays_visible_when_filter_does_not_match() {
    let rows = sample_profile_rows("Bravo", &["Zulu", "Alpha", "Bravo"]);

    let projected = project_profile_rows(&rows, "Bravo", "alp");

    assert_eq!(projected.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(), vec!["Bravo", "Alpha"]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: FAIL because `frame::profiles` does not exist yet.

- [ ] **Step 2.5: Create empty `frame/profiles/mod.rs` shell**

Create `crates/inputforge-gui-dx/src/frame/profiles/mod.rs` with just the submodule declarations needed for the rest of Task 4 to compile:

```rust
pub mod projection;

#[cfg(test)]
mod tests;
```

Add `pub mod profiles;` to `crates/inputforge-gui-dx/src/frame/mod.rs`. Task 5 will replace this shell with the full `ProfilesPanel` component.

- [ ] **Step 2.6: Define test helpers**

Add the following helpers to `crates/inputforge-gui-dx/src/frame/profiles/tests.rs` so subsequent task tests have shared fixtures:

```rust
use crate::context::{ProfileRowOrigin, ProfileRowView, SnapshotRowView};

pub(super) fn sample_profile_rows(active: &str, names: &[&str]) -> Vec<ProfileRowView> {
    names.iter().map(|name| ProfileRowView {
        id: (*name).to_owned(),
        name: (*name).to_owned(),
        path_label: format!("{}.toml", name),
        is_active: *name == active,
        origin: ProfileRowOrigin::Library,
        can_open: true,
        can_rename: true,
        can_duplicate: true,
        can_reveal: true,
        can_delete: true,
        can_add_to_library: false,
        can_snapshot_now: false,
    }).collect()
}

pub(super) fn sample_snapshot_rows() -> Vec<SnapshotRowView> {
    Vec::new()
}
```

Tasks 5, 8, and 10 will introduce additional render helpers (`render_profiles_panel`, `render_no_profile_frame`, `render_snapshot_drawer`, `sample_profiles_context`, `sample_snapshot_context`). Those helpers must mirror the existing SSR test wiring in `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs` (a `TestHarness` Props component plus `dioxus_ssr::render`), not invent a parallel pattern.

- [ ] **Step 3: Add GUI view models from engine state**

In `context.rs`, add compact GUI structs built from `AppState.profile_library_rows` and `AppState.active_snapshot_rows`. Visibility is `pub(crate)` to match `AppContext` (`context.rs:32`):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProfileRowOrigin {
    Library,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileRowView {
    pub id: String,
    pub name: String,
    pub path_label: String,
    pub is_active: bool,
    pub origin: ProfileRowOrigin,
    pub can_open: bool,
    pub can_rename: bool,
    pub can_duplicate: bool,
    pub can_reveal: bool,
    pub can_delete: bool,
    pub can_add_to_library: bool,
    pub can_snapshot_now: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotRowView {
    pub id: String,
    pub kind_label: String,
    pub label: Option<String>,
    pub time_label: String,
    pub sort_key: i64,
    pub pinned: bool,
}
```

Map core `ProfileOrigin::Library` to rename/delete/duplicate allowed. Map `ProfileOrigin::External` to add-to-library/snapshot-now/reveal allowed and rename/delete disallowed.

- [ ] **Step 4: Add panel presentation state**

In `view_state.rs`, keep the existing `PanelSlot::Profiles` enum variant and add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ProfilesPanelMode {
    #[default]
    Library,
    NewProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InlineRenameDraft {
    pub target_id: String,
    pub draft: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ProfilesPanelState {
    pub filter: String,
    pub mode: ProfilesPanelMode,
    pub open_row_menu_id: Option<String>,
    pub profile_rename: Option<InlineRenameDraft>,
    pub snapshot_drawer_open: bool,
    pub snapshot_rename: Option<InlineRenameDraft>,
    pub pending_snapshot_delete_toast_id: Option<String>,
}
```

- [ ] **Step 5: Implement pure projection helpers**

Create `profiles/projection.rs`:

```rust
use crate::context::ProfileRowView;

pub fn project_profile_rows(rows: &[ProfileRowView], active_id: &str, filter: &str) -> Vec<ProfileRowView> {
    let needle = filter.trim().to_lowercase();
    let mut active = rows.iter().filter(|row| row.id == active_id).cloned().collect::<Vec<_>>();
    let mut inactive = rows
        .iter()
        .filter(|row| row.id != active_id)
        .filter(|row| needle.is_empty() || row.name.to_lowercase().contains(&needle))
        .cloned()
        .collect::<Vec<_>>();

    active.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    inactive.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    active.extend(inactive);
    active
}
```

- [ ] **Step 6: Run tests and commit**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: PASS.

```bash
git add crates/inputforge-gui-dx/src/context.rs crates/inputforge-gui-dx/src/frame/view_state.rs crates/inputforge-gui-dx/src/frame/mod.rs crates/inputforge-gui-dx/src/frame/profiles/mod.rs crates/inputforge-gui-dx/src/frame/profiles/projection.rs crates/inputforge-gui-dx/src/frame/profiles/tests.rs
git commit -m "feat(gui-dx): add profiles projection state"
```

---

## Task 5: Profiles Panel Shell And Placeholder Replacement

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/mod.rs` (replace Task 4 shell with full panel root + `PROFILES_CSS` asset)
- Create: `crates/inputforge-gui-dx/src/frame/profiles/no_profile.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/layout/mod.rs`
- Create: `crates/inputforge-gui-dx/assets/frame/profiles.css`
- Test: `crates/inputforge-gui-dx/src/frame/profiles/tests.rs`

- [ ] **Step 1: Write failing SSR tests**

```rust
#[test]
fn profiles_panel_replaces_placeholder_copy() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("data-testid=\"profile-library\""));
    assert!(!html.contains("Placeholder"));
}

#[test]
fn no_profile_state_shows_center_explanation_and_panel_actions() {
    let html = render_no_profile_frame();

    assert!(html.contains("No profile loaded"));
    assert!(html.contains("New profile"));
    assert!(html.contains("Open file"));
    assert!(!html.contains("mapping-list"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: FAIL.

- [ ] **Step 3: Implement panel shell**

Use the existing `AppContext` type. CSS loads via the codebase's `asset!()` + `Stylesheet { href: ... }` pattern (see `theme/mod.rs:22,76` and `panel_slot/mod.rs:7,23`); do not edit `global.css`.

```rust
use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::profiles::no_profile::NoProfileBar;

pub mod no_profile;
pub mod projection;

const PROFILES_CSS: Asset = asset!("/assets/frame/profiles.css");

#[component]
pub fn ProfilesPanel() -> Element {
    let ctx = use_context::<AppContext>();
    let state = ctx.state.read();
    let snapshot_count = state.active_snapshot_rows.len();
    let has_profile = state.active_profile.is_some();
    drop(state);

    rsx! {
        Stylesheet { href: PROFILES_CSS }
        section { class: "profiles-panel", "data-testid": "profile-library",
            header { class: "profiles-panel__header",
                h2 { "Profiles" }
                button { class: "button button--primary", "data-action": "new-profile", "+ New profile" }
                button { class: "button", "data-action": "open-profile", "Open file..." }
            }
            div { class: "profiles-panel__body",
                if has_profile {
                    div { class: "profiles-panel__library", "Profile library" }
                } else {
                    NoProfileBar {}
                }
            }
            footer { class: "profiles-panel__snapshot-toggle", "Snapshots · {snapshot_count}" }
        }
    }
}
```

- [ ] **Step 4: Replace placeholder in panel slot**

In `panel_slot/mod.rs`, replace the existing `PanelSlotEnum::Profiles => ...` placeholder arm (the alias is local to this file; line 3 of the module: `use crate::frame::view_state::{PanelSlot as PanelSlotEnum, ViewState};`) with:

```rust
PanelSlotEnum::Profiles => rsx! { ProfilesPanel {} },
```

Outside `panel_slot/mod.rs`, refer to the variant as `PanelSlot::Profiles` (the alias does not exist in other modules).

- [ ] **Step 5: Run tests and commit**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: PASS.

```bash
git add crates/inputforge-gui-dx/src/frame/profiles crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs crates/inputforge-gui-dx/src/frame/layout/mod.rs crates/inputforge-gui-dx/assets/frame/profiles.css
git commit -m "feat(gui-dx): replace profiles placeholder panel"
```

---

## Task 6: Profile Library Rows And Action Dispatch

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/profiles/actions.rs`
- Create: `crates/inputforge-gui-dx/src/frame/profiles/library.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/mod.rs`
- Test: `crates/inputforge-gui-dx/src/frame/profiles/tests.rs`

- [ ] **Step 1: Write failing dispatch tests**

```rust
#[test]
fn profile_delete_action_dispatches_real_engine_command() {
    let action = profile_delete_action("Alpha");

    assert_eq!(action.command, EngineCommand::DeleteProfile { name: "Alpha".to_owned() });
    // Confirmation assertion lives in Task 9.
}

#[test]
fn snapshot_delete_action_dispatches_real_engine_command_and_undo_toast() {
    let id = sample_snapshot_id();
    let action = snapshot_delete_action(id);

    assert_eq!(action.command, EngineCommand::DeleteSnapshot { id });
    assert_eq!(action.toast_action, Some(ToastAction::UndoSnapshotDelete { id }));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: FAIL.

- [ ] **Step 3: Implement action descriptors with real commands**

In `actions.rs` (visibility `pub(crate)` to match other panel-internal types):

```rust
use inputforge_core::engine::EngineCommand;
use inputforge_core::snapshot::SnapshotId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfirmationKind {
    DestructiveF4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToastAction {
    UndoSnapshotDelete { id: SnapshotId },
}

// `Eq` is intentionally omitted: ProfilesAction.command holds an EngineCommand,
// which only derives PartialEq.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfilesAction {
    pub command: EngineCommand,
    pub confirmation: Option<ConfirmationKind>,
    pub toast_action: Option<ToastAction>,
}

pub(crate) fn profile_delete_action(name: &str) -> ProfilesAction {
    ProfilesAction {
        command: EngineCommand::DeleteProfile { name: name.to_owned() },
        confirmation: Some(ConfirmationKind::DestructiveF4),
        toast_action: None,
    }
}

pub(crate) fn snapshot_delete_action(id: SnapshotId) -> ProfilesAction {
    ProfilesAction {
        command: EngineCommand::DeleteSnapshot { id },
        confirmation: None,
        toast_action: Some(ToastAction::UndoSnapshotDelete { id }),
    }
}
```

- [ ] **Step 4: Render library rows**

Render rows from `AppContext` projected rows. Library rows show Open, Rename, Duplicate, Reveal, Delete. External rows show Open, Add to library, Snapshot now, Reveal, and hide Rename/Delete.

- [ ] **Step 5: Run tests and commit**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: PASS.

```bash
git add crates/inputforge-gui-dx/src/frame/profiles/actions.rs crates/inputforge-gui-dx/src/frame/profiles/library.rs crates/inputforge-gui-dx/src/frame/profiles/mod.rs crates/inputforge-gui-dx/src/frame/profiles/tests.rs
git commit -m "feat(gui-dx): add profiles library actions"
```

---

## Task 7: New Profile And Open File Flow

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/profiles/new_profile.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/actions.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/mod.rs`
- Test: `crates/inputforge-gui-dx/src/frame/profiles/tests.rs`

- [ ] **Step 1: Write failing flow tests**

```rust
#[test]
fn new_blank_profile_dispatches_create_profile() {
    let command = create_new_profile_command(NewProfileSource::Blank, "Alpha", None, &[]).unwrap();

    assert_eq!(command, EngineCommand::CreateProfile { name: "Alpha".to_owned() });
}

#[test]
fn open_file_load_once_dispatches_external_load() {
    let path = PathBuf::from("E:/Profiles/external.toml");
    let command = open_file_load_once_command(path.clone());

    assert_eq!(command, EngineCommand::LoadExternalProfileOnce(path));
}

#[test]
fn add_external_to_library_dispatches_import_command() {
    let path = PathBuf::from("E:/Profiles/external.toml");
    let command = add_external_to_library_command(path.clone(), "Imported", &[]).unwrap();

    assert_eq!(command, EngineCommand::AddExternalProfileToLibrary { path, name: "Imported".to_owned() });
}
```

- [ ] **Step 1b: Write failing GUI-side validation tests**

Spec lines 278-288 require GUI pre-validation of common profile-library inputs before dispatch (empty/whitespace names, illegal filename characters, duplicate library names, missing external paths, snapshot-folder rename collisions, case-only renames). Add a `NewProfileValidationError` enum to `frame/profiles/actions.rs` and a `validate_new_profile_name(name, &[existing_names])` helper that mirrors `crate::profile::manager::validate_profile_name` semantics so GUI-side rejection matches engine-side rejection.

```rust
#[test]
fn empty_or_whitespace_name_is_rejected_inline() {
    let result = create_new_profile_command(NewProfileSource::Blank, "   ", None, &[]);
    assert!(matches!(result, Err(NewProfileValidationError::EmptyName)));
}

#[test]
fn duplicate_library_name_is_rejected_inline() {
    let existing = vec!["Alpha".to_owned()];
    let result = validate_new_profile_name("Alpha", &existing);
    assert!(matches!(result, Err(NewProfileValidationError::DuplicateName)));
}

#[test]
fn case_only_duplicate_rename_is_allowed() {
    // A user renaming "Alpha" to "ALPHA" must not be rejected as duplicate.
    let existing = vec!["Alpha".to_owned()];
    let result = validate_rename("Alpha", "ALPHA", &existing);
    assert!(result.is_ok());
}

#[test]
fn illegal_filename_char_is_rejected_inline() {
    let result = validate_new_profile_name("bad/name", &[]);
    assert!(matches!(result, Err(NewProfileValidationError::IllegalCharacter(_))));
}

#[test]
fn missing_external_path_is_rejected_inline() {
    let result = open_file_load_once_command(PathBuf::new());
    assert!(matches!(result, Err(NewProfileValidationError::MissingPath)));
}
```

Adjust the existing happy-path tests to pass an `&[existing_names]` slice through `create_new_profile_command` and `add_external_to_library_command` so callers exercise the new validation surface.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: FAIL.

- [ ] **Step 3: Implement sub-mode and commands**

Implement `NewProfileSource::{Blank, CopyActive, CopyProfile, OpenPath}`. The GUI builds `EngineCommand` values only; it does not read or write profile files.

- [ ] **Step 4: Run tests and commit**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: PASS.

```bash
git add crates/inputforge-gui-dx/src/frame/profiles/new_profile.rs crates/inputforge-gui-dx/src/frame/profiles/actions.rs crates/inputforge-gui-dx/src/frame/profiles/mod.rs crates/inputforge-gui-dx/src/frame/profiles/tests.rs
git commit -m "feat(gui-dx): add profile creation flow"
```

---

## Task 8: Snapshot Drawer UI And Keyboard Handling

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/profiles/snapshot_drawer.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/actions.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/profiles.css`
- Test: `crates/inputforge-gui-dx/src/frame/profiles/tests.rs`

- [ ] **Step 1: Write failing drawer and keyboard tests**

```rust
#[test]
fn drawer_header_uses_sibling_toggle_and_snapshot_now_button() {
    let html = render_snapshot_drawer(sample_snapshot_context(), true);

    assert!(html.contains("class=\"snapshot-drawer__header\""));
    assert!(html.contains("class=\"snapshot-drawer__toggle\""));
    assert!(html.contains("aria-label=\"Snapshot now\""));
    assert!(!html.contains("<button class=\"snapshot-drawer__toggle\"><button"));
}

#[test]
fn ctrl_s_is_suppressed_inside_editable_or_modal_context() {
    assert!(!should_handle_snapshot_shortcut(FocusScope::TextInput));
    assert!(!should_handle_snapshot_shortcut(FocusScope::InlineRename));
    assert!(!should_handle_snapshot_shortcut(FocusScope::Menu));
    assert!(!should_handle_snapshot_shortcut(FocusScope::Dialog));
    assert!(!should_handle_snapshot_shortcut(FocusScope::OsPickerReturn));
    assert!(should_handle_snapshot_shortcut(FocusScope::Panel));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: FAIL.

- [ ] **Step 3: Implement valid drawer markup**

Create `snapshot_drawer.rs` with sibling controls:

```rust
use dioxus::prelude::*;

use crate::context::SnapshotRowView;

#[component]
pub fn SnapshotDrawer(active_profile_name: String, rows: Vec<SnapshotRowView>, open: bool) -> Element {
    let count = rows.len();
    rsx! {
        section { class: "snapshot-drawer",
            div { class: "snapshot-drawer__header",
                button {
                    class: "snapshot-drawer__toggle",
                    "aria-expanded": "{open}",
                    span { class: "snapshot-drawer__chevron", if open { "v" } else { ">" } }
                    span { "Snapshots · {active_profile_name}" }
                    span { class: "badge", "{count}" }
                }
                button {
                    class: "icon-button",
                    "aria-label": "Snapshot now",
                    title: "Snapshot now",
                    "+"
                }
            }
            if open {
                div { class: "snapshot-drawer__ledger",
                    for row in rows {
                        article { class: "snapshot-row", "data-snapshot-id": "{row.id}",
                            span { class: "snapshot-row__kind", "{row.kind_label}" }
                            span { class: "snapshot-row__time", "{row.time_label}" }
                            if let Some(label) = &row.label { strong { "{label}" } }
                            if row.pinned { span { class: "badge", "Pinned" } }
                            button { class: "button button--primary", "Restore" }
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Implement shortcut gating**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusScope {
    Panel,
    TextInput,
    InlineRename,
    Menu,
    Dialog,
    OsPickerReturn,
}

pub fn should_handle_snapshot_shortcut(scope: FocusScope) -> bool {
    matches!(scope, FocusScope::Panel)
}
```

- [ ] **Step 5: Run tests and commit**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: PASS.

```bash
git add crates/inputforge-gui-dx/src/frame/profiles/snapshot_drawer.rs crates/inputforge-gui-dx/src/frame/profiles/actions.rs crates/inputforge-gui-dx/src/frame/profiles/mod.rs crates/inputforge-gui-dx/src/frame/profiles/tests.rs crates/inputforge-gui-dx/assets/frame/profiles.css
git commit -m "feat(gui-dx): add active profile snapshot drawer"
```

---

## Task 9: Dialogs, Toasts, Destructive Confirmation

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/actions.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/library.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/snapshot_drawer.rs`
- Test: `crates/inputforge-gui-dx/src/frame/profiles/tests.rs`

- [ ] **Step 1: Write failing confirmation/toast tests**

```rust
#[test]
fn destructive_profile_delete_uses_f4_confirmation() {
    let action = profile_delete_action("Alpha");

    assert_eq!(action.confirmation, Some(ConfirmationKind::DestructiveF4));
    assert_eq!(action.command, EngineCommand::DeleteProfile { name: "Alpha".to_owned() });
}

#[test]
fn snapshot_restore_uses_f4_confirmation() {
    let id = sample_snapshot_id();
    let action = snapshot_restore_action(id);

    assert_eq!(action.confirmation, Some(ConfirmationKind::DestructiveF4));
    assert_eq!(action.command, EngineCommand::RestoreSnapshot { id });
}

#[test]
fn undo_toast_dispatches_undo_snapshot_delete() {
    let id = sample_snapshot_id();
    let toast_action = ToastAction::UndoSnapshotDelete { id };

    assert_eq!(toast_action.command(), EngineCommand::UndoSnapshotDelete { id });
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: FAIL.

- [ ] **Step 3: Wire descriptors into existing F4 primitives**

Use existing dialog and toast primitives. Profile delete and snapshot restore require destructive confirmation. Snapshot delete dispatches immediately, stages pending delete in core, and queues an undo toast that dispatches `EngineCommand::UndoSnapshotDelete { id }`.

- [ ] **Step 4: Run tests and commit**

Run: `cargo test -p inputforge-gui-dx frame::profiles::tests -- --nocapture`

Expected: PASS.

```bash
git add crates/inputforge-gui-dx/src/frame/profiles/actions.rs crates/inputforge-gui-dx/src/frame/profiles/library.rs crates/inputforge-gui-dx/src/frame/profiles/snapshot_drawer.rs crates/inputforge-gui-dx/src/frame/profiles/tests.rs
git commit -m "feat(gui-dx): confirm destructive profile actions"
```

---

## Task 10: Final Visual, Accessibility, And Verification Pass

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/profiles.css`
- Modify: `crates/inputforge-gui-dx/src/frame/profiles/*.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/layout/mod.rs`
- Test: `crates/inputforge-gui-dx/src/frame/profiles/tests.rs`

- [ ] **Step 1: Write final acceptance tests**

```rust
#[test]
fn profiles_surface_never_renders_mapping_counts() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(!html.contains("mapping"));
    assert!(!html.contains("mappings"));
}

#[test]
fn drawer_is_panel_scoped_not_global_drawer() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("snapshot-drawer"));
    assert!(!html.contains("app-global-drawer"));
}
```

- [ ] **Step 2: Run package tests**

Run: `cargo test -p inputforge-gui-dx -- --nocapture`

Expected: PASS.

Run: `cargo test -p inputforge-core -- --nocapture`

Expected: PASS.

- [ ] **Step 3: Run formatting and lint checks**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: both commands PASS.

- [ ] **Step 4: Manual visual pass**

Run: `dx run -p inputforge-app`

(`dx run` is required for the manual interactive pass; `cargo run` skips Dioxus asset bundling and hot-reload.)

Verify manually:

- Profiles panel opens from the existing right-panel tools cluster.
- Active profile is pinned above inactive alphabetical rows.
- Filtering keeps active row visible.
- Row menus overlay without changing row height.
- External Load once row shows `External`, `Add to library`, and `Snapshot now`, and hides Rename/Delete.
- Reveal opens Explorer with the profile file selected (Windows); other platforms open the parent directory.
- Snapshot drawer opens inside the right panel only.
- Snapshot drawer toggle and Snapshot now are sibling controls.
- Snapshot rows show Restore as primary and no mapping counts.
- `Ctrl+S` opens snapshot creation only outside text input, rename, menu, dialog, and OS picker return flow.
- No-profile state shows center explanation plus New/Open actions and disables device/calibration/mapping surfaces.
- Narrow width keeps row text clipped cleanly without overlap.
- Reduced motion removes drawer/menu transition movement.

Keyboard reachability:

- Tab and Shift+Tab reach each row's overflow menu trigger; Enter or Space opens it; arrow keys navigate items; Esc closes the menu and returns focus to the trigger.
- Inline rename: Enter commits, Esc cancels and restores the prior name, blur commits.
- Snapshot drawer toggle is reachable by Tab and announces the correct `aria-expanded` state.

External snapshot namespace:

- Load once an external profile, take a snapshot, close and reopen the app, Load once the same external file. The snapshot history is preserved (canonical-path-hash namespace).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/profiles crates/inputforge-gui-dx/src/frame/layout/mod.rs crates/inputforge-gui-dx/assets/frame/profiles.css
git commit -m "test(gui-dx): verify profiles snapshot acceptance"
```

---

## Spec Coverage Check

- Profiles panel replacement: Task 5.
- Active pinned first and inactive alphabetical: Task 4 and Task 6.
- No mapping counts: Task 6, Task 8, Task 10.
- Overlay row actions: Task 6.
- New Profile sub-mode and copy source select: Task 7.
- Open file, Load once, Add to library: Task 2 and Task 7.
- Library snapshot storage preserved as adjacent `<profile_stem>.snapshots`: Task 1, Task 2, Task 3.
- External Load-once snapshots use an app-managed canonical-path-hash namespace at `<config_dir>/external_snapshots/<hash>/`: Task 2 (helper) and Task 3 (namespace resolution).
- Duplicate excludes snapshots: Task 1 and Task 2.
- Rename carries sibling snapshot directory: Task 1 and Task 2.
- Active delete enters no-profile without auto-load: Task 2 and Task 5.
- Engine-owned projection rows in `AppState`: Task 2 and Task 4.
- Reveal opens the OS file manager via the `opener` crate: Task 1 (helper) and Task 2 (engine wire-up).
- Panel-scoped snapshot drawer: Task 8 and Task 10.
- Restore confirmation: Task 9.
- Persistent pending-delete undo toast: Task 3 (helpers) and Task 9 (toast wiring); startup purge of expired pending deletes: Task 3 Step 4b.
- `Ctrl+S` shortcut gating: Task 8.
- Validation and inline errors: Task 4 (panel state) and Task 7 (commands + Step 1b validation tests).
- Engine command routing for durable mutations: Task 2 and Task 3.
- Visual and accessibility guidance: Task 6, Task 8, Task 10.

## Plan Complete

Plan complete and saved to `docs/superpowers/plans/2026-05-06-f13-profiles-snapshots.md`. Two execution options:

**1. Subagent-Driven (recommended)** - dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** - execute tasks in this session using executing-plans, batch execution with checkpoints.
