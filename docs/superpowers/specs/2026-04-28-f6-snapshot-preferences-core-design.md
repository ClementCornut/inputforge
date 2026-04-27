# F6 — Snapshot Module + Settings Extension + Forced-Mode Plumbing in `inputforge-core`: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-28
**Parent specs:**
- [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md) — master rewrite plan, F6 is its first post-F5 feature
- [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md) — IA redesign that defines F6's surface

**Predecessors:** F1 (state bridge), F2 (design system), F3 (shell + tray), F4 (toast + dialog), F5 (IA redesign — design only, no code)
**Type:** core-only, no GUI surface
**Crate touched:** `crates/inputforge-core` only

---

## Context

F5 committed a clean-slate IA redesign for the Dioxus rewrite. Three engine-side capabilities the new IA depends on do not yet exist in `inputforge-core`:

1. **Snapshots.** F5's save model is *auto-commit + session undo + on-disk snapshots*. The on-disk snapshot layer needs an engine-owned module before any GUI can bind to it (F12 calibration save, F13 Profiles + Snapshots panel).
2. **Forced runtime mode.** F7's chrome shows a runtime-mode marker and "Activate / Release" banner; that requires an engine field that pauses mode-change rules and a pair of commands to flip it.
3. **User preferences.** Snapshot defaults (rolling-buffer count, content-hash dedup) are user-configurable. The spec wants direct-TOML-edit access from day one — no UI required — and an editor surface in F15.

F6 is the engine-side foundation for items 1–3. It adds zero pixels of GUI. After F6, F7 can bind to `mode_force`, F12/F13 can dispatch the new snapshot commands, and F15 can ship a typed editor on top of the same data layer.

This is also the point at which we adapt F5's "preferences module" naming to the codebase's existing reality: `crates/inputforge-core/src/settings.rs` already exists and persists `AppSettings { last_profile }` to `%APPDATA%/inputforge/settings.toml`. F6 extends `AppSettings` rather than introducing a parallel `preferences` module — the user-edited prefs live as a sub-table inside the existing TOML.

---

## Confirmed design decisions

The decisions below were validated during brainstorming dialogue; each is recorded in dependency order.

### Crate dependencies

**1. `chrono` for timestamps.** Snapshot `taken_at: DateTime<Utc>` per F5 verbatim. Adds `chrono = { version = "0.4", features = ["serde"] }` to the workspace. Latest-packages skill must run when wiring.

**2. `ulid` for snapshot IDs.** Sortable + monotonic; gives free time-ordering without a separate timestamp index. Adds `ulid = "1"` (with `serde` feature) to the workspace.

**3. `blake3` for content hashing.** Fast, well-suited for content dedup. Adds `blake3 = "1"` to the workspace.

**4. Reuse `dirs` (not `directories`).** F5 spec mentions the `directories` crate, but `crates/inputforge-core/src/settings.rs` already uses `dirs::config_dir()`. F6 stays consistent with existing code; this is a small documentation drift in F5 that the implementation corrects silently.

### Settings extension (formerly "preferences module")

**5. F6 extends `AppSettings`, no new `preferences` module.** F5 introduces a `Preferences` struct conceptually distinct from the existing `AppSettings`. The brainstorm picked option C: fold prefs into `settings.toml` as a sub-table. Implementation: extend `AppSettings` with a `pub snapshot: SnapshotConfig` field. The file at `%APPDATA%/inputforge/settings.toml` gains a `[snapshot]` table. Single source of truth; no migration; no parallel module.

**6. `EngineCommand::ReloadPreferences` is renamed `EngineCommand::ReloadSettings`.** F5 calls it `ReloadPreferences` to match its proposed module name. Since the data lives in `AppSettings`, the command name should match. F15's settings UI will dispatch `ReloadSettings`.

### Snapshot file format

