# F12 Device Info Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build F12 as a compact right-side device information panel with persistent global aliases, remembered disconnected devices, identity diagnostics, profile usage counts, and a copyable device report.

**Architecture:** Keep hardware facts in `inputforge-core`, project display-ready rows through the Dioxus `ConfigSnapshot`, and render F12 inside the existing shared `PanelSlot` aside. Alias edits flow from the GUI to the engine through a new `EngineCommand`, then persist to `settings.toml` through `AppSettings::save_to`.

**Tech Stack:** Rust, SDL3 joystick APIs, serde/TOML settings, Dioxus desktop, Dioxus SSR tests, `arboard` text clipboard support, existing InputForge CSS tokens.

---

## File Structure

- Modify: `crates/inputforge-core/src/types/device.rs`
  - Add normalized diagnostic enums and `DeviceDiagnostics`.
  - Keep `DeviceInfo` focused on stable capability shape.
- Modify: `crates/inputforge-core/src/types/mod.rs`
  - Re-export `DeviceDiagnostics`, `DeviceConnectionState`, and `DeviceBatteryState` from `inputforge_core::types`.
- Modify: `crates/inputforge-core/src/state/device.rs`
  - Extend `DeviceState` with `diagnostics: DeviceDiagnostics`.
- Modify: `crates/inputforge-core/src/state/mod.rs`
  - Mirror app-wide `device_aliases` and `device_registry` into shared state so the Dioxus bridge observes alias saves and registry upserts without restarting.
- Modify: `crates/inputforge-core/src/settings.rs`
  - Add `device_aliases: HashMap<DeviceId, String>` and `device_registry: HashMap<DeviceId, DeviceRecord>`.
  - Add alias helper methods so GUI and engine share one display-name rule.
- Modify: `crates/inputforge-core/src/device/sdl3.rs`
  - Build `DeviceDiagnostics` next to `DeviceInfo` from SDL3.
  - Publish connected hotplug updates with diagnostics.
- Modify: `crates/inputforge-core/src/device/traits.rs`
  - Update `HotplugEvent::Connected` shape if it currently carries only `DeviceInfo`.
- Modify: `crates/inputforge-core/src/engine/command.rs`
  - Add `SetDeviceAlias { device, alias }`.
- Modify: `crates/inputforge-core/src/engine/run.rs`
  - Handle alias persistence.
  - Upsert connected devices into `settings.device_registry` and `AppState::device_registry`.
- Modify: `crates/inputforge-core/src/engine/tests.rs`
  - Cover alias save, alias clear, registry upsert, and remembered disconnected device projection behavior.
- Modify: `crates/inputforge-gui-dx/src/context.rs`
  - Add `DevicePanelRow`, `DeviceUsageSummary`, and helper functions that merge live devices, remembered devices, aliases, and mappings from `AppState`.
- Create: `crates/inputforge-gui-dx/src/frame/panel_slot/device_panel.rs`
  - Render the F12 ledger, fixed inspector, alias form, diagnostics, usage, and report action.
- Modify: `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs`
  - Replace the Devices placeholder body with `DevicePanel`.
- Modify: `Cargo.toml`
  - Add `arboard = { version = "3.6.1", default-features = false }` to workspace dependencies.
- Modify: `crates/inputforge-gui-dx/Cargo.toml`
  - Add `arboard = { workspace = true }`.
- Modify: `crates/inputforge-gui-dx/assets/frame/panel_slot.css`
  - Add Ledger + Fixed Inspector CSS using the existing Evolved Glass Cockpit tokens.

## Task 1: Core Device Diagnostics And Remembered Records

**Files:**
- Modify: `crates/inputforge-core/src/types/device.rs`
- Modify: `crates/inputforge-core/src/types/mod.rs`
- Modify: `crates/inputforge-core/src/state/device.rs`
- Modify: `crates/inputforge-core/src/state/mod.rs`
- Modify: `crates/inputforge-core/src/settings.rs`

- [ ] **Step 1: Add failing tests for default diagnostics and settings round trip**

Add these tests to the existing `#[cfg(test)]` modules.

```rust
#[test]
fn device_diagnostics_defaults_to_unknown_identity() {
    let diagnostics = DeviceDiagnostics::default();

    assert_eq!(diagnostics.connection_state, None);
    assert_eq!(diagnostics.vendor_id, None);
    assert_eq!(diagnostics.product_id, None);
    assert_eq!(diagnostics.serial, None);
    assert_eq!(diagnostics.is_virtual, None);
}

#[test]
fn settings_round_trips_device_aliases_and_registry() {
    let device = DeviceId("030000005e0400008e02000000000000".to_owned());
    let mut settings = AppSettings::default();
    settings
        .device_aliases
        .insert(device.clone(), "Wheel Base".to_owned());
    settings.device_registry.insert(
        device.clone(),
        DeviceRecord {
            info: DeviceInfo {
                id: device.clone(),
                name: "SDL Wheel".to_owned(),
                axes: 6,
                buttons: 32,
                hats: 1,
                instance_path: Some(r"\\?\hid#vid_045e&pid_028e".to_owned()),
                axis_polarities: vec![],
            },
            diagnostics: DeviceDiagnostics {
                vendor_id: Some(0x045e),
                product_id: Some(0x028e),
                connection_state: Some(DeviceConnectionState::Wired),
                ..DeviceDiagnostics::default()
            },
            last_seen_unix_ms: Some(1_714_200_000_000),
        },
    );

    let toml = toml::to_string_pretty(&settings).expect("settings serialize");
    let loaded: AppSettings = toml::from_str(&toml).expect("settings deserialize");

    assert_eq!(loaded.device_aliases.get(&device), Some(&"Wheel Base".to_owned()));
    assert_eq!(
        loaded.device_registry.get(&device).map(|record| record.info.name.as_str()),
        Some("SDL Wheel")
    );
}
```

- [ ] **Step 2: Run the focused tests and confirm failure**

Run:

```powershell
cargo test -p inputforge-core device_diagnostics_defaults_to_unknown_identity settings_round_trips_device_aliases_and_registry
```

Expected: FAIL because `DeviceDiagnostics`, `DeviceRecord`, and the new settings fields do not exist.

- [ ] **Step 3: Add diagnostics and registry types**

