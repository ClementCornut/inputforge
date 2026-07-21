# InputForge Linux Portable Baseline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver Slice 1 of the Linux port: hardware-free defaults, target-specific backend composition, deterministic unsupported-Linux startup, preserved Windows behavior, generic virtual-device copy, and frozen v0.2.0 profile compatibility.

**Architecture:** `inputforge-core` becomes portable by default. The application owns a crate-private `PlatformBackends` facade with Windows and Linux implementations. Windows creates the existing SDL3/vJoy/Win32 backends inside the engine thread; Linux rejects startup before state, profile, tray, GUI, or hardware initialization.

**Tech Stack:** Rust 1.88+, Cargo workspace, Clap, anyhow, Dioxus, serde/TOML, platform-gated Windows dependencies.

## Global Constraints

- Preserve the ignored machine-local `.codex/config.toml` without editing or staging it; its expected blob is `5e63d8bdc011917ed961516e41eda194599b861a`.
- Treat `docs/superpowers/specs/2026-07-11-linux-portable-baseline-design.md` as the committed execution baseline; its expected blob is `0618b528c433b7c15958134e84d9a86396bf3436`.
- Derive the starting commit from this plan instead of embedding a checkout-specific SHA: `PLAN_BASE=$(git log -1 --format=%H -- docs/superpowers/plans/2026-07-21-linux-portable-baseline.md)`.
- Stage explicit paths only; never use `git add -A` or `git add .`.
- Do not change dependency versions, workspace version `0.2.0`, or `Cargo.lock`.
- Do not add evdev, uinput, grabs, placeholder sinks, backend-health state, Linux UI, packaging, or CI.
- Do not change `InputSource`.
- Keep `DeviceHider`, `HidHideManager`, `NoOpDeviceHider`, and `MockDeviceHider`; only remove hider ownership from `Engine`.
- Keep `VJoyAxis`, `Action::MapToVJoy`, and `map_to_vjoy`. Never introduce `map_to_virtual_device`.
- Treat `TapGesture` and `PressGesture` data, runtime semantics, validation, and UI behavior as synchronized baseline functionality; this Linux slice must not change them.
- This plan reconciliation commit is the new `PLAN_BASE`; keep the already-reviewed test-only baseline repair below it, the original six scoped implementation commits above it, and the two review-fix commits described below without rewriting history.
- Do not print, commit, or otherwise expose original profile or device identifiers.
- Use `cargo fmt --all` before each implementation commit and preserve warning-free `-D warnings` verification.

---

## File Structure and Interfaces

Create:

```text
crates/inputforge-core/tests/fixtures/v0_2_0/default.toml
crates/inputforge-core/tests/fixtures/v0_2_0/star-citizen.toml
crates/inputforge-core/tests/profile_v0_2_0_compat.rs
crates/inputforge-app/src/platform/mod.rs
crates/inputforge-app/src/platform/linux.rs
crates/inputforge-app/src/platform/windows.rs
crates/inputforge-app/src/windows_app.rs
crates/inputforge-app/tests/linux_startup.rs
```

Modify:

```text
Cargo.toml
Dioxus.toml
crates/inputforge-core/Cargo.toml
crates/inputforge-core/src/device/mod.rs
crates/inputforge-core/src/output/mod.rs
crates/inputforge-core/src/types/address.rs
crates/inputforge-core/src/types/input.rs
crates/inputforge-core/src/engine/mod.rs
crates/inputforge-core/src/engine/run.rs
crates/inputforge-core/src/engine/tests.rs
crates/inputforge-core/src/snapshot/fs.rs
crates/inputforge-core/src/snapshot/pending_delete.rs
crates/inputforge-core/src/snapshot/tests.rs
crates/inputforge-app/Cargo.toml
crates/inputforge-app/src/main.rs
crates/inputforge-app/src/cli.rs
crates/inputforge-app/src/platform/mod.rs
crates/inputforge-app/src/platform/linux.rs
crates/inputforge-app/tests/linux_startup.rs
focused GUI copy and test files listed in Task 7
```

Public compatibility contract:

- `Engine::new` loses its `Box<dyn DeviceHider>` argument. This is the only intentional breaking Rust API change.
- `VJoyAxis` gains explicit serde names without changing its variants or wire values.
- Core default features become empty; existing feature names remain available and additive.
- `PlatformBackends`, `preflight`, and `create` remain crate-private to `inputforge-app`.

## Review Fixes After the Original Six Commits

Keep the original implementation commits intact and append these two scoped
fixes:

1. `fix(core): isolate engine config roots`
   - Derive the external-snapshot root from the parent of the injected
     `Engine.settings_path`.
   - Preserve `AppSettings::config_dir()` only as the defensive fallback for
     an empty or parentless settings path.
   - Pass the injected root through snapshot namespace resolution and pending
     delete cleanup.
   - Replace empty-path engine fixtures with a test-only owner that keeps both
     `Engine` and its `TempDir` alive, and assert that external startup
     snapshots are written below that fixture root.
2. `fix(app): isolate Windows desktop runtime`
   - Move engine-thread, tray, and Dioxus lifecycle wiring into a
     Windows-gated `windows_app` module.
   - Keep `main.rs` portable: allocator, logging, Clap parsing, target dispatch,
     and preflight only.
   - Move GUI, tray, autostart, parking-lot, and direct tracing dependencies to
     the Windows target table while retaining the common `PlatformBackends`
     facade and defensive Linux `create()` error.
   - Remove `LD_LIBRARY_PATH`, `LD_PRELOAD`, and `LD_AUDIT` from Linux startup
     subprocesses so tests cannot hide accidental native desktop linkage.

The review fixes must leave `Cargo.lock`, the approved spec, and the original
14 commit hashes unchanged. Linux acceptance additionally requires an ELF
dependency check showing no GUI, tray, WebKit/GTK, or `libxdo` imports. Native
Windows acceptance in Task 8 remains mandatory before the slice is fully
accepted.

### Task 1: Lock the Starting State

**Files:** None.

**Interfaces:** Establishes the checkout and baseline that every later task must preserve.

- [ ] **Step 1: Confirm the checkout and protected files**

Run:

