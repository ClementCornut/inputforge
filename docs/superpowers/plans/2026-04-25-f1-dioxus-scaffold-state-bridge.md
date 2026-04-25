# F1 — Dioxus Crate Scaffold & State Bridge — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-04-24-f1-dioxus-scaffold-state-bridge-design.md`

**Goal:** Stand up `crates/inputforge-gui-dx` as a parallel Dioxus Desktop GUI crate, bridge `Arc<RwLock<AppState>>` into a three-signal live snapshot, and feature-flag the dispatch in `inputforge-app` — egui remains default, Dioxus opt-in.

**Architecture:** New workspace crate exposing `pub fn launch_gui(...) -> anyhow::Result<()>` with the same signature as the egui crate; `LaunchBuilder::with_context` ships raw handles (`Arc<RwLock<AppState>>`, `mpsc::Sender<EngineCommand>`, `Arc<AppSettings>`); root component creates three `Signal<T>` instances (Meta, Config, Live), assembles an `AppContext`, and spawns a 60Hz polling task that does `try_read()` + `from_state` + `PartialEq`-gated writes; `inputforge-app` picks the runtime via the `gui-egui` (default) or `gui-dioxus` feature, with two `cfg`-expressed lifecycle guards around the existing `launch_gui_blocking` call sites because `tao::EventLoop::run` is one-shot.

