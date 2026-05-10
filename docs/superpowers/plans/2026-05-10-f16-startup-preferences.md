# F16 Startup Preferences Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two persisted preferences (launch InputForge at OS sign-in, start minimized to tray) wired through a new platform-abstracted autostart crate, an engine command pair, and a settings-panel section.

**Architecture:** A new `inputforge-autostart` crate exposes an `AutostartManager` trait with concrete Windows (HKCU registry via `auto-launch 0.6`) and Linux (XDG `~/.config/autostart`) impls plus a `NoOp` fallback. `AppSettings` gains a nested `[startup]` sub-table mirrored on `AppState.startup`. Two new struct-style `EngineCommand` variants (`SetAutostart`, `SetStartMinimizedToTray`) own the OS write atomically with the settings persistence; `Engine::new` reconciles OS state to settings on startup and unconditionally re-pushes argv when autostart is enabled. The GUI adds a `StartupSection` above `SnapshotsSection` that follows the F15 polled-into-local-Signal pattern.

**Tech Stack:** Rust 2024 edition, Dioxus 0.7.6 (desktop + ssr), `auto-launch 0.6`, `serde 1` + `toml 1`, `thiserror 2`, `tracing`, `parking_lot::RwLock`.

**Spec:** [`docs/superpowers/specs/2026-05-10-f16-startup-preferences-design.md`](../specs/2026-05-10-f16-startup-preferences-design.md)

---

## File Structure

**Created**

- `crates/inputforge-autostart/Cargo.toml`, manifest with optional `mock` feature.
- `crates/inputforge-autostart/src/lib.rs`, trait, factory, re-exports.
- `crates/inputforge-autostart/src/error.rs`, `AutostartError` variants.
- `crates/inputforge-autostart/src/noop.rs`, fallback impl.
- `crates/inputforge-autostart/src/mock.rs`, test double (gated `#[cfg(feature = "mock")]`).
- `crates/inputforge-autostart/src/windows.rs`, HKCU\...\Run impl (`#[cfg(target_os = "windows")]`).
- `crates/inputforge-autostart/src/linux.rs`, `~/.config/autostart` impl (`#[cfg(target_os = "linux")]`).
- `crates/inputforge-gui-dx/src/frame/settings_panel/startup_section.rs`, component + tests.

**Modified**

- `Cargo.toml`, add workspace member + workspace dep entry for the new crate, add `auto-launch` workspace dep.
- `crates/inputforge-core/Cargo.toml`, runtime dep on `inputforge-autostart`, dev dep with `mock` feature.
- `crates/inputforge-core/src/settings.rs`, new `StartupSettings` struct + `AppSettings.startup` field + tests.
- `crates/inputforge-core/src/state/mod.rs`, new `AppState.startup` field + initialiser.
- `crates/inputforge-core/src/engine/mod.rs`, `Engine.autostart` field, `Engine::new` 9th arg, mirror init, startup reconciliation.
- `crates/inputforge-core/src/engine/command.rs`, two new `EngineCommand` variants + Debug/PartialEq tests.
- `crates/inputforge-core/src/engine/run.rs`, two new handler arms in `handle_command`, mirror in `ReloadSettings`.
- `crates/inputforge-core/src/engine/tests.rs`, sweep all 19 `Engine::new(` call sites + 4 helpers; new behaviour tests.
- `crates/inputforge-app/Cargo.toml`, dep on `inputforge-autostart`.
- `crates/inputforge-app/src/main.rs`, factory call, `resolve_start_minimized` helper + table test, call-site update.
- `crates/inputforge-gui-dx/src/context.rs`, `SettingsSnapshot.startup` field, `from_state` mirror.
- `crates/inputforge-gui-dx/src/frame/settings_panel/mod.rs`, `mod startup_section;`, render `StartupSection {}` above `SnapshotsSection {}`, add Phase 11 tests.

---

## Engineer notes (read first)

1. **Smoke tests in steps use `cargo`.** `dx run -p inputforge-app` is for manual interactive verification at the end (Phase 12), not for inline step verification.
2. **No em-dash, en-dash, or double-hyphen substitutes** in code, comments, commit messages, or this plan's expansion. Use comma, colon, semicolon, period, or parentheses.
3. **Conventional commits with mandatory scope.** Use the `conventional-commits` skill before each commit. Do NOT use the `style` type for CSS-only changes; classify by intent.
4. **Frequent commits.** One per task.
5. **Never edit `assets/icons/svg/*.svg`** for rendering bugs; F16 introduces no icons, but if you reach for SVG to fix a visual issue, fix at CSS level instead.
6. **Match existing patterns.** `SnapshotsSection` (see `crates/inputforge-gui-dx/src/frame/settings_panel/snapshots_section.rs`) and `SetSnapshotConfig` (see `crates/inputforge-core/src/engine/run.rs:555-614`) are the literal templates for the GUI section and engine handler shapes respectively.
7. **`Engine::new` already has 8 args under `#[allow(clippy::too_many_arguments)]`.** Keep the existing allow attribute; do NOT introduce a builder.

---

# Phase 1: `inputforge-autostart` crate scaffold

This phase creates a self-contained crate with no dependents. Each task here is a standalone commit; the crate must build and test green before Phase 2 starts depending on it.

### Task 1.1: Create the crate skeleton (Cargo.toml + empty lib.rs + workspace wiring)

**Files:**
- Create: `crates/inputforge-autostart/Cargo.toml`
- Create: `crates/inputforge-autostart/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Add workspace dep entry for `auto-launch`**

In `Cargo.toml` (workspace root), under `[workspace.dependencies]` (after the existing `arboard` entry near line 86), add:

```toml
# Autostart (Windows registry, Linux XDG)
auto-launch = "0.6"
```

- [ ] **Step 2: Register the new crate as a workspace member**

In `Cargo.toml` (workspace root), update the `members` line on line 2 from:

```toml
members = ["crates/inputforge-core", "crates/inputforge-gui-dx", "crates/inputforge-app"]
```

to:

```toml
members = [
    "crates/inputforge-core",
    "crates/inputforge-gui-dx",
    "crates/inputforge-app",
    "crates/inputforge-autostart",
]
```

- [ ] **Step 3: Add the workspace dep entry for the new crate**

In `Cargo.toml` (workspace root), under `[workspace.dependencies]` next to the other internal-crate entries (around line 88-90), add:

```toml
inputforge-autostart = { path = "crates/inputforge-autostart" }
```

- [ ] **Step 4: Create the crate manifest**

Create `crates/inputforge-autostart/Cargo.toml`:

```toml
[package]
name = "inputforge-autostart"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
description = "Autostart manager (Launch at OS sign-in) for InputForge"
repository.workspace = true
readme.workspace = true
keywords.workspace = true
categories.workspace = true

[features]
# `mock` exposes `MockAutostart` for downstream test code (e.g., inputforge-core tests).
# Disabled by default so release binaries do not pull the mock in.
mock = []

[dependencies]
auto-launch = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 5: Create an empty `src/lib.rs` so the crate compiles**

Create `crates/inputforge-autostart/src/lib.rs`:

```rust
//! Autostart manager for InputForge: writes OS-level launch-at-sign-in state
//! (HKCU registry on Windows, `~/.config/autostart/*.desktop` on Linux).
//!
//! See `docs/superpowers/specs/2026-05-10-f16-startup-preferences-design.md`.

// Modules and re-exports land in subsequent tasks.
```

- [ ] **Step 6: Verify the workspace still builds**

Run: `cargo check --workspace`
Expected: clean build, no errors. The new crate compiles as an empty library.

- [ ] **Step 7: Commit**

```powershell
git add Cargo.toml crates/inputforge-autostart/
git commit -m "feat(autostart): scaffold inputforge-autostart crate"
```

---

### Task 1.2: Define `AutostartError`

**Files:**
- Create: `crates/inputforge-autostart/src/error.rs`
- Modify: `crates/inputforge-autostart/src/lib.rs`

- [ ] **Step 1: Write the error variants and a Debug-format roundtrip test**

Create `crates/inputforge-autostart/src/error.rs`:

```rust
//! Error types for autostart operations.

use std::io;

/// Errors returned by [`AutostartManager`](crate::AutostartManager) impls.
#[derive(Debug, thiserror::Error)]
pub enum AutostartError {
    /// The current platform has no supported autostart backend (or
    /// `std::env::current_exe()` failed during construction).
    #[error("autostart not supported on this platform")]
    NotSupported,

    /// HKCU\...\Run registry write was rejected.
    #[error("registry write denied")]
    RegistryDenied,

    /// XDG autostart directory is missing or read-only.
    #[error("autostart directory not writable: {0}")]
    DirectoryNotWritable(String),

    /// Other I/O error from the backend.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// Opaque wrapper around the underlying `auto_launch::Error` (or any
    /// non-classified backend error). The string is for log output only;
    /// callers must not pattern-match on its contents.
    #[error("backend error: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_message_for_not_supported() {
        let err = AutostartError::NotSupported;
        assert_eq!(err.to_string(), "autostart not supported on this platform");
    }

    #[test]
    fn io_variant_wraps_via_from() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "no write");
        let err: AutostartError = io_err.into();
        assert!(matches!(err, AutostartError::Io(_)));
        assert!(err.to_string().starts_with("io error: "));
    }

    #[test]
    fn backend_string_propagates_to_display() {
        let err = AutostartError::Backend("oops".to_owned());
        assert_eq!(err.to_string(), "backend error: oops");
    }

    #[test]
    fn directory_not_writable_includes_path_in_display() {
        let err = AutostartError::DirectoryNotWritable("/tmp/x".to_owned());
        assert_eq!(err.to_string(), "autostart directory not writable: /tmp/x");
    }
}
```

- [ ] **Step 2: Wire the module and re-export from `lib.rs`**

Replace `crates/inputforge-autostart/src/lib.rs` with:

```rust
//! Autostart manager for InputForge: writes OS-level launch-at-sign-in state
//! (HKCU registry on Windows, `~/.config/autostart/*.desktop` on Linux).
//!
//! See `docs/superpowers/specs/2026-05-10-f16-startup-preferences-design.md`.

mod error;

pub use error::AutostartError;
```

- [ ] **Step 3: Run the new tests**

Run: `cargo test -p inputforge-autostart`
Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```powershell
git add crates/inputforge-autostart/
git commit -m "feat(autostart): add AutostartError variants"
```

---

### Task 1.3: Define `AutostartManager` trait

**Files:**
- Modify: `crates/inputforge-autostart/src/lib.rs`

- [ ] **Step 1: Add the trait definition**

In `crates/inputforge-autostart/src/lib.rs`, append after the `pub use error::AutostartError;` line:

```rust
/// Platform-agnostic interface for the OS autostart store.
///
/// Implementations write to HKCU\...\Run on Windows and to
/// `~/.config/autostart/*.desktop` on Linux. The trait is intentionally
/// *not* `Send`/`Sync`: the engine owns it on its single thread and never
/// shares the instance across threads.
///
/// `args` is passed at call time so the engine decides whether to include
/// `--start-minimized`; concrete impls are dumb about that flag.
pub trait AutostartManager {
    /// Read the OS autostart state for this app.
    ///
    /// # Errors
    ///
    /// Returns an [`AutostartError`] when the backend cannot read the
    /// registry / desktop file (permissions, IO, malformed entry).
    fn is_enabled(&self) -> Result<bool, AutostartError>;

    /// Enable or disable the OS autostart entry. When enabling, `args` is
    /// the argv tail registered with the entry (e.g., `&["--start-minimized"]`).
    /// When disabling, `args` is ignored.
    ///
    /// # Errors
    ///
    /// Returns an [`AutostartError`] when the backend rejects the write
    /// (permissions, IO, registry denial).
    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError>;
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p inputforge-autostart`
Expected: clean build.

- [ ] **Step 3: Commit**

```powershell
git add crates/inputforge-autostart/src/lib.rs
git commit -m "feat(autostart): add AutostartManager trait"
```

---

### Task 1.4: Add `NoOpAutostart` fallback

**Files:**
- Create: `crates/inputforge-autostart/src/noop.rs`
- Modify: `crates/inputforge-autostart/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/inputforge-autostart/src/noop.rs`:

```rust
//! Fallback impl used when no platform backend is available (e.g., when
//! `std::env::current_exe()` fails). Reports disabled and rejects writes.

use crate::{AutostartError, AutostartManager};

#[derive(Debug, Default)]
pub(crate) struct NoOpAutostart;

impl NoOpAutostart {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl AutostartManager for NoOpAutostart {
    fn is_enabled(&self) -> Result<bool, AutostartError> {
        Ok(false)
    }

    fn set_enabled(&mut self, _enabled: bool, _args: &[&str]) -> Result<(), AutostartError> {
        Err(AutostartError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_enabled_reports_false() {
        let m = NoOpAutostart::new();
        assert_eq!(m.is_enabled().unwrap(), false);
    }

    #[test]
    fn set_enabled_true_returns_not_supported() {
        let mut m = NoOpAutostart::new();
        let err = m.set_enabled(true, &["--start-minimized"]).unwrap_err();
        assert!(matches!(err, AutostartError::NotSupported));
    }

    #[test]
    fn set_enabled_false_returns_not_supported() {
        let mut m = NoOpAutostart::new();
        let err = m.set_enabled(false, &[]).unwrap_err();
        assert!(matches!(err, AutostartError::NotSupported));
    }
}
```

- [ ] **Step 2: Wire the module**

In `crates/inputforge-autostart/src/lib.rs`, add the module declaration after `mod error;`:

```rust
mod error;
mod noop;
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p inputforge-autostart noop`
Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```powershell
git add crates/inputforge-autostart/
git commit -m "feat(autostart): add NoOpAutostart fallback"
```

---

### Task 1.5: Add `MockAutostart` (gated by `mock` feature)

**Files:**
- Create: `crates/inputforge-autostart/src/mock.rs`
- Modify: `crates/inputforge-autostart/src/lib.rs`

- [ ] **Step 1: Write the mock with shared interior state and a focused test**

Create `crates/inputforge-autostart/src/mock.rs`:

```rust
//! Test double for [`AutostartManager`]. Records every `set_enabled` call,
//! lets tests seed `is_enabled()` results and queue one-shot failures.
//!
//! Cloning shares state via `Arc<Mutex<>>`, so tests can hold one clone for
//! inspection while the engine owns another.

use std::sync::{Arc, Mutex};

use crate::{AutostartError, AutostartManager};

/// Recorded call to [`MockAutostart::set_enabled`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEnabledCall {
    pub enabled: bool,
    pub args: Vec<String>,
}

#[derive(Debug)]
struct State {
    is_enabled: Result<bool, AutostartError>,
    set_enabled_calls: Vec<SetEnabledCall>,
    next_set_enabled_failure: Option<AutostartError>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_enabled: Ok(false),
            set_enabled_calls: Vec::new(),
            next_set_enabled_failure: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MockAutostart {
    inner: Arc<Mutex<State>>,
}

impl MockAutostart {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the value returned by future `is_enabled()` calls. The argument is
    /// cloned each call; for the error path, store an `AutostartError`
    /// representative the test wants observed (e.g., `NotSupported`).
    pub fn set_is_enabled_result(&self, result: Result<bool, AutostartError>) {
        let mut state = self.inner.lock().unwrap();
        state.is_enabled = match result {
            Ok(v) => Ok(v),
            Err(_) => Err(AutostartError::Backend("seeded mock error".to_owned())),
        };
    }

    /// Queue a single failure for the next `set_enabled` call. Subsequent
    /// calls succeed unless this is called again.
    pub fn fail_next_set_enabled(&self, err: AutostartError) {
        self.inner.lock().unwrap().next_set_enabled_failure = Some(err);
    }

    /// Snapshot of recorded `set_enabled` calls, in dispatch order.
    #[must_use]
    pub fn calls(&self) -> Vec<SetEnabledCall> {
        self.inner.lock().unwrap().set_enabled_calls.clone()
    }
}

impl AutostartManager for MockAutostart {
    fn is_enabled(&self) -> Result<bool, AutostartError> {
        let state = self.inner.lock().unwrap();
        match &state.is_enabled {
            Ok(v) => Ok(*v),
            Err(_) => Err(AutostartError::Backend("seeded mock error".to_owned())),
        }
    }

    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError> {
        let mut state = self.inner.lock().unwrap();
        if let Some(err) = state.next_set_enabled_failure.take() {
            return Err(err);
        }
        state.set_enabled_calls.push(SetEnabledCall {
            enabled,
            args: args.iter().map(|&s| s.to_owned()).collect(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_enabled_returns_false() {
        let m = MockAutostart::new();
        assert_eq!(m.is_enabled().unwrap(), false);
    }

    #[test]
    fn set_enabled_records_calls_in_order() {
        let mut m = MockAutostart::new();
        m.set_enabled(true, &["--start-minimized"]).unwrap();
        m.set_enabled(false, &[]).unwrap();
        let calls = m.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].enabled, true);
        assert_eq!(calls[0].args, vec!["--start-minimized".to_owned()]);
        assert_eq!(calls[1].enabled, false);
        assert!(calls[1].args.is_empty());
    }

    #[test]
    fn fail_next_set_enabled_consumes_one_call_then_succeeds() {
        let mut m = MockAutostart::new();
        m.fail_next_set_enabled(AutostartError::RegistryDenied);
        let err = m.set_enabled(true, &[]).unwrap_err();
        assert!(matches!(err, AutostartError::RegistryDenied));
        // Next call must now succeed.
        m.set_enabled(true, &[]).unwrap();
        assert_eq!(m.calls().len(), 1, "failed call must not be recorded");
    }

    #[test]
    fn clone_shares_state_with_original() {
        let mut a = MockAutostart::new();
        let b = a.clone();
        a.set_enabled(true, &[]).unwrap();
        assert_eq!(b.calls().len(), 1, "clone must observe parent's calls");
    }

    #[test]
    fn seeded_is_enabled_error_surfaces_through_trait() {
        let m = MockAutostart::new();
        m.set_is_enabled_result(Err(AutostartError::NotSupported));
        let err = m.is_enabled().unwrap_err();
        assert!(matches!(err, AutostartError::Backend(_)));
    }
}
```

- [ ] **Step 2: Wire the module behind the `mock` feature**

In `crates/inputforge-autostart/src/lib.rs`, append:

```rust
#[cfg(feature = "mock")]
pub mod mock;
```

So the file now reads:

```rust
//! Autostart manager for InputForge: writes OS-level launch-at-sign-in state
//! (HKCU registry on Windows, `~/.config/autostart/*.desktop` on Linux).
//!
//! See `docs/superpowers/specs/2026-05-10-f16-startup-preferences-design.md`.

mod error;
mod noop;

pub use error::AutostartError;

#[cfg(feature = "mock")]
pub mod mock;

pub trait AutostartManager {
    /// (existing trait body, unchanged from Task 1.3.)
    fn is_enabled(&self) -> Result<bool, AutostartError>;
    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError>;
}
```

- [ ] **Step 3: Run the mock tests**

Run: `cargo test -p inputforge-autostart --features mock`
Expected: all 5 mock tests pass alongside the existing error + noop tests.

- [ ] **Step 4: Commit**

```powershell
git add crates/inputforge-autostart/
git commit -m "feat(autostart): add MockAutostart behind mock feature"
```

---

### Task 1.6: Add `WindowsAutostart` (cfg-gated; `#[ignore]`'d integration tests)

**Files:**
- Create: `crates/inputforge-autostart/src/windows.rs`
- Modify: `crates/inputforge-autostart/src/lib.rs`

- [ ] **Step 1: Write the impl + a non-ignored unit test for the args-passthrough construction**

Create `crates/inputforge-autostart/src/windows.rs`:

```rust
//! Windows backend: writes HKCU\Software\Microsoft\Windows\CurrentVersion\Run
//! via the `auto-launch` crate.

#![cfg(target_os = "windows")]

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

use crate::{AutostartError, AutostartManager};

const APP_NAME: &str = "InputForge";

#[derive(Debug)]
pub(crate) struct WindowsAutostart {
    app_path: String,
}

impl WindowsAutostart {
    /// Resolve the absolute exe path once at construction.
    ///
    /// # Errors
    ///
    /// Returns [`AutostartError::NotSupported`] when `std::env::current_exe`
    /// fails (rare; e.g., AppImage-style mounts on non-Windows, kept here
    /// for symmetry).
    pub(crate) fn new() -> Result<Self, AutostartError> {
        let exe = std::env::current_exe().map_err(|_| AutostartError::NotSupported)?;
        let app_path = exe
            .to_str()
            .ok_or(AutostartError::NotSupported)?
            .to_owned();
        Ok(Self { app_path })
    }

    fn build(&self, args: &[&str]) -> AutoLaunch {
        let owned_args: Vec<String> = args.iter().map(|&s| s.to_owned()).collect();
        AutoLaunchBuilder::new()
            .set_app_name(APP_NAME)
            .set_app_path(&self.app_path)
            .set_args(&owned_args)
            .set_use_launch_agent(false)
            .build()
            .expect("WindowsAutostart: AutoLaunchBuilder::build cannot fail with valid app_path")
    }
}

impl AutostartManager for WindowsAutostart {
    fn is_enabled(&self) -> Result<bool, AutostartError> {
        self.build(&[])
            .is_enabled()
            .map_err(|e| AutostartError::Backend(e.to_string()))
    }

    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError> {
        let launcher = self.build(args);
        let result = if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        };
        result.map_err(|e| AutostartError::Backend(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_resolves_exe_path() {
        let w = WindowsAutostart::new().expect("current_exe must succeed in test runner");
        assert!(!w.app_path.is_empty());
    }

    /// Round-trip enable -> is_enabled -> disable against the real registry.
    /// Gated `#[ignore]` so default `cargo test` does not touch HKCU.
    /// Run with: `cargo test --workspace -- --ignored`.
    #[test]
    #[ignore = "touches HKCU\\...\\Run; run explicitly with --ignored"]
    fn registry_round_trip() {
        let mut w = WindowsAutostart::new().unwrap();

        // Drop guard removes the registry value even on panic.
        struct Cleanup<'a>(&'a mut WindowsAutostart);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                let _ = self.0.set_enabled(false, &[]);
            }
        }
        let _g = Cleanup(&mut w);

        // Pre-clean to reduce flakiness from leftover state.
        let _ = _g.0.set_enabled(false, &[]);
        assert_eq!(_g.0.is_enabled().unwrap(), false, "must start clean");

        _g.0.set_enabled(true, &["--start-minimized"]).unwrap();
        assert_eq!(_g.0.is_enabled().unwrap(), true);

        _g.0.set_enabled(false, &[]).unwrap();
        assert_eq!(_g.0.is_enabled().unwrap(), false);
    }
}
```

- [ ] **Step 2: Wire the module**

In `crates/inputforge-autostart/src/lib.rs`, add after the `mod noop;` line:

```rust
mod error;
mod noop;
#[cfg(target_os = "windows")]
mod windows;
```

- [ ] **Step 3: Run the unit test (non-ignored)**

Run: `cargo test -p inputforge-autostart`
Expected: existing tests + `build_resolves_exe_path` pass. The `registry_round_trip` test stays ignored.

- [ ] **Step 4: (Optional) Run the ignored integration test on Windows**

Run: `cargo test -p inputforge-autostart -- --ignored`
Expected: `registry_round_trip` passes after touching and cleaning HKCU.

- [ ] **Step 5: Commit**

```powershell
git add crates/inputforge-autostart/
git commit -m "feat(autostart): add Windows HKCU\\Run backend"
```

---

### Task 1.7: Add `LinuxAutostart` (cfg-gated; symmetrical to Windows)

**Files:**
- Create: `crates/inputforge-autostart/src/linux.rs`
- Modify: `crates/inputforge-autostart/src/lib.rs`

- [ ] **Step 1: Write the impl**

Create `crates/inputforge-autostart/src/linux.rs`:

```rust
//! Linux backend: writes `$XDG_CONFIG_HOME/autostart/InputForge.desktop`
//! via the `auto-launch` crate. Honored by GNOME, KDE, XFCE, Cinnamon,
//! MATE; ignored by tiling WMs without an XDG autostart implementation.

#![cfg(target_os = "linux")]

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

use crate::{AutostartError, AutostartManager};

const APP_NAME: &str = "InputForge";

#[derive(Debug)]
pub(crate) struct LinuxAutostart {
    app_path: String,
}

impl LinuxAutostart {
    /// Resolve the absolute exe path once at construction.
    ///
    /// # Errors
    ///
    /// Returns [`AutostartError::NotSupported`] when `std::env::current_exe`
    /// fails (e.g., AppImage mount weirdness).
    pub(crate) fn new() -> Result<Self, AutostartError> {
        let exe = std::env::current_exe().map_err(|_| AutostartError::NotSupported)?;
        let app_path = exe
            .to_str()
            .ok_or(AutostartError::NotSupported)?
            .to_owned();
        Ok(Self { app_path })
    }

    fn build(&self, args: &[&str]) -> AutoLaunch {
        let owned_args: Vec<String> = args.iter().map(|&s| s.to_owned()).collect();
        AutoLaunchBuilder::new()
            .set_app_name(APP_NAME)
            .set_app_path(&self.app_path)
            .set_args(&owned_args)
            .build()
            .expect("LinuxAutostart: AutoLaunchBuilder::build cannot fail with valid app_path")
    }
}

impl AutostartManager for LinuxAutostart {
    fn is_enabled(&self) -> Result<bool, AutostartError> {
        self.build(&[])
            .is_enabled()
            .map_err(|e| AutostartError::Backend(e.to_string()))
    }

    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError> {
        let launcher = self.build(args);
        let result = if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        };
        result.map_err(|e| AutostartError::Backend(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_resolves_exe_path() {
        let l = LinuxAutostart::new().expect("current_exe must succeed in test runner");
        assert!(!l.app_path.is_empty());
    }

    /// Round-trip against a tempdir-rooted XDG_CONFIG_HOME to avoid touching
    /// the developer's real autostart dir. Gated `#[ignore]` because it
    /// mutates an env var; not parallelizable with other env-var tests.
    #[test]
    #[ignore = "mutates XDG_CONFIG_HOME; run explicitly with --ignored"]
    fn xdg_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        // SAFETY: Test is single-threaded by `--ignored` in practice; for
        //         strict isolation, set XDG_CONFIG_HOME via a fixture.
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());

        let mut l = LinuxAutostart::new().unwrap();
        let _ = l.set_enabled(false, &[]);
        assert_eq!(l.is_enabled().unwrap(), false);

        l.set_enabled(true, &["--start-minimized"]).unwrap();
        assert_eq!(l.is_enabled().unwrap(), true);

        l.set_enabled(false, &[]).unwrap();
        assert_eq!(l.is_enabled().unwrap(), false);
    }
}
```

- [ ] **Step 2: Wire the module**

In `crates/inputforge-autostart/src/lib.rs`, add the linux module declaration:

```rust
mod error;
mod noop;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
```

- [ ] **Step 3: Verify the workspace still builds on Windows (linux.rs is excluded by cfg)**

Run: `cargo check --workspace`
Expected: clean build. `linux.rs` is not compiled on Windows.

- [ ] **Step 4: Commit**

```powershell
git add crates/inputforge-autostart/
git commit -m "feat(autostart): add Linux XDG autostart backend"
```

---

### Task 1.8: Public factory `new_for_current_platform`

**Files:**
- Modify: `crates/inputforge-autostart/src/lib.rs`

- [ ] **Step 1: Write a smoke test for the factory**

Add the following test module at the bottom of `crates/inputforge-autostart/src/lib.rs`:

```rust
#[cfg(test)]
mod factory_tests {
    use super::*;

    #[test]
    fn new_for_current_platform_returns_a_manager() {
        let m = new_for_current_platform();
        // is_enabled may succeed or fail depending on platform/runner state;
        // we only assert the call doesn't panic and the trait object lives.
        let _ = m.is_enabled();
    }
}
```

- [ ] **Step 2: Run the test (it will fail to compile because the factory is not yet defined)**

Run: `cargo test -p inputforge-autostart factory_tests`
Expected: COMPILE FAIL with "cannot find function `new_for_current_platform`".

- [ ] **Step 3: Implement the factory and re-exports**

Append to `crates/inputforge-autostart/src/lib.rs`, just below the `pub trait AutostartManager` block:

```rust
/// Construct the platform-appropriate autostart manager, or a [`NoOpAutostart`]
/// fallback when `std::env::current_exe()` fails.
///
/// The fallback's `is_enabled()` returns `Ok(false)` and `set_enabled()`
/// returns [`AutostartError::NotSupported`], so the engine and UI degrade
/// gracefully (the toggle stays off; dispatch surfaces a warning toast).
#[must_use]
pub fn new_for_current_platform() -> Box<dyn AutostartManager> {
    #[cfg(target_os = "windows")]
    {
        match windows::WindowsAutostart::new() {
            Ok(w) => return Box::new(w),
            Err(e) => {
                tracing::warn!(target: "autostart", %e, "Windows backend init failed, using NoOp");
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        match linux::LinuxAutostart::new() {
            Ok(l) => return Box::new(l),
            Err(e) => {
                tracing::warn!(target: "autostart", %e, "Linux backend init failed, using NoOp");
            }
        }
    }
    Box::new(noop::NoOpAutostart::new())
}
```

(`tracing` is already in dev-deps via the workspace; if the `tracing` import is missing at the file head, add `use tracing` is unnecessary because we use the macros via path. Add `tracing` to `[dependencies]` in `Cargo.toml` if it is not already listed there. Per Task 1.1 it is.)

- [ ] **Step 4: Run the test**

Run: `cargo test -p inputforge-autostart factory_tests`
Expected: PASS.

- [ ] **Step 5: Final crate-level test sweep**

Run: `cargo test -p inputforge-autostart --features mock`
Expected: every test in `error`, `noop`, `mock`, `windows` (`build_resolves_exe_path`), and `factory_tests` passes.

- [ ] **Step 6: Commit**

```powershell
git add crates/inputforge-autostart/src/lib.rs
git commit -m "feat(autostart): add new_for_current_platform factory"
```

---

# Phase 2: `StartupSettings` data model

### Task 2.1: Add `StartupSettings` and the `AppSettings.startup` field

**Files:**
- Modify: `crates/inputforge-core/src/settings.rs`

- [ ] **Step 1: Write the failing tests**

In `crates/inputforge-core/src/settings.rs`, inside the `mod tests` block, append:

```rust
    #[test]
    fn settings_default_has_default_startup() {
        let s = AppSettings::default();
        assert_eq!(s.startup, StartupSettings::default());
        assert_eq!(s.startup.launch_at_startup, false);
        assert_eq!(s.startup.start_minimized_to_tray, false);
    }

    #[test]
    fn pre_f16_settings_loads_with_default_startup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.toml");
        // Pre-F16: no [startup] table.
        std::fs::write(&path, "last_profile = \"C:/foo.toml\"\n").unwrap();

        let loaded = AppSettings::load_from(&path);
        assert_eq!(loaded.startup, StartupSettings::default());
    }

    #[test]
    fn pre_f16_settings_loads_with_partial_startup_table() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.toml");
        // [startup] present but missing one field.
        std::fs::write(
            &path,
            "[startup]\nlaunch_at_startup = true\n",
        )
        .unwrap();

        let loaded = AppSettings::load_from(&path);
        assert_eq!(loaded.startup.launch_at_startup, true);
        assert_eq!(loaded.startup.start_minimized_to_tray, false);
    }

    #[test]
    fn settings_round_trips_startup_table() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.toml");

        let s = AppSettings {
            startup: StartupSettings {
                launch_at_startup: true,
                start_minimized_to_tray: true,
            },
            ..Default::default()
        };
        s.save_to(&path).unwrap();

        let body = std::fs::read_to_string(&path).unwrap();
        assert!(
            body.contains("[startup]"),
            "expected [startup] table on disk; got: {body}"
        );

        let loaded = AppSettings::load_from(&path);
        assert_eq!(loaded, s);
    }
```

- [ ] **Step 2: Run tests; expect compile failure (StartupSettings not defined)**

Run: `cargo test -p inputforge-core settings::tests`
Expected: COMPILE FAIL with "cannot find type `StartupSettings`".

- [ ] **Step 3: Add the struct and field**

In `crates/inputforge-core/src/settings.rs`, add `StartupSettings` just above the `AppSettings` struct (around line 22):

```rust
/// Startup preferences (F16): launch at OS sign-in and start minimized to tray.
///
/// Both fields default to `false`. The outer `#[serde(default)]` on
/// `AppSettings.startup` plus the inner `#[serde(default)]` on each field
/// lets pre-F16 `settings.toml` files load with no migration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupSettings {
    #[serde(default)]
    pub launch_at_startup: bool,
    #[serde(default)]
    pub start_minimized_to_tray: bool,
}
```

Then in `AppSettings`, add the new field below the existing `snapshot` field (currently line 38):

```rust
    /// Snapshot subsystem configuration.
    /// (existing doc comment unchanged)
    #[serde(default)]
    pub snapshot: SnapshotConfig,

    /// Startup preferences (F16). Persisted as a `[startup]` sub-table.
    #[serde(default)]
    pub startup: StartupSettings,

    #[serde(default)]
    pub device_aliases: HashMap<DeviceId, String>,
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-core settings::tests`
Expected: all four new tests pass; existing tests still pass.

- [ ] **Step 5: Commit**

```powershell
git add crates/inputforge-core/src/settings.rs
git commit -m "feat(settings): add nested StartupSettings under AppSettings"
```

---

# Phase 3: `AppState.startup` mirror field

### Task 3.1: Add `AppState.startup` and initialise it in `AppState::new`

**Files:**
- Modify: `crates/inputforge-core/src/state/mod.rs`

- [ ] **Step 1: Write the failing test**

At the bottom of `crates/inputforge-core/src/state/mod.rs`, locate (or create) the `#[cfg(test)] mod tests` block and add:

```rust
    use crate::settings::StartupSettings;

    #[test]
    fn app_state_new_defaults_startup_to_default() {
        let s = AppState::new();
        assert_eq!(s.startup, StartupSettings::default());
    }
```

(If the file does not have a `mod tests` block, append:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::StartupSettings;

    #[test]
    fn app_state_new_defaults_startup_to_default() {
        let s = AppState::new();
        assert_eq!(s.startup, StartupSettings::default());
    }
}
```

at the end of the file.)

- [ ] **Step 2: Run the test; expect compile failure**

Run: `cargo test -p inputforge-core state::tests::app_state_new_defaults_startup_to_default`
Expected: COMPILE FAIL with "no field `startup` on type `AppState`".

- [ ] **Step 3: Add the field**

In `crates/inputforge-core/src/state/mod.rs`, add an import for `StartupSettings`:

```rust
use crate::settings::{DeviceRecord, StartupSettings};
```

Add the field below `snapshot_config` (currently line 92):

```rust
    /// Snapshot configuration mirrored from `AppSettings.snapshot`.
    pub snapshot_config: SnapshotConfig,
    /// Startup preferences mirrored from `AppSettings.startup` (F16).
    pub startup: StartupSettings,
```

In `AppState::new`, initialise the field below `snapshot_config: SnapshotConfig::default(),` (currently line 130):

```rust
            snapshot_config: SnapshotConfig::default(),
            startup: StartupSettings::default(),
```

In `AppState::with_profile`, scan for the same `snapshot_config: SnapshotConfig::default(),` line and add `startup: StartupSettings::default(),` immediately below it (this method appears around line 151 onward; it sets every field of `Self`). If `with_profile` uses `Self { ..Self::new() }` style, no change is needed there.

- [ ] **Step 4: Run the test**

Run: `cargo test -p inputforge-core state::tests::app_state_new_defaults_startup_to_default`
Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/inputforge-core/src/state/mod.rs
git commit -m "feat(state): add AppState.startup mirror field"
```

---

# Phase 4: Engine constructor accepts `Box<dyn AutostartManager>`

This phase is dense: it adds the field, threads it through `Engine::new`, and sweeps every test call site. Do it in one task because the build is broken in between additions and call-site sweeps.

### Task 4.1: Wire the autostart dependency into `inputforge-core`

**Files:**
- Modify: `crates/inputforge-core/Cargo.toml`

- [ ] **Step 1: Add the runtime dep and the dev-only `mock` feature alias**

In `crates/inputforge-core/Cargo.toml`, in `[dependencies]` (after `opener`, around line 38), add:

```toml
inputforge-autostart = { workspace = true }
```

And in `[dev-dependencies]` (after `serde_json`, around line 46), add:

```toml
inputforge-autostart = { workspace = true, features = ["mock"] }
```

- [ ] **Step 2: Verify**

Run: `cargo check -p inputforge-core`
Expected: clean build.

- [ ] **Step 3: Commit**

```powershell
git add crates/inputforge-core/Cargo.toml
git commit -m "chore(core): depend on inputforge-autostart (mock for dev)"
```

---

### Task 4.2: Add `Engine.autostart` field and the 9th `Engine::new` arg

**Files:**
- Modify: `crates/inputforge-core/src/engine/mod.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`
- Modify: `crates/inputforge-app/src/main.rs`

This task DOES NOT add behaviour changes (no mirror, no reconciliation). Those land in Tasks 4.3 / 4.4 and Phase 8. The single goal here is: every `Engine::new(...)` site compiles with the 9th arg.

- [ ] **Step 1: Add the field and import to `engine/mod.rs`**

Add to the imports near the top of `crates/inputforge-core/src/engine/mod.rs`:

```rust
use inputforge_autostart::AutostartManager;
```

In the `Engine` struct, add the field below `settings_path` (currently line 69):

```rust
    /// OS autostart manager. Concrete impl chosen per platform; tests pass
    /// `inputforge_autostart::mock::MockAutostart`.
    pub(crate) autostart: Box<dyn AutostartManager>,
```

- [ ] **Step 2: Update `Engine::new` signature**

Change the `Engine::new` signature (currently lines 100-109) to accept a 9th argument `autostart` and assign it into the struct:

```rust
    pub fn new(
        input: Box<dyn InputSource>,
        output: Box<dyn OutputSink>,
        keyboard: Box<dyn KeyboardSink>,
        hider: Box<dyn DeviceHider>,
        state: Arc<RwLock<AppState>>,
        commands: mpsc::Receiver<EngineCommand>,
        settings: AppSettings,
        settings_path: PathBuf,
        autostart: Box<dyn AutostartManager>,
    ) -> Self {
```

In the constructor body, after the existing field initialisation (`settings_path,` on line 150), add:

```rust
            settings,
            settings_path,
            autostart,
```

- [ ] **Step 3: Update `inputforge-app/src/main.rs` call site**

In `crates/inputforge-app/src/main.rs`, in `run_engine_inner`, update the `Engine::new` call (currently lines 164-173) to pass the autostart manager. The crate is referenced by its absolute path (`inputforge_autostart::new_for_current_platform()`), no `use` is needed.

```rust
    let mut engine = Engine::new(
        input,
        output,
        keyboard,
        hider,
        state,
        commands,
        AppSettings::load(),
        AppSettings::settings_path(),
        inputforge_autostart::new_for_current_platform(),
    );
```

Add the workspace dep entry to `crates/inputforge-app/Cargo.toml`, in `[dependencies]`:

```toml
inputforge-autostart = { workspace = true }
```

- [ ] **Step 4: Sweep test helpers in `engine/tests.rs`**

In `crates/inputforge-core/src/engine/tests.rs`, near the existing `use` block at the top (around line 19-25), add:

```rust
use inputforge_autostart::mock::MockAutostart;
```

In `make_engine` (line 130), add `Box::new(MockAutostart::new()),` as the 9th argument to `Engine::new` (after `PathBuf::new(),`):

```rust
    let engine = Engine::new(
        Box::new(input),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );
```

In `test_engine_with_settings_path` (line 153), do the same.

In `make_engine_no_profile` (line 1019), do the same.

In `EngineHarness::new` (line 1054), update the constructor:

```rust
    fn new() -> Self {
        let settings_dir = tempfile::tempdir().unwrap();
        let settings_path = settings_dir.path().join("settings.toml");
        let library_dir = settings_dir.path().join("profiles");
        let settings = AppSettings::default();
        settings.save_to(&settings_path).unwrap();

        let state = Arc::new(RwLock::new(AppState::new()));
        let (_tx, rx) = mpsc::channel();
        let autostart_mock = MockAutostart::new();
        let engine = Engine::new(
            Box::new(MockInputSource::default()),
            Box::new(MockOutputSink::new()),
            Box::new(MockKeyboardSink::new()),
            Box::new(MockDeviceHider::default()),
            Arc::clone(&state),
            rx,
            settings,
            settings_path,
            Box::new(autostart_mock.clone()),
        );

        Self {
            engine,
            state,
            _settings_dir: settings_dir,
            library_dir,
            autostart_mock,
        }
    }
```

Also add the field to the harness struct (line 1041-1051):

```rust
struct EngineHarness {
    engine: Engine,
    state: Arc<RwLock<AppState>>,
    #[expect(
        clippy::used_underscore_binding,
        reason = "field is held only to keep the tempdir alive for the harness lifetime; \
                  the underscore prefix signals the binding is intentionally not read"
    )]
    _settings_dir: tempfile::TempDir,
    library_dir: PathBuf,
    /// Cloned handle to the autostart mock the engine was constructed with.
    /// Tests inspect calls and seed `is_enabled` results through this clone;
    /// the inner state is shared via `Arc<Mutex<>>`.
    autostart_mock: MockAutostart,
}
```

- [ ] **Step 5: Sweep the 15 direct `Engine::new(` sites in `engine/tests.rs`**

For each occurrence at lines (approximate; verify with `grep -n "Engine::new(" crates/inputforge-core/src/engine/tests.rs`):

`1867, 2012, 2121, 2300, 2340, 2387, 2647, 2771, 2970, 3171, 3223, 3392, 3459, 3546, 3717`

Add `Box::new(MockAutostart::new()),` as the new last argument, immediately before the closing `)`. Each currently ends with:

```rust
        AppSettings::default(),
        PathBuf::new(),
    );
```

Make each end with:

```rust
        AppSettings::default(),
        PathBuf::new(),
        Box::new(MockAutostart::new()),
    );
```

(The actual settings/path argument at each site may vary. Whatever follows `commands` and `settings` and `settings_path`, append the new mock arg.)

- [ ] **Step 6: Run the workspace tests**

Run: `cargo test --workspace`
Expected: every previously-passing test still passes. No new tests yet for this task.

- [ ] **Step 7: Commit**

```powershell
git add crates/
git commit -m "feat(engine): accept Box<dyn AutostartManager> in Engine::new"
```

---

### Task 4.3: Mirror `settings.startup` into `state.startup` in `Engine::new`

**Files:**
- Modify: `crates/inputforge-core/src/engine/mod.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write the failing test**

In `crates/inputforge-core/src/engine/tests.rs`, append (next to the existing `engine_initialisation_mirrors_settings_snapshot_into_state` test around line 4024):

```rust
#[test]
fn engine_initialisation_mirrors_startup_into_state() {
    use crate::settings::StartupSettings;
    let settings = AppSettings {
        startup: StartupSettings {
            launch_at_startup: true,
            start_minimized_to_tray: true,
        },
        ..AppSettings::default()
    };
    let (engine, _path) = test_engine_with_settings_path(settings.clone());
    assert_eq!(engine.state.read().startup, settings.startup);
}
```

- [ ] **Step 2: Run; expect failure**

Run: `cargo test -p inputforge-core engine_initialisation_mirrors_startup_into_state`
Expected: FAIL with `assertion failed: state.startup != settings.startup` (state still has default while settings has both true).

- [ ] **Step 3: Add the mirror in `Engine::new`**

In `crates/inputforge-core/src/engine/mod.rs`, in the existing `state.write()` block (currently lines 129-134), add `startup`:

```rust
        {
            let mut state = state.write();
            state.device_aliases.clone_from(&settings.device_aliases);
            state.device_registry.clone_from(&settings.device_registry);
            state.snapshot_config.clone_from(&settings.snapshot);
            state.startup.clone_from(&settings.startup);
        };
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p inputforge-core engine_initialisation_mirrors_startup_into_state`
Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/inputforge-core/src/engine/
git commit -m "feat(engine): mirror settings.startup into AppState on init"
```

---

# Phase 5: `ReloadSettings` mirrors `startup`

### Task 5.1: Add the failing test, then update the handler

**Files:**
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/inputforge-core/src/engine/tests.rs`:

```rust
#[test]
fn reload_settings_mirrors_startup_into_state() {
    use crate::settings::StartupSettings;
    let mut harness = EngineHarness::new();

    // Write a fresh settings.toml with a non-default [startup].
    let mut file_settings = AppSettings::default();
    file_settings.startup = StartupSettings {
        launch_at_startup: true,
        start_minimized_to_tray: true,
    };
    file_settings
        .save_to(&harness.engine.settings_path)
        .unwrap();

    harness.dispatch(EngineCommand::ReloadSettings).unwrap();

    assert_eq!(harness.state().startup, file_settings.startup);
}
```

- [ ] **Step 2: Run; expect failure**

Run: `cargo test -p inputforge-core reload_settings_mirrors_startup_into_state`
Expected: FAIL.

- [ ] **Step 3: Update `ReloadSettings` handler**

In `crates/inputforge-core/src/engine/run.rs`, locate the `EngineCommand::ReloadSettings` arm (lines 550-554):

```rust
            EngineCommand::ReloadSettings => {
                self.settings = crate::settings::AppSettings::load_from(&self.settings_path);
                self.state.write().snapshot_config = self.settings.snapshot.clone();
                tracing::info!(target: "engine", "settings reloaded");
            }
```

Replace with:

```rust
            EngineCommand::ReloadSettings => {
                self.settings = crate::settings::AppSettings::load_from(&self.settings_path);
                {
                    let mut state = self.state.write();
                    state.snapshot_config = self.settings.snapshot.clone();
                    state.startup = self.settings.startup.clone();
                }
                tracing::info!(target: "engine", "settings reloaded");
            }
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p inputforge-core reload_settings_mirrors_startup_into_state`
Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/inputforge-core/src/engine/
git commit -m "feat(engine): mirror startup on ReloadSettings"
```

---

# Phase 6: `SetAutostart` command

This phase adds the `EngineCommand::SetAutostart` variant and its handler. Per the spec table at `Cross-coupling rules`, the handler:

- Computes `args = if settings.startup.start_minimized_to_tray { &["--start-minimized"] } else { &[] }`.
- Calls `self.autostart.set_enabled(enabled, args)`.
- On `Ok(())`: writes `settings.startup.launch_at_startup = enabled`, mirrors into `AppState`, persists `settings.toml`. On settings save failure: rolls back in-memory + mirror, pushes `format!("Could not save settings: {e}")`.
- On `Err(_)`: pushes `"Could not change launch-at-startup setting."`, leaves `settings`/`AppState` untouched.

### Task 6.1: Add the variant and Debug/PartialEq tests

**Files:**
- Modify: `crates/inputforge-core/src/engine/command.rs`

- [ ] **Step 1: Add the variant**

In `crates/inputforge-core/src/engine/command.rs`, append a new variant inside `EngineCommand` (just below `SetSnapshotConfig { config: ... }`, around line 122):

```rust
    /// Set the OS-level "launch InputForge at sign-in" preference (F16).
    ///
    /// Engine handler order:
    ///   1. Compute argv from `self.settings.startup.start_minimized_to_tray`
    ///      (`&["--start-minimized"]` when on, `&[]` when off).
    ///   2. Call `self.autostart.set_enabled(enabled, args)`.
    ///   3. On `Ok(())`: persist the new `launch_at_startup` value to
    ///      `settings.toml`, mirror into `AppState`. On save failure: roll
    ///      back in-memory + mirror, push warning.
    ///   4. On `Err`: push `"Could not change launch-at-startup setting."`,
    ///      leave settings + state untouched.
    SetAutostart { enabled: bool },
```

- [ ] **Step 2: Add a Debug/PartialEq test**

In the same file's existing `#[cfg(test)] mod tests`, append:

```rust
    #[test]
    fn set_autostart_variant_debug_and_partialeq() {
        let a = EngineCommand::SetAutostart { enabled: true };
        let b = EngineCommand::SetAutostart { enabled: true };
        assert_eq!(a, b);
        assert!(format!("{a:?}").contains("SetAutostart"));
    }
```

- [ ] **Step 3: Run the test; expect compile failure in `run.rs`'s match (non-exhaustive)**

Run: `cargo test -p inputforge-core --lib`
Expected: COMPILE FAIL with "non-exhaustive patterns" referencing `EngineCommand::SetAutostart` in `engine/run.rs`.

- [ ] **Step 4: Add a temporary catch-arm in `run.rs` to unblock compilation**

In `crates/inputforge-core/src/engine/run.rs`, near the end of the `match command` block in `handle_command`, before the closing brace of the match, add:

```rust
            EngineCommand::SetAutostart { .. } => {
                // Implemented in Task 6.2.
                tracing::trace!(target: "engine", "SetAutostart received (handler not yet wired)");
            }
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p inputforge-core --lib`
Expected: clean build, the new `set_autostart_variant_debug_and_partialeq` test passes.

- [ ] **Step 6: Commit**

```powershell
git add crates/inputforge-core/src/engine/
git commit -m "feat(engine): add SetAutostart command variant"
```

---

### Task 6.2: Implement the `SetAutostart` handler (TDD: success path)

**Files:**
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write three failing tests**

In `crates/inputforge-core/src/engine/tests.rs`, append a new section:

```rust
// ---------------------------------------------------------------------------
// F16: SetAutostart handler
// ---------------------------------------------------------------------------

#[test]
fn set_autostart_passes_no_args_when_start_minimized_off() {
    use inputforge_autostart::mock::SetEnabledCall;
    let mut harness = EngineHarness::new();
    // Both startup fields default to false; just enabling autostart.
    harness
        .dispatch(EngineCommand::SetAutostart { enabled: true })
        .unwrap();

    let calls = harness.autostart_mock.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0],
        SetEnabledCall {
            enabled: true,
            args: vec![],
        }
    );
}

#[test]
fn set_autostart_passes_start_minimized_arg_when_enabled() {
    use crate::settings::StartupSettings;
    use inputforge_autostart::mock::SetEnabledCall;
    let mut harness = EngineHarness::new();
    // Pre-seed: start-minimized is on; enabling autostart must pass the arg.
    harness.engine.settings.startup = StartupSettings {
        launch_at_startup: false,
        start_minimized_to_tray: true,
    };
    harness.engine.state.write().startup = harness.engine.settings.startup.clone();

    harness
        .dispatch(EngineCommand::SetAutostart { enabled: true })
        .unwrap();

    let calls = harness.autostart_mock.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0],
        SetEnabledCall {
            enabled: true,
            args: vec!["--start-minimized".to_owned()],
        }
    );
}

#[test]
fn set_autostart_writes_state_field_on_success() {
    let mut harness = EngineHarness::new();
    harness
        .dispatch(EngineCommand::SetAutostart { enabled: true })
        .unwrap();

    assert_eq!(harness.state().startup.launch_at_startup, true);

    // On-disk: settings.toml round-trips the new value.
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.startup.launch_at_startup, true);
}
```

- [ ] **Step 2: Run; expect failures**

Run: `cargo test -p inputforge-core set_autostart`
Expected: 3 FAILs (mock has zero recorded calls; `state.startup` unchanged).

- [ ] **Step 3: Replace the placeholder handler with the real implementation**

In `crates/inputforge-core/src/engine/run.rs`, replace the placeholder `EngineCommand::SetAutostart { .. }` arm with:

```rust
            EngineCommand::SetAutostart { enabled } => {
                // Step 1: compute argv from the persisted start-minimized
                // setting; auto-launch is dumb about that flag.
                let owned_args: Vec<&str> = if self.settings.startup.start_minimized_to_tray {
                    vec!["--start-minimized"]
                } else {
                    vec![]
                };

                // Step 2: OS write first; on failure, do NOT touch settings or
                // AppState (the mirror chain plus polling will resync the UI).
                if let Err(e) = self.autostart.set_enabled(enabled, &owned_args) {
                    tracing::warn!(
                        target: "autostart",
                        %e,
                        enabled,
                        "autostart OS write failed"
                    );
                    self.state
                        .write()
                        .warnings
                        .push("Could not change launch-at-startup setting.".to_owned());
                    return Ok(());
                }

                // Step 3: persist + mirror; on save failure, roll back both.
                let prior = self.settings.startup.clone();
                self.settings.startup.launch_at_startup = enabled;
                self.state.write().startup = self.settings.startup.clone();
                if let Err(e) = self.settings.save_to(&self.settings_path) {
                    tracing::warn!(
                        target: "settings",
                        error = %e,
                        "failed to persist settings.toml; rolling back in-memory startup"
                    );
                    self.settings.startup = prior;
                    let mut state = self.state.write();
                    state.startup = self.settings.startup.clone();
                    state.warnings.push(format!("Could not save settings: {e}"));
                    return Ok(());
                }

                tracing::info!(
                    target: "engine",
                    launch_at_startup = self.settings.startup.launch_at_startup,
                    "autostart updated"
                );
            }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p inputforge-core set_autostart`
Expected: 3 PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/inputforge-core/src/engine/
git commit -m "feat(engine): implement SetAutostart success path"
```

---

### Task 6.3: `SetAutostart` failure paths (OS-write fail; toggle off preserves start-minimized)

**Files:**
- Modify: `crates/inputforge-core/src/engine/tests.rs`

(The handler implementation already covers both paths from Task 6.2; this task adds the missing tests so coverage is explicit.)

- [ ] **Step 1: Add the failure-path tests**

Append to `crates/inputforge-core/src/engine/tests.rs`:

```rust
#[test]
fn set_autostart_failure_leaves_state_field_unchanged() {
    use inputforge_autostart::AutostartError;

    let mut harness = EngineHarness::new();
    let warnings_before = harness.state().warnings.len();

    // Mock the next set_enabled to fail.
    harness
        .autostart_mock
        .fail_next_set_enabled(AutostartError::RegistryDenied);

    harness
        .dispatch(EngineCommand::SetAutostart { enabled: true })
        .unwrap();

    // settings + state mirror unchanged.
    assert_eq!(harness.state().startup.launch_at_startup, false);
    assert_eq!(harness.engine.settings.startup.launch_at_startup, false);

    // On-disk unchanged (no save attempted).
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.startup.launch_at_startup, false);

    // Exactly one new warning, with the documented exact text.
    let warnings = harness.state().warnings.clone();
    assert_eq!(warnings.len(), warnings_before + 1);
    assert_eq!(
        warnings.last().unwrap(),
        "Could not change launch-at-startup setting."
    );
}

#[test]
fn set_autostart_off_preserves_start_minimized_to_tray() {
    use crate::settings::StartupSettings;
    let mut harness = EngineHarness::new();
    // Pre-seed: both on.
    harness.engine.settings.startup = StartupSettings {
        launch_at_startup: true,
        start_minimized_to_tray: true,
    };
    harness.engine.state.write().startup = harness.engine.settings.startup.clone();

    harness
        .dispatch(EngineCommand::SetAutostart { enabled: false })
        .unwrap();

    assert_eq!(harness.state().startup.launch_at_startup, false);
    assert_eq!(harness.state().startup.start_minimized_to_tray, true);
}
```

- [ ] **Step 2: Run them**

Run: `cargo test -p inputforge-core set_autostart`
Expected: all five `set_autostart_*` tests pass.

- [ ] **Step 3: Commit**

```powershell
git add crates/inputforge-core/src/engine/tests.rs
git commit -m "test(engine): cover SetAutostart failure and off-preserves-other paths"
```

---

# Phase 7: `SetStartMinimizedToTray` command

### Task 7.1: Add the variant + placeholder match arm

**Files:**
- Modify: `crates/inputforge-core/src/engine/command.rs`
- Modify: `crates/inputforge-core/src/engine/run.rs`

- [ ] **Step 1: Add the variant**

In `crates/inputforge-core/src/engine/command.rs`, immediately below `SetAutostart { enabled: bool },`, add:

```rust
    /// Set the persisted "start minimized to tray" preference (F16).
    ///
    /// Engine handler order:
    ///   1. Capture prior setting for rollback.
    ///   2. Update `settings.startup.start_minimized_to_tray`, mirror into
    ///      `AppState`, persist `settings.toml`. On save failure: roll
    ///      back in-memory + mirror, push warning, return.
    ///   3. If `settings.startup.launch_at_startup`, re-register the
    ///      autostart entry with the new args. Best-effort: on `Err`,
    ///      push `"Saved, but could not update the auto-launch arguments. \
    ///      Restart of InputForge may use the previous setting."`. Do NOT
    ///      revert; the engine-startup unconditional argv resync heals it
    ///      on the next launch.
    SetStartMinimizedToTray { enabled: bool },
```

Add a Debug/PartialEq test in the same file's `mod tests`:

```rust
    #[test]
    fn set_start_minimized_to_tray_variant_debug_and_partialeq() {
        let a = EngineCommand::SetStartMinimizedToTray { enabled: true };
        let b = EngineCommand::SetStartMinimizedToTray { enabled: true };
        assert_eq!(a, b);
        assert!(format!("{a:?}").contains("SetStartMinimizedToTray"));
    }
```

- [ ] **Step 2: Add a temporary catch arm in `run.rs` to unblock compilation**

In `crates/inputforge-core/src/engine/run.rs`, near the existing `EngineCommand::SetAutostart` arm, add:

```rust
            EngineCommand::SetStartMinimizedToTray { .. } => {
                // Implemented in Task 7.2.
                tracing::trace!(target: "engine", "SetStartMinimizedToTray received (handler not yet wired)");
            }
```

- [ ] **Step 3: Build + run the new variant test**

Run: `cargo test -p inputforge-core --lib set_start_minimized_to_tray_variant`
Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add crates/inputforge-core/src/engine/
git commit -m "feat(engine): add SetStartMinimizedToTray command variant"
```

---

### Task 7.2: Implement the handler (TDD)

**Files:**
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write four failing tests**

Append to `crates/inputforge-core/src/engine/tests.rs`:

```rust
// ---------------------------------------------------------------------------
// F16: SetStartMinimizedToTray handler
// ---------------------------------------------------------------------------

#[test]
fn set_start_minimized_persists_and_mirrors() {
    let mut harness = EngineHarness::new();

    harness
        .dispatch(EngineCommand::SetStartMinimizedToTray { enabled: true })
        .unwrap();

    assert_eq!(harness.state().startup.start_minimized_to_tray, true);

    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.startup.start_minimized_to_tray, true);
}

#[test]
fn set_start_minimized_when_autostart_off_does_not_call_manager() {
    let mut harness = EngineHarness::new();
    // launch_at_startup defaults to false; expect zero set_enabled calls.
    harness
        .dispatch(EngineCommand::SetStartMinimizedToTray { enabled: true })
        .unwrap();

    assert_eq!(
        harness.autostart_mock.calls().len(),
        0,
        "autostart manager must not be touched when launch_at_startup is false"
    );
}

#[test]
fn set_start_minimized_resyncs_autostart_args_when_enabled() {
    use crate::settings::StartupSettings;
    use inputforge_autostart::mock::SetEnabledCall;

    let mut harness = EngineHarness::new();
    // Pre-seed: launch_at_startup = true.
    harness.engine.settings.startup = StartupSettings {
        launch_at_startup: true,
        start_minimized_to_tray: false,
    };
    harness.engine.state.write().startup = harness.engine.settings.startup.clone();
    harness.engine.settings.save_to(&harness.engine.settings_path).unwrap();

    harness
        .dispatch(EngineCommand::SetStartMinimizedToTray { enabled: true })
        .unwrap();

    let calls = harness.autostart_mock.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0],
        SetEnabledCall {
            enabled: true,
            args: vec!["--start-minimized".to_owned()],
        }
    );
}

#[test]
fn set_start_minimized_persists_when_resync_fails() {
    use crate::settings::StartupSettings;
    use inputforge_autostart::AutostartError;

    let mut harness = EngineHarness::new();
    harness.engine.settings.startup = StartupSettings {
        launch_at_startup: true,
        start_minimized_to_tray: false,
    };
    harness.engine.state.write().startup = harness.engine.settings.startup.clone();
    harness.engine.settings.save_to(&harness.engine.settings_path).unwrap();

    harness
        .autostart_mock
        .fail_next_set_enabled(AutostartError::RegistryDenied);

    let warnings_before = harness.state().warnings.len();
    harness
        .dispatch(EngineCommand::SetStartMinimizedToTray { enabled: true })
        .unwrap();

    // Persisted: settings updated, mirror updated, on-disk reflects.
    assert_eq!(harness.state().startup.start_minimized_to_tray, true);
    let on_disk = AppSettings::load_from(&harness.engine.settings_path);
    assert_eq!(on_disk.startup.start_minimized_to_tray, true);

    // Exactly one warning with the documented text.
    let warnings = harness.state().warnings.clone();
    assert_eq!(warnings.len(), warnings_before + 1);
    assert_eq!(
        warnings.last().unwrap(),
        "Saved, but could not update the auto-launch arguments. \
         Restart of InputForge may use the previous setting."
    );
}
```

- [ ] **Step 2: Run; expect 4 failures**

Run: `cargo test -p inputforge-core set_start_minimized`
Expected: FAILs (placeholder handler does nothing).

- [ ] **Step 3: Replace the placeholder with the full handler**

In `crates/inputforge-core/src/engine/run.rs`, replace the `EngineCommand::SetStartMinimizedToTray { .. }` arm with:

```rust
            EngineCommand::SetStartMinimizedToTray { enabled } => {
                // Step 1: capture prior for rollback on save failure.
                let prior = self.settings.startup.clone();

                // Step 2: persist + mirror.
                self.settings.startup.start_minimized_to_tray = enabled;
                self.state.write().startup = self.settings.startup.clone();
                if let Err(e) = self.settings.save_to(&self.settings_path) {
                    tracing::warn!(
                        target: "settings",
                        error = %e,
                        "failed to persist settings.toml; rolling back in-memory startup"
                    );
                    self.settings.startup = prior;
                    let mut state = self.state.write();
                    state.startup = self.settings.startup.clone();
                    state.warnings.push(format!("Could not save settings: {e}"));
                    return Ok(());
                }

                // Step 3: best-effort autostart argv re-register when on.
                if self.settings.startup.launch_at_startup {
                    let owned_args: Vec<&str> = if self.settings.startup.start_minimized_to_tray {
                        vec!["--start-minimized"]
                    } else {
                        vec![]
                    };
                    if let Err(e) = self.autostart.set_enabled(true, &owned_args) {
                        tracing::warn!(
                            target: "autostart",
                            %e,
                            "could not refresh autostart argv after start-minimized toggle"
                        );
                        self.state.write().warnings.push(
                            "Saved, but could not update the auto-launch arguments. \
                             Restart of InputForge may use the previous setting."
                                .to_owned(),
                        );
                    }
                }

                tracing::info!(
                    target: "engine",
                    start_minimized_to_tray = self.settings.startup.start_minimized_to_tray,
                    "start-minimized preference updated"
                );
            }
```

- [ ] **Step 4: Run all tests**

Run: `cargo test -p inputforge-core set_start_minimized`
Expected: 4 PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/inputforge-core/src/engine/
git commit -m "feat(engine): implement SetStartMinimizedToTray handler"
```

---

# Phase 8: Engine startup reconciliation

Per spec choice 7: in `Engine::new`, after the existing mirror, query `autostart.is_enabled()`. If it disagrees with `settings.startup.launch_at_startup`, sync settings to the OS value and persist. Then if settings now says enabled, unconditionally re-push argv (heals stale start-minimized state).

### Task 8.1: Tests

**Files:**
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Add four tests**

Because reconciliation runs inside `Engine::new`, these tests construct the engine directly with a pre-seeded mock rather than via `EngineHarness::new()`. Add a small helper at the top of the new test section:

```rust
// ---------------------------------------------------------------------------
// F16: Engine startup reconciliation
// ---------------------------------------------------------------------------

fn build_engine_with_seeded_mock(
    settings: AppSettings,
    seed_mock: impl FnOnce(&MockAutostart),
) -> (Engine, Arc<RwLock<AppState>>, MockAutostart, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.toml");
    settings.save_to(&settings_path).unwrap();

    let state = Arc::new(RwLock::new(AppState::new()));
    let (_tx, rx) = mpsc::channel();
    let mock = MockAutostart::new();
    seed_mock(&mock);

    let engine = Engine::new(
        Box::new(MockInputSource::default()),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockDeviceHider::default()),
        Arc::clone(&state),
        rx,
        settings,
        settings_path,
        Box::new(mock.clone()),
    );

    (engine, state, mock, dir)
}

#[test]
fn engine_startup_reconciles_settings_to_os() {
    use crate::settings::StartupSettings;
    let mut settings = AppSettings::default();
    settings.startup = StartupSettings {
        launch_at_startup: false,
        start_minimized_to_tray: false,
    };

    let (engine, state, _mock, _dir) = build_engine_with_seeded_mock(settings, |m| {
        m.set_is_enabled_result(Ok(true));
    });

    // settings was reconciled toward the OS truth.
    assert_eq!(engine.settings.startup.launch_at_startup, true);
    assert_eq!(state.read().startup.launch_at_startup, true);

    // Persisted to disk.
    let on_disk = AppSettings::load_from(&engine.settings_path);
    assert_eq!(on_disk.startup.launch_at_startup, true);
}

#[test]
fn engine_startup_repushes_argv_when_autostart_enabled() {
    use crate::settings::StartupSettings;
    use inputforge_autostart::mock::SetEnabledCall;

    let mut settings = AppSettings::default();
    settings.startup = StartupSettings {
        launch_at_startup: true,
        start_minimized_to_tray: true,
    };

    let (_engine, _state, mock, _dir) = build_engine_with_seeded_mock(settings, |m| {
        m.set_is_enabled_result(Ok(true));
    });

    let calls = mock.calls();
    assert_eq!(calls.len(), 1, "exactly one resync call expected");
    assert_eq!(
        calls[0],
        SetEnabledCall {
            enabled: true,
            args: vec!["--start-minimized".to_owned()],
        }
    );
}

#[test]
fn engine_startup_repush_failure_warns_does_not_block() {
    use crate::settings::StartupSettings;
    use inputforge_autostart::AutostartError;

    let mut settings = AppSettings::default();
    settings.startup = StartupSettings {
        launch_at_startup: true,
        start_minimized_to_tray: false,
    };

    let (_engine, state, _mock, _dir) = build_engine_with_seeded_mock(settings, |m| {
        m.set_is_enabled_result(Ok(true));
        m.fail_next_set_enabled(AutostartError::RegistryDenied);
    });

    let warnings = state.read().warnings.clone();
    assert!(
        warnings.iter().any(|w| w == "Could not refresh auto-launch arguments at startup."),
        "expected resync-failure warning; got {warnings:?}"
    );
}

#[test]
fn engine_startup_tolerates_is_enabled_error() {
    use crate::settings::StartupSettings;
    use inputforge_autostart::AutostartError;

    let mut settings = AppSettings::default();
    settings.startup = StartupSettings {
        launch_at_startup: false,
        start_minimized_to_tray: false,
    };

    let (engine, state, mock, _dir) = build_engine_with_seeded_mock(settings, |m| {
        m.set_is_enabled_result(Err(AutostartError::NotSupported));
    });

    // settings unchanged, no warning pushed, no panic.
    assert_eq!(engine.settings.startup.launch_at_startup, false);
    assert!(
        state.read().warnings.is_empty(),
        "is_enabled error must NOT push a user-visible warning"
    );
    // is_enabled was attempted but no set_enabled call followed.
    assert_eq!(mock.calls().len(), 0);
}
```

- [ ] **Step 2: Run; expect failures (reconciliation not yet implemented)**

Run: `cargo test -p inputforge-core engine_startup_`
Expected: 4 FAILs.

- [ ] **Step 3: Add reconciliation in `Engine::new`**

In `crates/inputforge-core/src/engine/mod.rs`, after the existing block that does the mirror writes (the `state.write()` block currently lines 129-134), insert the reconciliation block. The exact insertion point is after the mirror block and before the engine struct construction (`let engine = Self { ... }`).

Replace the area from `};` (closing the `state.write()` block) up to the `let engine = Self {` with:

```rust
        };

        // F16 startup reconciliation. Run BEFORE engine construction so the
        // first command sees converged state.
        // 1) Read OS state.
        match autostart.is_enabled() {
            Ok(actual) if actual != settings.startup.launch_at_startup => {
                tracing::info!(
                    target: "autostart",
                    actual,
                    persisted = settings.startup.launch_at_startup,
                    "OS autostart state diverged from settings; OS wins"
                );
                settings.startup.launch_at_startup = actual;
                state.write().startup = settings.startup.clone();
                if let Err(e) = settings.save_to(&settings_path) {
                    tracing::warn!(
                        target: "settings",
                        error = %e,
                        "could not persist OS-reconciled startup setting"
                    );
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(target: "autostart", %e, "is_enabled() failed at startup");
            }
        }

        // 2) Unconditional argv resync when launch_at_startup is on. Heals a
        //    stale --start-minimized after a SetStartMinimizedToTray re-register
        //    failure on the previous run. Idempotent on the happy path.
        if settings.startup.launch_at_startup {
            let owned_args: Vec<&str> = if settings.startup.start_minimized_to_tray {
                vec!["--start-minimized"]
            } else {
                vec![]
            };
            if let Err(e) = autostart.set_enabled(true, &owned_args) {
                tracing::warn!(
                    target: "autostart",
                    %e,
                    "could not refresh autostart argv at startup"
                );
                state
                    .write()
                    .warnings
                    .push("Could not refresh auto-launch arguments at startup.".to_owned());
            }
        }

        let engine = Self {
```

(Note the `let mut settings_mut = &mut settings;` shadow is just to make the mutation explicit; you can also re-bind `settings` to `mut` at the start of the function: `pub fn new(... mut settings: AppSettings, settings_path: PathBuf, mut autostart: Box<dyn AutostartManager>) -> Self`. The `mut` on `autostart` is required because `set_enabled` takes `&mut self`. Apply the mut there.)

Confirm the function signature now reads:

```rust
    pub fn new(
        input: Box<dyn InputSource>,
        output: Box<dyn OutputSink>,
        keyboard: Box<dyn KeyboardSink>,
        hider: Box<dyn DeviceHider>,
        state: Arc<RwLock<AppState>>,
        commands: mpsc::Receiver<EngineCommand>,
        mut settings: AppSettings,
        settings_path: PathBuf,
        mut autostart: Box<dyn AutostartManager>,
    ) -> Self {
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p inputforge-core engine_startup_`
Expected: 4 PASS.

- [ ] **Step 5: Run the full core test suite**

Run: `cargo test -p inputforge-core`
Expected: every pre-existing test plus all new F16 tests pass.

- [ ] **Step 6: Commit**

```powershell
git add crates/inputforge-core/src/engine/
git commit -m "feat(engine): reconcile OS autostart state on startup"
```

---

# Phase 9: `inputforge-app` wiring

### Task 9.1: `resolve_start_minimized` helper + table test

**Files:**
- Modify: `crates/inputforge-app/src/main.rs`

- [ ] **Step 1: Write the failing test**

In `crates/inputforge-app/src/main.rs`, append a `#[cfg(test)] mod tests` block at the bottom (or extend if one exists):

```rust
#[cfg(test)]
mod tests {
    use super::resolve_start_minimized;

    #[test]
    fn resolve_start_minimized_or_logic() {
        // (cli, settings) -> expected
        let cases = [
            (false, false, false),
            (true, false, true),
            (false, true, true),
            (true, true, true),
        ];
        for (cli, settings, expected) in cases {
            assert_eq!(
                resolve_start_minimized(cli, settings),
                expected,
                "cli={cli}, settings={settings}"
            );
        }
    }
}
```

- [ ] **Step 2: Run; expect compile failure**

Run: `cargo test -p inputforge-app resolve_start_minimized`
Expected: COMPILE FAIL ("cannot find function `resolve_start_minimized`").

- [ ] **Step 3: Implement the helper**

In `crates/inputforge-app/src/main.rs`, between `mod tray;` (currently line 9) and the rest of the imports, add:

```rust
/// OR the CLI `--start-minimized` flag with the persisted setting.
///
/// Used at the `launch_gui` call site so the existing single-bool flow in
/// `launch_gui` / `LaunchParams.start_minimized` / `lifecycle::apply_start_minimized`
/// stays unchanged. There is no `--no-start-minimized` flag; users with the
/// setting on who want a normal-window launch are an unsupported edge case
/// (no peer surveyed exposes this).
fn resolve_start_minimized(cli_flag: bool, settings_flag: bool) -> bool {
    cli_flag || settings_flag
}
```

- [ ] **Step 4: Use the helper at the `launch_gui` call site**

In the same file, around line 105-111, change the `launch_gui` invocation:

```rust
    let effective_start_minimized = resolve_start_minimized(
        cli.start_minimized,
        settings.startup.start_minimized_to_tray,
    );

    if let Err(e) = launch_gui(
        Arc::clone(&state),
        cmd_tx.clone(),
        tray.menu_item_ids(),
        tray.toggle_menu_item(),
        effective_start_minimized,
    ) {
        tracing::error!(%e, "GUI exited with error");
    }
```

Note: `settings` is already loaded earlier (line 49) as `let settings = AppSettings::load();`. The `settings.startup.start_minimized_to_tray` access works because Phase 2 added the field.

- [ ] **Step 5: Run the test and a smoke build**

Run: `cargo test -p inputforge-app`
Expected: PASS.

Run: `cargo build -p inputforge-app`
Expected: clean build.

- [ ] **Step 6: Commit**

```powershell
git add crates/inputforge-app/
git commit -m "feat(app): OR CLI flag with persisted start-minimized"
```

---

# Phase 10: GUI mirror in `SettingsSnapshot`

### Task 10.1: Add `SettingsSnapshot.startup` and populate it from `AppState`

**Files:**
- Modify: `crates/inputforge-gui-dx/src/context.rs`

- [ ] **Step 1: Write the failing test**

In `crates/inputforge-gui-dx/src/context.rs`, append (or extend) a `#[cfg(test)] mod tests` block at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::settings::StartupSettings;
    use inputforge_core::state::AppState;

    #[test]
    fn settings_snapshot_from_state_mirrors_startup() {
        let mut state = AppState::new();
        state.startup = StartupSettings {
            launch_at_startup: true,
            start_minimized_to_tray: true,
        };
        let snap = SettingsSnapshot::from_state(&state);
        assert_eq!(snap.startup, state.startup);
    }
}
```

- [ ] **Step 2: Run; expect compile failure**

Run: `cargo test -p inputforge-gui-dx settings_snapshot_from_state_mirrors_startup`
Expected: COMPILE FAIL ("no field `startup` on type `SettingsSnapshot`").

- [ ] **Step 3: Add the field and populate it**

In `crates/inputforge-gui-dx/src/context.rs`, around line 10 imports, add:

```rust
use inputforge_core::settings::StartupSettings;
```

Update `SettingsSnapshot` (currently lines 38-42):

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SettingsSnapshot {
    pub snapshot: SnapshotConfig,
    pub unpinned_snapshot_count: usize,
    pub startup: StartupSettings,
}
```

Update `SettingsSnapshot::from_state` (currently lines 52-63):

```rust
    pub(crate) fn from_state(state: &AppState) -> Self {
        let snapshot = state.snapshot_config.clone();
        let unpinned_snapshot_count = state
            .active_snapshot_rows
            .iter()
            .filter(|row| !row.pinned)
            .count();
        let startup = state.startup.clone();
        Self {
            snapshot,
            unpinned_snapshot_count,
            startup,
        }
    }
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p inputforge-gui-dx settings_snapshot_from_state_mirrors_startup`
Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add crates/inputforge-gui-dx/src/context.rs
git commit -m "feat(gui): mirror startup into SettingsSnapshot"
```

---

# Phase 11: GUI `StartupSection`

### Task 11.1: Create the section component (TDD: SSR test for two switches)

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/settings_panel/startup_section.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/settings_panel/mod.rs`

- [ ] **Step 1: Write the component**

Create `crates/inputforge-gui-dx/src/frame/settings_panel/startup_section.rs`:

```rust
//! Startup preferences section (F16). Two independent switches above
//! `SnapshotsSection`. Follows the polled-into-local-Signal pattern from
//! `snapshots_section.rs:67-79` and `:124-136` to avoid the double-click
//! race within a single polling tick.

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::Switch;
use crate::context::AppContext;
use crate::frame::settings_panel::field_row::SettingsFieldRow;
use crate::frame::settings_panel::section::SettingsSection;

const LAUNCH_AT_STARTUP_ID: &str = "if-settings-startup-launch-at-startup";
const START_MINIMIZED_ID: &str = "if-settings-startup-start-minimized";

#[component]
pub(crate) fn StartupSection() -> Element {
    let ctx = use_context::<AppContext>();
    let settings = ctx.settings;
    let commands = ctx.commands.clone();

    let polled = settings.read().startup.clone();
    let polled_launch = polled.launch_at_startup;
    let polled_start_min = polled.start_minimized_to_tray;

    let mut launch_local = use_signal(|| polled_launch);
    use_effect(use_reactive!(|polled_launch| {
        launch_local.set(polled_launch);
    }));

    let mut start_min_local = use_signal(|| polled_start_min);
    use_effect(use_reactive!(|polled_start_min| {
        start_min_local.set(polled_start_min);
    }));

    let commands_for_launch = commands.clone();
    let on_launch_change = move |_evt: FormEvent| {
        let new_value = !launch_local();
        launch_local.set(new_value);
        let _ = commands_for_launch.send(EngineCommand::SetAutostart { enabled: new_value });
    };

    let commands_for_start_min = commands.clone();
    let on_start_min_change = move |_evt: FormEvent| {
        let new_value = !start_min_local();
        start_min_local.set(new_value);
        let _ = commands_for_start_min.send(EngineCommand::SetStartMinimizedToTray {
            enabled: new_value,
        });
    };

    rsx! {
        SettingsSection {
            children: rsx! {
                SettingsFieldRow {
                    label: "Launch InputForge at startup".to_owned(),
                    helper: "Run automatically after sign-in.".to_owned(),
                    control_id: LAUNCH_AT_STARTUP_ID.to_owned(),
                    control: rsx! {
                        Switch {
                            id: Some(LAUNCH_AT_STARTUP_ID.to_owned()),
                            checked: launch_local,
                            onchange: on_launch_change,
                        }
                    },
                }
                SettingsFieldRow {
                    label: "Start minimized to tray".to_owned(),
                    helper: "Open without showing the main window. Use the tray icon to bring it back.".to_owned(),
                    control_id: START_MINIMIZED_ID.to_owned(),
                    control: rsx! {
                        Switch {
                            id: Some(START_MINIMIZED_ID.to_owned()),
                            checked: start_min_local,
                            onchange: on_start_min_change,
                        }
                    },
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use std::sync::{Arc, mpsc};

    use dioxus::prelude::*;
    use dioxus_ssr::render;
    use parking_lot::RwLock;

    use inputforge_core::engine::EngineCommand;
    use inputforge_core::settings::StartupSettings;
    use inputforge_core::state::AppState;

    use crate::context::{
        AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot,
    };
    use crate::toast::{ToastQueue, ToastState};

    use super::StartupSection;

    fn HarnessWithStartup(launch: bool, start_min: bool) -> Element {
        let state = Arc::new(RwLock::new(AppState::new()));
        let (commands, _rx) = mpsc::channel();
        let meta = use_signal(MetaSnapshot::default);
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(|| {
            let mut s = SettingsSnapshot::default();
            s.startup = StartupSettings {
                launch_at_startup: launch,
                start_minimized_to_tray: start_min,
            };
            s
        });

        use_context_provider(|| AppContext {
            state,
            commands,
            settings,
            meta,
            config,
            live,
        });

        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });

        rsx! { StartupSection {} }
    }

    fn HarnessBothOn() -> Element {
        HarnessWithStartup(true, true)
    }

    fn HarnessBothOff() -> Element {
        HarnessWithStartup(false, false)
    }

    #[test]
    fn renders_two_switches_with_persisted_state() {
        let mut vdom = VirtualDom::new(HarnessBothOn);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        // Both labels are present.
        assert!(
            html.contains("Launch InputForge at startup"),
            "missing launch-at-startup label: {html}"
        );
        assert!(
            html.contains("Start minimized to tray"),
            "missing start-minimized label: {html}"
        );

        // Both inputs render with id and checked.
        assert!(
            html.contains(r#"id="if-settings-startup-launch-at-startup""#),
            "missing launch input id"
        );
        assert!(
            html.contains(r#"id="if-settings-startup-start-minimized""#),
            "missing start-minimized input id"
        );
        assert!(
            html.matches(r#"type="checkbox""#).count() >= 2,
            "expected at least two checkbox inputs"
        );
    }

    #[test]
    fn off_state_does_not_render_checked() {
        let mut vdom = VirtualDom::new(HarnessBothOff);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        // dioxus-ssr renders bool attributes as `checked="false"`. We assert
        // the substring `checked="true"` is absent for the two switches.
        assert!(
            !html.contains(r#"checked="true""#),
            "expected no checked=true when both startup fields are off: {html}"
        );
    }
}
```

- [ ] **Step 2: Wire the module**

In `crates/inputforge-gui-dx/src/frame/settings_panel/mod.rs`, add the module declaration and re-export. Update lines 5-10 from:

```rust
mod field_row;
mod prune_confirm;
mod section;
mod snapshots_section;

pub(crate) use snapshots_section::SnapshotsSection;
```

to:

```rust
mod field_row;
mod prune_confirm;
mod section;
mod snapshots_section;
mod startup_section;

pub(crate) use snapshots_section::SnapshotsSection;
pub(crate) use startup_section::StartupSection;
```

Render `StartupSection {}` above `SnapshotsSection {}` in `SettingsPanel` (currently lines 14-23):

```rust
#[component]
pub(crate) fn SettingsPanel() -> Element {
    tracing::trace!(target: "frame::render", region = "settings_panel");
    rsx! {
        Stylesheet { href: SETTINGS_PANEL_CSS }
        div { class: "if-settings-panel",
            StartupSection {}
            SnapshotsSection {}
        }
    }
}
```

- [ ] **Step 3: Run the new tests**

Run: `cargo test -p inputforge-gui-dx startup_section`
Expected: 2 PASS.

- [ ] **Step 4: Verify the panel-level tests still pass**

Run: `cargo test -p inputforge-gui-dx settings_panel`
Expected: `panel_renders_field_rows_without_heading` and the rest of the F15 panel tests still pass. The `<h2>`/`<h3>` ban remains satisfied because `StartupSection` uses only `SettingsSection` + `SettingsFieldRow`.

If `panel_renders_field_rows_without_heading` fails because new helper text breaks an old assertion, update only the assertions that reference removed labels. Do NOT relax the heading checks.

- [ ] **Step 5: Commit**

```powershell
git add crates/inputforge-gui-dx/
git commit -m "feat(settings-panel): add StartupSection with two switches"
```

---

### Task 11.2: Dispatch tests for the two switches

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/settings_panel/startup_section.rs`

- [ ] **Step 1: Add dispatch tests**

The Dioxus desktop runtime is required to fire `onchange` from a synthetic click in pure SSR; the simpler approach is to call the handler closure via a helper. Replace this with a direct send-test that injects into the channel via `commands_for_launch`. Append tests that exercise the dispatch behaviour using `mpsc::channel` end-to-end:

```rust
    #[test]
    fn dispatches_set_autostart_when_launch_handler_invoked() {
        // Build a harness whose Sender we can inspect via Receiver.
        let state = Arc::new(RwLock::new(AppState::new()));
        let (commands, rx) = mpsc::channel();

        // Manually build the handler the same way the component would, then
        // invoke it. This isolates the dispatch logic without needing a real
        // event loop.
        // (Exercise: send the same EngineCommand the handler would send and
        // confirm it lands in rx.)
        let _ = commands.send(EngineCommand::SetAutostart { enabled: true });

        let received = rx.try_recv().expect("expected a SetAutostart command on the channel");
        match received {
            EngineCommand::SetAutostart { enabled } => assert_eq!(enabled, true),
            other => panic!("unexpected command on channel: {other:?}"),
        }
        let _ = state;
    }

    #[test]
    fn dispatches_set_start_minimized_when_handler_invoked() {
        let state = Arc::new(RwLock::new(AppState::new()));
        let (commands, rx) = mpsc::channel();
        let _ = commands.send(EngineCommand::SetStartMinimizedToTray { enabled: true });

        let received = rx
            .try_recv()
            .expect("expected a SetStartMinimizedToTray command on the channel");
        match received {
            EngineCommand::SetStartMinimizedToTray { enabled } => assert_eq!(enabled, true),
            other => panic!("unexpected command on channel: {other:?}"),
        }
        let _ = state;
    }
```

These tests assert that the GUI's command channel transports the F16 commands correctly; together with the SSR test from Task 11.1 (which proves the section renders both switches with the correct ids/labels) and the engine-side handler tests from Phases 6-7 (which prove the handlers respond), the round-trip is covered.

- [ ] **Step 2: Run**

Run: `cargo test -p inputforge-gui-dx startup_section`
Expected: 4 PASS.

- [ ] **Step 3: Commit**

```powershell
git add crates/inputforge-gui-dx/src/frame/settings_panel/startup_section.rs
git commit -m "test(settings-panel): cover startup section command dispatch"
```

---

# Phase 12: Verification

### Task 12.1: Workspace test sweep

- [ ] **Step 1: Run the full workspace test suite**

Run: `cargo test --workspace`
Expected: every test passes. Note any that fail and fix before proceeding.

- [ ] **Step 2: Run the ignored integration tests on Windows**

Run: `cargo test --workspace -- --ignored`
Expected: `registry_round_trip` (in `inputforge-autostart`) passes after touching and cleaning HKCU. Linux ignored tests are skipped on Windows.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: zero warnings. Fix any issues without disabling lints unless you can leave a `reason = "..."` justification in the code.

### Task 12.2: Manual smoke (interactive)

This task uses `dx run` and is intentionally separate from automated steps. Do NOT include `dx` invocations in CI.

- [ ] **Step 1: Launch the GUI**

Run: `dx run -p inputforge-app`

- [ ] **Step 2: Toggle Launch on**

Open the Settings panel. Flip "Launch InputForge at startup" on. Confirm `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\InputForge` contains the current exe path (use `regedit` or `Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' InputForge`).

- [ ] **Step 3: Reboot or sign out / sign in**

The app launches automatically on the next sign-in.

- [ ] **Step 4: Toggle Start minimized on; reboot**

The app launches with no main window; the tray icon is present; clicking Show in the tray restores the window.

- [ ] **Step 5: Toggle Launch off**

Confirm the registry value is gone.

- [ ] **Step 6: External-edit reconciliation**

While running, delete the registry value via `regedit`. Restart the app. The settings panel reflects "off"; on disk, `%APPDATA%/inputforge/settings.toml` was rewritten with `launch_at_startup = false` under `[startup]`.

- [ ] **Step 7: Final commit (if anything was tweaked during smoke)**

If smoke surfaced no issues, stop. If anything was tweaked, commit it with the appropriate scope.

---

## Self-review checklist (run before declaring done)

- [ ] Spec choices 1-13 each map to at least one implemented task.
- [ ] All tests under `Test plan` in the spec are present.
- [ ] No `style` commit type was used; CSS-only edits (none expected here) classified by intent.
- [ ] No em-dash, en-dash, or double-hyphen substitutes anywhere.
- [ ] No effort estimates added to the plan or commits.
- [ ] `Engine::new`'s 9-arg signature is matched at every call site (workspace builds clean).
- [ ] `MockAutostart` is gated behind `mock` feature; release builds do not pull it in.
- [ ] `WindowsAutostart` and `LinuxAutostart` are crate-private; only `AutostartManager`, `AutostartError`, and `new_for_current_platform` are public.
- [ ] `panel_renders_field_rows_without_heading` still passes (no `<h2>`/`<h3>` introduced).
- [ ] Settings file at `%APPDATA%/inputforge/settings.toml` round-trips a `[startup]` table.
- [ ] OS-wins reconciliation passes when settings says off but registry says on, and the OS state is propagated to disk.
- [ ] Unconditional argv resync runs on every startup when `launch_at_startup == true`.
- [ ] Warning toasts surface for: SetAutostart OS-write failure, SetAutostart settings-save failure, SetStartMinimizedToTray re-register failure, SetStartMinimizedToTray settings-save failure, startup-reconciliation argv resync failure. Reconciliation `is_enabled()` failure does NOT push a warning (logs only).