```bash
PLAN_PATH=docs/superpowers/plans/2026-07-21-linux-portable-baseline.md
PLAN_BASE=$(git log -1 --format=%H -- "$PLAN_PATH")
git status --short --branch
git rev-parse HEAD
printf '%s\n' "$PLAN_BASE"
test "$(git rev-parse HEAD)" = "$PLAN_BASE"
test -f .codex/config.toml
git check-ignore --quiet .codex/config.toml
test -z "$(git ls-files -- .codex/config.toml)"
git hash-object .codex/config.toml
git hash-object docs/superpowers/specs/2026-07-11-linux-portable-baseline-design.md
```

Expected: the worktree is clean; `PLAN_BASE` equals `HEAD`; `.codex/config.toml` exists, is ignored, and is untracked; and both blob hashes match the Global Constraints. If either protected blob differs, stop without resetting it.

- [ ] **Step 2: Reproduce the feature-gating red baseline**

Run:

```bash
cargo check -p inputforge-core --locked
```

Expected: failure in `windows-future 0.3.2`, including unresolved `IMarshal`, `marshaler`, and `windows_threading::submit` errors.

- [ ] **Step 3: Confirm the hardware-free baseline**

Run:

```bash
cargo check -p inputforge-core --no-default-features --locked
cargo test -p inputforge-core --no-default-features --locked
```

Expected: both succeed; the test run reports 826 unit tests plus 1 doctest, and the check reports only the existing unused `AxisValue::raw` warning.

### Task 2: Freeze the v0.2.0 Profile Contract

**Files:**

- Create: `crates/inputforge-core/tests/fixtures/v0_2_0/default.toml`
- Create: `crates/inputforge-core/tests/fixtures/v0_2_0/star-citizen.toml`
- Create: `crates/inputforge-core/tests/profile_v0_2_0_compat.rs`

**Interfaces:**

- Consumes: `Profile::load`, `Profile::to_toml`, `Action`, and `VJoyAxis` from `inputforge-core`.
- Produces: complete sanitized fixtures and a hardware-free compatibility integration suite used by every later task.

- [ ] **Step 1: Generate sanitized fixtures from the verified sources**

Run the following as an ephemeral Python script from the InputForge repository root. Do not save the script in the repository.

```python
from hashlib import sha256
from pathlib import Path
import tomllib

SOURCE_ROOT = Path("../inputforge-windows-profile/profiles")
TARGET_ROOT = Path("crates/inputforge-core/tests/fixtures/v0_2_0")

CASES = [
    (
        "Default.toml",
        "default.toml",
        "fbfbc4bf8e9a3c3c9d121090275baac6295a2155221a7ebc07eed3e0cd4bd368",
        "Default",
        2,
        "00000000-0000-0000-0000-000000000001",
    ),
    (
        "Star Citizen.toml",
        "star-citizen.toml",
        "d51ac212d93c6bed514243309ff27ebeabd82cbb0b0a42e41dc3048128e0bce8",
        "Star Citizen",
        1,
        "00000000-0000-0000-0000-000000000002",
    ),
]

SANITIZED_DEVICE_IDS = [
    "00000000000000000000000000000001",
    "00000000000000000000000000000002",
    "00000000000000000000000000000003",
    "00000000000000000000000000000004",
]


def count_map_to_vjoy(value):
    if isinstance(value, dict):
        own = int(value.get("type") == "map_to_vjoy")
        return own + sum(count_map_to_vjoy(child) for child in value.values())
    if isinstance(value, list):
        return sum(count_map_to_vjoy(child) for child in value)
    return 0


def collect_string_devices(value, result):
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "device" and isinstance(child, str):
                result.add(child)
            collect_string_devices(child, result)
    elif isinstance(value, list):
        for child in value:
            collect_string_devices(child, result)


documents = []
source_device_ids = set()

for source_name, target_name, expected_hash, expected_name, mode_count, new_profile_id in CASES:
    source_path = SOURCE_ROOT / source_name
    source_bytes = source_path.read_bytes()
    assert sha256(source_bytes).hexdigest() == expected_hash

    source_text = source_bytes.decode("utf-8")
    document = tomllib.loads(source_text)

    assert document["profile"]["name"] == expected_name
    assert len(document["modes"]) == mode_count
    assert len(document["mappings"]) == 77
    assert count_map_to_vjoy(document) == 77

    document_device_ids = set()
    collect_string_devices(document, document_device_ids)
    source_device_ids.update(document_device_ids)
    documents.append(
        (
            source_text,
            target_name,
            document["profile"]["id"],
            new_profile_id,
            document_device_ids,
        )
    )

sorted_source_ids = sorted(source_device_ids)
assert len(sorted_source_ids) == 4
device_replacements = dict(
    zip(sorted_source_ids, SANITIZED_DEVICE_IDS, strict=True)
)

TARGET_ROOT.mkdir(parents=True, exist_ok=True)

for source_text, target_name, old_profile_id, new_profile_id, document_device_ids in documents:
    local_device_replacements = {
        source_id: device_replacements[source_id]
        for source_id in sorted(document_device_ids)
    }
    replacements = {old_profile_id: new_profile_id, **local_device_replacements}
    sanitized = source_text

    for old_value, new_value in replacements.items():
        assert old_value in sanitized
        sanitized = sanitized.replace(old_value, new_value)

    assert all(old_value not in sanitized for old_value in replacements)

    parsed = tomllib.loads(sanitized)
    observed_ids = set()
    collect_string_devices(parsed, observed_ids)
    assert observed_ids == set(local_device_replacements.values())

    target_path = TARGET_ROOT / target_name
    target_path.write_bytes(sanitized.encode("utf-8"))
    print(f"wrote {target_path}")
```

Expected: both target files are written. A missing source file, hash mismatch, mapping-count mismatch, action-count mismatch, or identifier-count mismatch is a hard stop.

- [ ] **Step 2: Add the complete compatibility integration test**

Create `crates/inputforge-core/tests/profile_v0_2_0_compat.rs`:

