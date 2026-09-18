# Linux discovery diagnostics (Slice 2a)

Exclusive-grab behavior is covered separately by the bounded
[Linux capture acceptance example](linux-capture.md); this diagnostic remains
read-only and does not test capture readiness or busy state.

From the repository checkout, run in your normal, unprivileged desktop session:

```sh
cargo run --locked -p inputforge-diagnostics
```

This command prints a one-shot report and exits. It does not start the GUI or engine,
load profiles, create settings, read event streams, grab devices, create virtual
devices, or change permissions. Normal Linux application startup still returns the
existing unavailable-backends error. Diagnostics are a separate workspace tool in
`tools/inputforge-diagnostics`, not an application flag. The tool accepts only
`--help` and `--version`; running it without arguments generates the report. No
Dioxus assets are needed. The app no longer enables core's `evdev-input` feature;
the tool owns that opt-in until an application backend needs it.
Building requires the existing Rust toolchain, pkg-config and libudev development
files; the command does not install system packages.

Exit status is 0 when a report is generated, including an empty inventory or
per-device errors; 1 for a fatal enumeration/output error; 2 for invalid arguments.
The report includes hardware identifiers and local paths: inspect it before sharing.

## Reading the report

Every libudev event interface is listed, including excluded and ambiguous ones.
Capabilities and names come from sysfs before opening any event node. Only
controller candidates with sufficient metadata are opened, explicitly read-only,
for evdev metadata and ABS-range queries. Handles close before discovery returns.
No current axis values or key states are printed. A candidate is not a guarantee
of capture readiness: exclusive access, global busy state, streaming and recovery
are untested. The snapshot is not atomic; disconnect/reconnect races are reported
when detected and require a rescan.

- **Controller:** joystick udev evidence or a joydev handler on the same input
  parent, plus compatible controller controls. Button-only controllers are valid.
- **Excluded:** actual keyboard/pointer/touch/tablet tags, kernel accelerometer
  property, or the reserved `inputforge/` physical-path signature. External virtual
  controllers are not automatically excluded.
- **Ambiguous:** insufficient, mixed or contradictory evidence. The reason remains
  visible; no ambiguous node is opened.

The Thrustmaster `044f:b371` XYZ-only pedal signature is narrowly recognized despite
its udev accelerometer tag, provided it has joystick evidence. Generic accelerometer
conflicts remain ambiguous. The observed MonsGeek System Control interface's system
keys plus hats/ABS_MISC are insufficient controller evidence, even with a joystick
tag. This does not diagnose its firmware or prove what its volume knob emits.

Native event codes are hexadecimal `u16` values, never positional binding indices.
Incomplete hat pairs remain incomplete. ABS min/max/fuzz/flat/resolution are kernel
metadata, not a new calibration or normalization policy. The sysfs bitmap parser
supports 32/64-bit native kernel words; run a binary matching the kernel word size.

## Identity

Identifiers use the independent `evdev:v1:` namespace. Serial-quality IDs combine
bus, native VID/PID, a genuine serial/uniq and a stable interface discriminator.
Port-quality IDs use stable udev topology and device/interface metadata; moving a
port can change them. Empty or `noserial` values and eventN/jsN/inputN runtime names
are not stable identity material. Missing information leaves identity ambiguous.
All members of an identity collision lose their proposed ID; none is silently
merged or assigned an enumeration-order suffix. Discovery does not persist IDs,
translate foreign bindings, or attach calibration.

## Access failures

Inspect the reported operation, path, error kind and errno. Ownership/mode is only
context: it does not establish effective ACL access. `EACCES`/`EPERM` advice calls
out the active seat/session, uaccess tag, ACL and sandbox. Read-only inspection
commands, replacing the example node with the reported path:

```sh
udevadm info --query=property --name=/dev/input/event25
getfacl /dev/input/event25
```

An accelerometer-tagged pedal may lack the joystick uaccess rule. This slice reports
that situation; it does not supply/install a rule. Do not use root execution, broad
input-group membership or permissive chmod as the diagnostic workaround.

`ENOENT` means unavailable in the current namespace or a device race, not necessarily
missing host hardware. In particular, a sandbox can expose sysfs while hiding
`/dev/input`. Compare with your normal desktop session. `ENODEV` indicates a device
that disappeared during access. Rerun after reconnecting.

`/dev/uinput` is **stat only**: no open, write-access test or device creation occurs.
Its presence/ownership is not proof that a later output backend can run.

## Automated checks (no hardware)

