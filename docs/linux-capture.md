# Linux exclusive-capture acceptance example (Slice 2b)

This bounded example exercises Linux controller inventory changes and exclusive
`EVIOCGRAB` ownership. It is an acceptance tool, not an InputForge input backend.
The `watch` and `capture` commands do not read input events or state. The separate
[`stream` command](linux-streaming.md) explicitly opts into native events and state
recovery. None of these commands creates output devices, loads profiles or
settings, changes permissions, retries acquisition, or selects every device
implicitly.

Build or run it as the regular desktop user:

```sh
cargo run --locked -p inputforge-core --features evdev-input --example linux-capture -- --help
cargo run --locked -p inputforge-core --features evdev-input --example linux-capture -- watch
cargo run --locked -p inputforge-core --features evdev-input --example linux-capture -- capture --seconds 10 --device 'evdev:v1:ID'
```

No arguments and `--help` print usage before Linux hardware discovery starts.
`watch` polls until interrupted and prints only inventory or ownership changes.
`capture` requires one or more explicit IDs in the independent `evdev:v1:`
namespace and a duration from 1 through 60 seconds. Obtain IDs with the read-only
diagnostic described in [Linux discovery diagnostics](linux-diagnostics.md).

Capture acquisition is all-or-nothing from the caller's perspective. The kernel
grabs selected event nodes sequentially; there is no simultaneous multi-device
kernel transaction. If any grab fails, the core releases every grab acquired by
that attempt and leaves the `Capture` reusable. Calling `acquire` while it already
owns devices is rejected without changing the existing set. The example does not
retry either case.

Before acquisition, `capture` prints and flushes the requested IDs. It performs no
stdout or stderr writes while it owns grabs, so a blocked output consumer cannot
extend the requested ownership duration. After release it prints the final inventory
and empty ownership set. The duration is checked between synchronous 10 ms lifecycle
polls; a slow or blocked kernel call can delay the next check.

After acquisition, the example polls every 10 ms. A poll failure invalidates the
owner and releases all selected devices. A selected-device disconnect is therefore
an error for the whole acquisition. An unselected-device disconnect changes the
inventory without ending capture. Reconnecting a selected device updates the
inventory but never grabs it automatically; start a new explicit capture instead.
The example releases on its duration deadline and attempts release after poll
failure. Any output failure occurs before acquisition or after release. Normal
process teardown, interruption and process death also close the owned descriptors,
so the kernel releases their grabs.

While another process owns a grabbed controller, applications that depend on its
event stream should stop receiving new events. Events prevented from reaching
those applications are not buffered or restored by InputForge, and this example
makes no claim that their prior state will be repaired after release.

## Separate manual acceptance

Automated tests use fixtures and parser inputs only. Do not run this checklist as
root, loosen device modes, add broad input-group membership, or change udev rules
during acceptance. Use three controller IDs and perform one check at a time:

1. Start `watch` with both VIRPIL sticks and the pedals connected. Confirm their
   controller classification and record the three stable IDs,
   disconnect and reconnect each controller, and confirm inventory changes appear
   without event data. A reconnected unit should recover its prior serial ID, or
   its port ID when returned to the same topology.
2. Capture all three IDs for a short duration. Confirm the pre-acquire output shows
   the exact requested set, a competing capture cannot acquire those IDs during the
   quiet ownership interval, the command exits at the deadline with an empty
   ownership set, and the same set can be captured again immediately afterward.
   Confirm independent consumers stop receiving new events from the selected
   controllers while unrelated keyboard and mouse interfaces remain usable.
3. In terminal A, capture one controller for 60 seconds. Choose a free controller
   whose ID sorts before terminal A's busy ID; acquisition sorts and deduplicates
   IDs regardless of argument order. Request both in terminal B. Terminal B must
   fail on the busy device and roll back the free device's earlier grab. Verify the
   free controller can be captured immediately in terminal C while terminal A still
   owns its controller. After terminal A exits, retry its controller successfully.
4. Capture two selected controllers, then disconnect one. The poll must fail and
   release the other selected controller. Reconnect the removed controller and
   confirm neither controller is automatically grabbed; a fresh explicit command
   should acquire them.
5. Capture one controller, then disconnect and reconnect a different, unselected
   controller. Use a separate `watch` process for live inventory changes; the capture
   process intentionally stays silent while grabbed. Confirm the selected ownership
   remains intact until the deadline.
6. During separate captures, test the duration deadline, Ctrl+C, and abrupt process
   termination. After each termination, immediately acquire the same ID from a new
   process. This checks explicit release plus kernel cleanup when no user-space
   cleanup can run.
7. Close the output consumer before acquisition and confirm no grab is attempted.
   Close it during the quiet ownership interval and confirm the deadline still
   releases the selected device; the post-release report may then fail without
   extending ownership.
8. Optionally trace only this example. Correlate `/dev/input/event*` opens with
   their file descriptors and verify metadata ioctls plus `EVIOCGRAB` acquire and
   release calls. There must be no event-stream `read` on those descriptors, no
   `/dev/uinput` open or write, and a failed multi-device attempt must issue release
   for every earlier successful grab.
9. Recheck the standalone read-only diagnostics and normal Linux application
   startup rejection after completing capture acceptance.

The existing diagnostics remain read-only and never test busy state. Normal Linux
application startup still rejects the unavailable backend, and
`inputforge-app --diagnose-linux` remains rejected. This example does not change
either behavior. Native-game/Proton output, binding translation, calibration,
packaging and permission setup remain outside this slice.

## Recorded Slice 2b acceptance

The following results were supplied by the user in the September 18, 2026 session.
They are session evidence, not results from the automated fixture suite or a new
acceptance run performed during Slice 2c implementation:

- Both VIRPIL sticks and Thrustmaster pedals were readable and exclusively captured.
- The left stick's serial identity and pedals' port identity survived reconnects.
- Competing captures failed as busy; failed multi-device acquisition released
  earlier grabs.
- Disconnecting a selected controller released surviving selected controllers;
  disconnecting an unselected controller did not interrupt capture.
- KDE's controller tester stopped updating during a confirmed left-stick grab and
  resumed after timed release. Keyboard and mouse remained usable.
- Timeout, SIGINT, SIGKILL, and closed-output checks released ownership.
- Brief permission delays after reconnect were reported and recovered automatically
  during inventory refresh; this did not automatically reacquire devices.
- Standalone diagnostics and normal Linux startup rejection were rechecked, and
  all acceptance processes were stopped.

Native Windows validation, independent pedal-movement testing, and optional syscall
tracing remain outstanding. These results do not establish Slice 2c streaming or
recovery acceptance.

Subsequent [Slice 2c live acceptance](linux-streaming.md#recorded-slice-2c-live-acceptance)
includes independent pedal movement and event-stream recovery; the results above
retain their original Slice 2b scope.