```rust
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use inputforge_core::action::Action;
use inputforge_core::profile::Profile;
use inputforge_core::types::VJoyAxis;
use toml::Value;

struct FixtureCase {
    file_name: &'static str,
    profile_name: &'static str,
    mode_count: usize,
    profile_id: &'static str,
}

const FIXTURES: [FixtureCase; 2] = [
    FixtureCase {
        file_name: "default.toml",
        profile_name: "Default",
        mode_count: 2,
        profile_id: "00000000-0000-0000-0000-000000000001",
    },
    FixtureCase {
        file_name: "star-citizen.toml",
        profile_name: "Star Citizen",
        mode_count: 1,
        profile_id: "00000000-0000-0000-0000-000000000002",
    },
];

const DEVICE_IDS: [&str; 4] = [
    "00000000000000000000000000000001",
    "00000000000000000000000000000002",
    "00000000000000000000000000000003",
    "00000000000000000000000000000004",
];

fn fixture_path(file_name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0_2_0")
        .join(file_name)
}

fn count_map_to_vjoy(actions: &[Action]) -> usize {
    actions
        .iter()
        .map(|action| match action {
            Action::MapToVJoy { .. } => 1,
            Action::Conditional {
                if_true, if_false, ..
            } => count_map_to_vjoy(if_true) + count_map_to_vjoy(if_false),
            Action::TapGesture {
                single_tap,
                double_tap,
                ..
            } => count_map_to_vjoy(single_tap) + count_map_to_vjoy(double_tap),
            Action::PressGesture {
                short_press,
                long_press,
                ..
            } => count_map_to_vjoy(short_press) + count_map_to_vjoy(long_press),
            _ => 0,
        })
        .sum()
}

fn serialized_action_lists(document: &Value) -> Vec<Value> {
    document
        .get("mappings")
        .and_then(Value::as_array)
        .expect("fixture must contain a mappings array")
        .iter()
        .map(|mapping| {
            mapping
                .get("actions")
                .cloned()
                .expect("every mapping must contain actions")
        })
        .collect()
}

fn collect_string_devices(value: &Value, result: &mut BTreeSet<String>) {
    match value {
        Value::Table(table) => {
            for (key, child) in table {
                if key == "device" {
                    if let Some(device) = child.as_str() {
                        result.insert(device.to_owned());
                    }
                }
                collect_string_devices(child, result);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_string_devices(child, result);
            }
        }
        _ => {}
    }
}

#[test]
fn v0_2_0_profiles_round_trip_without_data_loss() {
    let allowed_devices = DEVICE_IDS
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let mut combined_devices = BTreeSet::new();

    for fixture in FIXTURES {
        let path = fixture_path(fixture.file_name);
        let fixture_text = fs::read_to_string(&path).expect("fixture must be readable");
        let fixture_value =
            toml::from_str::<Value>(&fixture_text).expect("fixture must be valid TOML");
        let profile = Profile::load(&path).expect("fixture must load through Profile");

        assert_eq!(profile.name(), fixture.profile_name);
        assert_eq!(profile.id().as_str(), fixture.profile_id);
        assert_eq!(profile.modes().len(), fixture.mode_count);
        assert_eq!(profile.mappings().len(), 77);

        let map_to_vjoy_count = profile
            .mappings()
            .iter()
            .map(|mapping| count_map_to_vjoy(&mapping.actions))
            .sum::<usize>();
        assert_eq!(map_to_vjoy_count, 77);

        let emitted = profile.to_toml().expect("profile must serialize");
        assert!(emitted.contains("type = \"map_to_vjoy\""));
        assert!(!emitted.contains("map_to_virtual_device"));

        let emitted_value =
            toml::from_str::<Value>(&emitted).expect("serialized profile must be valid TOML");

        assert_eq!(
            serialized_action_lists(&fixture_value),
            serialized_action_lists(&emitted_value),
            "{} action sequence changed",
            fixture.file_name
        );
        assert_eq!(
            emitted_value, fixture_value,
            "{} did not round-trip losslessly",
            fixture.file_name
        );

        let mut fixture_devices = BTreeSet::new();
        collect_string_devices(&fixture_value, &mut fixture_devices);
        assert!(fixture_devices.is_subset(&allowed_devices));
        combined_devices.extend(fixture_devices);
    }

    assert_eq!(combined_devices, allowed_devices);
}

#[test]
fn vjoy_axis_wire_spellings_remain_exact() {
    let cases = [
        (VJoyAxis::X, "X"),
        (VJoyAxis::Y, "Y"),
        (VJoyAxis::Z, "Z"),
        (VJoyAxis::Rx, "Rx"),
        (VJoyAxis::Ry, "Ry"),
        (VJoyAxis::Rz, "Rz"),
        (VJoyAxis::Slider0, "Slider0"),
        (VJoyAxis::Slider1, "Slider1"),
    ];

    for (axis, spelling) in cases {
        let encoded = serde_json::to_string(&axis).expect("axis must serialize");
        assert_eq!(encoded, format!("\"{spelling}\""));

        let decoded =
            serde_json::from_str::<VJoyAxis>(&encoded).expect("axis must deserialize");
        assert_eq!(decoded, axis);
    }
}
```

- [ ] **Step 3: Run the characterization suite before production changes**

Run:

```bash
cargo fmt --all
cargo test -p inputforge-core --test profile_v0_2_0_compat --no-default-features --locked
git diff --check
```

Expected: two tests pass.

The pinned default fixture already contains a tap gesture. Synchronized `main` now supports that data, so this characterization suite must preserve and traverse its nested action branches without adding or changing any production gesture implementation.

- [ ] **Step 4: Commit the fixtures and test**

```bash
git add crates/inputforge-core/tests/fixtures/v0_2_0/default.toml
git add crates/inputforge-core/tests/fixtures/v0_2_0/star-citizen.toml
git add crates/inputforge-core/tests/profile_v0_2_0_compat.rs
git commit -m "test(profile): freeze v0.2.0 profile compatibility"
```

### Task 3: Pin `VJoyAxis` Wire Names Explicitly

**Files:**

- Modify: `crates/inputforge-core/src/types/address.rs`
- Test: `crates/inputforge-core/tests/profile_v0_2_0_compat.rs`

**Interfaces:**

- Consumes: the exact spellings asserted by Task 2.
- Produces: explicit serde names while preserving the existing public enum and wire format.

- [ ] **Step 1: Confirm the characterization test is green**

```bash
cargo test -p inputforge-core --test profile_v0_2_0_compat --no-default-features --locked
```

- [ ] **Step 2: Add explicit serde names**

