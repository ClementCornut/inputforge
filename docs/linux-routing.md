# Controller routing

InputForge uses one mapping editor and routing lifecycle on Linux and Windows.
Start the editor with `dx run -p inputforge-app`. Connected controllers are
monitored immediately: live input, Add mapping, and rebind work while routing is
stopped or running. Inspecting a controller does not select it for
routing or acquire exclusive capture.

## Configure and use

Create or load a profile. In Devices, select the physical controllers to route.
For configurable output backends, add virtual controllers and configure their
axes, buttons, and hats within the displayed driver limits. Fixed output layouts
are shown read-only. Linux uinput supports 2–53 buttons and 0–4 hats per slot.
Virtual slots are numbered 1–16; virtual buttons and hats start at 1.

Use Add mapping or rebind and move the physical control. Assign an output and
choose **Start** to route. Linux keyboard and mouse output is implemented, but
their uinput devices are created only when routing starts. Keyboard mappings
cover every physical key offered by the editor. Mouse mappings support left,
right, middle, back, and forward buttons plus vertical wheel pulses; pointer
movement is not emitted. Editing a mapping while running updates routing
without a separate editing session. Unsupported outputs are disabled in the
action palette according to backend capabilities. Existing
incompatible mappings remain editable and display their specific reason in the
editor; valid mappings continue working.

## Ownership and recovery

Start creates and verifies the keyboard device, then the mouse device, then the
neutral virtual-controller set before acquiring any supported exclusive input
ownership. Sampled held controls are disarmed until release or a genuine hat
change. **Stop** releases exclusive controller capture, releases and closes both
injection devices, and neutralizes output while retaining virtual-controller
identity and passive monitoring. Starting again creates fresh keyboard/mouse
devices, reuses matching controllers, and refreshes continuous axes without
replaying held button presses. Quitting releases every virtual output resource.

Stop routing before changing the physical selection or virtual controller
layout. Reconnect and changed capabilities reconcile existing native binding
tables; existing logical indices are preserved and new controls append.
Unavailable or ambiguous controls disable affected mappings rather than silently
moving them to a different physical control. Rebind an affected primary,
secondary, or condition input by moving the intended physical control to confirm
its identity. This also works when its displayed logical address is unchanged;
other ambiguous controls remain disabled.

A fault is shown with output-specific guidance, expandable technical and cleanup
details, and an explicit **Retry** action. Retry first finishes cleanup and then
recreates the injection devices in normal Start order. It does not change host
permissions or install access rules. Resets clear
pending GUI input candidates and establish a new baseline, even when a
replacement snapshot contains identical values. A held control in a recovery
snapshot is not interpreted as a new rebind gesture. Errors never trigger host
permission changes.

## Axis interpretation and calibration

Advanced axis settings offer **Auto**, **Centered**, and **Rests at minimum**.
Auto uses a trustworthy resting sample; **Detect again** requests a fresh sample
while the axis is at rest. Detection and manual overrides are stored in the
global device registry by native axis code and apply across profiles. The UI
resolves logical axis indices through the monitored binding table, with saved
profile tables as a fallback for disconnected devices. Axis settings are also
available before a profile is loaded. Manual overrides remain independent from automatic detection.

Profile controller configuration preserves selected device identities, native
control tables, and virtual output settings. The legacy Linux profile section
is accepted during migration. Sparse native control codes are stored separately
from the zero-based logical input indices. Profiles cannot silently bind a
foreign device by display name.

Shared input processing applies axis polarity and enabled calibration before
primary and secondary pipeline reads. Snapshots seed continuous values without
inventing edges. Output adapters encode the final normalized values for their
platform.

## Platform behavior and validation boundary

On Linux the app opens visibly without a tray; closing its window quits and
joins the engine thread. `--start-minimized` opens visibly with an explanation.
`--enable` requests activation after loading the profile. Linux keyboard/mouse
injection uses separate uinput event devices. Readiness verifies their exact sysfs
identity and capability bitmaps plus initialized udev classification without opening
their event nodes; `/dev/uinput` is the only keyboard/mouse output permission
prerequisite. InputForge does not capture or suppress physical keyboard/mouse input,
and it does not provide pointer movement.

