# F12 Device Info Panel: Design Spec

## Context

F12 was originally scoped as a Devices side panel plus calibration drill-in. Calibration is now deferred to a later spec. The revised F12 surface is a compact device information panel for hardware identity, profile usage, and global device aliases.

The panel remains a right-side secondary surface plugged into F7's shared Replace slot. Opening Devices closes Profiles, and opening Profiles closes Devices. The panel does not dim the workspace and does not trap focus away from the mapping list or editor.

## Confirmed Scope

F12 includes:

- Device inventory rows for all known physical devices.
- Connection state.
- Both the custom display name and raw hardware name.
- Grouped mapped coverage counts: axes, buttons, hats.
- A selected-device detail section.
- Explicit-save global custom device names.
- Human-readable device identity diagnostics.
- Non-live profile usage and troubleshooting utilities.

F12 excludes:

- Calibration editing.
- Live axis, button, or hat preview.
- Rumble, LED, player index, or device test actions.
- HidHide management actions.

## Design Direction

The surface is a restrained product UI in InputForge's existing Evolved Glass Cockpit dark theme. It should feel like a precise hardware inventory panel, not a diagnostic toy and not a metrics dashboard.

Primary references:

- Linear side panels for quiet density.
- Stripe settings rows for explicit commit controls.
- Figma inspector restraint for editing a selected object's properties without modal churn.

The physical scene is a sim-rig power user editing a profile on a dim desktop setup. They are focused on identifying hardware and checking profile coverage, not tuning live signal.

## Layout

Use the approved Ledger + Fixed Inspector layout.

The top section is a dense, scrollable device ledger. Each row shows:

- Connection indicator and state label.
- Custom display name as the primary name.
- Raw SDL/hardware name as the secondary name.
- Grouped coverage counts, for example `Axes 4/6`, `Buttons 12/32`, `Hats 0/1`.

The bottom section is an always-visible fixed inspector for the selected device. The ledger scrolls when a rig has more devices than fit; the inspector stays anchored so alias editing and troubleshooting actions do not jump around.

The inspector does not repeat the device name as a title. It starts with the `Display name` field and `Save name` action, then shows the hardware name, human-readable diagnostics, profile usage, and a quiet `Copy device report` action.

When the panel opens, select the first connected device if one exists. Otherwise select the first known disconnected device. If no devices are known, the ledger and inspector collapse into the no-signal empty state.

## Device Alias Model

Custom device names are app-wide, stored in `AppSettings`, not in profiles.

Rules:

- Alias keying uses the current stable `DeviceId`.
- Empty alias means "use the hardware name".
- Saving is explicit via `Save name`.
- Clearing an alias resets the display name to the hardware name.
- Save failure is shown inline near the field.

The GUI should resolve display names through:

1. Global alias, when present.
2. Hardware name from `DeviceInfo`.
3. Device identifier fallback, only if no name is available.

## Device Identity Diagnostics

F12 should collect and expose identity fields that SDL3 can provide:

- SDL GUID.
- Vendor ID and product ID, when available.
- Product version and firmware version, when available.
- Serial number, when available.
- SDL joystick type.
- Connection kind: wired, wireless, unknown, invalid.
- Battery state and percentage, when available.
- Platform instance path, already used by HidHide on Windows.
- Whether SDL reports the joystick as virtual.

These fields do not all need equal visual weight. The row stays compact; diagnostics live in the fixed inspector. The default presentation is human-readable first: connection kind, hardware type, VID/PID, serial when present, and platform path lower in the detail.

## Profile Usage

The panel should show non-live usage data derived from the current profile and `ConfigSnapshot.mappings`.

Required usage summaries:

- Grouped mapped coverage counts by input kind.
- Count of mappings that reference the device as primary input.
- Count of mappings that reference the device through merge or conditional inputs.

Selected-device detail may list mapping names or modes that touch the device. This is a static profile cross-reference, not a live preview.

## Troubleshooting Utility

Add a quiet but visible `Copy device report` action in the fixed inspector footer.

The report should include:

- Display name.
- Hardware name.
- Connection state.
- Input counts and mapped counts.
- SDL GUID.
- VID/PID/version fields when available.
- Serial when available.
- Instance path when available.

The report is plain text, device-only, and intended for debugging device-detection issues. It does not include profile path or active mode in F12.

## Key States

- Connected: connected label and success state.
- Disconnected: disconnected label, muted hardware name, known profile counts still visible.
- No devices known: compact no-signal empty state.
- No profile loaded: inventory still appears; mapped coverage is unavailable or zero with neutral treatment.
- No selected device: only possible when no devices are known; show the no-signal empty state instead of an empty inspector.
- Alias dirty: edited value differs from persisted alias; `Save name` becomes available.
- Alias save failure: inline error with retry possible.

## Implementation Notes

Current code stores `DeviceInfo` with name, axes, buttons, hats, `instance_path`, and `axis_polarities`. `Sdl3Input` currently builds `DeviceId` from SDL GUID.

This work requires engine/core changes:

- Extend `DeviceInfo` or add a related metadata struct for SDL identity diagnostics.
- Add app-wide device aliases to `AppSettings`.
- Add an engine command to set or clear a device alias and persist `settings.toml`.
- Project resolved display names into GUI snapshots so existing and future surfaces can use aliases consistently.
- Disconnected devices use the metadata currently retained in memory. F12 does not add a persistent last-known diagnostics cache.

## Future Enhancements

- If real-world rigs reveal duplicate identical devices that share the same alias key, add stronger physical-device identity handling using serial/path metadata and surface a duplicate-risk warning only when detected.
- HidHide read-only state or management belongs in a later device-management pass, not F12.

## Deferred Decisions

- Calibration drill-in remains deferred.
- Live input preview remains deferred.
- Duplicate-device identity warnings remain deferred until real hardware exposes the issue.