Replace the enum declaration with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VJoyAxis {
    #[serde(rename = "X")]
    X,
    #[serde(rename = "Y")]
    Y,
    #[serde(rename = "Z")]
    Z,
    #[serde(rename = "Rx")]
    Rx,
    #[serde(rename = "Ry")]
    Ry,
    #[serde(rename = "Rz")]
    Rz,
    #[serde(rename = "Slider0")]
    Slider0,
    #[serde(rename = "Slider1")]
    Slider1,
}
```

- [ ] **Step 3: Verify unchanged behavior**

```bash
cargo fmt --all
cargo test -p inputforge-core --test profile_v0_2_0_compat --no-default-features --locked
git diff --check
```

Expected: both compatibility tests remain green.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-core/src/types/address.rs
git commit -m "fix(profile): pin vjoy axis wire spellings"
```

### Task 4: Make Core Portable and Add Target-Specific Composition

**Files:**

- Modify: `Cargo.toml`
- Modify: `crates/inputforge-core/Cargo.toml`
- Modify: `crates/inputforge-core/src/device/mod.rs`
- Modify: `crates/inputforge-core/src/output/mod.rs`
- Modify: `crates/inputforge-core/src/types/input.rs`
- Modify: `crates/inputforge-app/Cargo.toml`
- Modify: `crates/inputforge-app/src/main.rs`
- Create: `crates/inputforge-app/src/platform/mod.rs`
- Create: `crates/inputforge-app/src/platform/linux.rs`
- Create: `crates/inputforge-app/src/platform/windows.rs`

**Interfaces:**

- Consumes: existing `InputSource`, `OutputSink`, `KeyboardSink`, `MouseSink`, and concrete Windows backends.
- Produces: crate-private `PlatformBackends`, `preflight() -> anyhow::Result<()>`, and `create() -> anyhow::Result<PlatformBackends>`.

This is one commit so no intermediate commit leaves `inputforge-app` referring to newly hidden Linux modules.

- [ ] **Step 1: Disable core defaults at the workspace boundary**

In root `Cargo.toml`:

```toml
inputforge-core = { path = "crates/inputforge-core", default-features = false }
```

- [ ] **Step 2: Make core features additive and target-aware**

In `crates/inputforge-core/Cargo.toml`:

```toml
[features]
default = []
sdl3-input = ["dep:sdl3"]
vjoy-output = ["dep:vjoy"]
win32-io = ["dep:windows"]
test-util = []

[dependencies]
# Existing portable dependencies remain unchanged.
sdl3 = { workspace = true, optional = true }

[target.'cfg(target_os = "windows")'.dependencies]
vjoy = { workspace = true, optional = true }
windows = { workspace = true, optional = true }
```

Remove `vjoy` and `windows` from the unconditional dependency table.

- [ ] **Step 3: Gate Windows modules and re-exports by target and feature**

In `device/mod.rs`:

```rust
#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub mod hidhide;

#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub use hidhide::HidHideManager;
```

In `output/mod.rs`:

```rust
#[cfg(all(target_os = "windows", feature = "vjoy-output"))]
pub mod vjoy_output;

#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub mod keyboard;

#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub mod mouse;

#[cfg(all(target_os = "windows", feature = "vjoy-output"))]
pub use vjoy_output::VJoyOutput;

#[cfg(all(target_os = "windows", feature = "win32-io"))]
pub use keyboard::KeyboardOutput;
```

Keep SDL3 gated only by `sdl3-input`, retain the hider/mock APIs, and do not edit the already target-aware fallback in `types/mapping.rs`.

- [ ] **Step 4: Gate `AxisValue::raw`**

```rust
#[cfg(any(test, feature = "sdl3-input"))]
#[must_use]
pub(crate) fn raw(value: f64) -> Self {
    Self(value)
}
```

- [ ] **Step 5: Configure the app's Windows-only core features**

Keep the regular `inputforge-core` dependency hardware-free, remove the app's direct `windows` dependency, and add:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
inputforge-core = {
    workspace = true,
    features = ["sdl3-input", "vjoy-output", "win32-io"],
}
```

- [ ] **Step 6: Create the platform facade**

Create `crates/inputforge-app/src/platform/mod.rs`:

```rust
use anyhow::Result;

use inputforge_core::device::InputSource;
use inputforge_core::output::{KeyboardSink, MouseSink, OutputSink};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use self::linux as implementation;
#[cfg(target_os = "windows")]
use self::windows as implementation;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("inputforge-app supports only Windows and Linux in Slice 1");

pub(crate) struct PlatformBackends {
    pub(crate) input: Box<dyn InputSource>,
    pub(crate) controller: Box<dyn OutputSink>,
    pub(crate) keyboard: Box<dyn KeyboardSink>,
    pub(crate) mouse: Box<dyn MouseSink>,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn preflight() -> Result<()> {
    implementation::preflight()
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn create() -> Result<PlatformBackends> {
    implementation::create()
}
```

- [ ] **Step 7: Create the Windows composition**

Create `crates/inputforge-app/src/platform/windows.rs`:

```rust
use anyhow::Result;

use inputforge_core::device::Sdl3Input;
use inputforge_core::output::mouse::MouseOutput;
use inputforge_core::output::{KeyboardOutput, VJoyOutput};

use super::PlatformBackends;

pub(super) fn preflight() -> Result<()> {
    Ok(())
}

pub(super) fn create() -> Result<PlatformBackends> {
    Ok(PlatformBackends {
        input: Box::new(Sdl3Input::new()?),
        controller: Box::new(VJoyOutput::new()?),
        keyboard: Box::new(KeyboardOutput::new()),
        mouse: Box::new(MouseOutput::new()),
    })
}
```

- [ ] **Step 8: Write the failing Linux rejection tests**

Create `crates/inputforge-app/src/platform/linux.rs`:

```rust
const UNAVAILABLE_MESSAGE: &str = "Linux input and output backends are unavailable in Slice 1; evdev and uinput arrive in later slices.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_returns_stable_unavailable_error() {
        let error = preflight().expect_err("Linux preflight must reject startup");
        assert_eq!(error.to_string(), UNAVAILABLE_MESSAGE);
    }

    #[test]
    fn create_returns_the_same_unavailable_error() {
        let error = match create() {
            Ok(_) => panic!("Linux must not construct placeholder backends"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), UNAVAILABLE_MESSAGE);
    }
}
```

- [ ] **Step 9: Verify the Linux rejection tests fail for the intended reason**

```bash
cargo test -p inputforge-app platform::linux::tests --locked
```

Expected: compilation fails because `preflight` and `create` are unresolved. Fix any unrelated failure before continuing; do not weaken the tests.

- [ ] **Step 10: Implement the Linux rejection path**

Add the imports and functions above the existing test module in `crates/inputforge-app/src/platform/linux.rs`:

```rust
use anyhow::{Result, anyhow};