On Windows, the desktop/tray lifecycle and fixed vJoy layouts use the same engine
commands. Actual hiding and game visibility depend on host configuration;
exclusive capture is not a controller-hiding guarantee.

Automated tests cover exact keyboard/mouse capability descriptions and encoding,
bounded scripted writes and cleanup, engine lifecycle ordering, projection,
mapping issue isolation, monitoring readiness, and snapshot rebaselining without
launching a GUI or creating a real uinput device. On 2026-10-12, the user reported
that the application works on Windows after their Windows fixes. This is user
acceptance, not a recorded exhaustive platform test matrix. Live Linux
keyboard/mouse and controller desktop acceptance and native/Proton game
recognition still require separate host acceptance. Prior standalone
results in [linux-streaming.md](linux-streaming.md) and
[linux-uinput.md](linux-uinput.md) apply only to those tested components.

Virtual layout edits are saved as the desired configuration. When controllers
already exist, use **Apply controller changes** while stopped to reconnect them
with that layout. Field edits do not recreate devices. Loading another profile
stops routing and retains matching controllers; a different layout requires
Apply before Start. Native failures can require releasing unusable resources;
Retry remains explicit and passive input monitoring continues.

## Platform parity status as of 2026-10-12

Linux implements controller discovery and hotplug, passive monitoring and live
rebind, exclusive evdev capture, configurable virtual controllers, keyboard
combinations, five mouse buttons and vertical wheel pulses, axis interpretation
and calibration, profile editing, recovery, and XDG autostart. Device removal and
the recent axis-setting fixes are shared engine/editor behavior. These features
do not need another Linux implementation.

The remaining work is:

| Area | Current behavior | Remaining work |
| --- | --- | --- |
| Physical-controller hiding | Exclusive evdev capture does not guarantee hiding from game input backends using joydev or hidraw. | Design and implement selected-controller hiding with rollback, crash/restart recovery, and unplug/replug restoration. Keep normal game launches unchanged. |
| Tray and background operation | Linux opens visibly, has no tray, and quits on window close. Start-minimized opens visibly with an explanation. Autostart itself is implemented. | Add tray controls, close-to-tray, and start-minimized behavior with a usable fallback when a desktop has no tray support. |
| Distribution and permission setup | Release automation tests and bundles Windows only. Linux access rules are a manual development setup. | Provide Linux CI, an Arch/CachyOS package or guided setup, narrowly scoped access rules, and upgrade/removal documentation. Keep the application unprivileged. |
| Integrated host acceptance | Standalone evdev/uinput and isolated native SDL results cover only their documented scenarios. | Verify real-controller routing, keyboard/mouse delivery, cleanup and recovery, then ordinary native/Proton game recognition and absence of duplicate physical input while hiding is active. |

Linux virtual controllers support 2–53 buttons, with 32 by default, eight available
axes, and up to four hats. The old 128-button proposal is deliberately unsupported
because its contiguous event-code range crossed unrelated input categories; see
[uinput capabilities](linux-uinput.md). Profiles requesting larger layouts require
adaptation. Loading a profile also does not make SDL and evdev device identities
interchangeable; unresolved controls need explicit local binding/rebind.
Pointer movement and physical keyboard/mouse capture are outside the existing
action scope on both platforms, not missing Linux parity features.

For the next session, first verify the integrated Linux routing baseline, then
design physical hiding and its recovery before implementation. Finish game
acceptance with normal launches after hiding is available. Packaging and tray
support remain separate deliverables. No implementation or installation of this
remaining work was performed in the documentation update.

The earlier documents under `docs/superpowers/` are historical designs and plans;
some are tracked, while newer ones are Git-ignored local files. Their original
status, API sketches, startup rejection,
128-button layout, and teardown descriptions are not the current implementation
contract; use this guide and the component acceptance documents for current
behavior.
