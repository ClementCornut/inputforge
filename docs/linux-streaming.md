# Linux native event streaming (Slice 2c)

Streaming extends the concrete evdev `Capture` API and the `linux-capture` acceptance
example. The engine now consumes it through the evdev `InputSource` adapter;
see [Linux engine routing](linux-routing.md) for the integrated workflow.
Standalone [discovery diagnostics](linux-diagnostics.md) remain read-only;
`watch` and `capture` remain event-free. The acceptance results below describe
the standalone Slice 2c checks, not integrated application acceptance.

## Explicit acquisition and readiness

Discovery eligibility, capture readiness, and streaming readiness are separate:
classification and metadata do not prove a device can be grabbed, and a successful
grab does not prove native state can be initialized.

Acquire explicit stable IDs first. `Capture::poll()` continues lifecycle checks
without reading events. The first `Capture::poll_stream(&mut updates)` explicitly
starts initialization. It returns readiness for the entire selection and whether
bounded work remains. Calling it without an acquired selection is an error and
never acquires anything. `stream_state(&id)` returns a borrowed state only while
that device's stream is ready.

Native updates are `Frame`, `Snapshot` (initial or recovered), and `Reset`. Frames
contain ordered native control changes and monotonic observation timestamps.
Snapshots contain sampled state, not reconstructed button edges or gestures. There
is no claim of atomic observation across devices or simultaneous kernel grabs.

A selected disconnect, descriptor replacement, revocation, capability change,
read/query error, or synchronization timeout invalidates streaming and releases
the complete selection. Updates appended by the failing call are removed; the
caller must invalidate all state from that acquisition. Unselected hotplug does
not interrupt healthy selected streams. Reconnecting never reacquires: construct
a fresh owner after fatal errors and explicitly acquire again. Explicit release
clears cached state and buffered records, closing every descriptor even if an
ungrab fails.

## Events, snapshots, and recovery

Events are read nonblocking from the same descriptors that own the exclusive
grabs. Each call checks lifecycle before reading and descriptor health before
publishing. Service order rotates among selected devices. A call permits at most
eight read attempts, 256 processed native records, and one device snapshot attempt.
Interrupted reads consume attempts; discarded synchronization records consume the
record budget. Already-fetched records are retained before another read on that
device, with a maximum batch of 256 records. Oversized batches fail rather than
silently truncating input.

Frames end at `SYN_REPORT`; half-updated hats and partial frames are not published.
Record ordering and multiple button transitions within a frame are preserved.
Unfinished frames are limited to 4,096 records and one second; an idle stream has
no unfinished-frame deadline. Lifecycle and deadline checks run between bounded
calls. Synchronous kernel and discovery operations mean these are userspace work
bounds, not hard realtime guarantees.

Initialization discards queued history within these budgets, queries current key
and ABS state, then verifies that no events arrived during the queries. If they
did, the candidate snapshot is discarded and reconciliation continues. Initial
held buttons and untouched axes therefore need no movement to become visible.

Each `SYN_DROPPED` invalidates the affected cached state immediately and emits `Reset`,
including repeated markers during initialization or recovery without extending the deadline.
The unfinished frame and records through the next `SYN_REPORT` are discarded.
The queue is reconciled and state queried again before a recovered snapshot makes
ordinary frames available. Older queued values are never replayed over the newer
snapshot. Initialization and recovery each have a one-second deadline checked
between calls; sustained activity that prevents reconciliation fails explicitly
and releases the selection. Missed actions and gestures cannot be reconstructed.

## Native codes and normalization

KEY and ABS state retain their sparse native `u16` codes. KEY values zero and one
change state; value two is autorepeat and creates no extra press edge. ABS values
remain raw `i32`. Ancillary events are ignored but consume budget. Relative and
multitouch protocols are unsupported for streaming. Undeclared KEY/ABS controls
and invalid button values fail with context.

Only defined native hat X/Y pairs form hats. Missing components remain absent;
complete pairs with values -1, 0, or 1 produce a direction, with negative Y north.
Invalid values yield no derived direction rather than retaining a stale one.

`AxisInfo::normalize(raw)` offers only this explicit advertised-range conversion:

```text
2 × (raw − minimum) / (maximum − minimum) − 1
```

Arithmetic uses `f64`; invalid ranges return no normalized value. Raw values and
out-of-range conversion results are retained without clamping. No resting-position
polarity inference, deadzone, smoothing, calibration, or `flat`/`fuzz` policy is
applied automatically. Hats use discrete interpretation.

Native codes are not positional profile indices. The engine adapter now persists
frozen native tables, owns calibration at the input-value boundary, and handles
fallible polling and reset/snapshot generations. See [routing](linux-routing.md)
for cleanup and recovery semantics. Native Windows validation remains a separate
gate. Historical Slice 2c results below predate that integration.

## Bounded acceptance command

Run only after live acceptance is authorized, as the regular desktop user:

```sh
cargo run --locked -p inputforge-core --features evdev-input --example linux-capture -- stream --seconds 10 --device 'evdev:v1:ID'
```

One or more `--device` IDs and `--seconds` from 1 through 60 are required. Invalid
arguments and help are handled before discovery. The request is printed and
flushed before acquiring. The command performs no stdout/stderr writes while
owning grabs, then releases every descriptor before reporting or formatting errors.
Closed or blocked output cannot extend ownership through a report write.

