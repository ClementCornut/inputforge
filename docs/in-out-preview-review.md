# IN/OUT preview timing review

This review changes no indicator behavior, polling cadence, gesture timing,
axis processing or 150 ms activity retention.

## Confirmed behavior

The 150 ms retention was introduced by commit `3228cf2` (May 14, 2026),
“fix(preview): make gesture output preview reliable.” A momentary gesture can
press and release within one engine tick. Its final output state is released,
so a GUI sampling that state every 16 ms would otherwise miss the action.

The core's `OutputActivityStore` retains momentary gesture activity on release.
The GUI's `apply_output_activity_latch` separately retains any observed activity
for a visibility window, without distinguishing momentary gestures from held
outputs. The renderer gives this retained activity precedence over the output
cache. Therefore a released output can still appear pressed. The latch exposes
new activity immediately; it does not impose a 150 ms delay before lighting up.
History and existing tests establish the visibility requirement, but do not
establish that two independent retention windows are necessary.

`engine/input_updates.rs` updates input cache before routing the event.
`engine/run.rs` writes output cache later, after dispatching the outputs. Those
writes use separate locks. `gui/bridge.rs` can acquire its read lock between
them. The readout also combines a GUI snapshot with a later direct state read
when constructing its expanded analysis.

The deterministic test
`engine::session_tests::preview_review_tests::review_bridge_can_observe_new_input_before_its_output_is_published`
reads through the bridge's nonblocking lock contract inside a fake output sink.
It observes `(input=true, output=false)` on press and `(input=false, output=true)`
on release. The pair agrees after each tick finishes. This proves an observable
publication window without adding delays or changing production code.

## Limits and follow-up

This does not measure the frequency or duration of that window on real hardware,
prove that it explains the user's observation, or establish game/output latency.
The GUI's normal 16 ms sampling interval and operating-system scheduling remain
additional factors. Gesture mappings can also intentionally defer dispatch
until their configured threshold; that is distinct from ordinary button routing.

A future, separately approved fix could publish coherent completed-update
readouts and distinguish current pressed state from recent gesture activity.
Before changing retention, preserve tests proving that same-tick taps remain
visible and add release-indication tests for ordinary held buttons. If press
onset still looks delayed, measure timestamps at input receipt, native output
dispatch and GUI publication on the affected setup.

The 150 ms core and GUI latches remain unchanged by this implementation.
