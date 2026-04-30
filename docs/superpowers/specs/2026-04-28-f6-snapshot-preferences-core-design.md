# F6, Snapshot Module + Settings Extension + Forced-Mode Plumbing in `inputforge-core`: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-28
**Parent specs:**
- [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md), master rewrite plan, F6 is its first post-F5 feature
- [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md), IA redesign that defines F6's surface

**Predecessors:** F1 (state bridge), F2 (design system), F3 (shell + tray), F4 (toast + dialog), F5 (IA redesign, design only, no code)
**Type:** core-only, no GUI surface
**Crate touched:** `crates/inputforge-core` only

---

## Context

F5 committed a clean-slate IA redesign for the Dioxus rewrite. Three engine-side capabilities the new IA depends on do not yet exist in `inputforge-core`:

1. **Snapshots.** F5's save model is *auto-commit + session undo + on-disk snapshots*. The on-disk snapshot layer needs an engine-owned module before any GUI can bind to it (F12 calibration save, F13 Profiles + Snapshots panel).
2. **Forced runtime mode.** F7's chrome shows a runtime-mode marker and "Activate / Release" banner; that requires an engine field that pauses mode-change rules and a pair of commands to flip it.
3. **User preferences.** Snapshot defaults (rolling-buffer count, content-hash dedup) are user-configurable. The spec wants direct-TOML-edit access from day one, no UI required, and an editor surface in F15.

F6 is the engine-side foundation for items 1-3. It adds zero pixels of GUI. After F6, F7 can bind to `mode_force`, F12/F13 can dispatch the new snapshot commands, and F15 can ship a typed editor on top of the same data layer.

This is also the point at which we adapt F5's "preferences module" naming to the codebase's existing reality: `crates/inputforge-core/src/settings.rs` already exists and persists `AppSettings { last_profile }` to `%APPDATA%/inputforge/settings.toml`. F6 extends `AppSettings` rather than introducing a parallel `preferences` module, the user-edited prefs live as a sub-table inside the existing TOML.

---

## Confirmed design decisions

The decisions below were validated during brainstorming dialogue; each is recorded in dependency order.

### Crate dependencies