use super::PlatformBackends;

pub(super) fn preflight() -> Result<()> {
    Err(anyhow!(UNAVAILABLE_MESSAGE))
}

pub(super) fn create() -> Result<PlatformBackends> {
    Err(anyhow!(UNAVAILABLE_MESSAGE))
}
```

Run:

```bash
cargo test -p inputforge-app platform::linux::tests --locked
```

Expected: both Linux platform tests pass.

- [ ] **Step 11: Consume `PlatformBackends` in the engine thread**

Add `mod platform;` to `main.rs`, remove direct SDL3/vJoy/Win32 backend imports, and change `run_engine_inner` to:

```rust
fn run_engine_inner(
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Receiver<EngineCommand>,
) -> Result<()> {
    let platform::PlatformBackends {
        input,
        controller,
        keyboard,
        mouse,
    } = platform::create()?;
    let hider: Box<dyn DeviceHider> = Box::new(NoOpDeviceHider);

    let mut engine = Engine::new(
        input,
        controller,
        keyboard,
        mouse,
        hider,
        state,
        commands,
        AppSettings::load(),
        AppSettings::settings_path(),
        inputforge_autostart::new_for_current_platform(),
    );
    engine.run()?;
    Ok(())
}
```

Do not call `preflight` from `main` until Task 6.

- [ ] **Step 12: Verify the portable feature matrix**

```bash
cargo fmt --all
cargo test -p inputforge-app platform::linux::tests --locked
cargo check -p inputforge-core --locked
cargo check -p inputforge-core --no-default-features --locked
cargo check -p inputforge-core --no-default-features --features sdl3-input --locked
cargo check -p inputforge-core --no-default-features --features vjoy-output --locked
cargo check -p inputforge-core --no-default-features --features win32-io --locked
cargo check -p inputforge-core --no-default-features --features test-util --locked
cargo check -p inputforge-core --all-features --all-targets --locked
cargo check --workspace --all-targets --locked
git diff --exit-code HEAD -- Cargo.lock
git diff --check
```

Expected: every check succeeds, Linux feature names do not compile Windows dependencies, two platform tests pass, and `Cargo.lock` is unchanged.

- [ ] **Step 13: Commit**

```bash
git add Cargo.toml
git add crates/inputforge-core/Cargo.toml
git add crates/inputforge-core/src/device/mod.rs
git add crates/inputforge-core/src/output/mod.rs
git add crates/inputforge-core/src/types/input.rs
git add crates/inputforge-app/Cargo.toml
git add crates/inputforge-app/src/main.rs
git add crates/inputforge-app/src/platform/mod.rs
git add crates/inputforge-app/src/platform/linux.rs
git add crates/inputforge-app/src/platform/windows.rs
git commit -m "feat(platform): add target-specific backend composition"
```

### Task 5: Remove Unused Hider Ownership from `Engine`

**Files:**

- Modify: `crates/inputforge-core/src/engine/mod.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`
- Modify: `crates/inputforge-app/src/main.rs`

**Interfaces:**

- Consumes: the `PlatformBackends` values from Task 4.
- Produces: a breaking `Engine::new` signature without `DeviceHider`; the hider trait and implementations remain exported.

- [ ] **Step 1: Establish a green engine-test baseline**

```bash
cargo test -p inputforge-core engine:: --locked
```

- [ ] **Step 2: Remove hider ownership from `Engine`**

Remove the `DeviceHider` import, field, constructor parameter, initialization, dead-code annotation, and obsolete `!Send` documentation. The resulting constructor begins:

```rust
pub fn new(
    input: Box<dyn InputSource>,
    output: Box<dyn OutputSink>,
    keyboard: Box<dyn KeyboardSink>,
    mouse: Box<dyn MouseSink>,
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Receiver<EngineCommand>,
    mut settings: AppSettings,
    settings_path: PathBuf,
    mut autostart: Box<dyn AutostartManager>,
) -> Self {
```

Update the thread-safety documentation to attribute `!Send` only to `InputSource`/SDL3.

- [ ] **Step 3: Update every construction site**

Remove the `MockDeviceHider` import and hider argument from every construction site: all 21 current `Engine::new` calls in `engine/tests.rs`, plus the app call site in `inputforge-app/src/main.rs`. Remove `DeviceHider`, `NoOpDeviceHider`, the local `hider`, and its constructor argument from the app call site.

- [ ] **Step 4: Verify removal and retained API surface**

```bash
cargo fmt --all
cargo test -p inputforge-core engine:: --locked
cargo test -p inputforge-core device:: --locked
cargo check -p inputforge-app --locked
rg -n "hider|DeviceHider|MockDeviceHider|NoOpDeviceHider" crates/inputforge-core/src/engine crates/inputforge-app/src/main.rs
rg -n "DeviceHider|HidHideManager|NoOpDeviceHider|MockDeviceHider" crates/inputforge-core/src/device
git diff --check
```

Expected: the first search has no matches; the second confirms the public trait and implementations still exist.

- [ ] **Step 5: Commit the breaking constructor change**

```bash
git add crates/inputforge-core/src/engine/mod.rs
git add crates/inputforge-core/src/engine/tests.rs
git add crates/inputforge-app/src/main.rs
git commit -m "refactor(engine)!: remove unused device hider ownership" -m "BREAKING CHANGE: Engine::new no longer accepts a DeviceHider argument"
```

### Task 6: Enforce the Linux Startup Contract

**Files:**

- Create: `crates/inputforge-app/tests/linux_startup.rs`
- Modify: `crates/inputforge-app/Cargo.toml`
- Modify: `crates/inputforge-app/src/main.rs`

**Interfaces:**

- Consumes: `platform::preflight()` from Task 4.
- Produces: a deterministic process contract for no arguments, `--help`, and `--version`.

- [ ] **Step 1: Add the already-locked test dependency**

```toml
[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 2: Add the Linux-only process test**

Create `crates/inputforge-app/tests/linux_startup.rs`:

```rust
#![cfg(target_os = "linux")]

use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const UNAVAILABLE_MESSAGE: &str = "Linux input and output backends are unavailable in Slice 1; evdev and uinput arrive in later slices.";

// This bounds regressions that accidentally reach a blocking GUI or tray path.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(5);

fn run_inputforge(arguments: &[&str]) -> Output {
    let isolated_home = tempfile::tempdir().expect("temporary XDG root must be created");

    let mut child = Command::new(env!("CARGO_BIN_EXE_inputforge"))
        .args(arguments)
        .env("XDG_CONFIG_HOME", isolated_home.path().join("config"))
        .env("XDG_DATA_HOME", isolated_home.path().join("data"))
        .env("XDG_CACHE_HOME", isolated_home.path().join("cache"))
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("RUST_LOG")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("inputforge process must start");

    let deadline = Instant::now() + PROCESS_TIMEOUT;

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .expect("completed process output must be readable");
            }
            Ok(None) => {}
            Err(error) => panic!("failed to poll inputforge process: {error}"),
        }

        if Instant::now() >= deadline {
            child.kill().expect("timed-out process must be terminated");
            let output = child
                .wait_with_output()
                .expect("timed-out process output must be readable");
            panic!(
                "inputforge exceeded five seconds\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn no_arguments_fail_before_ui_with_stable_message() {
    let output = run_inputforge(&[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout must be UTF-8"),
        ""
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr must be UTF-8"),
        format!("Error: {UNAVAILABLE_MESSAGE}\n")
    );
}

#[test]
fn help_succeeds_before_preflight() {
    let output = run_inputforge(&["--help"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr must be UTF-8");

    assert!(stdout.contains("Usage: inputforge [OPTIONS]"));
    assert!(!stdout.contains(UNAVAILABLE_MESSAGE));
    assert_eq!(stderr, "");
}

#[test]
fn version_succeeds_before_preflight() {
    let output = run_inputforge(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout must be UTF-8"),
        format!("inputforge {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr must be UTF-8"),
        ""
    );
}
```

- [ ] **Step 3: Run the process test to prove the no-argument path is red**

```bash
cargo test -p inputforge-app --test linux_startup --locked
```

Expected: the no-argument contract fails against the current startup path; the helper terminates the child after five seconds if it reaches a blocking path.

- [ ] **Step 4: Call preflight before application side effects**

Change the beginning of `main` to:

```rust
fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    platform::preflight()?;
    tracing::info!(?cli, "starting InputForge");

    // Existing state/profile/engine/tray/GUI flow follows.
```

Clap exits from `Cli::parse()` for `--help` and `--version`; no later preflight or hardware code runs on those paths.

- [ ] **Step 5: Run the green process suite**

```bash
cargo fmt --all
cargo test -p inputforge-app --test linux_startup --locked
cargo test -p inputforge-app --locked
git diff --exit-code HEAD -- Cargo.lock
git diff --check
```

Expected: all three process paths pass without display, tray, SDL3, profiles, or the user's configuration.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-app/Cargo.toml
git add crates/inputforge-app/src/main.rs
git add crates/inputforge-app/tests/linux_startup.rs
git commit -m "feat(app): fail unsupported linux startup before ui"
```

### Task 7: Replace Generic User-Facing vJoy Copy

**Files:**

- Modify: `Cargo.toml`
- Modify: `Dioxus.toml`
- Modify: `crates/inputforge-app/src/cli.rs`
- Modify: `crates/inputforge-app/tests/linux_startup.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/bulk_map/{apply.rs,empty_state.rs,mod.rs,tests.rs}`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/header.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/value_helpers.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/{add_palette.rs,stage.rs,tests.rs}`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_vjoy.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/{row.rs,tests.rs}`
- Modify: `crates/inputforge-gui-dx/assets/frame/{bulk_map.css,mapping_editor.css,mapping_list.css}`

**Interfaces:**

- Consumes: existing render helpers and the startup process test.
- Produces: full “virtual device” terminology in standalone copy, compact `Device N` output addresses in dense UI, and layout containment without renaming compatibility identifiers, modules, CSS classes, or telemetry keys.

Standalone labels must keep the full noun phrase where the surrounding UI does not already establish virtual-output context. Dense address labels may use `Device N` because their output chip, output stage, target field, or OUT readout already supplies that context. Do not abbreviate the address to `VD`, widen fixed containers, or replace compatibility identifiers.

- [ ] **Step 1: Update the existing assertions first**

Use these exact expectations:

| Test surface | Required expectation |
|---|---|
| Package/CLI help | `virtual devices`; reject `virtual vJoy devices` |
| Bulk-map snapshot | `FlightStick · Device 1` |
| Bulk-map empty title | `No virtual devices configured` |
| Empty-state remediation | `Configure virtual devices in vJoyConf, then reopen.` |
| Bulk-map selector | `Device 1 · 8 axes · 32 btn · 1 hat` |
| Bulk-map selector maximum | `Device 16 · 8 axes · 128 btn · 4 hats` |
| Pipeline title | `Map to virtual device` |
| Pipeline device option | `Virtual device 1` |
| Pipeline invalid-device hint | `Device 1 not configured` |
| Pipeline summary | `Device 1 · X axis` |
| Mapping-list chip | `Device 2 · X` |
| Header/live readout | `Device N · …` |

Add to `pipeline/tests.rs`:

```rust
#[test]
fn map_to_vjoy_summary_uses_compact_device_address() {
    let action = Action::MapToVJoy {
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    };

    assert_eq!(
        stage_summary_for(&action, &synth_cfg()),
        "Device 1 · X axis"
    );
}

#[test]
fn tap_gesture_nested_map_to_vjoy_uses_generic_title() {
    let html = render_stage_body_with_addr(
        Action::TapGesture {
            threshold_ms: 250,
            fire_single_immediately: false,
            single_tap: vec![Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Axis { id: VJoyAxis::X },
                },
            }],
            double_tap: Vec::new(),
        },
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        },
    );

    assert!(html.contains("Map to virtual device"), "{html}");
    assert!(!html.contains("Map to vJoy"), "{html}");
}

