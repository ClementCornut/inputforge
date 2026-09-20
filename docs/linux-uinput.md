# Linux uinput acceptance example

The `linux-uinput` example is a bounded, noninteractive acceptance harness for
InputForge's Linux virtual-controller output. It creates only the explicitly named
slots, drives one scenario for at most 60 seconds, resets them, releases every
device, and exits. It does not read or grab physical input, load profiles or
settings, start the GUI, install access rules, or provide force feedback.

Build and run it from the repository checkout with the opt-in feature:

```sh
cargo run --locked -p inputforge-core --features uinput-output --example linux-uinput -- \
  exercise --slot 1 --seconds 5
```

`--slot` is repeatable and mandatory. `--seconds` must be 1 through 60. The
default capabilities are eight axes, 32 buttons, and four hats. Override them with
`--buttons 2..53`, `--hats 0..4`, and
`--axes X,Y,Z,Rx,Ry,Rz,Slider0,Slider1`; use `--axes none` for no axes. Slots must
be unique values from 1 through 16. No arguments and `--help` print usage before any
uinput object is constructed. A non-Linux stub supports help and rejects scenarios
as requiring Linux uinput support; native Windows compilation remains unvalidated.

The command prints and flushes the complete request before device creation. From
the first creation attempt until release finishes it writes nothing to stdout,
stderr, or logging. The duration starts immediately before creation, so setup time
counts against the requested lifetime. Creation is transactional in the backend;
the harness also calls release after every attempted run. It retains primary run,
reset, and release failures separately and formats them only after release. It then
always writes and flushes a post-release ownership-ended report; a report failure
is preserved alongside the earlier failures.

## Scenarios and exact values

- `neutral` leaves every control centered/released for the requested lifetime.
- `hold` sets each axis to `+0.5` when `slot + zero-based axis position` is even,
  otherwise `-0.5`; presses button 1; and points hats through N, E, S, W, offset
  by slot and one-based hat number. It then holds those values until reset.
- `exercise` advances every 250 ms. Axes repeat the four-value cycle
  `[-1.0, -0.5, +0.5, +1.0]`, offset by phase, slot, and zero-based axis position.
  Exactly one button is pressed: `((phase + slot) % button_count) + 1`. Hats use
  N, E, S, W, offset by phase, slot, and one-based hat number. Slot offsets make
  simultaneous virtual devices visibly independent. Use `--seconds 16` or longer
  to observe all 53 configured buttons at least once under normal scheduling.

Axis names map to Linux codes as follows: X/Y/Z are `ABS_X/Y/Z`; Rx/Ry/Rz are
`ABS_RX/RY/RZ`; Slider0 is `ABS_THROTTLE`; Slider1 is `ABS_RUDDER`. Finite values
are clamped to `[-1, 1]`, multiplied by 32767, and rounded; nonfinite values become
neutral. Hats 1 through 4 use `ABS_HAT0` through `ABS_HAT3`. Buttons 1 through 12
use `0x120..0x12b`, button 13 uses `0x12f`, and buttons 14 through 53 use
`0x2c0..0x2e7`.

The old 128-button proposal is deliberately unsupported: a contiguous
`0x120..0x19f` map crosses reserved gaps, gamepad, digitizer, and keyboard control
meanings. This slice accepts 2 through 53 buttons using joystick and
`BTN_TRIGGER_HAPPY` codes, with 32 by default. Existing profile addresses and
`map_to_vjoy` serialization are unchanged; unsupported configurations fail instead
of being renumbered. Linux consumers enumerate or remap these codes differently;
this is a generic controller, with no Xbox/XInput or Proton compatibility promise.

Setters stage state. A flush may require multiple kernel write packets. The first
packet contains all changed ABS events (including complete hat pairs), at most seven
key events, and `SYN_REPORT`; later packets contain at most seven more key events
plus `SYN_REPORT`. The complete logical flush is therefore not atomic. Submission
does not prove when or how a consumer observes a revision. The `v1` physical
identity and fixed button map prevent later releases from silently renumbering
controls; consumers may still expose high `BTN_TRIGGER_HAPPY` buttons differently.
The harness keeps no large event history.

## Backend lifecycle

`Output::new` and inactive `configure` validate one through 16 configurations
without hardware access. `configs` reports the sorted configuration and `is_active`
reports ownership. `create` creates the whole set transactionally, waits up to two
seconds on one shared event-node readiness deadline, and submits neutral state.
Setters stage values; `flush`, `reset`, and idempotent `release` perform the output
lifecycle described above. A flush shares one 50 ms write budget across all packets
and devices; cleanup has its own shared 50 ms budget. Each packet gets at most four
attempts, including interruptions and aligned short writes. `WouldBlock`, zero
progress, and invalid byte counts fail immediately. No accepted records are replayed.