**1. `chrono` for timestamps.** Snapshot `taken_at: DateTime<Utc>` per F5 verbatim (used both for the `taken_at` field and as the canonical timestamp for `list()`'s newest-first sort, see acceptance criteria). Adds `chrono = { version = "0.4", features = ["serde"] }` to the workspace. Latest-packages skill must run when wiring.

**2. `ulid` for snapshot IDs.** Sortable + monotonic; gives free time-ordering without a separate timestamp index. Adds `ulid = "1"` (with `serde` feature) to the workspace.

**3. `blake3` for content hashing.** Fast, well-suited for content dedup. Adds `blake3 = "1"` to the workspace.

**3a. Promote `tempfile` to `[dependencies]`.** `tempfile` currently appears under `[dev-dependencies]` in `crates/inputforge-core/Cargo.toml:37` (used by existing tests). The snapshot module's atomic-write helpers use it in production code, so it must be a regular dependency. The latest-packages skill verifies the pinned version when wiring.

**4. Reuse `dirs` (not `directories`).** F5 spec mentions the `directories` crate, but `crates/inputforge-core/src/settings.rs` already uses `dirs::config_dir()`. F6 stays consistent with existing code; this is a small documentation drift in F5 that the implementation corrects silently.

### Settings extension (formerly "preferences module")

**5. F6 extends `AppSettings`, no new `preferences` module.** F5 introduces a `Preferences` struct conceptually distinct from the existing `AppSettings`. The brainstorm picked option C: fold prefs into `settings.toml` as a sub-table. Implementation: extend `AppSettings` with a `pub snapshot: SnapshotConfig` field. The file at `%APPDATA%/inputforge/settings.toml` gains a `[snapshot]` table. Single source of truth; no migration; no parallel module.

**6. `EngineCommand::ReloadPreferences` is renamed `EngineCommand::ReloadSettings`.** F5 calls it `ReloadPreferences` to match its proposed module name. Since the data lives in `AppSettings`, the command name should match. F15's settings UI will dispatch `ReloadSettings`.

### Snapshot file format

**7. Snapshot file = profile TOML + leading `[snapshot_meta]` table.** Single file per snapshot. The meta table lives at the top of the file; the rest is the full profile TOML. On restore, the snapshot module deserializes the file as `toml::Value`, removes the `snapshot_meta` table, and serializes the remainder to the live profile path. `index.toml` is purely a cache rebuilt from headers when missing or stale, no single point of failure.

**8. Storage layout is co-located with the profile** (already in F5 spec):

```
<profile-dir>/
├── TFM_Throttle.toml                  # the live profile
└── TFM_Throttle.snapshots/
    ├── index.toml                     # metadata cache
    ├── 01H8ZK0M9Q5R3WVT8XEN1GS2HF.toml   # snapshot file (meta + profile)
    └── ...
```

The `<stem>.snapshots/` folder name is computed from the profile path's file stem (Profile path `TFM_Throttle.toml` → `TFM_Throttle.snapshots/`). Move/copy/delete a profile and its snapshots travel with it.

`snapshots_dir_for(profile_path)` strips the **first extension only** (via `Path::file_stem`). If a future spec adopts `<stem>.profile.toml` as the canonical profile extension, this helper must be revisited, it would currently produce `TFM_Throttle.profile.snapshots/`, which may or may not be the intended behavior depending on whether `.profile.toml` is treated as a single compound extension.

### Restore semantics

**9. `RestoreSnapshot` uses write-then-reload.** Engine handler does, in order:
1. Take an `AutoBeforeRestore` snapshot of current profile state. Always fires (no hash dedup).
2. Snapshot module strips `[snapshot_meta]` from the snapshot file and writes the result atomically over the live profile path.
3. Engine reuses the same state-rebuild code path that handles `LoadProfile` (refresh `ModeState`, `DeviceCalibrationStore`, `current_mode`, `active_profile`). Implemented as a private helper extracted from the existing `LoadProfile` handler so both call sites share one source of truth.

This keeps state-mutation logic in one place, restore can never drift from load.

### Forced mode

**10. `ForcedMode` is a struct, not an enum.** F5 spec line 379 uses the word "enum" but describes a single sticky override shape with no variants. F6 commits a struct:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForcedMode {
    pub mode: String,
}
```

Field is on `AppState` (not `Engine`) because the GUI reads it through the existing `Arc<RwLock<AppState>>` snapshot pattern.

**11. Mode-change rules pause via gates at the two pause points.** Engine has **five `ModeState` mutation sites** (six counting `LoadProfile`'s full-replacement path, which is intentionally ungated). The five gated sites collapse to **two pause points**:

- `engine/output_handler.rs::process_pipeline_outputs`'s `Action::ChangeMode` arm covers four mutators (`ModeState::switch_to`, `push_temporary`, `go_previous`, `cycle`), all reachable through the helper `apply_mode_change` invoked from this single arm. **One** guard at the top of the arm gates all four.
- `engine/run.rs::tick`'s `ReleaseCallback::PopTemporaryMode` handler is the fifth mutator. **One** guard at the top of this handler gates it.
- The sixth site is `engine/run.rs`'s `LoadProfile` arm (full ModeState replacement). It is **intentionally ungated**, `LoadProfile` clears `mode_force` so the gate would be moot, and a forced override should not survive a profile change.

Both gates skip mutation when `state.mode_force.is_some()`. The forced state is read once per tick (folded into the existing `state.read()` block at `engine/run.rs:91-97` that already clones `mappings` and `mode_tree`, no additional read-lock acquisition; see "Engine wiring" for the exact pattern) into a local `mode_forced: bool` flag so we don't acquire the read lock per event.

`EngineCommand::ForceMode { mode }` bypasses the gate (per D15, idempotent on same-mode): when not currently forced, or when forced to a different mode, it calls `mode_state.switch_to(&mode, &tree)?`, sets `state.mode_force = Some(ForcedMode { mode })`, then runs `refresh_axes_for_mode_change` so vJoy outputs reflect the new mode immediately. When already forced to `mode`, it early-returns.

`EngineCommand::ReleaseMode` clears `state.mode_force = None`. The current mode stays where it was (last forced mode); subsequent rules can change it.

### Concurrency

**12. Atomic writes for snapshots; non-atomic for everything else.** Snapshot files are written via `tempfile::NamedTempFile::persist` (write to temp in same dir + rename), atomic on NTFS and POSIX **only when the temp file lives on the same volume as the destination**. The `snapshot::fs` helper enforces this by creating the temp file inside `<stem>.snapshots/` (same directory as destination). Profile and `AppSettings` writes stay as plain `std::fs::write` (current behavior; out of F6 scope to change).

**13. Single-thread engine guarantees serial commands.** All snapshot operations dispatch from the engine thread via `EngineCommand` handlers. Commands are processed serially in `process_commands`. There is no in-engine concurrency between two snapshot ops, or between a snapshot op and a profile write. External writers (other processes editing the same files) are out of scope; atomic writes give a best-effort guarantee against torn reads anyway.

### Additional decisions (post-review)

The following four decisions were resolved during the post-spec design review and added before plan-writing.

**14. `content_hash` is BLAKE3 over canonical TOML.** The hash input is `blake3(toml::to_string(toml::from_str(profile_bytes)?)?)`, **not** the raw profile bytes. A hand-formatted profile and the same profile after round-tripping through `toml::Value` produce the same `content_hash`. This prevents spurious dedup misses when the user manually reformats `TFM_Throttle.toml` (e.g., re-orders keys, adds/removes blank lines, normalizes indentation). Cost: one extra TOML round-trip per snapshot creation; acceptable given dedup runs only when `cfg.skip_if_unchanged` is set.

**15. `ForceMode` is idempotent on the same mode.** When `state.mode_force == Some(ForcedMode { mode: target })`, the dispatch arm early-returns: no `switch_to`, no axis refresh, no `pending_output_refresh` flag. When `state.mode_force == Some(different)`, the override rotates: `switch_to(&target, &tree)?`, replace `mode_force`, set `pending_output_refresh`. When `state.mode_force == None`, the full force path runs. Re-clicking F7's "Activate" banner with the same mode is free; clicking with a different mode rotates the override.

**16. `RestoreSnapshot` auto-rollbacks to `AutoBeforeRestore` if reload fails.** Engine handler sequence:
1. Create `AutoBeforeRestore` snapshot of current profile state. Capture its `SnapshotId`.
2. `snapshot::restore(&path, &target_id)`, atomic write of restored profile bytes.
3. `reload_profile_from_disk(&path)`. **On failure:** atomically write the `AutoBeforeRestore` body back to `path` (via `snapshot::restore(&path, &auto_before_id)`), call `reload_profile_from_disk(&path)` once more, log `tracing::error!` describing the original reload failure, propagate that error to the caller. If the rollback reload **also** fails: propagate the second error; engine in-memory state stays at the pre-restore snapshot's content and is now out of sync with disk. Worst-case requires a manual `LoadProfile` to recover.

The `AutoBeforeRestore` snapshot is preserved in the rolling buffer regardless of restore success/failure, so the user can always re-trigger the restore manually.

**17. `Engine::new` gains a `settings: AppSettings` parameter.** The constructor takes `settings` explicitly (not internally `AppSettings::load()`). Production callers, `crates/inputforge-core/src/main.rs:226` and engine test harnesses at `engine/tests.rs:132`, `:690`, `:1274`, pass `AppSettings::load()` (production) or a test-injected value (tests). This is a minor breaking change to the test harness; production callers update once. The trade-off is explicit testability: tests can inject custom `AppSettings` (e.g., reduced `max_count` for FIFO eviction tests) without monkey-patching the on-disk file. The `EngineCommand::ReloadSettings` handler does run `AppSettings::load()` internally to refresh the field after dispatch.

---

## Public API

### `inputforge_core::snapshot` (new module)

```rust
// crates/inputforge-core/src/snapshot/mod.rs
//! On-disk profile snapshot store.

pub use self::config::SnapshotConfig;
pub use self::types::{Snapshot, SnapshotId, SnapshotKind};

mod config;
mod fs;          // atomic-write helpers, layout calculations
mod hash;        // BLAKE3 wrapper
mod index;       // index.toml read/write/rebuild
mod types;

use std::path::Path;

use crate::error::Result;

/// Create a snapshot of the profile at `profile_path`.
///
/// `pinned` is derived from `kind` (no caller override at create time):
/// - `Manual` → `pinned = true` unconditionally. To unpin a manual
///   snapshot afterward, dispatch
///   `EngineCommand::PinSnapshot { id, pinned: false }`.
/// - `AutoSessionStart` / `AutoBeforeRestore` → `pinned = false`.
///
/// Returns `Ok(None)` when the snapshot was deduped against the latest
/// existing snapshot (only applies to `AutoSessionStart` when
/// `cfg.skip_if_unchanged` is true). `AutoBeforeRestore` and `Manual`
/// always create.
///
/// Does not call `prune`, caller is responsible for invoking that when
/// FIFO eviction is desired (engine handler calls both in sequence).
///
/// # Errors
///
/// Profile file missing, I/O failure, or serialization failure.
pub fn create(
    profile_path: &Path,
    kind: SnapshotKind,
    label: Option<String>,
    cfg: &SnapshotConfig,
) -> Result<Option<Snapshot>>;

/// List all snapshots for a profile, newest first.
///
/// Rebuilds `index.toml` from snapshot file headers if missing or stale.
///
/// # Errors
///
/// Snapshot directory unreadable or any snapshot file unparseable.
pub fn list(profile_path: &Path) -> Result<Vec<Snapshot>>;

/// Delete a snapshot by id.
///
/// # Errors
///
/// Snapshot not found, or I/O failure.
pub fn delete(profile_path: &Path, id: &SnapshotId) -> Result<()>;

/// Pin or unpin a snapshot. Pinned snapshots are exempt from FIFO eviction.
///
/// # Errors
///
/// Snapshot not found, or I/O failure.
pub fn pin(profile_path: &Path, id: &SnapshotId, pinned: bool) -> Result<()>;

/// Rename a snapshot's display label. Pass `None` to clear.
///
/// # Errors
///
/// Snapshot not found, or I/O failure.
pub fn rename(profile_path: &Path, id: &SnapshotId, label: Option<String>) -> Result<()>;

/// Restore the live profile to a snapshot's content.
///
/// Strips `[snapshot_meta]` from the snapshot file and atomically
/// writes the result to `profile_path`. Caller (engine) is responsible
/// for taking the `AutoBeforeRestore` snapshot first and for reloading
/// in-memory state after this call returns.
///
/// # Errors
///
/// Snapshot not found, or I/O failure.
pub fn restore(profile_path: &Path, id: &SnapshotId) -> Result<()>;

/// Apply FIFO eviction down to `cfg.max_count`, skipping pinned snapshots.
/// Returns the number of snapshots evicted.
///
/// # Errors
///
/// Snapshot directory unreadable, or eviction-time I/O failure.
pub fn prune(profile_path: &Path, cfg: &SnapshotConfig) -> Result<usize>;
```

### `inputforge_core::snapshot::types`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotId(pub Ulid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotKind {
    AutoSessionStart,
    AutoBeforeRestore,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub id:           SnapshotId,
    pub kind:         SnapshotKind,
    pub label:        Option<String>,
    pub taken_at:     DateTime<Utc>,
    pub content_hash: [u8; 32],   // BLAKE3 of the canonical-round-tripped profile TOML body (D14); stable across whitespace/comment/key-order changes in the on-disk file
    pub pinned:       bool,
}
```

### `inputforge_core::snapshot::config`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotConfig {
    pub max_count:         usize,   // default 10
    pub skip_if_unchanged: bool,    // default true
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self { max_count: 10, skip_if_unchanged: true }
    }
}
```

### `inputforge_core::settings` (extended)

```rust
// crates/inputforge-core/src/settings.rs (extended in F6)