#[test]
fn press_gesture_nested_map_to_vjoy_uses_generic_title() {
    let html = render_stage_body_with_addr(
        Action::PressGesture {
            threshold_ms: 500,
            fire_long_when_threshold_crossed: false,
            short_press: vec![Action::MapToVJoy {
                output: OutputAddress {
                    device: 1,
                    output: OutputId::Axis { id: VJoyAxis::X },
                },
            }],
            long_press: Vec::new(),
        },
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        },
    );

    assert!(html.contains("Map to virtual device"), "{html}");
    assert!(!html.contains("Map to vJoy"), "{html}");
}
```

Add to `mapping_editor/header.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_label_uses_compact_device_address() {
        let output = OutputAddress {
            device: 2,
            output: OutputId::Axis { id: VJoyAxis::X },
        };

        assert_eq!(format_output_label(&output), "Device 2 · X axis");
    }
}
```

Extend `help_succeeds_before_preflight` with:

```rust
assert!(stdout.contains("virtual devices"));
assert!(!stdout.contains("virtual vJoy devices"));
```

Extract a private `format_virtual_device_option(&VirtualDeviceConfig) -> String` helper in `bulk_map/mod.rs`. Test device 1 with the normal capability counts, device 16 with 8 axes/128 buttons/4 hats, `1 axis` versus `N axes`, and `1 hat` versus `N hats`. Keep `btn` invariant so the dense label does not branch between `btn` and `btns`.

Update every existing `Map to vJoy` render expectation in `pipeline/tests.rs` to `Map to virtual device`. The two new nested-action tests above are the explicit red/green coverage for output stages under both tap and press gesture branches; do not substitute a claim that existing expectations cover those paths. Retain the exact `Tap gesture` and `Press gesture` terminology and preserve all gesture palette, timing, branch, validation, undo, and rendering behavior.

- [ ] **Step 2: Run the copy tests to prove they are red**

```bash
cargo test -p inputforge-gui-dx --lib --locked
cargo test -p inputforge-app --test linux_startup help_succeeds_before_preflight --locked
```

Expected: assertions that still encounter the old vJoy copy fail.

- [ ] **Step 3: Apply the exact production copy replacements**

| File/surface | Old | New |
|---|---|---|
| `Cargo.toml`, `Dioxus.toml`, `cli.rs` | `virtual vJoy devices` | `virtual devices` |
| `bulk_map/apply.rs` | `{source} · vJoy {id}` | `{source} · Device {id}` |
| `bulk_map/empty_state.rs` | `No vJoy devices configured` | `No virtual devices configured` |
| `bulk_map/empty_state.rs` | `Configure outputs in vJoyConf, then reopen.` | `Configure virtual devices in vJoyConf, then reopen.` |
| `bulk_map/mod.rs` | `vJoy {id}: …` | `Device {id} · {axes} axis/axes · {buttons} btn · {hats} hat/hats` |
| `mapping_editor/header.rs` | `vJoy {id} · …` | `Device {id} · …` |
| `live_readout/value_helpers.rs` | `vJoy {id} · …` | `Device {id} · …` |
| `pipeline/add_palette.rs` | `Map to vJoy` | `Map to virtual device` |
| `pipeline/stage.rs` | `Map to vJoy` | `Map to virtual device` |
| `pipeline/stage.rs` summary | `vJoy {id} · …` | `Device {id} · …` |
| `stage_body/map_to_vjoy.rs` option | `vJoy device {id}` | `Virtual device {id}` |
| Same file, invalid device | `vJoy device {id} not configured` | `Device {id} not configured` |
| Same file, undo before/after | `vJoy device {id}` | `Device {id}` |
| Same file, undo stage name | `Map to vJoy` | `Map to virtual device` |
| `mapping_list/row.rs` | `vJoy {id} · …` | `Device {id} · …` |

- [ ] **Step 4: Add layout containment before running green**

In `bulk_map.css`, retain the existing `width: min(100%, 56rem)`, `max-width: 56rem`, gaps, and two-column `max-width: 1200px` breakpoint. Rebalance only the default tracks so the maximum compact target label fits without widening the metadata strip:

```css
grid-template-columns:
    minmax(16.5rem, 1.2fr)
    minmax(17.5rem, 1.3fr)
    minmax(9rem, 0.7fr)
    minmax(9rem, 0.7fr);