**Tech Stack:** Rust 2024 / rustc 1.85, Dioxus 0.7.x (desktop feature), tokio 1 (rt + time + sync), parking_lot, anyhow, tracing, muda 0.17 (kept for type-identity with `main.rs`'s `MenuId` triple — events are stubbed at F1, F3 wires them).

---

## Context

The parent rewrite plan (`2026-04-24-egui-to-dioxus-rewrite-design.md`) breaks the egui→Dioxus migration into 16 features (F1–F16). F1 is pure foundation: a smoke-test readout proving the engine→GUI state bridge works under Dioxus, with **zero** behavior change on the default-feature `main` branch.

Why this is worth a plan rather than just "hack it in":
- **Three-signal pattern is load-bearing.** Splitting state by update frequency (`meta` = lifecycle, `config` = topology, `live` = per-frame values) at F1 sets the rerender-economy contract for F2–F14. Doing it later means rewriting every component built against a single signal.
- **Lifecycle change is non-trivial.** `tao::EventLoop::run` is one-shot per process (Windows/wry/tao constraint); today's `main.rs` calls `launch_gui_blocking` at two sites. Without F1's `cfg`-expressed guards, `--features gui-dioxus` panics on the second tray **Show GUI** click.
- **Engine cleanup must not regress.** `shutdown(cmd_tx, engine_handle)` flushes `HidHide` unhide and `vJoy` release via engine `Drop` impls. Both lifecycle guards must fall through to that path on both feature flags.

Outcome at F1: under default features, behavior is identical to today; under `--features gui-dioxus`, a 1280×800 window opens showing engine status / mode / profile / device counts / warning count, ticks live at ~60Hz, and `dx serve` against `examples/bridge_demo.rs` hot-reloads RSX edits within ~1s.

## Critical files to modify

**Created (in `crates/inputforge-gui-dx/`):**
- `Cargo.toml` — package metadata (workspace-inherited) + deps (dioxus, tokio, muda, parking_lot, tracing, anyhow, serde, inputforge-core)
- `src/lib.rs` — `pub fn launch_gui(...) -> anyhow::Result<()>` + module declarations
- `src/context.rs` — `RawHandles`, `AppContext`, three snapshot structs (`MetaSnapshot`, `ConfigSnapshot`, `LiveSnapshot`), `DeviceInputValues`, `VjoyOutputValues`, `from_state` helpers, `#[cfg(test)] mod tests`
- `src/bridge.rs` — `spawn_polling_task`
- `src/app.rs` — `app_root` + `F1Readout` component
- `examples/bridge_demo.rs` — seeded `AppState` harness for primary RSX dev loop
- `README.md` — `dx serve` workflow, pinned versions, hot-reload validation

**Modified:**
- `Cargo.toml` (root) — add `crates/inputforge-gui-dx` to `members`; add `dioxus`, `tokio`, `inputforge-gui-dx` to `[workspace.dependencies]`
- `crates/inputforge-app/Cargo.toml` — `[features]` table (`default = ["gui-egui"]`, `gui-egui`, `gui-dioxus`); make `inputforge-gui` optional; add optional `inputforge-gui-dx`
- `crates/inputforge-app/src/main.rs` — two `compile_error!`s, `cfg`-gated `use` line, `IS_GUI_DIOXUS` const sentinel, two `cfg`-expressed post-launch guards (line 115 startup site & line 300 `ShowGui` arm)
- `crates/inputforge-gui/Cargo.toml` — add `anyhow` dep
- `crates/inputforge-gui/src/lib.rs` — `launch_gui` returns `anyhow::Result<()>` via `.map_err(anyhow::Error::from)`
- `crates/inputforge-core/src/state/status.rs` — add `Default` to `EngineStatus` derives, mark `#[default] Stopped`
- `crates/inputforge-core/src/state/device.rs` — add `PartialEq, Eq` to `DeviceState` derives

## Existing utilities to reuse

- `inputforge_core::state::AppState::new()` — construct test fixtures (`crates/inputforge-core/src/state/mod.rs:65`).
- `inputforge_core::state::InputCacheStore::update(&addr, &value)` — populate input cache for `LiveSnapshot::from_state` tests (`cache.rs:26`).
- `inputforge_core::state::OutputCacheStore::set_axis/set_button/set_hat` — populate output cache for `LiveSnapshot::from_state` tests (`output_cache.rs:34/39/44`).
- `inputforge_core::state::AppState::with_profile(profile)` — alternative constructor for tests that need `active_profile` populated (`state/mod.rs:86`).
- `Profile::name()` / `Profile::mappings()` — read accessors used in `MetaSnapshot::from_state` and `ConfigSnapshot::from_state` (`profile/mod.rs:152` / `:170`).
- `InputCache` trait methods on `InputCacheStore` (`get_axis(&addr) -> f64`, `get_button(&addr) -> bool`, `get_hat(&addr) -> HatDirection`) — directly used by `LiveSnapshot::from_state`.
- `OutputCacheStore::get_axis(device, axis) -> f64`, `get_button(device, button) -> bool`, `get_hat(device, hat) -> HatDirection` — same.
- `inputforge_core::types::{AxisPolarity, HatDirection, InputAddress, InputId, VJoyAxis, VirtualDeviceConfig}` — already `PartialEq, Eq, Clone` (verified during exploration).
- Existing `#[cfg(test)] mod tests { use super::*; }` pattern from `state/cache.rs` and `state/mod.rs` — copy this for `context.rs` test module.
- `crates/inputforge-app/src/main.rs::launch_gui_blocking` — unchanged; calls `inputforge_gui::launch_gui` (or the cfg-swapped Dioxus version) and post-processes the result. Already drains stale events via `drain_stale_gui_events(tray)`.

---

## Task 1: Pin Dioxus toolchain versions

Research-only. No commit. Output is a recorded version string used in Tasks 3 and 11.

**Files:** none modified.

- [ ] **Step 1: Run the `latest-packages` skill against `dioxus`**

Invoke the `latest-packages` skill with target `dioxus` on `crates.io`. Record the latest stable `0.7.x` version. Constrain to 0.7 because the spec is API-targeted at 0.7 (`LaunchBuilder::desktop`, `Config::with_window`, `WindowCloseBehaviour`, `DesktopService::set_visible`).

Expected output: a concrete version string like `0.7.X`.

- [ ] **Step 2: Run the `latest-packages` skill against `dioxus-cli`**

Same procedure, target `dioxus-cli`. The CLI version must be compatible with the chosen `dioxus` crate version (`dx --version` and `cargo install dioxus-cli --version <X>` are how this is consumed).

- [ ] **Step 3: Record both versions for downstream tasks**

Note the exact versions chosen. Tasks 3 (workspace `Cargo.toml`) and 11 (`README.md`) consume them verbatim. Refer to them below as `<DIOXUS_VERSION>` and `<DIOXUS_CLI_VERSION>`.

---

## Task 2: Add `Default` to `EngineStatus` and `PartialEq, Eq` to `DeviceState`

Two trivial additive derive changes the snapshot structs in Task 4 require to compile. TDD — both have one-line tests asserting the new behavior.

**Files:**
- Modify: `crates/inputforge-core/src/state/status.rs:8-16`
- Modify: `crates/inputforge-core/src/state/device.rs:10-15`

- [ ] **Step 1: Write the failing test for `EngineStatus::default()`**

Add to `crates/inputforge-core/src/state/status.rs`'s existing `mod tests` block (already present at lines 18–44):

```rust
#[test]
fn default_is_stopped() {
    assert_eq!(EngineStatus::default(), EngineStatus::Stopped);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p inputforge-core --lib state::status::tests::default_is_stopped`
Expected: FAIL — `EngineStatus` does not implement `Default`.

- [ ] **Step 3: Add `Default` to `EngineStatus`**

Edit `crates/inputforge-core/src/state/status.rs` lines 8–16 — change the derive line and annotate `Stopped`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EngineStatus {
    /// Actively polling input and executing pipelines.
    Running,
    /// Alive but dormant; input processing is skipped.
    Paused,
    /// Fully deactivated; virtual devices released.
    #[default]
    Stopped,
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p inputforge-core --lib state::status::tests::default_is_stopped`
Expected: PASS.

- [ ] **Step 5: Write the failing test for `DeviceState` equality**

Add to `crates/inputforge-core/src/state/device.rs`'s existing tests module (or create one if absent) — fetch a real `DeviceState` shape; the test only needs the equality check itself:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, DeviceInfo};

    fn sample() -> DeviceState {
        DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Stick".to_owned(),
                axes: 2,
                buttons: 4,
                hats: 1,
                instance_path: None,
                axis_polarities: vec![],
            },
            connected: true,
        }
    }

    #[test]
    fn equality_is_structural() {
        assert_eq!(sample(), sample());
        let mut other = sample();
        other.connected = false;
        assert_ne!(sample(), other);
    }
}
```

(If the file already has a `mod tests` block, place the test inside it and skip the wrapper boilerplate.)

- [ ] **Step 6: Run the test to verify it fails**

Run: `cargo test -p inputforge-core --lib state::device::tests::equality_is_structural`
Expected: FAIL — `DeviceState` does not implement `PartialEq`.

- [ ] **Step 7: Add `PartialEq, Eq` to `DeviceState`**

Edit `crates/inputforge-core/src/state/device.rs` line 10 — extend the derive:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceState {
```

`DeviceInfo` already derives `PartialEq, Eq` (`crates/inputforge-core/src/types/device.rs:24`); no transitive cascade needed.

- [ ] **Step 8: Run the test to verify it passes**

Run: `cargo test -p inputforge-core --lib state::device::tests::equality_is_structural`
Expected: PASS.

- [ ] **Step 9: Run the full core test suite to confirm nothing else regressed**

Run: `cargo test -p inputforge-core`
Expected: PASS, no new failures.

- [ ] **Step 10: Commit**

```bash
git add crates/inputforge-core/src/state/status.rs crates/inputforge-core/src/state/device.rs
git commit -m "feat(core): derive Default on EngineStatus and PartialEq on DeviceState"
```

(Use `superpowers:conventional-commits` skill before committing per the workspace convention.)

---

## Task 3: Workspace plumbing — new crate skeleton + workspace `Cargo.toml`

Create the empty `inputforge-gui-dx` crate so the workspace compiles before any logic lands. Adds `dioxus`, `tokio`, and the new internal crate to `[workspace.dependencies]`.

**Files:**
- Create: `crates/inputforge-gui-dx/Cargo.toml`
- Create: `crates/inputforge-gui-dx/src/lib.rs`
- Modify: `Cargo.toml` (root) — `members` array and `[workspace.dependencies]`

- [ ] **Step 1: Create the new crate's `Cargo.toml`**

Write `crates/inputforge-gui-dx/Cargo.toml`:

```toml
[package]
name = "inputforge-gui-dx"
version.workspace      = true
edition.workspace      = true
license.workspace      = true
rust-version.workspace = true
description.workspace  = true
repository.workspace   = true
readme.workspace       = true
keywords.workspace     = true
categories.workspace   = true

[dependencies]
inputforge-core = { workspace = true }
dioxus          = { workspace = true, features = ["desktop"] }
tokio           = { workspace = true, features = ["rt", "time", "sync"] }
muda            = { workspace = true }
parking_lot     = { workspace = true }
tracing         = { workspace = true }
serde           = { workspace = true }
anyhow          = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Create the placeholder `lib.rs`**

Write `crates/inputforge-gui-dx/src/lib.rs`:

```rust
//! Dioxus Desktop GUI for InputForge (F1 scaffold).
//!
//! `launch_gui` lands in subsequent tasks. This file exists so the
//! workspace compiles after `Cargo.toml` registers the crate.
```

- [ ] **Step 3: Add the new crate to workspace members and add deps**

Edit the root `Cargo.toml`:
- In `[workspace] members = [...]`, add `"crates/inputforge-gui-dx"` to the existing list (currently three entries: core, gui, app).
- In `[workspace.dependencies]`, add three new entries (substitute `<DIOXUS_VERSION>` from Task 1):

```toml
dioxus            = "<DIOXUS_VERSION>"
tokio             = { version = "1", default-features = false, features = ["rt", "time", "sync"] }
inputforge-gui-dx = { path = "crates/inputforge-gui-dx" }
```

The crate-level `[dependencies]` re-enables Dioxus's `desktop` feature on top of workspace defaults. `tokio` is feature-scoped: only `rt`/`time`/`sync` are needed by the polling task.

- [ ] **Step 4: Verify the workspace compiles**

Run: `cargo check --workspace`
Expected: PASS — all four crates compile (the new crate has no code yet, just the lib.rs comment).

- [ ] **Step 5: Verify lints are silent**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS — no new lint output from the new crate (it has no code).

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/Cargo.toml crates/inputforge-gui-dx/src/lib.rs Cargo.toml
git commit -m "feat(gui-dx): scaffold inputforge-gui-dx crate"
```

---

## Task 4: Snapshot types in `context.rs` (no `from_state` yet)

Define the structural pieces every later task depends on: `RawHandles`, the three snapshot structs, the inner `DeviceInputValues` / `VjoyOutputValues`, and `AppContext`. `from_state` helpers come in Tasks 5–7.

**Files:**
- Create: `crates/inputforge-gui-dx/src/context.rs`
- Modify: `crates/inputforge-gui-dx/src/lib.rs` — declare `mod context;`

- [ ] **Step 1: Write the failing test for snapshot defaults**

Create `crates/inputforge-gui-dx/src/context.rs` with the test module first (RED phase). Write:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::state::EngineStatus;

    #[test]
    fn meta_snapshot_default_is_empty() {
        let m = MetaSnapshot::default();
        assert_eq!(m.engine_status, EngineStatus::Stopped);
        assert!(m.current_mode.is_empty());
        assert!(m.profile_name.is_none());
        assert!(m.profile_path.is_none());
        assert!(m.warnings.is_empty());
    }

    #[test]
    fn config_snapshot_default_is_empty() {
        let c = ConfigSnapshot::default();
        assert!(c.devices.is_empty());
        assert!(c.virtual_devices.is_empty());
        assert!(c.mapped_inputs.is_empty());
        assert!(c.mapping_names.is_empty());
    }

    #[test]
    fn live_snapshot_default_is_empty() {
        let l = LiveSnapshot::default();
        assert!(l.device_inputs.is_empty());
        assert!(l.output_values.is_empty());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails to compile**

Run: `cargo test -p inputforge-gui-dx --lib`
Expected: FAIL — `MetaSnapshot`, `ConfigSnapshot`, `LiveSnapshot` are not defined; `mod context` is not declared.

- [ ] **Step 3: Add `mod context;` declaration to `lib.rs`**

Edit `crates/inputforge-gui-dx/src/lib.rs` — replace the placeholder doc-comment with:

```rust
//! Dioxus Desktop GUI for InputForge.

mod context;
```

(Remaining modules `bridge` and `app` land in Tasks 8 / 9.)

- [ ] **Step 4: Define the snapshot structs and `RawHandles` and `AppContext`**

Add to `crates/inputforge-gui-dx/src/context.rs` (above the `mod tests` block):

```rust
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState, EngineStatus};
use inputforge_core::types::{
    AxisPolarity, HatDirection, InputAddress, VJoyAxis, VirtualDeviceConfig,
};

/// Raw signal-free handles installed via `LaunchBuilder::with_context`.
///
/// `Arc<AppSettings>` is a zero-cost read-only handle at F1; F14 will
/// unwind this wrapping when adding the mutation path.
#[derive(Clone, Debug)]
pub(crate) struct RawHandles {
    pub state:    Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
    pub settings: Arc<AppSettings>,
}

/// Full per-window context: raw handles plus the three reactive signals.
/// Assembled inside `app_root` (signals must be created within the runtime).
#[derive(Clone, Debug)]
pub(crate) struct AppContext {
    pub state:    Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
    pub settings: Arc<AppSettings>,
    pub meta:     Signal<MetaSnapshot>,
    pub config:   Signal<ConfigSnapshot>,
    pub live:     Signal<LiveSnapshot>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MetaSnapshot {
    pub engine_status: EngineStatus,
    pub current_mode:  String,
    pub profile_name:  Option<String>,
    pub profile_path:  Option<PathBuf>,
    pub warnings:      Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ConfigSnapshot {
    pub devices:         Vec<DeviceState>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    pub mapped_inputs:   HashSet<InputAddress>,
    pub mapping_names:   HashMap<InputAddress, String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LiveSnapshot {
    pub device_inputs: Vec<DeviceInputValues>,
    pub output_values: Vec<VjoyOutputValues>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DeviceInputValues {
    pub axes:    Vec<(f64, AxisPolarity)>,
    pub buttons: Vec<bool>,
    pub hats:    Vec<HatDirection>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct VjoyOutputValues {
    pub axes:    Vec<(VJoyAxis, f64)>,
    pub buttons: Vec<bool>,
    pub hats:    Vec<HatDirection>,
}
```

If `Signal<T>` doesn't yet implement `Debug` on the pinned 0.7.x, hand-write a `Debug` impl for `AppContext` that skips the three `Signal` fields. The spec calls this out (line 299–304); the workaround is trivial.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib`
Expected: PASS — all three default-snapshot tests pass.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/lib.rs crates/inputforge-gui-dx/src/context.rs
git commit -m "feat(gui-dx): define RawHandles, AppContext, and snapshot structs"
```

---

## Task 5: `MetaSnapshot::from_state` (TDD)

Pure function over `&AppState`. Lives next to the struct in `context.rs`; tested in the same `mod tests` block.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/context.rs`

- [ ] **Step 1: Write the failing test**

Append to the `mod tests` block in `context.rs`:

```rust
#[test]
fn meta_from_state_extracts_lifecycle_fields() {
    use inputforge_core::state::{AppState, EngineStatus};
    use std::path::PathBuf;

    let mut state = AppState::new();
    state.engine_status = EngineStatus::Running;
    state.current_mode = "FlightAssist".to_owned();
    state.warnings.push("HidHide unavailable".to_owned());
    state.profile_path = Some(PathBuf::from("/tmp/profile.json"));

    let meta = MetaSnapshot::from_state(&state);
    assert_eq!(meta.engine_status, EngineStatus::Running);
    assert_eq!(meta.current_mode, "FlightAssist");
    assert_eq!(meta.profile_name, None); // active_profile is None
    assert_eq!(meta.profile_path, Some(PathBuf::from("/tmp/profile.json")));
    assert_eq!(meta.warnings, vec!["HidHide unavailable".to_owned()]);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib meta_from_state_extracts_lifecycle_fields`
Expected: FAIL — `MetaSnapshot::from_state` is not defined.

- [ ] **Step 3: Implement `from_state`**

Add to `context.rs` (anywhere above `mod tests`):

```rust
impl MetaSnapshot {
    pub fn from_state(s: &AppState) -> Self {
        Self {
            engine_status: s.engine_status,
            current_mode:  s.current_mode.clone(),
            profile_name:  s.active_profile.as_ref().map(|p| p.name().to_owned()),
            profile_path:  s.profile_path.clone(),
            warnings:      s.warnings.clone(),
        }
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p inputforge-gui-dx --lib meta_from_state_extracts_lifecycle_fields`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/context.rs
git commit -m "feat(gui-dx): add MetaSnapshot::from_state"
```

---

## Task 6: `ConfigSnapshot::from_state` (TDD)

Extracts device topology and the indexed-by-`InputAddress` mapping lookups (used by every UI that draws "is this input mapped?" indicators in F6+).

**Files:**
- Modify: `crates/inputforge-gui-dx/src/context.rs`

- [ ] **Step 1: Write the failing test**

Append to `mod tests`:

```rust
#[test]
fn config_from_state_clones_devices_and_virtual_devices() {
    use inputforge_core::state::{AppState, DeviceState};
    use inputforge_core::types::{
        AxisPolarity, DeviceId, DeviceInfo, VJoyAxis, VirtualDeviceConfig,
    };

    let mut state = AppState::new();
    state.devices.push(DeviceState {
        info: DeviceInfo {
            id: DeviceId("dev-1".to_owned()),
            name: "Throttle".to_owned(),
            axes: 1,
            buttons: 0,
            hats: 0,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Unipolar],
        },
        connected: true,
    });
    state.virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X],
        button_count: 4,
        hat_count: 0,
    });

    let cfg = ConfigSnapshot::from_state(&state);
    assert_eq!(cfg.devices.len(), 1);
    assert_eq!(cfg.devices[0].info.name, "Throttle");
    assert_eq!(cfg.virtual_devices.len(), 1);
    assert_eq!(cfg.virtual_devices[0].button_count, 4);
    assert!(cfg.mapped_inputs.is_empty());   // no profile loaded
    assert!(cfg.mapping_names.is_empty());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib config_from_state_clones_devices_and_virtual_devices`
Expected: FAIL — `ConfigSnapshot::from_state` is not defined.

- [ ] **Step 3: Implement `from_state`**

Add to `context.rs`:

```rust
impl ConfigSnapshot {
    pub fn from_state(s: &AppState) -> Self {
        let mut mapped_inputs = HashSet::new();
        let mut mapping_names = HashMap::new();
        if let Some(profile) = &s.active_profile {
            for mapping in profile.mappings() {
                mapped_inputs.insert(mapping.input.clone());
                if let Some(name) = &mapping.name {
                    mapping_names.insert(mapping.input.clone(), name.clone());
                }
            }
        }
        Self {
            devices:         s.devices.clone(),
            virtual_devices: s.virtual_devices.clone(),
            mapped_inputs,
            mapping_names,
        }
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p inputforge-gui-dx --lib config_from_state_clones_devices_and_virtual_devices`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/context.rs
git commit -m "feat(gui-dx): add ConfigSnapshot::from_state"
```

---

## Task 7: `LiveSnapshot::from_state` (TDD)

The longest of the three. Reads `input_cache` for every connected device's axes/buttons/hats and `output_cache` for every virtual device's outputs. Takes a pre-built `ConfigSnapshot` so positional indices stay coherent.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/context.rs`

- [ ] **Step 1: Write the failing test**

Append to `mod tests`:

```rust
#[test]
fn live_from_state_reads_caches_per_device_shape() {
    use inputforge_core::state::{AppState, DeviceState};
    use inputforge_core::types::{
        AxisPolarity, AxisValue, DeviceId, DeviceInfo, HatDirection, InputAddress, InputId,
        InputValue, VJoyAxis, VirtualDeviceConfig,
    };

    let mut state = AppState::new();
    let did = DeviceId("dev-1".to_owned());

    state.devices.push(DeviceState {
        info: DeviceInfo {
            id: did.clone(),
            name: "Joystick".to_owned(),
            axes: 1,
            buttons: 1,
            hats: 1,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar],
        },
        connected: true,
    });
    state.virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X],
        button_count: 1,
        hat_count: 1,
    });

    state.input_cache.update(
        &InputAddress { device: did.clone(), input: InputId::Axis { index: 0 } },
        &InputValue::Axis { value: AxisValue::new(0.5) },
    );
    state.input_cache.update(
        &InputAddress { device: did.clone(), input: InputId::Button { index: 0 } },
        &InputValue::Button { pressed: true },
    );
    state.input_cache.update(
        &InputAddress { device: did, input: InputId::Hat { index: 0 } },
        &InputValue::Hat { direction: HatDirection::N },
    );

    state.output_cache.set_axis(1, VJoyAxis::X, -0.25);
    state.output_cache.set_button(1, 1, true);
    state.output_cache.set_hat(1, 0, HatDirection::SE);

    let cfg = ConfigSnapshot::from_state(&state);
    let live = LiveSnapshot::from_state(&state, &cfg);

    assert_eq!(live.device_inputs.len(), 1);
    assert_eq!(live.device_inputs[0].axes.len(), 1);
    assert!((live.device_inputs[0].axes[0].0 - 0.5).abs() < f64::EPSILON);
    assert_eq!(live.device_inputs[0].axes[0].1, AxisPolarity::Bipolar);
    assert_eq!(live.device_inputs[0].buttons, vec![true]);
    assert_eq!(live.device_inputs[0].hats, vec![HatDirection::N]);

    assert_eq!(live.output_values.len(), 1);
    assert!((live.output_values[0].axes[0].1 - (-0.25)).abs() < f64::EPSILON);
    assert_eq!(live.output_values[0].axes[0].0, VJoyAxis::X);
    assert_eq!(live.output_values[0].buttons, vec![true]);
    assert_eq!(live.output_values[0].hats, vec![HatDirection::SE]);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib live_from_state_reads_caches_per_device_shape`
Expected: FAIL — `LiveSnapshot::from_state` is not defined.

- [ ] **Step 3: Implement `from_state`**

Add to `context.rs`:

```rust
use inputforge_core::pipeline::InputCache;

use inputforge_core::types::InputId;

impl LiveSnapshot {
    /// Takes a pre-built `ConfigSnapshot` so device / virtual-device shape is
    /// read from a single coherent source.
    pub fn from_state(s: &AppState, cfg: &ConfigSnapshot) -> Self {
        let device_inputs: Vec<DeviceInputValues> = cfg.devices.iter().map(|device| {
            let did = &device.info.id;
            DeviceInputValues {
                axes: (0..device.info.axes).map(|i| {
                    let addr = InputAddress {
                        device: did.clone(),
                        input: InputId::Axis { index: i },
                    };
                    let pol = device.info.axis_polarities
                        .get(usize::from(i))
                        .copied()
                        .unwrap_or_default();
                    (s.input_cache.get_axis(&addr), pol)
                }).collect(),
                buttons: (0..device.info.buttons).map(|i| {
                    let addr = InputAddress {
                        device: did.clone(),
                        input: InputId::Button { index: i },
                    };
                    s.input_cache.get_button(&addr)
                }).collect(),
                hats: (0..device.info.hats).map(|i| {
                    let addr = InputAddress {
                        device: did.clone(),
                        input: InputId::Hat { index: i },
                    };
                    s.input_cache.get_hat(&addr)
                }).collect(),
            }
        }).collect();

        let output_values: Vec<VjoyOutputValues> = cfg.virtual_devices.iter().map(|v| {
            VjoyOutputValues {
                axes: v.axes.iter()
                    .map(|&a| (a, s.output_cache.get_axis(v.device_id, a)))
                    .collect(),
                buttons: (1..=v.button_count)
                    .map(|i| s.output_cache.get_button(v.device_id, i))
                    .collect(),
                hats: (0..v.hat_count)
                    .map(|i| s.output_cache.get_hat(v.device_id, i))
                    .collect(),
            }
        }).collect();

        Self { device_inputs, output_values }
    }
}
```

`InputCache` trait import is required because `get_axis` / `get_button` / `get_hat` are trait methods on `InputCacheStore`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p inputforge-gui-dx --lib live_from_state_reads_caches_per_device_shape`
Expected: PASS.

- [ ] **Step 5: Run all crate tests**

Run: `cargo test -p inputforge-gui-dx`
Expected: 3 default tests + 3 from_state tests = 6 PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/context.rs
git commit -m "feat(gui-dx): add LiveSnapshot::from_state"
```

---

## Task 8: `bridge.rs::spawn_polling_task`

The 60Hz polling loop. `try_read()` + build snapshots + `PartialEq`-gated `set` on each signal. Idle state ⇒ no rerenders.

**Files:**
- Create: `crates/inputforge-gui-dx/src/bridge.rs`
- Modify: `crates/inputforge-gui-dx/src/lib.rs` — declare `mod bridge;`

- [ ] **Step 1: Add `mod bridge;` to `lib.rs`**

Edit `crates/inputforge-gui-dx/src/lib.rs`:

```rust
//! Dioxus Desktop GUI for InputForge.

mod bridge;
mod context;
```

- [ ] **Step 2: Implement `spawn_polling_task`**

Create `crates/inputforge-gui-dx/src/bridge.rs`:

```rust
use std::time::Duration;

use dioxus::prelude::*;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};

/// Spawn the 60Hz state-bridge polling task.
///
/// Each tick: non-blocking `try_read()` of `AppState`, rebuild the three
/// snapshots, write each via `Signal::set` only when `PartialEq` differs.
/// Idle state produces no wake-ups even while ticking.
///
/// The task is bound to the Dioxus runtime: it is auto-cancelled when the
/// runtime tears down on window close.
pub(crate) fn spawn_polling_task(ctx: AppContext) {
    spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(16));
        loop {
            tick.tick().await;

            // Non-blocking: if the engine is currently writing, skip this tick.
            // One missed tick at 60Hz is imperceptible.
            let Some(guard) = ctx.state.try_read() else { continue };

            let meta   = MetaSnapshot::from_state(&guard);
            let config = ConfigSnapshot::from_state(&guard);
            let live   = LiveSnapshot::from_state(&guard, &config);
            drop(guard);

            // peek() reads without subscribing — the diff gate doesn't wake any component.
            let mut meta_signal   = ctx.meta;
            let mut config_signal = ctx.config;
            let mut live_signal   = ctx.live;
            if *meta_signal.peek()   != meta   { meta_signal.set(meta); }
            if *config_signal.peek() != config { config_signal.set(config); }
            if *live_signal.peek()   != live   { live_signal.set(live); }
        }
    });
}
```

(`Signal::set` requires `&mut self` on Dioxus 0.7 — copy the field into a local mutable binding. `Signal: Copy`, so the rebind is free.)

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p inputforge-gui-dx`
Expected: PASS.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/lib.rs crates/inputforge-gui-dx/src/bridge.rs
git commit -m "feat(gui-dx): add 60Hz state-bridge polling task"
```

---

## Task 9: `app.rs::app_root` + `F1Readout`

The root component creates the three signals (within the runtime), assembles `AppContext`, installs it via `use_context_provider`, spawns the polling task once via `use_hook`, and renders the F1 smoke-test readout.

**Files:**
- Create: `crates/inputforge-gui-dx/src/app.rs`
- Modify: `crates/inputforge-gui-dx/src/lib.rs` — declare `mod app;`

- [ ] **Step 1: Add `mod app;` to `lib.rs`**

Edit `crates/inputforge-gui-dx/src/lib.rs`:

```rust
//! Dioxus Desktop GUI for InputForge.

