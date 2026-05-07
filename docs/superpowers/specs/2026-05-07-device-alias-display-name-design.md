# Device Alias Display Name Design Spec

## Context

The engine already persists per-device aliases. `AppSettings.device_aliases` at
`crates/inputforge-core/src/settings.rs:41` holds them, `AppSettings::display_name_for`
at `crates/inputforge-core/src/settings.rs:140` resolves alias-or-fallback, and
`EngineCommand::SetDeviceAlias` at `crates/inputforge-core/src/engine/command.rs:111`
mutates them. The Device Inspector in the device panel already lets the user set
and clear an alias, and the device panel itself reads the precomputed
`DevicePanelRow.display_name` field correctly.

The remaining gap is the rest of the GUI. Several call sites still take a raw
`DeviceInfo.name` directly from `cfg.devices`, ignoring the alias. The user
wants every user-facing surface to honor the alias without having call sites
sprinkle alias-vs-name conditions.

## Goals

- Show the user's alias instead of the hardware name on every user-facing GUI
  surface that currently shows a device name.
- Keep alias resolution centralized so call sites do not branch on alias
  presence.
- Cover both connected devices and remembered (disconnected) devices, so a
  mapping that references an unplugged device still shows the alias.
- Eliminate the duplicate alias-resolution helper in the GUI crate.

## Non-Goals

- No changes to engine `tracing::info!` output or other engine diagnostics.
  Logs continue to emit `info.name` so bug reports stay tied to stable
  hardware identity.
- No changes to the existing `EngineCommand::SetDeviceAlias` flow, the
  `AppSettings.device_aliases` storage, or the device panel inspector UI.
- No new caching layer beyond the existing snapshot lifecycle. The precomputed
  display name is rebuilt every snapshot tick like the rest of `ConfigSnapshot`.
- No changes to engine state shape (`DeviceState`). Alias resolution stays a
  presentation concern in `inputforge-gui-dx`.

## Layering

Two helpers, no duplication.

### Core canonical resolver

Add a free function in `crates/inputforge-core/src/settings.rs` next to
`AppSettings::display_name_for`:

```rust
pub fn display_name_for_device(
    aliases: &HashMap<DeviceId, String>,
    info: &DeviceInfo,
) -> String
```

This is the single source of truth for the resolution rule:

1. Use the alias from `device_aliases` when present and non-blank after trim.
2. Otherwise use `info.name` when non-blank after trim.
3. Otherwise fall back to `info.id.0`.

`AppSettings::display_name_for(&self, info: &DeviceInfo) -> String` at
`crates/inputforge-core/src/settings.rs:140` delegates to
`display_name_for_device(&self.device_aliases, info)`. GUI snapshot code uses
the same free function with the mirrored `AppState.device_aliases` map instead
of keeping a duplicate resolver in the GUI crate.

### GUI by-id projection

A new method on `ConfigSnapshot` in `crates/inputforge-gui-dx/src/context.rs`:

```rust
pub(crate) fn device_display_name(&self, id: &DeviceId) -> String {
    self.device_display_names
        .get(id)
        .cloned()
        .unwrap_or_else(|| id.0.clone())
}
```

This is the surface that GUI call sites consume. It takes a `DeviceId` and
returns the resolved display name, with a deterministic fallback to the raw id
string when the snapshot does not know the device.

### Snapshot field

Add to `ConfigSnapshot`:

```rust
pub device_display_names: HashMap<DeviceId, String>,
```

Populated in `ConfigSnapshot::from_state`. Build a mutable
`HashMap<DeviceId, String>` by inserting remembered devices from
`s.device_registry` first, then connected devices from `s.devices`, so a live
connected entry overwrites a stale remembered entry on key collision. Resolve
each entry through `display_name_for_device(&s.device_aliases, &info)`.

This mirrors the existing `build_device_panel_rows` logic in
`crates/inputforge-gui-dx/src/context.rs`, which currently assembles connected
rows around line 528 and remembered rows around line 552.

### Cleanup

The duplicate private helper `display_name_for(aliases: &HashMap<DeviceId, String>, info: &DeviceInfo) -> String`
at `crates/inputforge-gui-dx/src/context.rs:573` is deleted. Its existing
`build_device_panel_rows` callers around lines 528 and 552 switch to
`display_name_for_device(&s.device_aliases, &info)`, calling the shared core
helper directly.

Because `ConfigSnapshot` gains a required `device_display_names` field, update
existing explicit `ConfigSnapshot { ... }` test literals to compile. Prefer
`..Default::default()` for tests that only care about a subset of fields. Tests
that assert alias behavior should populate `device_display_names` directly or
construct the snapshot through `ConfigSnapshot::from_state`.

