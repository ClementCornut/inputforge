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

`AppSettings::display_name_for(&self, info: &DeviceInfo) -> String` already
exists at `crates/inputforge-core/src/settings.rs:140`. It is the single
source of truth for the resolution rule:

1. Use the alias from `device_aliases` when present and non-blank after trim.
2. Otherwise use `info.name` when non-blank after trim.
3. Otherwise fall back to `info.id.0`.

The body does not change. This stays the building block for any caller that
already holds a `DeviceInfo`.

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

Populated in `ConfigSnapshot::from_state`. The map is built by iterating
`state.devices` (connected) and `state.settings.device_registry` (remembered)
and resolving each entry through `state.settings.display_name_for(&info)`.
Connected entries take precedence over registry entries on key collision. This
mirrors the existing logic the snapshot already uses to assemble
`DevicePanelRow` rows for both connected and remembered devices at
`crates/inputforge-gui-dx/src/context.rs:408` and `:432`.

### Cleanup

The duplicate private helper `display_name_for(aliases: &HashMap<DeviceId, String>, info: &DeviceInfo) -> String`
at `crates/inputforge-gui-dx/src/context.rs:453` is deleted. Its two existing
callers (lines 408 and 432) switch to `s.settings.display_name_for(&info)`,
calling the engine helper directly.

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
   connected device that has an alias in settings; helper returns the alias.
2. `device_display_name_falls_back_to_hardware_name_when_alias_blank`: alias is
   empty or whitespace; helper returns `info.name`.
3. `device_display_name_returns_alias_for_remembered_disconnected_device`:
   device is in `device_registry` but not in `state.devices`; helper still
   returns the alias.
4. `device_display_name_returns_id_for_unknown_device`: helper called with a
   `DeviceId` not in the snapshot; returns `id.0`.

Existing `device_panel_*` tests in
`crates/inputforge-gui-dx/src/frame/panel_slot/device_panel.rs` cover the
device panel's use of `DevicePanelRow.display_name` and do not change.

Engine-side tests do not change. `AppSettings::display_name_for` semantics are
locked in by `set_device_alias_persists_trimmed_alias` at
`crates/inputforge-core/src/engine/tests.rs:2029` and
`set_device_alias_with_blank_value_clears_alias` at
`crates/inputforge-core/src/engine/tests.rs:2049`.

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