mod app;
mod bridge;
mod context;
```

- [ ] **Step 2: Implement the root component and readout**

Create `crates/inputforge-gui-dx/src/app.rs`:

```rust
use dioxus::prelude::*;

use crate::bridge::spawn_polling_task;
use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};

/// Root Dioxus component — assembles `AppContext`, installs it for descendants,
/// spawns the polling task once, and renders the F1 readout.
pub(crate) fn app_root() -> Element {
    let raw = use_context::<RawHandles>();

    let meta   = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live   = use_signal(LiveSnapshot::default);

    let ctx = AppContext {
        state:    raw.state.clone(),
        commands: raw.commands.clone(),
        settings: raw.settings.clone(),
        meta, config, live,
    };
    use_context_provider(|| ctx.clone());

    // One-shot per scope mount; auto-cancelled when the runtime tears down.
    use_hook(|| spawn_polling_task(ctx.clone()));

    rsx! { F1Readout {} }
}

#[component]
fn F1Readout() -> Element {
    let ctx = use_context::<AppContext>();

    let status   = use_memo(move || format!("{:?}", ctx.meta.read().engine_status));
    let mode     = use_memo(move || ctx.meta.read().current_mode.clone());
    let profile  = use_memo(move || {
        ctx.meta.read().profile_name.clone().unwrap_or_else(|| "<none>".into())
    });
    let devices  = use_memo(move || ctx.config.read().devices.len());
    let vdevices = use_memo(move || ctx.config.read().virtual_devices.len());
    let warnings = use_memo(move || ctx.meta.read().warnings.len());

    rsx! {
        main {
            style: "font-family: system-ui; padding: 24px; color: #ddd; \
                    background: #1A1A2E; min-height: 100vh;",
            h1 { "InputForge — Dioxus (F1 bridge smoke test)" }
            p { "Engine status: "     strong { "{status}" } }
            p { "Current mode: "      strong { "{mode}" } }
            p { "Active profile: "    strong { "{profile}" } }
            p { "Connected devices: " strong { "{devices}" } }
            p { "Virtual devices: "   strong { "{vdevices}" } }
            p { "Warnings: "          strong { "{warnings}" } }
            hr {}
            small { "Tray wiring: stubbed (F3). Theme: F2. Layout: F3." }
        }
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p inputforge-gui-dx`
Expected: PASS — `app_root` is `pub(crate)`; `lib.rs` will reference it in Task 10.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/lib.rs crates/inputforge-gui-dx/src/app.rs
git commit -m "feat(gui-dx): add app_root and F1Readout component"
```

---

## Task 10: `lib.rs::launch_gui`

The public entry point. Identical signature to `inputforge_gui::launch_gui` so `inputforge-app/src/main.rs` can swap them with a single `cfg`-gated `use` line.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/lib.rs`

- [ ] **Step 1: Replace `lib.rs` with the full implementation**

Overwrite `crates/inputforge-gui-dx/src/lib.rs`:

```rust
//! Dioxus Desktop GUI for InputForge.

mod app;
mod bridge;
mod context;

use std::sync::Arc;
use std::sync::mpsc;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;
use muda::MenuId;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::RawHandles;

/// Launch the Dioxus Desktop GUI. Blocks the calling thread on the OS event
/// loop (wry/tao underneath) — matches the egui crate's `eframe::run_native`
/// blocking semantics.
///
/// `tray_menu_ids` is accepted for signature parity with `inputforge_gui::launch_gui`
/// but is stubbed at F1; F3 wires the listener task that consumes it.
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
) -> anyhow::Result<()> {
    tracing::debug!(?tray_menu_ids, "tray wiring stubbed until F3");
    let _ = tray_menu_ids;

    let handles = RawHandles {
        state,
        commands,
        settings: Arc::new(settings),
    };

    let window = WindowBuilder::new()
        .with_title("InputForge")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(800.0, 500.0));

    let cfg = Config::new().with_window(window);

    LaunchBuilder::desktop()
        .with_cfg(cfg)
        .with_context(handles)
        .launch(app::app_root);

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p inputforge-gui-dx`
Expected: PASS.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 4: Run tests to confirm no regression**

Run: `cargo test -p inputforge-gui-dx`
Expected: 6 PASS (3 default + 3 from_state).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/lib.rs
git commit -m "feat(gui-dx): expose launch_gui entry point"
```

---

## Task 11: `examples/bridge_demo.rs` and `README.md`

The primary RSX dev-loop harness. Mocks `AppState` with seeded data, leaks the `EngineCommand` receiver, and calls `launch_gui` directly. Zero side effects per hot-reload cycle.

**Files:**
- Create: `crates/inputforge-gui-dx/examples/bridge_demo.rs`
- Create: `crates/inputforge-gui-dx/README.md`

- [ ] **Step 1: Write the example**

Create `crates/inputforge-gui-dx/examples/bridge_demo.rs`:

```rust
//! Primary RSX dev-loop harness.
//!
//! Builds a mock `AppState` with seeded device / virtual-device / profile
//! entries, wraps it in `Arc<RwLock<_>>`, builds a drop-channel
//! `mpsc::Sender<EngineCommand>` whose receiver is leaked, and calls
//! `launch_gui` directly. No engine thread, no tray, no profile I/O,
//! no `HidHide` scan — predictable seeded data, hot-reload safe.
//!
//! Run via:
//!     dx serve --example bridge_demo --platform desktop

use std::sync::{Arc, mpsc};

use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState, EngineStatus};
use inputforge_core::types::{
    AxisPolarity, DeviceId, DeviceInfo, VJoyAxis, VirtualDeviceConfig,
};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();

    let mut state = AppState::new();
    state.engine_status = EngineStatus::Running;
    state.current_mode = "Demo".to_owned();
    state.warnings.push("This is a seeded demo — no engine attached.".to_owned());

    state.devices.push(DeviceState {
        info: DeviceInfo {
            id: DeviceId("demo-stick".to_owned()),
            name: "Demo Stick".to_owned(),
            axes: 4,
            buttons: 12,
            hats: 1,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; 4],
        },
        connected: true,
    });

    state.virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X, VJoyAxis::Y, VJoyAxis::Rz],
        button_count: 8,
        hat_count: 1,
    });

    let state = Arc::new(RwLock::new(state));

    // Drop-channel: the receiver is leaked so engine sends don't error.
    let (commands, rx) = mpsc::channel::<EngineCommand>();
    Box::leak(Box::new(rx));

    // Stub menu IDs — `launch_gui` ignores them at F1.
    let menu_ids = (
        muda::MenuId::new("show-gui"),
        muda::MenuId::new("toggle-activation"),
        muda::MenuId::new("quit"),
    );

    inputforge_gui_dx::launch_gui(state, commands, menu_ids, AppSettings::default())
}
```

This requires `tracing-subscriber` and `anyhow` as dev-dependencies. Add them to `crates/inputforge-gui-dx/Cargo.toml`:

```toml
[dev-dependencies]
tracing-subscriber = { workspace = true }
anyhow             = { workspace = true }
```

(Verify `tracing-subscriber` is already in `[workspace.dependencies]` in the root `Cargo.toml`. If not, add it with the workspace-standard version.)

- [ ] **Step 2: Verify the example compiles**

Run: `cargo build -p inputforge-gui-dx --example bridge_demo`
Expected: PASS.

- [ ] **Step 3: Run the example as a smoke test**

Run: `cargo run -p inputforge-gui-dx --example bridge_demo`
Expected: A 1280×800 dark-themed window opens titled "InputForge", showing:
- Engine status: **Running**
- Current mode: **Demo**
- Active profile: **&lt;none&gt;**
- Connected devices: **1**
- Virtual devices: **1**
- Warnings: **1**

Close the window to exit.

- [ ] **Step 4: Write the README**

Create `crates/inputforge-gui-dx/README.md`:

```markdown
# inputforge-gui-dx

Dioxus Desktop GUI for InputForge — parallel runtime, opt-in via the
`gui-dioxus` feature on `inputforge-app`. The egui crate (`inputforge-gui`)
remains the default until the F16 cutover.

## Pinned versions

- `dioxus`: `<DIOXUS_VERSION>` (workspace-pinned, `desktop` feature)
- `dioxus-cli`: `<DIOXUS_CLI_VERSION>`

## Dev workflow — primary RSX loop (recommended)

The `bridge_demo` example seeds a mock `AppState` and calls `launch_gui`
directly. No engine, no tray, no profile I/O — safe to hot-reload.

```bash
cargo install dioxus-cli --version <DIOXUS_CLI_VERSION>
cd crates/inputforge-gui-dx
dx serve --example bridge_demo --platform desktop
```

Edit RSX in `src/app.rs` — the running window updates within ~1s without
restarting. Rust logic / state / non-RSX changes still require a full rebuild.

## Dev workflow — full app integration smoke

Exercises the real engine thread, tray, profile autoload, and HidHide
warning scan. **Not** the daily loop — each hot-reload respawns the engine
thread, re-registers the tray, re-runs HidHide detection.

```bash
cd crates/inputforge-app
dx serve --platform desktop --no-default-features --features gui-dioxus
```

## Build / run matrix

| Command | Result |
|---|---|
| `cargo build` / `cargo run` | egui (default) |
| `cargo build --no-default-features --features gui-dioxus` | Dioxus |
| `cargo run --no-default-features --features gui-dioxus`   | Dioxus |
| `cargo build --features gui-dioxus` (default still on)    | compile error |
| `cargo build --no-default-features`                       | compile error |
```

Substitute `<DIOXUS_VERSION>` and `<DIOXUS_CLI_VERSION>` from Task 1.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/Cargo.toml crates/inputforge-gui-dx/examples/bridge_demo.rs crates/inputforge-gui-dx/README.md
git commit -m "docs(gui-dx): add bridge_demo example and README"
```

---

## Task 12: Change egui crate's `launch_gui` to return `anyhow::Result<()>`

Single-line body change to keep both crates' signatures identical. Add `anyhow` as a dependency since the egui crate doesn't currently have it.

**Files:**
- Modify: `crates/inputforge-gui/Cargo.toml`
- Modify: `crates/inputforge-gui/src/lib.rs`

- [ ] **Step 1: Add `anyhow` to the egui crate's dependencies**

Edit `crates/inputforge-gui/Cargo.toml`'s `[dependencies]`:

```toml
anyhow = { workspace = true }
```

- [ ] **Step 2: Update the return type and body**

Edit `crates/inputforge-gui/src/lib.rs` lines 34–68 — change the return type on `launch_gui` from `eframe::Result<()>` to `anyhow::Result<()>` and adapt the final `eframe::run_native(..., Box::new(...))` call to map its error:

```rust
pub fn launch_gui(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    tray_menu_ids: (MenuId, MenuId, MenuId),
    settings: AppSettings,
) -> anyhow::Result<()> {
    // ... existing setup unchanged ...

    eframe::run_native(/* existing args */).map_err(anyhow::Error::from)
}
```

The exact body around the `eframe::run_native` call is unchanged except for the trailing `.map_err(...)`. Do not refactor anything else in this task.

- [ ] **Step 3: Verify the egui crate compiles**

Run: `cargo build -p inputforge-gui`
Expected: PASS.

- [ ] **Step 4: Verify `inputforge-app` still compiles (it's the only caller)**

Run: `cargo build -p inputforge-app`
Expected: PASS — `tracing::error!(%e, "GUI exited with error")` formats `anyhow::Error` cleanly via `Display` (verified in spec §Risks).

- [ ] **Step 5: Run existing tests**

Run: `cargo test -p inputforge-gui`
Expected: PASS — no test depends on the return type.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui/Cargo.toml crates/inputforge-gui/src/lib.rs
git commit -m "refactor(gui): return anyhow::Result from launch_gui"
```

---

## Task 13: Feature flags + optional deps in `inputforge-app/Cargo.toml`

Make `inputforge-gui` optional, add optional `inputforge-gui-dx`, declare the two-feature mutually-exclusive matrix.

**Files:**
- Modify: `crates/inputforge-app/Cargo.toml`

- [ ] **Step 1: Add the `[features]` table and adjust dependencies**

Edit `crates/inputforge-app/Cargo.toml`:

```toml
[features]
default    = ["gui-egui"]
gui-egui   = ["dep:inputforge-gui"]
gui-dioxus = ["dep:inputforge-gui-dx"]

[dependencies]
inputforge-core   = { workspace = true }
inputforge-gui    = { workspace = true, optional = true }
inputforge-gui-dx = { workspace = true, optional = true }
anyhow            = { workspace = true }
parking_lot       = { workspace = true }
tracing           = { workspace = true }
tracing-subscriber = { workspace = true }
mimalloc          = { workspace = true }
clap              = { workspace = true }
tray-icon         = { workspace = true }
windows           = { workspace = true }
```

(`inputforge-gui` was unconditionally required before; this makes it `optional = true` and gates it behind the `gui-egui` feature. Keep the rest of the file — `[lints] workspace = true`, `[package]`, etc. — unchanged.)

- [ ] **Step 2: Verify default build still passes**

Run: `cargo build -p inputforge-app`
Expected: PASS — default features include `gui-egui`, so `inputforge-gui` is pulled in.

Note: the build will still fail with a `cannot find crate` error inside `main.rs` because `main.rs` still does an unconditional `use inputforge_gui::launch_gui;` — that's fixed in Task 14. **Run this step's check assuming Task 14 will follow within the same session.**

If you'd rather have a green build at this checkpoint, swap Tasks 13 and 14: do the `main.rs` `cfg`-gating first, then the `Cargo.toml` flip. Both orderings work; the order chosen here is the spec's authoring order.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-app/Cargo.toml
git commit -m "feat(app): add gui-egui (default) and gui-dioxus features"
```

---

## Task 14: `main.rs` — compile guards, cfg-gated import, sentinel const

Add the two `compile_error!`s for invalid feature combos, swap the `use inputforge_gui::launch_gui;` line for a `cfg`-gated pair, and declare `IS_GUI_DIOXUS` for use by the lifecycle guards (Task 15).

**Files:**
- Modify: `crates/inputforge-app/src/main.rs`

- [ ] **Step 1: Add compile guards and cfg-gated imports**

At the top of `crates/inputforge-app/src/main.rs`, immediately after the existing module-level doc comments (line 1 area), insert:

```rust
#[cfg(all(feature = "gui-egui", feature = "gui-dioxus"))]
compile_error!("features `gui-egui` and `gui-dioxus` are mutually exclusive");

#[cfg(not(any(feature = "gui-egui", feature = "gui-dioxus")))]
compile_error!("one of `gui-egui` or `gui-dioxus` must be enabled");
```

Find the line in main.rs that today reads `use inputforge_gui::launch_gui;` (currently absent — main.rs uses `inputforge_gui::launch_gui` at the call site in `launch_gui_blocking` line 216, fully qualified). Search for `inputforge_gui::launch_gui` in main.rs:

- If it's a fully qualified call inside `launch_gui_blocking` (current state per exploration: line 216, `inputforge_gui::launch_gui(gui_state, gui_tx, menu_ids, settings.clone())`), **replace that fully qualified path** with a local `launch_gui(...)` call and add the `cfg`-gated `use` near the top of main.rs:

```rust
#[cfg(feature = "gui-egui")]
use inputforge_gui::launch_gui;
#[cfg(feature = "gui-dioxus")]
use inputforge_gui_dx::launch_gui;
```

And in `launch_gui_blocking` (line 216), change:
```rust
if let Err(e) = inputforge_gui::launch_gui(gui_state, gui_tx, menu_ids, settings.clone()) {
```
to:
```rust
if let Err(e) = launch_gui(gui_state, gui_tx, menu_ids, settings.clone()) {
```

- [ ] **Step 2: Add the `IS_GUI_DIOXUS` sentinel const**

Below the cfg-gated `use` block, add:

```rust
#[cfg(feature = "gui-egui")]
const IS_GUI_DIOXUS: bool = false;
#[cfg(feature = "gui-dioxus")]
const IS_GUI_DIOXUS: bool = true;
```

- [ ] **Step 3: Verify default build passes**

Run: `cargo build -p inputforge-app`
Expected: PASS.

- [ ] **Step 4: Verify the alt build passes**

Run: `cargo build -p inputforge-app --no-default-features --features gui-dioxus`
Expected: PASS.

- [ ] **Step 5: Verify the conflict guard fires**

Run: `cargo check -p inputforge-app --features gui-dioxus`
Expected: FAIL with `error: features 'gui-egui' and 'gui-dioxus' are mutually exclusive` — both features are enabled (default `gui-egui` plus the explicit `gui-dioxus`), tripping the first `compile_error!`.

- [ ] **Step 6: Verify the no-feature guard fires**

Run: `cargo check -p inputforge-app --no-default-features`
Expected: FAIL with `error: one of 'gui-egui' or 'gui-dioxus' must be enabled`.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-app/src/main.rs
git commit -m "feat(app): cfg-gate launch_gui import and add IS_GUI_DIOXUS sentinel"
```

---

## Task 15: Lifecycle guards in `main.rs`

Two `cfg`-expressed guards around the existing `launch_gui_blocking` call sites. Both fall through to the existing `shutdown(cmd_tx, engine_handle)` call at line 137 — `HidHide` unhide / `vJoy` release run on both feature flags.

**Files:**
- Modify: `crates/inputforge-app/src/main.rs`

- [ ] **Step 1: Add Guard 1 — startup site (around line 115)**

Edit `crates/inputforge-app/src/main.rs` lines 113–134. The current block reads:

```rust
let mut quit_requested = false;
if !cli.start_minimized {
    for action in launch_gui_blocking(&tray, &state, &cmd_tx, &settings) {
        match action {
            TrayAction::Quit => quit_requested = true,
            TrayAction::ToggleActivation => {
                let status = state.read().engine_status;
                let cmd = match status {
                    EngineStatus::Running => EngineCommand::Deactivate,
                    EngineStatus::Paused | EngineStatus::Stopped => EngineCommand::Activate,
                };
                let _ = cmd_tx.send(cmd);
            }
            TrayAction::ShowGui => {} // already drained, but satisfy exhaustiveness
        }
    }
}

// Run the tray event loop until the user selects Quit.
if !quit_requested {
    run_tray_loop(&tray, &state, &cmd_tx, &settings);
}
```

Modify the `if !cli.start_minimized` block to add the post-loop guard:

```rust
let mut quit_requested = false;
if !cli.start_minimized {
    for action in launch_gui_blocking(&tray, &state, &cmd_tx, &settings) {
        match action {
            TrayAction::Quit => quit_requested = true,
            TrayAction::ToggleActivation => {
                let status = state.read().engine_status;
                let cmd = match status {
                    EngineStatus::Running => EngineCommand::Deactivate,
                    EngineStatus::Paused | EngineStatus::Stopped => EngineCommand::Activate,
                };
                let _ = cmd_tx.send(cmd);
            }
            TrayAction::ShowGui => {}
        }
    }
    if IS_GUI_DIOXUS {
        // F1: tao EventLoop::run is one-shot. Skip run_tray_loop to avoid a
        // second launch_gui_blocking on tray Show-GUI. F3 restores the full
        // lifecycle with WindowCloseBehaviour::WindowHides.
        tracing::info!("Dioxus window closed; treating as app exit (hide-to-tray lands in F3).");
        quit_requested = true;
    }
}

// Run the tray event loop until the user selects Quit.
if !quit_requested {
    run_tray_loop(&tray, &state, &cmd_tx, &settings);
}
```

- [ ] **Step 2: Add Guard 2 — `ShowGui` arm in `run_tray_loop` (around line 300)**

Edit `crates/inputforge-app/src/main.rs` lines 299–319. The current `ShowGui` arm reads:

```rust
TrayAction::ShowGui => {
    let deferred = launch_gui_blocking(tray, state, cmd_tx, settings);
    tray.refresh_toggle_label();
    for deferred_action in deferred {
        match deferred_action {
            TrayAction::Quit => return,
            TrayAction::ToggleActivation => {
                let status = state.read().engine_status;
                let cmd = match status {
                    EngineStatus::Running => EngineCommand::Deactivate,
                    EngineStatus::Paused | EngineStatus::Stopped => {
                        EngineCommand::Activate
                    }
                };
                let _ = cmd_tx.send(cmd);
                tray.refresh_toggle_label();
            }
            TrayAction::ShowGui => {}
        }
    }
}
```

Add the post-loop guard immediately after the deferred-action loop, before the closing `}`:

```rust
TrayAction::ShowGui => {
    let deferred = launch_gui_blocking(tray, state, cmd_tx, settings);
    tray.refresh_toggle_label();
    for deferred_action in deferred {
        match deferred_action {
            TrayAction::Quit => return,
            TrayAction::ToggleActivation => {
                let status = state.read().engine_status;
                let cmd = match status {
                    EngineStatus::Running => EngineCommand::Deactivate,
                    EngineStatus::Paused | EngineStatus::Stopped => {
                        EngineCommand::Activate
                    }
                };
                let _ = cmd_tx.send(cmd);
                tray.refresh_toggle_label();
            }
            TrayAction::ShowGui => {}
        }
    }
    if IS_GUI_DIOXUS {
        // F1: tao EventLoop::run is one-shot. Return from the tray loop so
        // main()'s shutdown() runs. F3 restores tray re-open via
        // DesktopService::set_visible(true) signaled from the listener.
        return;
    }
}
```

- [ ] **Step 3: Verify default build still passes**

Run: `cargo build -p inputforge-app`
Expected: PASS.

- [ ] **Step 4: Verify dioxus build passes**

Run: `cargo build -p inputforge-app --no-default-features --features gui-dioxus`
Expected: PASS.

- [ ] **Step 5: Run clippy on both feature configurations**

Run: `cargo clippy -p inputforge-app --all-targets -- -D warnings`
Run: `cargo clippy -p inputforge-app --no-default-features --features gui-dioxus --all-targets -- -D warnings`
Expected: Both PASS.

If clippy complains about `if IS_GUI_DIOXUS { ... }` (e.g. `clippy::if_same_then_else` or constant-condition warnings) under one feature, apply the spec's documented mitigation (§Risks): replace the two `if IS_GUI_DIOXUS { ... }` bodies with `#[cfg(feature = "gui-dioxus")] { ... }` blocks at the guard sites, removing the `IS_GUI_DIOXUS` const entirely. Both options are explicitly endorsed by the spec.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-app/src/main.rs
git commit -m "feat(app): add lifecycle guards for one-shot tao EventLoop under gui-dioxus"
```

---

## Verification

End-to-end sanity check across both feature flags. Most of these were spot-checked in earlier task steps; this is the consolidated final pass before declaring F1 complete.

### Build matrix

- [ ] `cargo build` (default) — PASS, egui included
- [ ] `cargo run` (default) — opens egui window identically to today
- [ ] `cargo build --no-default-features --features gui-dioxus` — PASS, Dioxus included
- [ ] `cargo run --no-default-features --features gui-dioxus` — opens 1280×800 Dioxus window titled "InputForge", F1 readout visible
- [ ] `cargo check --features gui-dioxus` (defaults still on) — FAIL with the mutual-exclusion `compile_error!`
- [ ] `cargo check --no-default-features` — FAIL with the no-feature `compile_error!`
- [ ] `cargo test --workspace` — PASS, all unit tests including the 6 new ones in `inputforge-gui-dx`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — PASS (default features)
- [ ] `cargo clippy -p inputforge-app --no-default-features --features gui-dioxus --all-targets -- -D warnings` — PASS (per-package alt features; workspace clippy doesn't accept package-scoped features)
- [ ] `cargo clippy -p inputforge-gui-dx --all-targets -- -D warnings` — PASS

### Manual smoke under default (egui) — confirm zero regression

- [ ] `cargo run` — egui window opens, behavior identical to today
- [ ] Tray "Show GUI" / "Toggle Activation" / "Quit" all work
- [ ] Window close keeps the tray alive (egui lifecycle preserved)
- [ ] `cargo run -- --start-minimized` — tray-only startup, click "Show GUI" opens window, close exits via tray "Quit"
- [ ] `cargo run -- --enable` — engine starts in `Running`, readout in egui shows it
- [ ] `cargo run -- --profile <path>` — profile loads, name appears in egui

### Manual smoke under `--features gui-dioxus`

- [ ] `cargo run --no-default-features --features gui-dioxus` — Dioxus window opens at 1280×800
- [ ] Readout shows: Engine status `Stopped`, mode `Default`, profile `<none>`, devices `0`, virtual devices `count from vJoy probe`, warnings `0` (or 1 if HidHide is unavailable on the test box)
- [ ] `--enable` flag surfaces engine status changing to `Running` live in the readout (within ~16ms)
- [ ] Plugging a joystick increments "Connected devices" live without restart
- [ ] `--profile <path>` surfaces the profile name in the readout
- [ ] Closing the Dioxus window exits the app cleanly (look for tracing log: "Dioxus window closed; treating as app exit (hide-to-tray lands in F3).")
- [ ] After window close, no engine thread is left hanging (process truly exits)
- [ ] HidHide warning surfaces correctly when run without admin (warnings counter goes to 1)

### Hot-reload validation (primary dev loop)

- [ ] `cargo install dioxus-cli --version <DIOXUS_CLI_VERSION>` (from Task 1)
- [ ] `cd crates/inputforge-gui-dx && dx serve --example bridge_demo --platform desktop`
- [ ] Window opens with seeded readout (Engine status `Running`, mode `Demo`, devices `1`, virtual devices `1`, warnings `1`)
- [ ] Edit a string in `crates/inputforge-gui-dx/src/app.rs`'s F1Readout RSX (e.g. change "InputForge — Dioxus (F1 bridge smoke test)" to "F1 hot-reload check") — window updates within ~1s without restart
- [ ] README's pinned versions match what the implementer just installed

### Engine-cleanup verification (the must-not-regress check)

Under `--features gui-dioxus`, both lifecycle guards must reach `shutdown()`:

- [ ] **Startup-path guard** — Run `cargo run --no-default-features --features gui-dioxus`, close the window with the X button, watch tracing logs:
  - "Dioxus window closed; treating as app exit (hide-to-tray lands in F3)." appears
  - "engine thread joined cleanly" appears (or panicked variant if engine errored)
  - Process exits cleanly
- [ ] **Tray ShowGui-path guard** — Run `cargo run --no-default-features --features gui-dioxus -- --start-minimized`, click tray "Show GUI", close the window:
  - The tray loop returns (no second `launch_gui_blocking` attempt)
  - "engine thread joined cleanly" appears
  - Process exits cleanly

### Self-review checklist

- [ ] Spec coverage: every §Architecture / §Files / §Acceptance criteria item maps to at least one task
- [ ] No placeholders: all `<DIOXUS_VERSION>` / `<DIOXUS_CLI_VERSION>` substitutions made; no "TBD" / "fill in later" anywhere
- [ ] Type consistency: `MetaSnapshot` / `ConfigSnapshot` / `LiveSnapshot` field names match across context.rs definitions, `from_state` impls, polling task, and `F1Readout` `use_memo` reads
- [ ] CI matrix: spec mentions `--no-default-features --features gui-dioxus` as a new CI entry; **the repo has no `.github/workflows/` directory today** (verified during exploration), so this is a no-op for F1 — flagged for whichever future task introduces CI

### Wrap-up

- [ ] All commits made via the `superpowers:conventional-commits` skill
- [ ] No `Co-Authored-By` footers per workspace convention
- [ ] Branch is on `main` (or whatever feature branch the executor was assigned)
- [ ] Spec status field updated to "Implemented" once all checkboxes pass