In `crates/inputforge-core/src/types/device.rs`, add the public diagnostic types near `DeviceInfo`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceConnectionState {
    Wired,
    Wireless,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceBatteryState {
    Unknown,
    Empty,
    Low,
    Medium,
    Full,
    Charging,
    Charged,
    Wired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DeviceDiagnostics {
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub product_version: Option<u16>,
    pub firmware_version: Option<u16>,
    pub serial: Option<String>,
    pub joystick_type: Option<String>,
    pub connection_state: Option<DeviceConnectionState>,
    pub battery_percent: Option<u8>,
    pub battery_state: Option<DeviceBatteryState>,
    pub is_virtual: Option<bool>,
}
```

In `crates/inputforge-core/src/state/device.rs`, import and store diagnostics:

```rust
use crate::types::{DeviceDiagnostics, DeviceInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceState {
    pub info: DeviceInfo,
    pub connected: bool,
    pub diagnostics: DeviceDiagnostics,
}
```

In `crates/inputforge-core/src/types/mod.rs`, extend the existing device re-export:

```rust
pub use device::{
    AxisPolarity, DeviceBatteryState, DeviceConnectionState, DeviceDiagnostics, DeviceId,
    DeviceInfo, VirtualDeviceConfig,
};
```

In `crates/inputforge-core/src/state/mod.rs`, add settings mirrors:

```rust
use std::collections::HashMap;

use crate::settings::DeviceRecord;
use crate::types::{DeviceId, VirtualDeviceConfig};
```

Extend `AppState`:

```rust
/// App-wide custom device aliases mirrored from `AppSettings`.
pub device_aliases: HashMap<DeviceId, String>,
/// Last-known physical device records mirrored from `AppSettings`.
pub device_registry: HashMap<DeviceId, DeviceRecord>,
```

Initialize both fields with `HashMap::new()` in `AppState::new()` and `AppState::with_profile()`.

In `crates/inputforge-core/src/settings.rs`, add `HashMap` import and settings types:

```rust
use std::collections::HashMap;

use crate::types::{DeviceDiagnostics, DeviceId, DeviceInfo};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub info: DeviceInfo,
    #[serde(default)]
    pub diagnostics: DeviceDiagnostics,
    #[serde(default)]
    pub last_seen_unix_ms: Option<u64>,
}
```

Extend `AppSettings`:

```rust
#[serde(default)]
pub device_aliases: HashMap<DeviceId, String>,
#[serde(default)]
pub device_registry: HashMap<DeviceId, DeviceRecord>,
```

Add helper methods in `impl AppSettings`:

```rust
#[must_use]
pub fn display_name_for(&self, info: &DeviceInfo) -> String {
    self.device_aliases
        .get(&info.id)
        .filter(|alias| !alias.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| {
            if info.name.trim().is_empty() {
                info.id.0.clone()
            } else {
                info.name.clone()
            }
        })
}

pub fn set_device_alias(&mut self, device: DeviceId, alias: Option<String>) {
    match alias.map(|value| value.trim().to_owned()).filter(|value| !value.is_empty()) {
        Some(alias) => {
            self.device_aliases.insert(device, alias);
        }
        None => {
            self.device_aliases.remove(&device);
        }
    }
}
```

- [ ] **Step 4: Update every workspace `DeviceState` constructor**

Every `DeviceState { ... }` literal must include:

```rust
diagnostics: DeviceDiagnostics::default(),
```

Add the import where needed:

```rust
use crate::types::DeviceDiagnostics;
```

Run the sweep:

```powershell
rg "DeviceState \{" crates/inputforge-core crates/inputforge-gui-dx
```

Expected: every result either defines the `DeviceState` struct itself or constructs it with `diagnostics: DeviceDiagnostics::default()` or a cloned real diagnostic value. Update core tests, GUI tests, examples, and synthetic `ConfigSnapshot` fixtures in the same commit so no intermediate commit leaves the workspace uncompilable.

- [ ] **Step 5: Run tests and commit**

Run:

```powershell
cargo test -p inputforge-core settings:: state::device:: types::device::
cargo check --workspace
```

Expected: PASS.

Commit:

```powershell
git add crates/inputforge-core/src/types/device.rs crates/inputforge-core/src/types/mod.rs crates/inputforge-core/src/state/device.rs crates/inputforge-core/src/state/mod.rs crates/inputforge-core/src/settings.rs crates/inputforge-core/src/engine/run.rs crates/inputforge-core/src/engine/tests.rs crates/inputforge-gui-dx/examples/bridge_demo.rs crates/inputforge-gui-dx/src/context.rs crates/inputforge-gui-dx/src/frame/bulk_map/tests.rs crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/tests.rs crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs crates/inputforge-gui-dx/src/frame/mapping_list/tests.rs crates/inputforge-gui-dx/src/frame/status_bar/logic.rs
git commit -m "feat(devices): add diagnostics and remembered records"
```

## Task 2: SDL3 Diagnostic Collection And Registry Upsert

**Files:**
- Modify: `crates/inputforge-core/src/device/traits.rs`
- Modify: `crates/inputforge-core/src/device/sdl3.rs`
- Modify: `crates/inputforge-core/src/engine/mod.rs`
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write failing tests for registry upsert**

In `crates/inputforge-core/src/engine/tests.rs`, add:

```rust
#[test]
fn connected_hotplug_upserts_device_registry() {
    let profile = make_profile(simple_mode_tree(), vec![]);
    let device = DeviceId("dev-1".to_owned());
    let info = DeviceInfo {
        id: device.clone(),
        name: "Wheel".to_owned(),
        axes: 4,
        buttons: 12,
        hats: 1,
        instance_path: Some("hid-path".to_owned()),
        axis_polarities: vec![],
    };
    let diagnostics = DeviceDiagnostics {
        vendor_id: Some(0x045e),
        product_id: Some(0x028e),
        connection_state: Some(DeviceConnectionState::Wired),
        ..DeviceDiagnostics::default()
    };

    let settings_dir = tempfile::tempdir().expect("settings tempdir");
    let settings_path = settings_dir.path().join("settings.toml");
    let mut input = MockInputSource::default();
    input.hotplug.push(HotplugEvent::Connected {
        info: info.clone(),
        diagnostics: diagnostics.clone(),
    });
    let (mut engine, state, _tx) = make_engine(input, profile);
    engine.settings_path = settings_path.clone();

    engine.tick().expect("hotplug tick succeeds");

    let loaded = AppSettings::load_from(&settings_path);
    let record = loaded.device_registry.get(&device).expect("registry record");
    assert_eq!(record.info, info);
    assert_eq!(record.diagnostics, diagnostics);
    assert!(record.last_seen_unix_ms.is_some());
    assert!(state.read().device_registry.contains_key(&device));
    assert!(state.read().devices.iter().any(|row| row.info.id == device && row.connected));
}
```

Use the existing `make_engine`, `make_profile`, and `MockInputSource` helpers in `engine/tests.rs`; do not add a new test-only hotplug API.

- [ ] **Step 2: Run the test and confirm failure**

Run:

```powershell
cargo test -p inputforge-core connected_hotplug_upserts_device_registry
```

Expected: FAIL because connected hotplug still carries only `DeviceInfo` and no registry upsert exists.

- [ ] **Step 3: Extend `HotplugEvent::Connected`**

In `crates/inputforge-core/src/device/traits.rs`, change the connected variant to:

```rust
Connected {
    info: DeviceInfo,
    diagnostics: DeviceDiagnostics,
},
```

Update imports:

```rust
use crate::types::{DeviceDiagnostics, DeviceInfo, InputEvent};
```

- [ ] **Step 4: Collect SDL3 diagnostics**

In `crates/inputforge-core/src/device/sdl3.rs`, add a helper beside `device_info_from_joystick`:

```rust
fn diagnostics_from_joystick(joystick: &Joystick) -> DeviceDiagnostics {
    DeviceDiagnostics {
        vendor_id: joystick.vendor(),
        product_id: joystick.product(),
        product_version: joystick.product_version(),
        firmware_version: joystick.firmware_version(),
        serial: joystick.serial().ok().flatten(),
        joystick_type: Some(format!("{:?}", joystick.get_type())),
        connection_state: Some(DeviceConnectionState::Unknown),
        battery_percent: None,
        battery_state: None,
        is_virtual: None,
    }
}
```

If an SDL3 method returns a different shape in this crate version, normalize it at this boundary and keep the `DeviceDiagnostics` fields unchanged.

In `try_open_joystick`, build and store diagnostics:

```rust
let diagnostics = diagnostics_from_joystick(&joystick);
self.hotplug_buffer.push(HotplugEvent::Connected {
    info: info.clone(),
    diagnostics: diagnostics.clone(),
});
self.open_devices.insert(
    instance_id,
    OpenDevice {
        joystick,
        device_id,
        info,
        diagnostics,
    },
);
```

Extend `OpenDevice`:

```rust
diagnostics: DeviceDiagnostics,
```

Update every remaining `HotplugEvent::Connected` producer to the struct variant and preserve diagnostics on re-emitted SDL updates:

```rust
self.hotplug_buffer.push(HotplugEvent::Connected {
    info: device.info.clone(),
    diagnostics: device.diagnostics.clone(),
});
```

Run the sweep:

```powershell
rg "HotplugEvent::Connected" crates/inputforge-core crates/inputforge-gui-dx
```

Expected: `crates/inputforge-core/src/device/sdl3.rs` initial open, deferred polarity reprobe, and first-event reclassification all emit `{ info, diagnostics }`; `crates/inputforge-core/src/engine/run.rs` matches `HotplugEvent::Connected { info, diagnostics }`; engine tests use the struct variant.

- [ ] **Step 5: Upsert registry when the engine handles connected devices**

In `crates/inputforge-core/src/engine/mod.rs`, seed shared state with settings data in `Engine::new` before returning `Self`:

```rust
{
    let mut state = state.write();
    state.device_aliases = settings.device_aliases.clone();
    state.device_registry = settings.device_registry.clone();
}
```

In `crates/inputforge-core/src/engine/run.rs`, make hotplug handling mutable so settings can be updated:

```rust
fn handle_hotplug(&mut self, events: &[HotplugEvent]) {
```

Update the `run.rs` type import to include the new helper inputs:

```rust
use crate::types::{DeviceDiagnostics, DeviceInfo, InputAddress, InputEvent, InputId, InputValue};
```

Add a helper in `run.rs` that performs settings mutation and file I/O before any shared-state write lock is taken:

```rust
fn upsert_device_record(
    &mut self,
    info: &DeviceInfo,
    diagnostics: &DeviceDiagnostics,
) -> crate::settings::DeviceRecord {
    let record = crate::settings::DeviceRecord {
        info: info.clone(),
        diagnostics: diagnostics.clone(),
        last_seen_unix_ms: Some(current_unix_ms()),
    };
    self.settings
        .device_registry
        .insert(info.id.clone(), record.clone());
    if let Err(error) = self.settings.save_to(&self.settings_path) {
        tracing::warn!(
            target: "engine",
            error = %error,
            device = %info.id.0,
            "failed to persist remembered device registry"
        );
    }
    record
}
```

Then update the connected branch so the state lock only covers shared-state mutation:

```rust
HotplugEvent::Connected { info, diagnostics } => {
    if info.name.to_ascii_lowercase().contains("vjoy") {
        continue;
    }
    let record = self.upsert_device_record(info, diagnostics);
    let mut state = self.state.write();
    state.device_registry.insert(info.id.clone(), record);
    if let Some(dev) = state.devices.iter_mut().find(|dev| dev.info.id == info.id) {
        dev.info = info.clone();
        dev.connected = true;
        dev.diagnostics = diagnostics.clone();
    } else {
        state.devices.push(DeviceState {
            info: info.clone(),
            connected: true,
            diagnostics: diagnostics.clone(),
        });
    }
}
```

Remove the existing method-level `let mut state = self.state.write();`; take write locks inside the connected and disconnected branches so `settings.save_to` never runs while `AppState` is locked.

Add the timestamp helper in `run.rs`:

```rust
fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
}
```

- [ ] **Step 6: Update disconnected handling**

When a device disconnects, preserve the `DeviceState` entry and set `connected = false` instead of dropping the row. If the existing code already marks disconnected state, only add `diagnostics` preservation.

- [ ] **Step 7: Run tests and commit**

Run:

```powershell
cargo test -p inputforge-core device::sdl3:: engine::
cargo check --workspace
```

Expected: PASS.

Commit:

```powershell
git add crates/inputforge-core/src/device/traits.rs crates/inputforge-core/src/device/sdl3.rs crates/inputforge-core/src/engine/mod.rs crates/inputforge-core/src/engine/run.rs crates/inputforge-core/src/engine/tests.rs
git commit -m "feat(devices): capture sdl identity diagnostics"
```

## Task 3: Alias Command And Persistence

**Files:**
- Modify: `crates/inputforge-core/src/engine/command.rs`
- Modify: `crates/inputforge-core/src/engine/run.rs`
- Modify: `crates/inputforge-core/src/engine/tests.rs`

- [ ] **Step 1: Write failing command tests**

Add tests:

```rust
#[test]
fn set_device_alias_persists_trimmed_alias() {
    let device = DeviceId("dev-1".to_owned());
    let (mut engine, settings_path) = test_engine_with_settings_path(AppSettings::default());

    engine
        .handle_command(EngineCommand::SetDeviceAlias {
            device: device.clone(),
            alias: Some("  Wheel Base  ".to_owned()),
        })
        .expect("alias command succeeds");

    let loaded = AppSettings::load_from(&settings_path);
    assert_eq!(loaded.device_aliases.get(&device), Some(&"Wheel Base".to_owned()));
}

#[test]
fn set_device_alias_with_blank_value_clears_alias() {
    let device = DeviceId("dev-1".to_owned());
    let mut settings = AppSettings::default();
    settings.device_aliases.insert(device.clone(), "Wheel Base".to_owned());
    let (mut engine, settings_path) = test_engine_with_settings_path(settings);

    engine
        .handle_command(EngineCommand::SetDeviceAlias {
            device: device.clone(),
            alias: Some(" ".to_owned()),
        })
        .expect("alias clear succeeds");

    let loaded = AppSettings::load_from(&settings_path);
    assert!(!loaded.device_aliases.contains_key(&device));
}
```

Add this helper beside the existing `panel_row` test helper:

```rust
fn panel_row_with_alias(
    id: &str,
    display_name: &str,
    alias: &str,
    connected: bool,
) -> DevicePanelRow {
    let mut row = panel_row(id, display_name, connected);
    row.alias = alias.to_owned();
    row
}
```

- [ ] **Step 2: Run and confirm failure**

Run:

```powershell
cargo test -p inputforge-core set_device_alias_
```

Expected: FAIL because `EngineCommand::SetDeviceAlias` does not exist.

- [ ] **Step 3: Add the command variant**

In `EngineCommand`:

```rust
/// Set or clear an app-wide display alias for a physical device.
SetDeviceAlias {
    device: DeviceId,
    alias: Option<String>,
},
```

Extend command debug tests:

```rust
let c = EngineCommand::SetDeviceAlias {
    device: DeviceId("dev-1".to_owned()),
    alias: Some("Wheel".to_owned()),
};
assert!(format!("{c:?}").contains("SetDeviceAlias"));
```

- [ ] **Step 4: Handle the command**

In `handle_command`:

```rust
EngineCommand::SetDeviceAlias { device, alias } => {
    self.settings.set_device_alias(device.clone(), alias);
    self.settings.save_to(&self.settings_path)?;
    self.state.write().device_aliases = self.settings.device_aliases.clone();
    tracing::info!(
        target: "engine",
        device = %device.0,
        "device alias persisted"
    );
}
```

- [ ] **Step 5: Run tests and commit**

Run:

```powershell
cargo test -p inputforge-core set_device_alias_ engine_command_derives_debug_partialeq
```

Expected: PASS.

Commit:

```powershell
git add crates/inputforge-core/src/engine/command.rs crates/inputforge-core/src/engine/run.rs crates/inputforge-core/src/engine/tests.rs
git commit -m "feat(devices): persist global device aliases"
```

## Task 4: Device Panel Snapshot Projection

**Files:**
- Modify: `crates/inputforge-gui-dx/src/context.rs`

- [ ] **Step 1: Add failing projection tests**

Add tests in `context.rs`:

```rust
#[test]
fn config_snapshot_merges_live_and_remembered_device_rows() {
    let live = DeviceState {
        info: test_device("dev-live", "Live Wheel", 4, 12, 1),
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    };
    let remembered = test_device("dev-old", "Old Pedals", 3, 0, 0);
    let mut state = AppState::new();
    state.devices.push(live);
    state
        .device_aliases
        .insert(DeviceId("dev-live".to_owned()), "Rig Wheel".to_owned());
    state.device_registry.insert(
        DeviceId("dev-old".to_owned()),
        DeviceRecord {
            info: remembered,
            diagnostics: DeviceDiagnostics::default(),
            last_seen_unix_ms: Some(1),
        },
    );

    let snapshot = ConfigSnapshot::from_state(&state, None);

    assert_eq!(snapshot.device_panel_rows.len(), 2);
    assert_eq!(snapshot.device_panel_rows[0].display_name, "Rig Wheel");
    assert!(snapshot.device_panel_rows[0].connected);
    assert_eq!(snapshot.device_panel_rows[1].display_name, "Old Pedals");
    assert!(!snapshot.device_panel_rows[1].connected);
}

#[test]
fn config_snapshot_counts_primary_merge_and_conditional_usage() {
    let mut state = AppState::with_profile(profile_with_primary_merge_and_conditional_device_refs());
    state.devices.push(DeviceState {
        info: test_device("dev-1", "Stick", 6, 32, 1),
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });

    let snapshot = ConfigSnapshot::from_state(&state, None);
    let row = snapshot
        .device_panel_rows
        .iter()
        .find(|row| row.device_id == DeviceId("dev-1".to_owned()))
        .expect("device row");

    assert_eq!(row.usage.primary_mappings, 1);
    assert_eq!(row.usage.secondary_mappings, 2);
    assert_eq!(row.usage.axes.mapped, 1);
    assert_eq!(row.usage.buttons.mapped, 1);
    assert_eq!(row.usage.hats.mapped, 0);
}
```

Use existing action/profile test helpers in `context.rs`; if no helper exists for the mixed action graph, define a private test helper beside the existing mapping summary tests.

- [ ] **Step 2: Run and confirm failure**

Run:

```powershell
cargo test -p inputforge-gui-dx config_snapshot_merges_live_and_remembered_device_rows config_snapshot_counts_primary_merge_and_conditional_usage
```

Expected: FAIL because the snapshot has no device panel projection.

- [ ] **Step 3: Add snapshot row types**

Add to `context.rs` near `MappingSummary`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DeviceCoverage {
    pub mapped: u8,
    pub total: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DeviceUsageSummary {
    pub axes: DeviceCoverage,
    pub buttons: DeviceCoverage,
    pub hats: DeviceCoverage,
    pub primary_mappings: usize,
    pub secondary_mappings: usize,
    pub touched_modes: Vec<String>,
    pub touched_mapping_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DevicePanelRow {
    pub device_id: DeviceId,
    pub display_name: String,
    pub alias: String,
    pub hardware_name: String,
    pub connected: bool,
    pub info: DeviceInfo,
    pub diagnostics: DeviceDiagnostics,
    pub usage: DeviceUsageSummary,
    pub last_seen_unix_ms: Option<u64>,
}
```

Add field to `ConfigSnapshot`:

```rust
pub device_panel_rows: Vec<DevicePanelRow>,
```

- [ ] **Step 4: Extend `ConfigSnapshot::from_state`**

Keep the existing signature and append panel-row projection from `AppState`:

```rust
Self {
    devices: s.devices.clone(),
    virtual_devices: s.virtual_devices.clone(),
    mapped_inputs,
    mapping_names,
    mappings,
    selected_mapping_actions,
    selected_mapping_key: selection.cloned(),
    device_panel_rows: build_device_panel_rows(s),
}
```

- [ ] **Step 5: Build device rows**

Add pure helpers:

```rust
fn build_device_panel_rows(s: &AppState) -> Vec<DevicePanelRow> {
    let mut rows = Vec::new();
    let mut live_ids = HashSet::new();

    for device in &s.devices {
        live_ids.insert(device.info.id.clone());
        rows.push(DevicePanelRow {
            device_id: device.info.id.clone(),
            display_name: display_name_for(&s.device_aliases, &device.info),
            alias: s.device_aliases.get(&device.info.id).cloned().unwrap_or_default(),
            hardware_name: device.info.name.clone(),
            connected: device.connected,
            info: device.info.clone(),
            diagnostics: device.diagnostics.clone(),
            usage: usage_for_device(&device.info.id, &device.info, s),
            last_seen_unix_ms: s
                .device_registry
                .get(&device.info.id)
                .and_then(|record| record.last_seen_unix_ms),
        });
    }

    for (device_id, record) in &s.device_registry {
        if live_ids.contains(device_id) {
            continue;
        }
        rows.push(DevicePanelRow {
            device_id: device_id.clone(),
            display_name: display_name_for(&s.device_aliases, &record.info),
            alias: s.device_aliases.get(device_id).cloned().unwrap_or_default(),
            hardware_name: record.info.name.clone(),
            connected: false,
            info: record.info.clone(),
            diagnostics: record.diagnostics.clone(),
            usage: usage_for_device(device_id, &record.info, s),
            last_seen_unix_ms: record.last_seen_unix_ms,
        });
    }

    rows.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
    });
    rows
}
```

Add the display-name helper:

```rust
fn display_name_for(aliases: &HashMap<DeviceId, String>, info: &DeviceInfo) -> String {
    aliases
        .get(&info.id)
        .filter(|alias| !alias.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| {
            if info.name.trim().is_empty() {
                info.id.0.clone()
            } else {
                info.name.clone()
            }
        })
}
```

Implement `usage_for_device` by walking `profile.mappings()`:

```rust
fn usage_for_device(device_id: &DeviceId, info: &DeviceInfo, s: &AppState) -> DeviceUsageSummary {
    let mut axes = HashSet::new();
    let mut buttons = HashSet::new();
    let mut hats = HashSet::new();
    let mut primary_mappings = 0;
    let mut secondary_mappings = 0;
    let mut touched_modes = Vec::new();
    let mut touched_mapping_names = Vec::new();

    if let Some(profile) = &s.active_profile {
        for mapping in profile.mappings() {
            let primary = mapping.input.device().is_some_and(|id| id == device_id);
            let referenced_devices = derive_referenced_devices(&mapping.input, &mapping.actions);
            let referenced = referenced_devices
                .iter()
                .any(|referenced| referenced == device_id);

            if primary {
                primary_mappings += 1;
                record_input_kind(&mapping.input, &mut axes, &mut buttons, &mut hats);
            } else if referenced {
                secondary_mappings += 1;
            }

            if primary || referenced {
                push_unique(&mut touched_modes, mapping.mode.clone());
                if let Some(name) = &mapping.name {
                    push_unique(&mut touched_mapping_names, name.clone());
                }
            }
        }
    }

    DeviceUsageSummary {
        axes: DeviceCoverage { mapped: axes.len().try_into().unwrap_or(u8::MAX), total: info.axes },
        buttons: DeviceCoverage { mapped: buttons.len().try_into().unwrap_or(u8::MAX), total: info.buttons },
        hats: DeviceCoverage { mapped: hats.len().try_into().unwrap_or(u8::MAX), total: info.hats },
        primary_mappings,
        secondary_mappings,
        touched_modes,
        touched_mapping_names,
    }
}
```

- [ ] **Step 6: Run tests and commit**

Run:

```powershell
cargo test -p inputforge-gui-dx config_snapshot_
```

Expected: PASS.

Commit:

```powershell
git add crates/inputforge-gui-dx/src/context.rs
git commit -m "feat(devices): project panel rows into dioxus snapshots"
```

## Task 5: Panel Selection, Report Text, And Alias State Helpers

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/panel_slot/device_panel.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs`

- [ ] **Step 1: Write pure helper tests**

In the new file, include tests for behavior before rendering:

```rust
#[test]
fn select_initial_device_prefers_first_connected() {
    let rows = vec![
        panel_row("old", "Old Pedals", false),
        panel_row("live", "Wheel", true),
    ];

    assert_eq!(select_initial_device(&rows), Some(DeviceId("live".to_owned())));
}

#[test]
fn select_initial_device_uses_first_remembered_when_none_connected() {
    let rows = vec![panel_row("old", "Old Pedals", false)];

    assert_eq!(select_initial_device(&rows), Some(DeviceId("old".to_owned())));
}

#[test]
fn device_report_is_plain_text_and_device_only() {
    let row = panel_row("dev-1", "Wheel Base", true);
    let report = build_device_report(&row);

    assert!(report.contains("Display name: Wheel Base"));
    assert!(report.contains("Hardware name: SDL Wheel"));
    assert!(report.contains("Connection: connected"));
    assert!(report.contains("Axes: 0/4 mapped"));
    assert!(!report.contains("Profile path"));
    assert!(!report.contains("Active mode"));
}

#[test]
fn alias_draft_comes_from_current_selection() {
    let first = panel_row_with_alias("wheel", "Wheel Base", "Rig Wheel", true);
    let second = panel_row_with_alias("pedals", "Pedals", "", true);

    assert_eq!(alias_draft_for_selected_row(&first), "Rig Wheel");
    assert_eq!(alias_draft_for_selected_row(&second), "");
}
```

- [ ] **Step 2: Run and confirm failure**

Run:

```powershell
cargo test -p inputforge-gui-dx panel_slot::device_panel::
```

Expected: FAIL until the module and helpers exist.

- [ ] **Step 3: Add module and helpers**

In `panel_slot/mod.rs`:

```rust
mod device_panel;
```

In `device_panel.rs`:

```rust
use dioxus::prelude::*;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::DeviceId;

use crate::context::{AppContext, DevicePanelRow};

pub(super) fn select_initial_device(rows: &[DevicePanelRow]) -> Option<DeviceId> {
    rows.iter()
        .find(|row| row.connected)
        .or_else(|| rows.first())
        .map(|row| row.device_id.clone())
}

pub(super) fn alias_draft_for_selected_row(row: &DevicePanelRow) -> String {
    row.alias.clone()
}

pub(super) fn build_device_report(row: &DevicePanelRow) -> String {
    let diagnostics = &row.diagnostics;
    let serial = diagnostics.serial.as_deref().unwrap_or("unavailable");
    let instance_path = row.info.instance_path.as_deref().unwrap_or("unavailable");
    format!(
        "Display name: {}\nHardware name: {}\nConnection: {}\nAxes: {}/{} mapped\nButtons: {}/{} mapped\nHats: {}/{} mapped\nSDL GUID: {}\nVID: {}\nPID: {}\nProduct version: {}\nFirmware version: {}\nSerial: {}\nInstance path: {}",
        row.display_name,
        row.hardware_name,
        if row.connected { "connected" } else { "disconnected" },
        row.usage.axes.mapped,
        row.usage.axes.total,
        row.usage.buttons.mapped,
        row.usage.buttons.total,
        row.usage.hats.mapped,
        row.usage.hats.total,
        row.device_id.0,
        format_optional_hex(diagnostics.vendor_id),
        format_optional_hex(diagnostics.product_id),
        format_optional_u16(diagnostics.product_version),
        format_optional_u16(diagnostics.firmware_version),
        serial,
        instance_path
    )
}
```

Add formatting helpers:

```rust
fn format_optional_hex(value: Option<u16>) -> String {
    value.map_or_else(|| "unavailable".to_owned(), |value| format!("0x{value:04x}"))
}

fn format_optional_u16(value: Option<u16>) -> String {
    value.map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
}
```

- [ ] **Step 4: Run tests and commit**

Run:

```powershell
cargo test -p inputforge-gui-dx panel_slot::device_panel::
```

Expected: PASS.

Commit:

```powershell
git add crates/inputforge-gui-dx/src/frame/panel_slot/device_panel.rs crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs
git commit -m "feat(devices): add panel selection and report helpers"
```

## Task 6: Render F12 Ledger And Fixed Inspector

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/inputforge-gui-dx/Cargo.toml`
- Modify: `crates/inputforge-gui-dx/src/frame/panel_slot/device_panel.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs`

- [ ] **Step 1: Add SSR tests for the required states**

Add SSR tests in `device_panel.rs`:

```rust
#[test]
fn device_panel_renders_ledger_and_fixed_inspector() {
    let html = render_device_panel(vec![panel_row("dev-1", "Wheel Base", true)]);

    assert!(html.contains("if-device-panel__ledger"));
    assert!(html.contains("Wheel Base"));
    assert!(html.contains("SDL Wheel"));
    assert!(html.contains("Axes 0/4"));
    assert!(html.contains("Display name"));
    assert!(html.contains("Save name"));
    assert!(html.contains("Copy device report"));
}

#[test]
fn device_panel_renders_no_signal_empty_state() {
    let html = render_device_panel(vec![]);

    assert!(html.contains("No devices known"));
    assert!(!html.contains("if-device-panel__inspector"));
}

#[test]
fn disconnected_row_keeps_profile_counts_visible() {
    let html = render_device_panel(vec![panel_row("dev-old", "Remembered Pedals", false)]);

    assert!(html.contains("Disconnected"));
    assert!(html.contains("Axes 0/4"));
}

#[test]
fn copy_device_report_helper_forwards_report_text() {
    let row = panel_row("dev-1", "Wheel Base", true);
    let report = build_device_report(&row);
    let mut copied = None::<String>;

    copy_device_report_to_clipboard_with(&report, |text| {
        copied = Some(text);
        Ok(())
    })
    .expect("copy helper succeeds");

    let copied = copied.expect("copied text");
    assert!(copied.contains("Display name: Wheel Base"));
    assert!(copied.contains("Hardware name: SDL Wheel"));
}
```

- [ ] **Step 2: Run and confirm failure**

Run:

```powershell
cargo test -p inputforge-gui-dx device_panel_renders_
cargo test -p inputforge-gui-dx copy_device_report_helper_forwards_report_text
```

Expected: FAIL because the component is not implemented.

- [ ] **Step 3: Add clipboard dependency**

In workspace `Cargo.toml`, add the verified stable text-clipboard dependency:

```toml
arboard = { version = "3.6.1", default-features = false }
```

In `crates/inputforge-gui-dx/Cargo.toml`, add:

```toml
arboard = { workspace = true }
```

- [ ] **Step 4: Implement `DevicePanel`**

Add:

```rust
#[component]
pub(super) fn DevicePanel() -> Element {
    let ctx = use_context::<AppContext>();
    let rows = use_memo(move || ctx.config.read().device_panel_rows.clone());
    let initial_selected = select_initial_device(&rows.read());
    let initial_alias = {
        let current_rows = rows.read();
        initial_selected
            .as_ref()
            .and_then(|id| current_rows.iter().find(|row| &row.device_id == id))
            .map(alias_draft_for_selected_row)
            .unwrap_or_default()
    };
    let mut selected = use_signal(|| initial_selected.clone());
    let mut draft_alias = use_signal(|| initial_alias);
    let mut save_error = use_signal(|| None::<String>);

    let current_rows = rows.read();
    if current_rows.is_empty() {
        return rsx! {
            div { class: "if-device-panel if-device-panel--empty",
                div { class: "if-device-panel__empty-title", "No devices known" }
                div { class: "if-device-panel__empty-copy", "Connect a controller, wheel, pedals, or other SDL device to populate this panel." }
            }
        };
    }

    let selected_id = selected.read().clone().or_else(|| select_initial_device(&current_rows));
    let selected_row = selected_id
        .as_ref()
        .and_then(|id| current_rows.iter().find(|row| &row.device_id == id))
        .cloned()
        .unwrap_or_else(|| current_rows[0].clone());

    rsx! {
        div { class: "if-device-panel",
            div { class: "if-device-panel__ledger", role: "list",
                for row in current_rows.iter().cloned() {
                    DeviceLedgerRow {
                        row: row.clone(),
                        selected: row.device_id == selected_row.device_id,
                        onselect: move |row| {
                            draft_alias.set(alias_draft_for_selected_row(&row));
                            save_error.set(None);
                            selected.set(Some(row.device_id.clone()));
                        },
                    }
                }
            }
            DeviceInspector {
                row: selected_row,
                draft_alias,
                save_error,
            }
        }
    }
}
```

Implement `DeviceLedgerRow`; the selection event passes the full row so the parent can reset the alias draft from the selected row instead of mutating state during render:

```rust
#[component]
fn DeviceLedgerRow(row: DevicePanelRow, selected: bool, onselect: EventHandler<DevicePanelRow>) -> Element {
    let state_label = if row.connected { "Connected" } else { "Disconnected" };
    rsx! {
        button {
            r#type: "button",
            class: if selected { "if-device-row if-device-row--selected" } else { "if-device-row" },
            "aria-pressed": "{selected}",
            onclick: move |_| onselect.call(row.clone()),
            span { class: "if-device-row__state", "data-connected": "{row.connected}", "{state_label}" }
            span { class: "if-device-row__names",
                span { class: "if-device-row__display", "{row.display_name}" }
                span { class: "if-device-row__hardware", "{row.hardware_name}" }
            }
            span { class: "if-device-row__counts",
                span { "Axes {row.usage.axes.mapped}/{row.usage.axes.total}" }
                span { "Buttons {row.usage.buttons.mapped}/{row.usage.buttons.total}" }
                span { "Hats {row.usage.hats.mapped}/{row.usage.hats.total}" }
            }
        }
    }
}
```

- [ ] **Step 5: Implement inspector with alias save and real copy dispatch**

Add:

```rust
#[component]
fn DeviceInspector(
    row: DevicePanelRow,
    draft_alias: Signal<String>,
    save_error: Signal<Option<String>>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let persisted_alias = row.alias.clone();
    let dirty = draft_alias.read().trim() != persisted_alias;
    let report = build_device_report(&row);

    rsx! {
        section { class: "if-device-panel__inspector", "aria-label": "Selected device details",
            label { class: "if-device-inspector__field",
                span { "Display name" }
                input {
                    class: "if-device-inspector__input",
                    value: "{draft_alias}",
                    oninput: move |event| draft_alias.set(event.value()),
                }
            }
            button {
                r#type: "button",
                class: "if-device-inspector__save",
                disabled: !dirty,
                onclick: move |_| {
                    let alias = draft_alias.read().trim().to_owned();
                    let command = EngineCommand::SetDeviceAlias {
                        device: row.device_id.clone(),
                        alias: if alias.is_empty() { None } else { Some(alias) },
                    };
                    if let Err(error) = ctx.commands.send(command) {
                        save_error.set(Some(error.to_string()));
                    } else {
                        save_error.set(None);
                    }
                },
                "Save name"
            }
            if let Some(error) = save_error.read().as_ref() {
                div { class: "if-device-inspector__error", "{error}" }
            }
            div { class: "if-device-inspector__hardware", "{row.hardware_name}" }
            DiagnosticsBlock { row: row.clone() }
            UsageBlock { row: row.clone() }
            button {
                r#type: "button",
                class: "if-device-inspector__copy",
                onclick: move |_| {
                    if let Err(error) = copy_device_report_to_clipboard(&report) {
                        save_error.set(Some(format!("Copy failed: {error}")));
                    } else {
                        save_error.set(None);
                    }
                },
                "Copy device report"
            }
        }
    }
}
```

Add the clipboard helpers below `build_device_report`:

```rust
fn copy_device_report_to_clipboard(report: &str) -> anyhow::Result<()> {
    copy_device_report_to_clipboard_with(report, |text| {
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(text)?;
        Ok(())
    })
}

fn copy_device_report_to_clipboard_with(
    report: &str,
    write_text: impl FnOnce(String) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    write_text(report.to_owned())
}
```

- [ ] **Step 6: Render Diagnostics and Usage blocks**

Add:

```rust
#[component]
fn DiagnosticsBlock(row: DevicePanelRow) -> Element {
    rsx! {
        dl { class: "if-device-diagnostics",
            dt { "Connection" }
            dd { if row.connected { "connected" } else { "disconnected" } }
            dt { "Type" }
            dd { "{row.diagnostics.joystick_type.as_deref().unwrap_or(\"unknown\")}" }
            dt { "VID/PID" }
            dd { "{format_optional_hex(row.diagnostics.vendor_id)} / {format_optional_hex(row.diagnostics.product_id)}" }
            dt { "Serial" }
            dd { "{row.diagnostics.serial.as_deref().unwrap_or(\"unavailable\")}" }
            dt { "Path" }
            dd { "{row.info.instance_path.as_deref().unwrap_or(\"unavailable\")}" }
        }
    }
}

#[component]
fn UsageBlock(row: DevicePanelRow) -> Element {
    rsx! {
        div { class: "if-device-usage",
            div { "Primary mappings {row.usage.primary_mappings}" }
            div { "Merge and conditional references {row.usage.secondary_mappings}" }
        }
    }
}
```

- [ ] **Step 7: Wire Devices slot**

In `PanelSlot`, replace the Devices placeholder body with:

```rust
PanelSlotEnum::Devices if !calib => rsx! { device_panel::DevicePanel {} },
```

Keep the calibration placeholder for `via_calibration == true` untouched.

- [ ] **Step 8: Run tests and commit**

Run:

```powershell
cargo test -p inputforge-gui-dx panel_slot::
```

Expected: PASS.

Commit:

```powershell
git add Cargo.toml crates/inputforge-gui-dx/Cargo.toml Cargo.lock crates/inputforge-gui-dx/src/frame/panel_slot/device_panel.rs crates/inputforge-gui-dx/src/frame/panel_slot/mod.rs
git commit -m "feat(devices): render the f12 device panel"
```

## Task 7: Panel Styling

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/panel_slot.css`

- [ ] **Step 1: Add CSS for Ledger + Fixed Inspector**

Append these rules:

```css
.if-device-panel {
    min-height: 0;
    flex: 1;
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    gap: var(--space-3);
}

.if-device-panel__ledger {
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    scrollbar-gutter: stable;
}

.if-device-row {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-2);
    padding: var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-2);
    background: color-mix(in srgb, var(--color-bg-elevated) 90%, var(--color-text) 3%);
    color: var(--color-text);
    text-align: left;
}