The report records whether all selected streams were ready at the end, initial
and final historical states, per-control event counts
and observed extrema (including snapshots), reset counts, and the first 256 frames.
Omitted frame counts show sample truncation; aggregation and state processing
continue after the sample fills. Snapshots do not increment event counts. A reset
or stream failure clears the affected usable final state. The initial state and
frame sample remain explicitly historical; a printed final state means the last
valid sampled state before release, not a currently captured device. Stream,
release, and report failures are all retained in the error result.

## Separate live acceptance

The September 18, 2026 results are recorded below. Hardware-free tests do not replace
these checks. Use explicit controller IDs and perform one check at a time:

1. Verify initial held buttons, untouched axes, and hats on both VIRPIL sticks and
   the Thrustmaster pedals before moving controls.
2. Exercise buttons, available hats, axes, and each pedal independently; compare raw
   codes, ranges, directions, and final released state.
3. Stream all three under sustained movement; confirm each device progresses and
   timed release remains responsive.
4. Cause a queue overflow with a controlled process pause while changing a held
   button; confirm a reset, recovery, and no stale pressed state. If a delay exceeds
   a synchronization deadline, verify explicit failure and complete release.
5. Disconnect a selected device; confirm surviving selected ownership releases.
   Reconnect and confirm there is no automatic acquisition.
6. Disconnect an unselected device; selected streaming must continue.
7. Check timeout, SIGINT, SIGKILL, closed output, and a blocked output consumer in
   separate runs. Confirm another process can acquire immediately afterward.
8. Recheck exclusivity, keyboard/mouse usability, standalone diagnostics, and normal
   Linux startup rejection.

Revocation is fixture-tested unless a separate live seat/session-revocation exercise
is authorized. Optional syscall tracing must show event reads only on explicitly
selected captured descriptors and no uinput access. Native Windows validation is
separate from Linux checks. Do not change device modes, groups, or udev rules as
part of this acceptance.

## Recorded Slice 2c live acceptance

Observed on September 18, 2026 using the uncommitted `codex/linux-streaming`
implementation, both VIRPIL WarBRD-D sticks with Constellation ALPHA Prime grips,
and Thrustmaster Sim Pedals. The user operated and disconnected hardware; the agent
ran the bounded harness and inspected its reports. Runs used the normal desktop
user outside the sandbox, which hides `/dev/input`; no host permissions changed.

- Initial state: all three devices became ready without deliberate axis movement.
  The held left-stick button was present in the initial snapshot without a synthetic
  press event. Untouched pedal axes were reported without any axis events.
- Left-stick movement: 2,433 frames, three moving axes, and multiple button
  transitions, including sparse codes above 255. No resets; timed release succeeded.
  Processing continued after the 256-frame sample filled. The user declined a
  duplicate single-stick test on the right stick.
- Independent pedal movement: all three axes changed and returned to zero;
  326 frames, no resets, and normal timed release. Full mechanical travel was not
  required or established for every axis.
- Simultaneous streaming: 5,975 frames across both sticks and the pedals, with
  substantial updates from every device, no resets, and normal timed release.
- Overflow recovery: paused the left-stick reader for ten seconds while the user
  released a held momentary button and moved the stick. One reset occurred, and
  streaming recovered. Native key 300 changed from initially pressed to finally
  released with zero key events: the replacement snapshot corrected the missed
  release. Key 289 remained pressed, rather than being neutralized.
- Selected disconnect: unplugging the left stick stopped streaming, invalidated
  every final state, and reported descriptor loss plus `ENODEV` during ungrab.
  A separate explicit capture immediately acquired and released both surviving
  devices. Reconnection restored discovery under the same left-stick identity;
  the failed streaming process had exited and did not reacquire.
- Unselected disconnect: with only the right stick and pedals selected, discovery
  confirmed the left stick disappeared while both selected streams stayed ready.
  Pedal input delivered 226 frames; no resets or interruption occurred.
- SIGINT and SIGKILL: competing capture first confirmed exclusive ownership.
  Each signal terminated its own streaming child, after which a separate capture
  successfully acquired and released all three controllers.
- Closed output: closing the consumer while capture was active caused a report
  write failure after timed release; all three devices could then be acquired.
- Blocked output: a 4,096-byte pipe blocked the report writer in the kernel's pipe
  write path. While that process remained blocked, a second capture acquired and
  released all three controllers. Draining the pipe then allowed normal exit.
- Final read-only diagnostics found all three controllers readable. Normal Linux
  application startup still rejected unavailable backends. No `linux-capture`
  processes remained after testing.

Hands-off does not imply every native button is false. VIRPIL documents a maintained
up-position input on the ALPHA Prime flip trigger ([official specifications](https://virpil-controls.eu/vpc-constellation-alpha-prime-r-b-stock.html)).
The user's configured devices can therefore legitimately retain pressed states;
the two active codes from the movement run were not individually mapped to controls.
These sticks advertised no native ABS hat pairs, so no native hat pair was available
for live verification. Native hat-pair interpretation remains covered by fixtures only.

The user confirmed that keyboard and mouse remained usable throughout Slice 2c
capture, with no issues. Native Windows validation, live seat/session revocation,
and optional syscall tracing were not run.