A creation failure rolls back the partial set and leaves the owner available for a
later creation attempt. A runtime write failure closes every device and invalidates
the owner; construct a new one to retry. Explicit release attempts neutral output.
`Drop` only destroys/closes devices and cannot guarantee a final neutral revision.
The created identity is `InputForge Virtual Controller <slot>` with physical path
`inputforge/controller/v1/slot/<slot>`; readiness validates the matching event node
before the create call succeeds. It requires an initialized udev event node under
the exact sysfs identity returned by uinput and verifies identity and capabilities
using read-only metadata ioctls. This does not mean SDL or a game has opened it.
Slot uniqueness applies within one owner, not across independent processes.

The concrete API also has an engine `UinputSink` adapter. It defers runtime-error
teardown until the engine releases physical capture; standalone API cleanup remains
unchanged. See [Linux engine routing](linux-routing.md) for session configuration,
recovery, and the integrated editor workflow.

## Access and cleanup limits

Run as the normal desktop user. Missing or denied `/dev/uinput` access is a
prerequisite failure; the example does not run as root, install packages, change
permissions, or create udev rules. Use the existing read-only diagnostics first:

```sh
cargo run --locked -p inputforge-diagnostics
```

That report remains metadata-only and does not establish write access or create a
device. Normal Linux application startup now uses the engine adapter; this
standalone example remains independent of application/profile state.

Normal completion resets and closes all virtual devices. A reported runtime failure
still attempts reset and release; the backend has already closed devices when a
write invalidates the owner. That failed owner cannot be recreated. `release` is
idempotent. Reporting happens only before creation and after release, so blocked
stdout/stderr cannot retain owned devices. An in-flight kernel write is synchronous:
userspace deadlines bound retry policy, not real-time syscall return. Default SIGINT
and SIGKILL handling terminates the process; Linux then closes its uinput file
descriptors, but neither signal guarantees that a neutral state was emitted first.
Test signal cleanup separately from the timed reset path.

## Live acceptance checklist

Perform live checks later, one explicit check at a time:

1. Create one slot and independently verify its identity, exact axes/buttons/hats,
   button-code mapping, and InputForge exclusion signature.
2. Run `hold`; verify the exact expected values for that slot in an independent
   event monitor.
3. Verify the controller in a native SDL application.
4. Verify Proton separately; native SDL success does not establish Proton behavior.
5. Let a timed scenario finish and confirm neutral reset plus device removal.
6. Test SIGINT and SIGKILL separately and record the observed cleanup behavior.
7. Create multiple slots and confirm their identities and independently offset
   exercise states.
8. Force a write failure and confirm all devices close, errors return promptly,
   and the failed owner cannot be reused.
9. Block output long enough to cross the deadline and confirm release follows the
   blocked call; the deadline bounds requested lifetime but cannot interrupt a
   kernel call already in progress.

Automated parser and fake-owner tests cover bounds, explicit slots, capability
selection, cleanup ordering, and preservation of run/reset/release/report failures.
Proton, Windows, signal, or blocked-output acceptance has not been
performed. The profile, engine, GUI startup, Windows behavior,
force feedback, and keyboard/mouse output remain outside this harness.

## Automated verification

Hardware-free checks on Linux, 2026-09-18:

- `make verify`: passed (1,856 tests passed, seven existing tests ignored).
- `cargo test --locked --offline -p inputforge-core --all-features --all-targets`:
  passed (957 tests, including the capture and uinput examples).
- `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings`:
  passed.
- Core `cargo check --locked --offline --all-targets` with no default features,
  and separately with only `uinput-output`: passed.