.if-device-row--selected {
    border-color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 12%, var(--color-bg-elevated));
}

.if-device-row__state {
    width: fit-content;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--color-text-subtle);
    text-transform: uppercase;
}

.if-device-row__state[data-connected="true"] {
    color: var(--color-success);
}

.if-device-row__names {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: var(--space-1);
}

.if-device-row__display,
.if-device-row__hardware,
.if-device-diagnostics dd {
    overflow-wrap: anywhere;
}

.if-device-row__display {
    font-weight: var(--weight-semibold);
    color: var(--color-text);
}

.if-device-row__hardware {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
}

.if-device-row__counts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    color: var(--color-text-subtle);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
}

.if-device-panel__inspector {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: 1px solid var(--color-border-strong);
}

.if-device-inspector__field {
    display: grid;
    gap: var(--space-1);
    color: var(--color-text-muted);
    font-size: var(--text-sm);
}

.if-device-inspector__input {
    min-width: 0;
    width: 100%;
    padding: var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-2);
    background: var(--color-bg);
    color: var(--color-text);
}

.if-device-inspector__save,
.if-device-inspector__copy {
    align-self: flex-start;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-2);
    background: var(--color-bg-elevated);
    color: var(--color-text);
}

.if-device-inspector__error {
    color: var(--color-danger);
    font-size: var(--text-sm);
}