**7. Snapshot file = profile TOML + leading `[snapshot_meta]` table.** Single file per snapshot. The meta table lives at the top of the file; the rest is the full profile TOML. On restore, the snapshot module deserializes the file as `toml::Value`, removes the `snapshot_meta` table, and serializes the remainder to the live profile path. `index.toml` is purely a cache rebuilt from headers when missing or stale — no single point of failure.

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

### Restore semantics

**9. `RestoreSnapshot` uses write-then-reload.** Engine handler does, in order:
1. Take an `AutoBeforeRestore` snapshot of current profile state. Always fires (no hash dedup).
2. Snapshot module strips `[snapshot_meta]` from the snapshot file and writes the result atomically over the live profile path.
3. Engine reuses the same state-rebuild code path that handles `LoadProfile` (refresh `ModeState`, `DeviceCalibrationStore`, `current_mode`, `active_profile`). Implemented as a private helper extracted from the existing `LoadProfile` handler so both call sites share one source of truth.

This keeps state-mutation logic in one place — restore can never drift from load.

### Forced mode

**10. `ForcedMode` is a struct, not an enum.** F5 spec line 379 uses the word "enum" but describes a single sticky override shape with no variants. F6 commits a struct:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForcedMode {
    pub mode: String,
}
```

Field is on `AppState` (not `Engine`) because the GUI reads it through the existing `Arc<RwLock<AppState>>` snapshot pattern.

**11. Mode-change rules pause via gate at the two mutation points.** Engine has exactly two places that mutate `ModeState`:
- `crates/inputforge-core/src/engine/output_handler.rs::process_pipeline_outputs` — handles `Action::ChangeMode` outputs
- `crates/inputforge-core/src/engine/run.rs::tick` — handles `ReleaseCallback::PopTemporaryMode`

Both gain an early-return guard that skips mutation when `state.mode_force.is_some()`. The forced state is read once per tick (before the event loop) into a local `mode_forced: bool` flag so we don't acquire the read lock per event.

`EngineCommand::ForceMode { mode }` bypasses the gate: it calls `mode_state.switch_to(&mode, &tree)?`, sets `state.mode_force = Some(ForcedMode { mode })`, then runs `refresh_axes_for_mode_change` so vJoy outputs reflect the new mode immediately.

`EngineCommand::ReleaseMode` clears `state.mode_force = None`. The current mode stays where it was (last forced mode); subsequent rules can change it.

### Concurrency

**12. Atomic writes for snapshots; non-atomic for everything else.** Snapshot files are written via `tempfile::NamedTempFile::persist` (write to temp in same dir + rename) — atomic on NTFS and POSIX. Profile and `AppSettings` writes stay as plain `std::fs::write` (current behavior; out of F6 scope to change).

**13. Single-thread engine guarantees serial commands.** All snapshot operations dispatch from the engine thread via `EngineCommand` handlers. Commands are processed serially in `process_commands`. There is no in-engine concurrency between two snapshot ops, or between a snapshot op and a profile write. External writers (other processes editing the same files) are out of scope; atomic writes give a best-effort guarantee against torn reads anyway.

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
/// Manual snapshots are pinned by default; auto snapshots are unpinned.
///
/// Returns `Ok(None)` when the snapshot was deduped against the latest
/// existing snapshot (only applies to `AutoSessionStart` when
/// `cfg.skip_if_unchanged` is true). `AutoBeforeRestore` and `Manual`
/// always create.
///
/// Does not call `prune` — caller is responsible for invoking that when
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
    pub content_hash: [u8; 32],   // BLAKE3 of the profile TOML body
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
    /// Snapshot subsystem picks up the new `SnapshotConfig` immediately.
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

`crates/inputforge-core/src/engine/mod.rs::Engine` gains `settings: AppSettings`. Loaded once during engine construction via `AppSettings::load()`. `EngineCommand::ReloadSettings` re-reads the file and replaces the field. Snapshot calls take `&self.settings.snapshot`.

### Command dispatch (`engine/run.rs::handle_command`)

New arms added to the existing `match cmd { ... }`:

```rust
EngineCommand::ForceMode { mode } => {
    let tree = /* read mode tree from active_profile */;
    self.mode_state.switch_to(&mode, &tree)?;
    let mut state = self.state.write();
    state.mode_force = Some(ForcedMode { mode: mode.clone() });
    state.current_mode = mode;
    drop(state);
    self.pending_output_refresh = true;
}
EngineCommand::ReleaseMode => {
    let mut state = self.state.write();
    state.mode_force = None;
}
EngineCommand::ReloadSettings => {
    // AppSettings::load() already returns Default on missing/corrupt
    // file with a tracing::warn — same behavior as engine startup.
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
        // 1. AutoBeforeRestore (always fires)
        let _ = snapshot::create(
            &path,
            SnapshotKind::AutoBeforeRestore,
            None,
            &self.settings.snapshot,
        )?;
        // 2. Strip meta + write profile TOML to live path
        snapshot::restore(&path, &id)?;
        // 3. Reuse load-profile state rebuild — also clears mode_force
        //    since the restored profile may not contain the forced mode.
        self.reload_profile_from_disk(&path)?;
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

- `tick` reads `mode_forced = state.mode_force.is_some()` once before the event loop.
- `process_pipeline_outputs` is updated to skip applying `Action::ChangeMode` effects (and any sub-mode push) when `mode_forced` is true; the rest of the pipeline still runs.
- The `ReleaseCallback::PopTemporaryMode` handler in `tick` early-returns when `mode_forced`.

`refresh_axes_for_mode_change` is still called when `ForceMode` is dispatched (above) — that call sets `pending_output_refresh = true` so the next tick reapplies cached axes through the now-forced mode.

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

`Profile::from_toml` already accepts unknown top-level keys (no `deny_unknown_fields`), so the same file would happily round-trip through the profile parser if it were ever loaded directly — but the snapshot module always strips the meta table before writing back to a profile path so the live profile stays meta-free.

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

If `list()` finds the index missing, malformed, or out of sync with the snapshot files on disk (orphaned files / dangling entries), it rebuilds the index from each file's `[snapshot_meta]` header. Orphaned files are re-indexed; dangling entries are dropped silently.

---

## Errors

`EngineError` (in `crates/inputforge-core/src/error.rs`) gains snapshot-specific variants, keeping the existing flat-enum pattern:

```rust
#[error("snapshot not found: {id}")]
SnapshotNotFound { id: String },

#[error("snapshot file corrupt at {path}: {reason}")]
SnapshotCorrupt { path: PathBuf, reason: String },

#[error("snapshot directory I/O error at {path}: {source}")]
SnapshotDirIo { path: PathBuf, source: std::io::Error },
```

Existing `Io`, `ProfileParse`, `ProfileWrite` variants are reused where appropriate. `#[from]` on `std::io::Error` already provides automatic conversion at most call sites; the snapshot-specific variants are used at API boundaries where the path context matters.

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
├── error.rs             # SnapshotNotFound / SnapshotCorrupt / SnapshotDirIo variants
└── lib.rs               # `pub mod snapshot;`
```

---

## Critical files (read these to execute the plan)

- `crates/inputforge-core/src/state/mod.rs:30-118` — `AppState` struct, `new` / `with_profile` constructors that need the `mode_force` initializer.
- `crates/inputforge-core/src/engine/command.rs:11-39` — `EngineCommand` enum; 8 variants append cleanly.
- `crates/inputforge-core/src/engine/run.rs:257-331` — `handle_command` dispatch; `LoadProfile` arm at lines 259-290 is the source for `reload_profile_from_disk` extraction.
- `crates/inputforge-core/src/engine/run.rs:104-190` — per-event loop; mode-pause gate goes here (mode_forced flag + release-callback skip + ChangeMode skip propagated through `process_pipeline_outputs`).
- `crates/inputforge-core/src/engine/output_handler.rs` — `process_pipeline_outputs` signature gains `mode_forced: bool`; ChangeMode-output handling early-skips when set.
- `crates/inputforge-core/src/profile/mod.rs:122-142` — existing `Profile::load` / `save`; snapshot::restore calls `Profile::load` after writing.
- `crates/inputforge-core/src/profile/manager.rs` — synchronous file ops pattern reused by atomic-write helpers in `snapshot::fs`.
- `crates/inputforge-core/src/settings.rs:14-110` — `AppSettings`; extend with `snapshot: SnapshotConfig`; existing tests round-trip the extended struct.
- `crates/inputforge-core/src/error.rs:9-51` — `EngineError`; flat enum, append three variants.
- `Cargo.toml:16-77` — workspace `[workspace.dependencies]`; add `chrono`, `ulid`, `blake3` (use latest-packages skill to pin versions).
- `crates/inputforge-core/Cargo.toml:21-37` — crate dependencies; add `chrono`, `ulid`, `blake3` references to the new workspace deps.

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
- `create(kind = Manual, ...)` produces a snapshot with `pinned = true` by default. `create(kind = AutoSessionStart | AutoBeforeRestore, ...)` produces a snapshot with `pinned = false`.
- Pruning honors `pinned`: with `max_count = 2`, creating 3 unpinned snapshots evicts the oldest; pinning the oldest first keeps it. Manual snapshots (pinned by default) are exempt from eviction unless the user explicitly unpins them.
- `AutoSessionStart` is skipped when `cfg.skip_if_unchanged && latest.content_hash == new_content_hash`. `create` returns `Ok(None)` in this case.
- `AutoBeforeRestore` always fires (no dedup, no skip path).
- `CreateSnapshot` / `RestoreSnapshot` / `DeleteSnapshot` / `PinSnapshot` / `RenameSnapshot` are silent no-ops (with a `tracing::warn!`) when no profile is loaded.
- Index recovery: deleting `index.toml` and calling `list` rebuilds it from snapshot file headers; orphaned files appear in the rebuilt list; index entries pointing at missing files are dropped silently.
- Atomic writes: torn-write tests (kill mid-write via tempfile fault injection) leave no partially-written snapshot file at the final path.

**Forced-mode behavior**
- `ForceMode { mode }` switches `mode_state` to `mode`, sets `state.mode_force = Some(...)`, refreshes vJoy axes through the new mode.
- `ReleaseMode` clears `state.mode_force`; current mode unchanged.
- While forced: `Action::ChangeMode` outputs do not mutate `mode_state`; `ReleaseCallback::PopTemporaryMode` is a no-op; non-mode pipeline outputs (vJoy, keyboard) still execute.
- Loading a new profile clears `state.mode_force`.
- Restoring a snapshot clears `state.mode_force` (the snapshot's mode tree may not contain the forced mode).
- `ForceMode { mode }` returns `EngineError::ModeNotFound` when `mode` is not in the active profile's mode tree; state is unchanged.

**Settings reload**
- Hand-edit `settings.toml` → dispatch `ReloadSettings` → next snapshot operation observes the new `max_count` / `skip_if_unchanged`.

**Tracing**
- Every public `snapshot::*` op emits a structured `tracing` event (info on success, warn on recoverable failure, error on unrecoverable). Events include `id`, `kind`, `profile_path` where applicable.

**Tests**
- Unit tests cover each public function end-to-end with `tempfile::tempdir`-rooted profile dirs.
- Integration test: `LoadProfile` triggers `AutoSessionStart`; deduped on second load with identical content.
- Integration test: `RestoreSnapshot` end-to-end (create → mutate → restore round-trips bytes; `AutoBeforeRestore` fires).
- Concurrency: 8 threads creating snapshots in parallel produce 8 distinct files, no torn writes (regression test for atomic write path; uses `std::thread::scope`).
- Round-trip: write `AppSettings` with custom `snapshot` config → read it back → values byte-identical.

**Workspace hygiene**
- `cargo build --workspace`, `cargo test --workspace`, and `cargo clippy --workspace -- -D warnings` all pass.
- `cargo build --features gui-dioxus` and `cargo build --features gui-egui` still both succeed unchanged (F6 doesn't touch GUI crates).
- Latest-packages skill verifies the pinned versions of `chrono`, `ulid`, `blake3` against their registries.

---

## Out of scope for F6

- **Any GUI work.** F7 (chrome) consumes `mode_force`; F12/F13 dispatch the snapshot commands; F15 builds the settings editor. Each gets its own brainstorm.
- **Migrating profile / `AppSettings` writes to atomic.** Out of scope; current non-atomic writes stay. Only snapshot writes are atomic in F6.
- **Schema versioning of profile or snapshot files.** F13's open question; F6 ships v1 implicitly (no version field). F13 owns migration policy when restoring older snapshots after profile-schema changes.
- **Cross-process file locking.** Single-user desktop app; out of scope. Atomic writes give best-effort safety.
- **Snapshot pruning toasts.** F13 open question; engine fires no user-facing notification, just emits tracing events.

---

## Risks

- **`reload_profile_from_disk` extraction touches the existing `LoadProfile` arm.** Low risk: the arm is currently ~30 lines of straightforward state mutation. Extraction is mechanical refactor; existing engine tests cover the behavior. Verify with `cargo test --package inputforge-core engine::tests`.
- **Mode-pause gate plumbing through `process_pipeline_outputs`.** Function signature gains a parameter; one external caller (engine `tick`). Low risk; explicit param beats reading `AppState` inside the function (avoids re-acquiring the lock per event).
- **`toml::Value` round-trip preservation of `[snapshot_meta]` strip.** TOML ordering and comments are not preserved by `toml::Value`. The strip-and-rewrite path produces a re-formatted profile TOML at restore time — the live profile file's exact byte representation may differ from the original even when the logical content is identical. Acceptable: the file remains semantically equivalent; users don't typically diff TOML byte-for-byte. Round-trip tests assert *parsed* equality, not byte equality.
- **`chrono` pulls more transitive deps than `time`.** Accepted; F13 will need relative-time formatting and chrono is more ergonomic.
- **ULID time-jump regressions.** ULIDs encode wall-clock time; if the system clock moves backward between calls, sorted order can briefly invert. Out of scope; F6 does not depend on strict monotonicity beyond `taken_at`.

---

## Verification (end-to-end)

After implementation, before requesting review:

1. **Compile + lint clean:**
   ```
   cargo build --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
2. **Targeted snapshot integration:** run `cargo test --package inputforge-core snapshot` — every test in the new module passes.
3. **Engine integration:** run `cargo test --package inputforge-core engine` — existing engine tests pass unchanged; new tests for `ForceMode` / `ReleaseMode` / `RestoreSnapshot` pass.
4. **Hand-edit settings round-trip:**
   - Run a debug binary, observe `settings.toml` is created with `[snapshot]` table.
   - Edit `max_count = 3` by hand.
   - Trigger `ReloadSettings` (test command via the existing CLI hook or a unit test).
   - Issue 4 `LoadProfile` commands across distinct profile content (forces 4 `AutoSessionStart` snapshots since auto-session is unpinned and not deduped against differing content); verify only the 3 newest remain — the oldest auto snapshot was FIFO-evicted.
   - Then create a manual snapshot, verify it is pinned by default and survives a subsequent prune at `max_count = 1`.
5. **GUI build still passes:** `cargo build --features gui-egui` and `cargo build --features gui-dioxus` both succeed; the egui GUI continues to work as the default.

---

## Next steps

1. Commit this spec to git.
2. Ask the user to review.
3. On approval, invoke `superpowers:writing-plans` to produce the focused implementation plan for F6.
