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
choose **Start** to route. Editing a mapping while running updates routing
without a separate editing session. Unsupported keyboard or mouse outputs are
disabled in the action palette according to backend capabilities. Existing
incompatible mappings remain editable and display their specific reason in the
editor; valid mappings continue working.

## Ownership and recovery

Start creates the neutral virtual output set before acquiring any supported
exclusive input ownership. Sampled held controls are disarmed until release or
a genuine hat change. **Stop** releases exclusive capture and neutralizes output
while retaining virtual controller identity and passive monitoring. Starting
again reuses those controllers and refreshes continuous axes without replaying
held button presses. Quitting releases the virtual output resources.

Stop routing before changing the physical selection or virtual controller
layout. Reconnect and changed capabilities reconcile existing native binding
tables; existing logical indices are preserved and new controls append.
Unavailable or ambiguous controls disable affected mappings rather than silently
moving them to a different physical control. Rebind an affected primary,
secondary, or condition input by moving the intended physical control to confirm
its identity. This also works when its displayed logical address is unchanged;
other ambiguous controls remain disabled.

A fault is shown with its reason and an explicit recovery action. Resets clear
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
`--enable` requests activation after loading the profile. Keyboard and mouse
injection are currently unavailable through the Linux adapters.

On Windows, the desktop/tray lifecycle and fixed vJoy layouts use the same engine
commands. Actual hiding and game visibility depend on host configuration;
exclusive capture is not a controller-hiding guarantee.

Automated tests cover projection, mapping issue isolation, capability-driven
controls, monitoring readiness, and snapshot rebaselining without launching a
GUI. Live Linux hardware acceptance, native Windows runtime behavior, and
native/Proton game recognition require separate host acceptance. Prior standalone
results in [linux-streaming.md](linux-streaming.md) and
[linux-uinput.md](linux-uinput.md) apply only to those tested components.

Virtual layout edits are saved as the desired configuration. When controllers
already exist, use **Apply controller changes** while stopped to reconnect them
with that layout. Field edits do not recreate devices. Loading another profile
stops routing and retains matching controllers; a different layout requires
Apply before Start. Native failures can require releasing unusable resources;
Retry remains explicit and passive input monitoring continues.