.if-device-diagnostics {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: var(--space-1) var(--space-2);
    margin: 0;
    font-size: var(--text-sm);
}

.if-device-diagnostics dt {
    color: var(--color-text-subtle);
}

.if-device-diagnostics dd {
    margin: 0;
    color: var(--color-text);
}

.if-device-panel--empty {
    display: flex;
    justify-content: center;
    gap: var(--space-1);
}
```

- [ ] **Step 2: Check palette and layout constraints**

Run:

```powershell
rg "if-device|color-mix|radius" crates/inputforge-gui-dx/assets/frame/panel_slot.css
```

Expected: CSS uses existing tokens, stable radii, and no large one-note palette.

- [ ] **Step 3: Commit**

```powershell
git add crates/inputforge-gui-dx/assets/frame/panel_slot.css
git commit -m "style(devices): style the f12 ledger inspector panel"
```

## Task 8: Final Verification

**Files:**
- Verify: Rust crates and GUI behavior

- [ ] **Step 1: Run focused test suites**

Run:

```powershell
cargo test -p inputforge-core device_alias diagnostics registry hotplug
cargo test -p inputforge-gui-dx device_panel config_snapshot panel_slot
```

Expected: PASS.

- [ ] **Step 2: Run full workspace tests**

Run:

```powershell
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 3: Run formatting and lint checks**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p inputforge-app --features gui-dioxus
```

Expected: PASS.

- [ ] **Step 4: Manual GUI smoke**

Run:

```powershell
cargo run -p inputforge-app --features gui-dioxus
```

Expected:
- F12 opens Devices in the shared right-side panel.
- F13 Profiles closes Devices.
- F12 Devices closes Profiles.
- The ledger scrolls when many remembered devices exist.
- The inspector remains fixed.
- `Save name` is disabled until the display-name field changes.
- Saving a non-empty alias updates the row name after the next snapshot tick.
- Clearing the alias falls back to the raw hardware name.
- Disconnected remembered devices show muted state and coverage counts.
- No devices known shows the compact no-signal state.
- Calibration and live preview are absent from F12.

- [ ] **Step 5: Commit verification fixes if any**

Only commit if verification required a code or CSS correction:

```powershell
git add <changed-files>
git commit -m "fix(devices): correct f12 panel verification issues"
```

## Self-Review Checklist

- Spec coverage:
  - Device inventory rows: Task 4 and Task 6.
  - Connection state: Task 1, Task 2, Task 6.
  - Custom display name and raw hardware name: Task 3, Task 4, Task 6.
  - Grouped axes/buttons/hats coverage counts: Task 4 and Task 6.
  - Selected-device fixed inspector: Task 5, Task 6, Task 7.
  - Explicit-save aliases: Task 3 and Task 6.
  - App-wide remembered device records: Task 1, Task 2, Task 4.
  - Human-readable diagnostics: Task 1, Task 2, Task 6.
  - Public diagnostic re-exports and workspace compile sweeps: Task 1 and Task 2.
  - Profile usage and troubleshooting report: Task 4, Task 5, Task 6.
  - Real copyable report text through `arboard`: Task 6.
  - No dimming or focus trap: unchanged shared `PanelSlot`; verified in Task 8.
  - Calibration, live preview, device test actions, and HidHide actions excluded: verified in Task 8.
  - Non-interactive GUI app compile check: Task 8.
- Placeholder scan:
  - No placeholder markers or vague open-ended implementation instructions are present.
- Type consistency:
  - `DeviceDiagnostics`, `DeviceRecord`, `DevicePanelRow`, `DeviceUsageSummary`, and `EngineCommand::SetDeviceAlias` names are consistent across tasks.
  - `inputforge_core::types` re-exports all diagnostic types used by downstream modules.
  - Alias persistence uses `AppSettings::set_device_alias` in both plan tests and command handling.
  - Alias draft state is initialized and reset from selection handlers, not mutated during render.
  - Snapshot construction reads live alias and registry mirrors from `AppState`, so the open Dioxus panel observes engine updates.