The `DevicePanelRow.display_name` field stays as is. The device panel already
consumes it correctly at `crates/inputforge-gui-dx/src/frame/panel_slot/device_panel.rs:90`
and the report builder uses it at line 298. There is no reason to disturb that
codepath.

## Call Site Migration

Each call site below currently writes a `find().map_or()` boilerplate that
reads `info.name`. After the change, each line collapses to a
`cfg.device_display_name(&id)` call.

| File | Line | After |
| --- | --- | --- |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs` | 292 | `cfg.device_display_name(device_id)` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs` | 429 | `cfg.device_display_name(id)` |
| `crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs` | 63 | `cfg.device_display_name(&id)` |
| `crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs` | 59 | `cfg.device_display_name(device)` |
| `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs` | 193 | `.map(\|device\| (device.info.id.0.clone(), cfg.device_display_name(&device.info.id)))` |

Two of the existing helpers return `&str` to avoid allocation
(`stage.rs:292` and `:429`). The new helper returns `String` to match the
precomputed map's owned values. Adjacent `format!` and `.to_owned()` lines
drop the now-redundant conversions. No call site holds the `&str` past the
immediate format expression, so the lifetime change is local cleanup, not a
broader refactor.

## Sites Not Touched

- `crates/inputforge-core/src/device/sdl3.rs:202,326` and
  `crates/inputforge-core/src/engine/run.rs:1254`: engine tracing and vJoy
  filtering. These keep `info.name` per the engine-logs non-goal.
- `crates/inputforge-core/src/engine/tests.rs:804,1557` and
  `crates/inputforge-core/src/state/device.rs:68`: assertions verifying engine
  state stores hardware names verbatim. These remain.
- `crates/inputforge-gui-dx/src/frame/panel_slot/device_panel.rs:90,298`:
  already use `row.display_name` and `row.hardware_name` correctly.

## Testing

### Smoke (cargo)

`cargo test -p inputforge-gui-dx` and `cargo test -p inputforge-core`. No
Dioxus runtime needed.

New unit tests added in `crates/inputforge-gui-dx/src/context.rs`:

1. `device_display_name_returns_alias_when_present`: snapshot built with one
   connected device that has an alias in `s.device_aliases`; helper returns the
   alias.
2. `device_display_name_falls_back_to_hardware_name_when_alias_blank`: alias is
   empty or whitespace; helper returns `info.name`.
3. `device_display_name_returns_alias_for_remembered_disconnected_device`:
   device is in `device_registry` but not in `state.devices`; helper still
   returns the alias.
4. `device_display_name_returns_id_for_unknown_device`: helper called with a
   `DeviceId` not in the snapshot; returns `id.0`.
5. `device_display_name_connected_device_overrides_registry_record`: the same
   `DeviceId` exists in both `s.device_registry` and `s.devices`; helper uses
   the connected device's resolved display name.

Add or extend call-site regression tests so the migrated surfaces cannot
silently regress to `info.name`:

- `crates/inputforge-gui-dx/src/frame/mapping_list/source_label.rs`: source
  labels use `cfg.device_display_name(device)`.
- `crates/inputforge-gui-dx/src/frame/mapping_list/filter.rs`: device filter
  chips use aliases and still append the raw id when two chips share the same
  display label.
- `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs` or its
  existing pipeline tests: merge-axis and conditional stage labels use aliases.
- `crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs`: source dropdown options
  use aliases. If needed, extract the option-building expression into a small
  pure helper so this behavior is unit-testable without a Dioxus runtime.

Existing `device_panel_*` tests in
`crates/inputforge-gui-dx/src/frame/panel_slot/device_panel.rs` cover the
device panel's use of `DevicePanelRow.display_name` and do not change.

Add core resolver tests in `crates/inputforge-core/src/settings.rs` for alias,
blank alias fallback, hardware-name fallback, and id fallback. Existing
engine-side persistence tests for `EngineCommand::SetDeviceAlias` remain in
place.

### Manual (dx run)

`dx run -p inputforge-app`. After cargo passes, launch the app, set an alias
on a connected device through the device panel inspector, then walk through:

- mapping list: every row's source label,
- filter chips,
- mapping editor stage labels (action labels and detail rows),
- bulk map source dropdown options.

All four surfaces should show the alias instead of the hardware name. With
the device disconnected, the same surfaces should still show the alias rather
than the raw id string. Clearing the alias from the inspector should revert
all surfaces to the hardware name.