```

In `mapping_editor.css`, make the stage summary track genuinely shrinkable and let long titles wrap inside their own track rather than widening the stage:

```css
.if-stage__header {
    grid-template-columns: minmax(0, 1fr) minmax(0, auto) 32px;
}

.if-stage__title {
    min-width: 0;
    overflow-wrap: anywhere;
}
```

Retain the summary's existing `overflow: hidden`, `text-overflow: ellipsis`, `white-space: nowrap`, and `min-width: 0` declarations.

In `mapping_list/row.rs`, put the compact output text in an `if-row__output-chip-text` span while keeping the complete compact address on the chip's `title`. In `mapping_list.css`, replace the stale “vJoy identifiers are short” assumption with a bounded secondary-output contract:

```css
.if-row__output-chip {
    flex: 0 1 auto;
    min-width: 0;
    max-width: 50%;
}

.if-row__output-chip-text {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
```

Add focused CSS contract assertions to the existing bulk-map, pipeline, and mapping-list test modules. They must pin the 56 rem metadata cap and target track, the two shrinkable stage-header tracks, and the mapping-chip cap/ellipsis rules.

- [ ] **Step 5: Run the green tests and compatibility guard**

```bash
cargo fmt --all
cargo test -p inputforge-gui-dx --lib --locked
cargo test -p inputforge-app --test linux_startup --locked
cargo test -p inputforge-core --test profile_v0_2_0_compat --locked
```

- [ ] **Step 6: Check for missed user-facing literals and accidental width changes**

```bash
rg -n '"[^"]*(Map to vJoy|vJoy device|No vJoy devices configured|virtual vJoy devices|· vJoy|vJoy [0-9])' Cargo.toml Dioxus.toml crates/inputforge-app/src crates/inputforge-app/tests crates/inputforge-gui-dx/src
rg -n "vJoyConf" crates/inputforge-gui-dx/src
rg -n "map_to_vjoy|MapToVJoy|VJoyAxis" crates/inputforge-core/src crates/inputforge-core/tests
rg -n "map_to_virtual_device" crates/inputforge-core/src crates/inputforge-core/tests
git diff --check
```

Expected: the first search has no old user-facing literals; concrete `vJoyConf` remediation and legacy Rust/wire identifiers remain; `map_to_virtual_device` appears only in the negative compatibility assertion; the fixed 320 px mapping rail and 56 rem metadata maximum are unchanged.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml
git add Dioxus.toml
git add crates/inputforge-app/src/cli.rs
git add crates/inputforge-app/tests/linux_startup.rs
git add crates/inputforge-gui-dx/src/frame/bulk_map/apply.rs
git add crates/inputforge-gui-dx/src/frame/bulk_map/empty_state.rs
git add crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs
git add crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs
git add crates/inputforge-gui-dx/src/frame/mapping_editor/header.rs
git add crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/value_helpers.rs
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_vjoy.rs
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git add crates/inputforge-gui-dx/src/frame/mapping_list/row.rs
git add crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs
git add crates/inputforge-gui-dx/assets/frame/bulk_map.css
git add crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git add crates/inputforge-gui-dx/assets/frame/mapping_list.css
git commit -m "fix(gui): use layout-safe virtual device copy"
```

### Task 8: Full Acceptance Verification

**Files:** None unless verification exposes a defect, in which case return to the task that owns it.

**Interfaces:** Validates the complete Slice 1 contract on Linux and Windows.

- [ ] **Step 1: Run the complete Linux matrix**

```bash
cargo fmt --all -- --check
cargo check -p inputforge-core --locked
cargo check -p inputforge-core --no-default-features --locked
cargo check -p inputforge-core --no-default-features --features sdl3-input --locked
cargo check -p inputforge-core --no-default-features --features vjoy-output --locked
cargo check -p inputforge-core --no-default-features --features win32-io --locked
cargo check -p inputforge-core --no-default-features --features test-util --locked
cargo check -p inputforge-core --all-features --all-targets --locked
cargo check --workspace --all-targets --locked
cargo check --workspace --all-targets --all-features --locked
cargo test -p inputforge-core --test profile_v0_2_0_compat --locked
cargo test -p inputforge-app --test linux_startup --locked
cargo test --workspace --locked
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Expected: every command exits zero with no warnings.

- [ ] **Step 2: Confirm the lockfile and execution baseline are unchanged**

```bash
PLAN_PATH=docs/superpowers/plans/2026-07-21-linux-portable-baseline.md
PLAN_BASE=$(git log -1 --format=%H -- "$PLAN_PATH")
git diff --exit-code "$PLAN_BASE" -- Cargo.lock
test -f .codex/config.toml
git check-ignore --quiet .codex/config.toml
test -z "$(git ls-files -- .codex/config.toml)"
git hash-object .codex/config.toml
git hash-object docs/superpowers/specs/2026-07-11-linux-portable-baseline-design.md
test -z "$(git status --porcelain)"
git status --short --branch
git log --oneline "$PLAN_BASE"..HEAD
```

Expected: `Cargo.lock` has no diff from `PLAN_BASE`; the local config remains ignored, untracked, and unchanged; the committed spec blob still matches; the worktree is clean; and the original six scoped implementation commits plus the two review-fix commits appear above `PLAN_BASE`.

- [ ] **Step 3: Run mandatory native Windows acceptance**

On Windows with the repository's existing SDL and vJoy prerequisites:

```text
cargo check -p inputforge-core --locked
cargo check -p inputforge-core --no-default-features --locked
cargo check -p inputforge-core --no-default-features --features sdl3-input --locked
cargo check -p inputforge-core --no-default-features --features vjoy-output --locked
cargo check -p inputforge-core --no-default-features --features win32-io --locked
cargo check -p inputforge-core --no-default-features --features test-util --locked
cargo check -p inputforge-core --all-features --all-targets --locked
cargo check --workspace --all-targets --locked
cargo check --workspace --all-targets --all-features --locked
cargo test --workspace --locked
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
dx build -p inputforge-app --locked
```

Do not claim the slice fully accepted until this native Windows matrix passes. If Windows is unavailable, report implementation and Linux verification as complete with Windows acceptance explicitly outstanding.

- [ ] **Step 4: Verify dense virtual-device copy at both supported window sizes**

On the same Windows acceptance host, configure a long physical-device display name, virtual device 16 with 8 axes/128 buttons/4 hats, a mapping to `Slider1`, and a bulk-map snapshot. Launch the debug GUI with `dx run -p inputforge-app`, attach through the repository's Chrome DevTools connection, and inspect the mapping rail, collapsed and expanded output stage, live OUT readout, bulk-map target selector, and snapshot drawer at both 1280×800 and the enforced minimum 800×500.

At each size:

- Confirm `document.documentElement.scrollWidth == document.documentElement.clientWidth` and the same equality for the application layout root.
- Confirm the mapping rail remains 320 px wide and the bulk-map metadata strip remains at or below 56 rem.
- Confirm the stage title, summary, and right slot do not overlap; the compact summary either fits or ellipsizes inside its own track.
- Confirm the bulk target displays the complete maximum label `Device 16 · 8 axes · 128 btn · 4 hats`.
- Confirm the physical-device label yields before the output chip, the output chip never exceeds half the source row, and its title exposes the complete compact address when the visible text ellipsizes.
- Capture one screenshot per surface and window size for the acceptance record; keep these artifacts out of Git.

Do not accept container widening, root horizontal scrolling, clipped controls, or text painting over adjacent columns. If any check fails, return to Task 7 rather than weakening the compact-copy assertions.

## Acceptance Criteria

- Linux default builds never activate vJoy or Win32 dependencies.
- All retained core features compile independently and together on Linux.
- No-argument Linux startup returns the exact stable error before application side effects.
- `--help` and `--version` remain successful and hardware/display independent.
- Windows still constructs SDL3, vJoy, and Win32 outputs inside the engine thread.
- `Engine` no longer owns a hider, while the hider API remains intact.
- Both complete sanitized profiles retain 77 mappings, 77 `map_to_vjoy` actions, exact values, ordering, identifiers, and axis spellings.
- Standalone generic copy says “virtual device”; dense output addresses use compact `Device N` labels, never widen their fixed containers, and keep `vJoy` only for concrete product/remediation references and legacy compatibility identifiers.
- Synchronized `TapGesture` and `PressGesture` data, runtime semantics, validation, and UI behavior remain unchanged; profile traversal and render expectations cover their nested branches.
- No out-of-scope Linux backend, lifecycle, UI, packaging, or CI work is introduced.