The tests use scripted device ownership, readiness, writers, and a clock; they
exercise exact encoding, partial records, errors, rollback, deadlines, teardown,
and discovery exclusion without opening uinput or reading physical events.
Kernel behavior was checked against the [uinput documentation](https://docs.kernel.org/input/uinput.html),
[event-code documentation](https://docs.kernel.org/input/event-codes.html), and
Linux 6.18 input/uinput source, alongside the locked evdev 0.13.2 implementation.

## Live result: first neutral controller

On 2026-09-18, an explicitly approved `neutral --slot 1 --seconds 10` run passed
as the normal desktop user using existing host access. Independent sysfs and
read-only `EVIOCGABS` inspection verified:

- Name `InputForge Virtual Controller 1`, physical path
  `inputforge/controller/v1/slot/1`, bus `0006`, vendor `0000`, product `0001`,
  version `0001`.
- Exactly `EV_SYN`, `EV_KEY`, `EV_ABS`; the expected 32 sparse button codes;
  eight axes and all four hat pairs.
- Axis ranges `-32767..32767`, hat ranges `-1..1`, initial ABS values zero,
  and zero fuzz/flat/resolution.
- Successful harness exit after 10.031 seconds, no stderr, and removal of both
  `/dev/input/event13` and `/sys/devices/virtual/input/input41`.

The first attempt exposed evdev 0.13.2's incorrect `UI_SET_PHYS` ioctl size
(`char` instead of the Linux UAPI's `char*`) and failed before device creation.
The backend now owns its setup descriptor directly and uses correctly encoded
native setup ioctls; it still uses evdev for metadata decoding and event records.
A hardware-free ABI regression test reproduces the size mismatch and verifies the
correction. No dependency version or host configuration was changed.

Only virtual-device metadata was inspected; no physical input events were read
or grabbed. This check does not establish changing output values, event delivery,
native/Proton consumer recognition, signal cleanup, or multiple-device acceptance.

## Live result: held values and neutral teardown

On 2026-09-18, the approved `hold --slot 1 --seconds 10` check passed. A bounded
read-only monitor verified the opened descriptor's name, physical path, and input
ID before inspecting state or reading events. It opened only this run's virtual
event node (`/dev/input/event13`) and never requested an exclusive grab.

- X/Z/Ry/Slider0 read `-16384`; Y/Rx/Rz/Slider1 read `16384`, matching the
  configured half-scale values and rounding rule.
- Only logical button 1 (`BTN_TRIGGER`, `0x120`) was pressed; the other 31
  configured buttons were released.
- Hats 1 through 4 read N `(0,-1)`, E `(1,0)`, S `(0,1)`, W `(-1,0)`.
- The monitor received a complete held-state frame and a complete neutral frame,
  each ending with `SYN_REPORT`. The latter released button 1 and centered all
  axes/hats before removal.
- The harness exited successfully after 10.023 seconds without stderr. The
  monitor then received `ENODEV`; the event node and matching sysfs device vanished.

This establishes native evdev state and event delivery for this scenario, including
observed neutral delivery on timed teardown. It does not guarantee that every
consumer will drain a final neutral frame before destruction. Proton recognition,
the full 53-button exercise, signals, multiple devices, and live failure injection
remain unvalidated. No implementation or host configuration change was needed.

## Live result: native SDL recognition

On 2026-09-18, installed SDL 3.4.16 successfully consumed a ten-second
`hold --slot 1` run. The small native consumer ran in a temporary Bubblewrap
mount/PID namespace with a fresh `/dev` containing only the test's virtual event
node. HIDAPI was disabled in that consumer. Physical input nodes were unavailable;
the host's device permissions and configuration were unchanged.

- SDL found exactly one joystick with the expected name and event-node path.
- SDL reported eight axes, 32 buttons, and four hats without a custom mapping.
- All held controls matched: button index 0 pressed, the other buttons released,
  and hats reported Up/Right/Down/Left (`1,2,4,8`). Axis values alternated
  `-16385,16384`, within one count of the native half-scale values after SDL's
  conversion to its own signed range.
- SDL detected disconnection. Consumer and output owner both exited successfully,
  with empty stderr, after 10.021 seconds; the virtual device was removed.

This proves the native SDL joystick path in the isolated consumer. It does not
establish the SDL Gamepad mapping API, ordinary desktop hotplug behavior,
Proton/Wine, or a particular game's input support. SDL's explicit-device hint
alone is not an isolation boundary: its
[Linux driver](https://github.com/libsdl-org/SDL/blob/release-3.4.16/src/joystick/linux/SDL_sysjoystick.c)
also performs device discovery, hence the separate device namespace for this test.

## Elite Dangerous: acceptance pending

On 2026-09-18, a user-authorized 60-second slot-1 exercise completed successfully
and removed its virtual controller. The user could not reach a binding field in
time, so the run supplies no evidence of game/Proton recognition. Retry only once
the chosen binding screen is ready; game compatibility remains unvalidated.

A second 60-second run completed and released its controller, but the user reported
no captured input in Elite Dangerous. The cause was not diagnosed; this does not
establish a general limitation of the game's joystick support. The user chose
Star Citizen for the next game acceptance test, which is pending.