```sh
cargo test --locked -p inputforge-core --features evdev-input device::evdev --lib
cargo test --locked -p inputforge-diagnostics -p inputforge-app
cargo clippy --locked -p inputforge-core --features evdev-input -p inputforge-diagnostics -p inputforge-app --all-targets -- -D warnings
```

Fixtures cover classification, identities/collisions, sparse native codes, malformed
sysfs data, absent/denied/disconnected nodes, read-only query failure cleanup,
standalone CLI argument validation, output errors, escaping and application-startup isolation. They do not
substitute for the following live acceptance checks.

## Separate live-device acceptance

1. Run the command as the regular desktop user with both VIRPIL sticks and pedals
   connected. Compare event nodes, VID/PID, native capabilities and ABS ranges to
   sysfs/udev and independent read-only diagnostics. Expect the two observed VIRPIL
   interfaces and the pedal candidate, and a reasoned ambiguous MonsGeek interface.
2. Check each candidate's actual access result. Permission failures must retain the
   device and give the exact operation/errno with relevant seat/ACL advice. Do not
   change ACLs, rules or group membership as part of acceptance.
3. Reconnect a controller and rerun. eventN/jsN changes must not change serial IDs;
   port identities stay stable on the same topology. Port movement can change a
   port-quality ID. Identical units lacking distinct serial/interface information
   must remain explicitly ambiguous if they collide.
4. Unplug during repeated one-shot scans: a retained per-device error or absence
   from the later inventory is acceptable; crashes, hangs and fabricated identity
   are not. There is no hotplug monitor in this slice.
5. Confirm no new virtual devices, settings files or permission changes. The command
   must terminate promptly and release descriptors. Optional syscall tracing should
   show event nodes opened O_RDONLY and metadata ioctls, no event reads, EVIOCGRAB or
   uinput open/write. Do not trace unrelated application input streams.
6. Run the application (`inputforge-app`) without arguments: expect the same
   intentional Linux startup rejection. Its CLI no longer accepts `--diagnose-linux`.

Native-game/Proton output compatibility, capture acquisition/rollback, evdev state
recovery, binding translation, calibration and keyboard/mouse output remain later
slice work.

## Recorded live checks (2026-09-17)

Before extracting the command into the standalone tool, the host run and
user-provided terminal results established:

- Both VIRPIL WarBRD-D sticks were readable, each reporting six native ABS axes
  with ranges 0..60000 and 32 supported key codes. Each retained its serial-quality
  identity and read access after reconnecting to its original USB port.
- Thrustmaster Sim Pedals were classified as a controller while the report retained
  the conflicting udev accelerometer evidence. The initial read-only open failed
  with EACCES; there was no user-specific ACL or uaccess tag. The desktop session
  was active, local and attached to seat0.
- In a separately approved host configuration step, the user created
  `/etc/udev/rules.d/71-inputforge-pedals.rules`, matching input event nodes with
  VID 044f, PID b371 and USB interface 00, and adding the uaccess tag. After reloading
  rules and reconnecting, the user confirmed an ACL for dyecode and a successful
  diagnostic read: three ABS axes, each ranging from 0 to 65535. The port-quality
  identity remained unchanged. InputForge itself did not install this rule.
- With the right VIRPIL disconnected, the user confirmed that diagnostics completed
  normally and omitted it. The user then reconnected it; another identical scan was
  deliberately skipped. This establishes absence between scans, not a tested race
  during an ioctl.
- The MonsGeek System Control interface remained ambiguous and unopened.

Port movement, forced event-number reassignment, a physical disconnect during
metadata acquisition, and syscall tracing were not manually exercised. Automated
fixtures separately cover identity ordering, collisions and injected device errors.
Streaming, exclusive capture and virtual output remain outside this slice.

## Linux release requirement: permission setup

End users should not need to discover udev commands or hand-edit rule files. Linux
packaging or a one-time guided setup must provide reviewed, narrowly scoped access
rules where the distribution does not already grant suitable session access. Any
privileged installation requires administrator approval; the normal application
runs unprivileged. Diagnostics should explain the affected device and offer the
supported setup path. Access configuration is local to each computer.

This is a release requirement, not implemented Slice 2a behavior. The pedal rule
above is a development-machine workaround. Package ownership, upgrade/removal,
distribution coverage and any guided setup mechanism require a separate design
before shipping. Avoid broad input-group grants, permissive chmod or running the
application as root. Existing working controllers should not require extra setup.