use crate::snapshot::SnapshotConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub last_profile: Option<PathBuf>,

    /// Snapshot subsystem configuration. Edited via direct TOML edit
    /// from day one; F15 will ship a typed editor on top of this.
    #[serde(default)]
    pub snapshot: SnapshotConfig,
}
```

`#[serde(default)]` ensures pre-F6 `settings.toml` files (no `[snapshot]` table) load with default snapshot prefs without error. Save round-trip writes the table.

### `inputforge_core::state::AppState` (field added)

```rust
pub struct AppState {
    // ...existing fields...

    /// When `Some`, the engine is in a forced mode override.
    /// Mode-change rules are paused while this is `Some`. Cleared by
    /// `EngineCommand::ReleaseMode` or by loading a new profile.
    pub mode_force: Option<ForcedMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForcedMode {
    pub mode: String,
}
```

These derives are sufficient for projection through F7's `MetaSnapshot` GUI bridge; no additional traits required.

`AppState::new` and `AppState::with_profile` initialize `mode_force: None`.
`EngineCommand::LoadProfile` clears `mode_force` (a forced override doesn't survive a profile change).

### `inputforge_core::engine::EngineCommand` (variants added)

```rust
pub enum EngineCommand {
    // ...existing 9 variants...

    /// Force the engine into the named mode and pause mode-change rules.
    ForceMode { mode: String },

    /// Release any active forced-mode override.
    ReleaseMode,

    /// Re-read settings.toml and update in-memory `AppSettings`.
    /// Snapshot subsystem picks up the new `SnapshotConfig` on the next
    /// command processed. In-flight snapshot operations earlier in the
    /// same `process_commands` drain still see the old config.
    ReloadSettings,

    /// Take a snapshot of the active profile.
    CreateSnapshot { kind: SnapshotKind, label: Option<String> },

    /// Delete a snapshot by id.
    DeleteSnapshot { id: SnapshotId },

    /// Pin or unpin a snapshot.
    PinSnapshot { id: SnapshotId, pinned: bool },

    /// Rename (or clear the label of) a snapshot.
    RenameSnapshot { id: SnapshotId, label: Option<String> },

    /// Restore the active profile to the named snapshot.
    /// Engine handles `AutoBeforeRestore` internally before applying.
    RestoreSnapshot { id: SnapshotId },
}
```

8 new variants, total 17.

---

## Engine wiring

### Engine struct gains an `AppSettings` field

`crates/inputforge-core/src/engine/mod.rs::Engine` gains `settings: AppSettings`. Per D17, the constructor takes the settings as an explicit parameter, production callers pass `AppSettings::load()`, tests can inject a custom value. `EngineCommand::ReloadSettings` re-reads the file via `AppSettings::load()` and replaces the field. Snapshot calls take `&self.settings.snapshot`.

### Command dispatch (`engine/run.rs::handle_command`)

New arms added to the existing `match cmd { ... }`:

```rust
EngineCommand::ForceMode { mode } => {
    // Per D15: idempotent on same-mode; rotate on different-mode.
    let already_same = self.state.read()
        .mode_force.as_ref()
        .map(|f| f.mode == mode)
        .unwrap_or(false);
    if !already_same {
        let tree = /* read mode tree from active_profile */;
        self.mode_state.switch_to(&mode, &tree)?;
        let mut state = self.state.write();
        state.mode_force = Some(ForcedMode { mode: mode.clone() });
        state.current_mode = mode;
        drop(state);
        self.pending_output_refresh = true;
    }
}
EngineCommand::ReleaseMode => {
    let mut state = self.state.write();
    state.mode_force = None;
}
EngineCommand::ReloadSettings => {
    // AppSettings::load() already returns Default on missing/corrupt
    // file with a tracing::warn, same behavior as engine startup.
    self.settings = AppSettings::load();
}
EngineCommand::CreateSnapshot { kind, label } => {
    if let Some(path) = self.state.read().profile_path.clone() {
        let _ = snapshot::create(&path, kind, label, &self.settings.snapshot)?;
        let _ = snapshot::prune(&path, &self.settings.snapshot)?;
    }
}
EngineCommand::DeleteSnapshot { id } => { /* dispatch to snapshot::delete */ }
EngineCommand::PinSnapshot { id, pinned } => { /* snapshot::pin */ }
EngineCommand::RenameSnapshot { id, label } => { /* snapshot::rename */ }
EngineCommand::RestoreSnapshot { id } => {
    let path = self.state.read().profile_path.clone();
    if let Some(path) = path {
        // 1. AutoBeforeRestore (always fires; capture id for rollback).
        let auto = snapshot::create(
            &path,
            SnapshotKind::AutoBeforeRestore,
            None,
            &self.settings.snapshot,
        )?;
        // 2. Strip meta + atomic write profile TOML to live path.
        snapshot::restore(&path, &id)?;
        // 3. Reuse load-profile state rebuild, also clears mode_force.
        if let Err(reload_err) = self.reload_profile_from_disk(&path) {
            tracing::error!(
                target: "snapshot",
                ?reload_err,
                "restore reload failed; rolling back to AutoBeforeRestore",
            );
            // Per D16: auto-rollback to AutoBeforeRestore.
            if let Some(auto_snap) = auto {
                snapshot::restore(&path, &auto_snap.id)?;
                self.reload_profile_from_disk(&path)?;
            }
            return Err(reload_err);
        }
    }
}
```

`reload_profile_from_disk(&self, path: &Path)` is the new private helper extracted from the existing `LoadProfile` arm. The existing arm calls it. The arm continues to clear `mode_force = None` directly (a forced override should not survive a fresh `LoadProfile`).

### `LoadProfile` triggers AutoSessionStart

After `reload_profile_from_disk` returns successfully, the `LoadProfile` arm calls:

```rust
let _ = snapshot::create(
    &path,
    SnapshotKind::AutoSessionStart,
    None,
    &self.settings.snapshot,
)?;
let _ = snapshot::prune(&path, &self.settings.snapshot)?;
```

Snapshot module's `create` returns `Ok(None)` when `cfg.skip_if_unchanged && current_hash == latest_hash`; the engine ignores the return value.

### Mode-change pause gate

`process_pipeline_outputs` and the release-callback handler in `tick` gain a `mode_forced: bool` parameter:

- `tick` folds the `mode_forced` read into the existing once-per-tick `state.read()` block at `engine/run.rs:91-97` that already clones `mappings` and `mode_tree`. No additional read-lock acquisition:

  ```rust
  let (mappings, mode_tree, mode_forced) = {
      let s = self.state.read();
      (s.mappings.clone(), s.mode_tree.clone(), s.mode_force.is_some())
  };
  ```

- `process_pipeline_outputs` is updated to skip applying `Action::ChangeMode` effects (and any sub-mode push) via `apply_mode_change` when `mode_forced` is true; the rest of the pipeline still runs.
- The `ReleaseCallback::PopTemporaryMode` handler in `tick` early-returns when `mode_forced`.

`refresh_axes_for_mode_change` is still called when `ForceMode` is dispatched (above), that call sets `pending_output_refresh = true` so the next tick reapplies cached axes through the now-forced mode.

---

## On-disk formats

### `settings.toml` (extended, backward-compatible)

```toml
last_profile = "C:/Users/cornu/AppData/Roaming/inputforge/profiles/TFM Throttle.toml"

[snapshot]
max_count = 10
skip_if_unchanged = true
```

Older `settings.toml` files without `[snapshot]` continue to load (`#[serde(default)]`).

### Snapshot file `<id>.toml`

```toml
[snapshot_meta]
id = "01H8ZK0M9Q5R3WVT8XEN1GS2HF"
kind = "auto_session_start"
label = "before retest"          # optional
taken_at = "2026-04-28T10:24:11Z"
content_hash = "f3a1b2c4...32 hex bytes..."
pinned = false

# --- everything below is the full profile TOML, byte-for-byte ---
[profile]
id = "..."
name = "TFM Throttle"
startup_mode = "Default"

[modes]
Default = []
# ...
```

`Profile::from_toml` already accepts unknown top-level keys (no `deny_unknown_fields`), so the same file would happily round-trip through the profile parser if it were ever loaded directly, but the snapshot module always strips the meta table before writing back to a profile path so the live profile stays meta-free.

### `index.toml` cache

```toml
[[entries]]
id = "01H8ZK0M9Q5R3WVT8XEN1GS2HF"
kind = "auto_session_start"
label = ""
taken_at = "2026-04-28T10:24:11Z"
content_hash = "f3a1..."
pinned = false

[[entries]]
# ...
```

If `list()` finds `index.toml` **missing**, **unparseable** (TOML parse error), **truncated** (mid-write crash), or **out of sync** with the snapshot files on disk, it **rebuilds the index** from each `<id>.toml`'s `[snapshot_meta]` header. Specifically:

- Index file missing → rebuild.
- Index parse error / truncation → log `tracing::warn!` with path, rebuild.
- Snapshot file referenced in index but missing on disk → drop entry silently during rebuild.
- Snapshot file present on disk but missing from index → re-index it (orphan recovery).
- Snapshot file present but its `[snapshot_meta]` header is unparseable (e.g., malformed ULID, missing required field) → log `tracing::warn!` with path, skip the file (it does not appear in the rebuilt list and is treated as deleted for `prune` purposes).

Index rebuild is non-atomic (per decision #12), a crash during rebuild leaves the index in some intermediate state, which is itself recoverable on the next `list()` call by the same logic.

---

## Errors

`EngineError` (in `crates/inputforge-core/src/error.rs`) gains **six** snapshot-specific variants, keeping the existing flat-enum pattern:

```rust
#[error("snapshot not found: {id}")]
SnapshotNotFound { id: String },

#[error("snapshot file corrupt at {path}: {reason}")]
SnapshotCorrupt { path: PathBuf, reason: String },

#[error("snapshot directory I/O error at {path}: {source}")]
SnapshotDirIo { path: PathBuf, source: std::io::Error },

#[error("snapshot id is not a valid ULID: {value}")]
SnapshotIdInvalid { value: String },

#[error("could not create snapshot directory at {path}: {source}")]
SnapshotDirCreate { path: PathBuf, source: std::io::Error },

#[error("profile path has no parent directory: {path}")]
ProfilePathHasNoParent { path: PathBuf },
```

Existing `Io`, `ProfileParse`, `ProfileWrite` variants are reused where appropriate. `#[from]` on `std::io::Error` already provides automatic conversion at most call sites; the snapshot-specific variants are used at API boundaries where the path context matters.

Malformed ULIDs found inside a snapshot's `[snapshot_meta]` header during `list()` rebuild map to `SnapshotIdInvalid` (not `SnapshotCorrupt`) so F13's UI can present a clearer message. Index parse errors do **not** propagate as errors, they trigger the rebuild path described in "On-disk formats" (the caller never sees them).

---

## Module layout

```
crates/inputforge-core/src/
├── snapshot/
│   ├── mod.rs           # public API (create / list / delete / pin / rename / restore / prune)
│   ├── config.rs        # SnapshotConfig
│   ├── fs.rs            # atomic-write helpers, layout calc, snapshots_dir_for(profile_path)
│   ├── hash.rs          # BLAKE3 wrapper
│   ├── index.rs         # IndexFile read/write/rebuild
│   └── types.rs         # Snapshot, SnapshotId, SnapshotKind
├── settings.rs          # extended with snapshot: SnapshotConfig
├── state/
│   └── mod.rs           # AppState gains mode_force: Option<ForcedMode>; ForcedMode struct
├── engine/
│   ├── command.rs       # 8 new EngineCommand variants
│   ├── mod.rs           # Engine gains settings: AppSettings
│   ├── run.rs           # handle_command extended; reload_profile_from_disk extracted; mode-pause gate
│   └── output_handler.rs # process_pipeline_outputs takes mode_forced flag
├── error.rs             # 6 new snapshot variants (NotFound, Corrupt, DirIo, IdInvalid, DirCreate, ProfilePathHasNoParent)
└── lib.rs               # `pub mod snapshot;`
```

---

## Critical files (read these to execute the plan)

- `crates/inputforge-core/src/state/mod.rs:30-118`, `AppState` struct, `new` / `with_profile` constructors that need the `mode_force` initializer.
- `crates/inputforge-core/src/engine/command.rs:11-39`, `EngineCommand` enum; 8 variants append cleanly.
- `crates/inputforge-core/src/engine/run.rs:257-331`, `handle_command` dispatch; `LoadProfile` arm at lines 259-290 is the source for `reload_profile_from_disk` extraction.
- `crates/inputforge-core/src/engine/run.rs:104-190`, per-event loop; mode-pause gate goes here (mode_forced flag + release-callback skip + ChangeMode skip propagated through `process_pipeline_outputs`).
- `crates/inputforge-core/src/engine/output_handler.rs`, `process_pipeline_outputs` signature gains `mode_forced: bool`; ChangeMode-output handling early-skips when set.
- `crates/inputforge-core/src/engine/mod.rs:85-126`, `Engine::new` body; gains a `settings: AppSettings` parameter (D17), stores it on the struct.
- `crates/inputforge-core/src/engine/tests.rs:132`, `:690`, `:1274`, engine test harness call sites; pass test-injected `AppSettings` to the new constructor.
- `crates/inputforge-core/src/main.rs:226`, production engine construction; passes `AppSettings::load()`.
- `crates/inputforge-core/src/profile/mod.rs:122-142`, existing `Profile::load` / `save`; snapshot::restore calls `Profile::load` after writing.
- `crates/inputforge-core/src/profile/manager.rs`, synchronous file ops pattern reused by atomic-write helpers in `snapshot::fs`.
- `crates/inputforge-core/src/settings.rs:14-110`, `AppSettings`; extend with `snapshot: SnapshotConfig`; existing tests round-trip the extended struct.
- `crates/inputforge-core/src/error.rs:9-51`, `EngineError`; flat enum, append six new snapshot variants (per Errors section).
- `Cargo.toml:16-77`, workspace `[workspace.dependencies]`; add `chrono`, `ulid`, `blake3` (use latest-packages skill to pin versions).
- `crates/inputforge-core/Cargo.toml:21-37`, crate dependencies; add `chrono`, `ulid`, `blake3` references to the new workspace deps.

---

## Acceptance criteria

The following must all hold for F6 to merge:

**Public API**
- `inputforge_core::snapshot` module compiles with the surface above; all public items have doc comments.
- `EngineCommand` has 8 new variants; existing 9 variants unchanged.
- `AppState.mode_force: Option<ForcedMode>` field exists, initialized to `None` in both constructors.
- `AppSettings.snapshot: SnapshotConfig` field exists; missing `[snapshot]` table loads with defaults; round-trip writes the table.

**Snapshot module behavior**
- Round-trip: `create` → `list` returns one entry → `restore` writes a profile file that, when loaded via `Profile::from_toml`, parses to a `Profile` equal (`PartialEq`) to the original. Byte-equality through `toml::Value` strip-and-rewrite is *not* required (TOML ordering and comments are not preserved by the round-trip; semantic equality is).
- `create(kind = Manual, ...)` produces a snapshot with `pinned == true` unconditionally (no caller override at create time). `create(kind = AutoSessionStart | AutoBeforeRestore, ...)` produces a snapshot with `pinned == false`. Test asserts both directions (Manual auto-pinned; subsequent `pin(.., pinned: false)` successfully unpins).
- `content_hash` is stable across TOML reformatting (per D14): writing the same profile with different whitespace, comment placement, or top-level key order produces the same `content_hash`. Test: serialize a profile via `toml::to_string`, then re-serialize the same `toml::Value` after key shuffling, hash both byte streams via `snapshot::hash`, assert equal.
- Pruning honors `pinned`: with `max_count = 2`, creating 3 unpinned snapshots evicts the oldest; pinning the oldest first keeps it. Manual snapshots (pinned by default) are exempt from eviction unless the user explicitly unpins them.
- `AutoSessionStart` is skipped when `cfg.skip_if_unchanged && latest.content_hash == new_content_hash`. `create` returns `Ok(None)` in this case.
- `AutoBeforeRestore` always fires (no dedup, no skip path).
- `CreateSnapshot` / `RestoreSnapshot` / `DeleteSnapshot` / `PinSnapshot` / `RenameSnapshot` are silent no-ops (with a `tracing::warn!`) when no profile is loaded.
- Index recovery: deleting `index.toml` and calling `list` rebuilds it from snapshot file headers; orphaned files appear in the rebuilt list; index entries pointing at missing files are dropped silently.
- Atomic writes: torn-write tests (kill mid-write via tempfile fault injection) leave no partially-written snapshot file at the final path.
- `RestoreSnapshot` rollback (per D16): when `reload_profile_from_disk` is forced to fail (test injects a malformed profile body for the target snapshot, e.g., a TOML that parses but fails `Profile::from_toml`), the engine restores `AutoBeforeRestore` to the live profile and reloads it; engine in-memory state matches the pre-restore state; the `AutoBeforeRestore` snapshot remains in the rolling buffer; the original `reload_err` is propagated to the caller.
- `RestoreSnapshot` corrupt-target: when the target snapshot file's `[snapshot_meta]` header is malformed (e.g., invalid ULID, missing required field), `AutoBeforeRestore` still fires, the corrupt-target restore returns `SnapshotCorrupt` (or `SnapshotIdInvalid`), the live profile is unchanged, the rolling buffer reflects the new `AutoBeforeRestore`.
- `list()` orders by `taken_at` descending. Ties (same `taken_at` to ms precision) order by `id` lex-descending as a stable tiebreaker.

**Forced-mode behavior**
- `ForceMode { mode }` from an unforced state: switches `mode_state` to `mode`, sets `state.mode_force = Some(...)`, refreshes vJoy axes through the new mode.
- `ReleaseMode` clears `state.mode_force`; current mode unchanged.
- While forced: `Action::ChangeMode` outputs do not mutate `mode_state`; `ReleaseCallback::PopTemporaryMode` is a no-op; non-mode pipeline outputs (vJoy, keyboard) still execute.
- Loading a new profile clears `state.mode_force`.
- Restoring a snapshot clears `state.mode_force` (the snapshot's mode tree may not contain the forced mode).
- `ForceMode { mode }` returns `EngineError::ModeNotFound` when `mode` is not in the active profile's mode tree; state is unchanged.
- `ForceMode { mode }` while already forced to the **same** mode is a no-op (per D15): `switch_to` is not called, `pending_output_refresh` is not set, no additional `tracing::info!` "force-mode" event is emitted beyond the original.
- `ForceMode { mode }` while forced to a **different** mode rotates the override: a new `switch_to` runs, axes refresh, `mode_force` is replaced with the new target.

**Settings reload**
- Hand-edit `settings.toml` → dispatch `ReloadSettings` → next snapshot operation observes the new `max_count` / `skip_if_unchanged`.
- Hand-corrupt `settings.toml` to invalid TOML → `ReloadSettings` logs `tracing::warn!`, replaces in-memory `AppSettings` with `AppSettings::default()`, and **does not** overwrite the corrupt file on disk (no `save()` is called). A subsequent intentional `save()` (triggered by some other engine action) will overwrite the corruption with the in-memory defaults, the user is responsible for noticing the warn-log before that happens.

**Tracing**
- Every public `snapshot::*` op emits a structured `tracing` event (info on success, warn on recoverable failure, error on unrecoverable). Events include `id`, `kind`, `profile_path` where applicable.

**Tests**
- Unit tests cover each public function end-to-end with `tempfile::tempdir`-rooted profile dirs.
- Integration test: `LoadProfile` triggers `AutoSessionStart`; deduped on second load with identical content.
- Integration test: `RestoreSnapshot` end-to-end (create → mutate → restore round-trips bytes; `AutoBeforeRestore` fires).
- **Sequential serial test:** 8 `EngineCommand::CreateSnapshot` dispatches in sequence produce 8 distinct files with monotonically increasing `taken_at` and distinct `id`s; the 9th dispatch with `cfg.max_count = 8` evicts the first (oldest unpinned) snapshot. Exercises the production single-thread command-dispatch path; replaces the previous parallel-thread test (decision #13 makes parallel snapshot dispatch impossible in production).
- Round-trip: write `AppSettings` with custom `snapshot` config → read it back → values byte-identical.

**Workspace hygiene**
- `cargo build --workspace`, `cargo test --workspace`, and `cargo clippy --workspace -- -D warnings` all pass.
- `cargo build --features gui-egui` (egui app, current default) and `cargo build --features gui-dx` (Dioxus app under rewrite) both succeed unchanged (F6 doesn't touch GUI crates). Feature names per `crates/inputforge-app/Cargo.toml`, implementer must verify the actual feature names before merging; if the spec text is wrong, fix the spec.
- Latest-packages skill verifies the pinned versions of `chrono`, `ulid`, `blake3` against their registries.

---

## Out of scope for F6

- **Any GUI work.** F7 (chrome) consumes `mode_force`; F12/F13 dispatch the snapshot commands; F15 builds the settings editor. Each gets its own brainstorm.
- **Migrating profile / `AppSettings` writes to atomic.** Out of scope; current non-atomic writes stay. Only snapshot writes are atomic in F6.
- **Schema versioning of profile or snapshot files.** F13's open question; F6 ships v1 implicitly (no version field). F13 owns migration policy when restoring older snapshots after profile-schema changes.
- **Cross-process file locking.** Single-user desktop app; out of scope. Atomic writes give best-effort safety.
- **Snapshot pruning toasts.** F13 open question; engine fires no user-facing notification, just emits tracing events.
- **F13's "diff snapshot vs live" view.** Since `restore` writes the live profile through a `toml::Value` round-trip, the live profile may be byte-different from the original even when semantically equal. F6 commits `content_hash` over canonical TOML (per D14), so hash-equality answers "is this snapshot identical to live?" reliably. F13 will need to decide whether the visible diff is over raw bytes (will show whitespace drift) or canonical bytes (clean but less surprising), out of F6 scope.

---

## Risks

- **`reload_profile_from_disk` extraction touches the existing `LoadProfile` arm.** Low risk: the arm is currently ~30 lines of straightforward state mutation. Extraction is mechanical refactor; existing engine tests cover the behavior. Verify with `cargo test --package inputforge-core engine::tests`.
- **Mode-pause gate plumbing through `process_pipeline_outputs`.** Function signature gains a parameter; one external caller (engine `tick`). Low risk; explicit param beats reading `AppState` inside the function (avoids re-acquiring the lock per event).
- **`toml::Value` round-trip preservation of `[snapshot_meta]` strip.** TOML ordering and comments are not preserved by `toml::Value`. The strip-and-rewrite path produces a re-formatted profile TOML at restore time, the live profile file's exact byte representation may differ from the original even when the logical content is identical. Acceptable: the file remains semantically equivalent; users don't typically diff TOML byte-for-byte. Round-trip tests assert *parsed* equality, not byte equality.
- **`chrono` pulls more transitive deps than `time`.** Accepted; F13 will need relative-time formatting and chrono is more ergonomic.
- **ULID time-jump regressions.** ULIDs encode wall-clock time; if the system clock moves backward between snapshot creations, ULID lex-order can disagree with `taken_at` order. F6 sorts `list()` by `taken_at` (see acceptance criteria), so the user-visible newest-first order is unaffected. ULIDs remain unique (random low bits) so no collision risk. Out of scope to detect or warn on clock skew.
- **F7 `MetaSnapshot` projection.** F7 owns the GUI bridge that exposes `mode_force` to Dioxus; F6 only ships the engine-side field. F7 is responsible for adding `mode_force` to its `MetaSnapshot` projection in its own brainstorm/spec.

---

## Verification (end-to-end)

After implementation, before requesting review:

1. **Compile + lint clean:**
   ```
   cargo build --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
2. **Targeted snapshot integration:** run `cargo test --package inputforge-core snapshot`, every test in the new module passes.
3. **Engine integration:** run `cargo test --package inputforge-core engine`, existing engine tests pass unchanged; new tests for `ForceMode` / `ReleaseMode` / `RestoreSnapshot` pass.
4. **Hand-edit settings round-trip:**
   - Run a debug binary, observe `settings.toml` is created with `[snapshot]` table.
   - Edit `max_count = 3` by hand.
   - Trigger `ReloadSettings` (test command via the existing CLI hook or a unit test).
   - Issue 4 `LoadProfile` commands across distinct profile content (forces 4 `AutoSessionStart` snapshots since auto-session is unpinned and not deduped against differing content); verify only the 3 newest remain, the oldest auto snapshot was FIFO-evicted.
   - Then create a manual snapshot, verify it is pinned by default and survives a subsequent prune at `max_count = 1`.
5. **Index recovery hand-checks:**
   - Delete `index.toml` between two `list()` calls; second call rebuilds from headers.
   - Truncate `index.toml` mid-file (simulating a crashed write); `list()` logs `tracing::warn!`, rebuilds.
   - Hand-craft a malformed ULID into a snapshot file's `[snapshot_meta]` header; `list()` logs warn, skips that file in the result, returns the remaining snapshots.
6. **Hand-corrupt `settings.toml`:** write invalid TOML to the file → dispatch `ReloadSettings` → confirm warn log, in-memory defaults active, no on-disk overwrite (file still contains the invalid TOML until something else triggers a `save()`).
7. **GUI build still passes:** `cargo build --features gui-egui` (current default) and `cargo build --features gui-dx` both succeed; the egui GUI continues to work as the default. Implementer to verify exact feature names against `crates/inputforge-app/Cargo.toml`.

---

## Next steps

1. Commit this spec to git.
2. Ask the user to review.
3. On approval, invoke `superpowers:writing-plans` to produce the focused implementation plan for F6.
