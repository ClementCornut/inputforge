# GUI rendering boundary audit

Date: 2026-10-12. Latest implementation update: 2026-10-18.

## Scope and conclusion

The GUI still owns substantial application behavior. The tray extraction is
complete, but the rest does not meet the rendering-only boundary.

Requirements:

- `inputforge-gui-dx` owns rendering.
- Non-rendering logic and event handling belong in `inputforge-app`.
- Keep maximum code shared between Windows and Linux.
- Native Wayland is the primary Linux target.
- Do not introduce direct X11 APIs, commands, or workarounds in project code.
  Transitive dependencies may retain X11 support.

The original audit was read-only. The status below now also records the first
implemented routing, polling, controller-workflow, advanced-axis, live-capture,
keyboard-capture, shared mapping-dispatch/history, mapping-rail, batch-mapping, mode-management,
editor-instrument, profile-workflow and live-readout extractions;
the remaining findings are still open.

## Resolved routing controls — 2026-10-12

The first part of finding **#2** is resolved:

- [Shared app routing policy](../crates/inputforge-app/src/routing/policy.rs)
  prepares action availability and pending-layout review for the tray, engine
  pill, Devices routing buttons, and editor notice banner.
- [App routing dispatcher](../crates/inputforge-app/src/routing/mod.rs)
  rechecks current state before Start/Stop/Retry/Refresh, sends engine commands,
  and owns review navigation into Devices. It releases the engine lock before
  navigation and reports failed command delivery.
- [GUI routing contract](../crates/inputforge-gui-dx/src/routing.rs) supplies
  prepared rendering data and an app callback. These controls no longer build
  routing commands or mutate navigation. The former tray-only policy and GUI
  pending-layout gate have been removed.
- Offline engines have no available routing actions. Running/Starting sessions
  can still Stop when the active profile disappears. A stale Start/Retry click
  cannot bypass a newly pending retained-layout review.
- Core remains authoritative for session execution and output-layout comparison.
  Activate and Retry continue through the same existing session-start path.

The app-owned 16 ms bridge calls the routing projection using the same locked
state as its other snapshots, then publishes the result after releasing the lock.
Unchanged routing models do not trigger redundant signal updates. No second
polling loop was introduced, and the native tray dispatch cadence is unchanged.

**Controller selection, basic virtual-layout edits, and Apply in #2 are now
app-owned:** see the controller-workflow slice below. Advanced axis settings are
also app-owned in the later axis slice. Other configuration-editing/readiness gates
outside these controls remain open.
**#1 is partially resolved:** polling, snapshot preparation, and the basic
controller workflow now belong to the app. Other GUI workflow handlers still
receive engine handles and commands while their ownership remains to be extracted.
**#8 remains open:** only routing-related review navigation moved; other event
handling and UI state transitions remain.

Behavioral routing tests now live in
[the app](../crates/inputforge-app/src/routing/tests.rs). GUI tests cover prepared
disabled actions, accessible review labeling, callback delivery, and unchanged
model publication. The user reported the routing controls working in desktop
checks and subsequently reported a successful native Apply retest; see below.

## Engine polling first slice — 2026-10-12

The polling and snapshot-preparation part of finding **#1** is resolved:

- [The app bridge](../crates/inputforge-app/src/bridge/mod.rs) owns the single
  polling task and prepares configuration, live input/output, metadata, settings,
  and routing data from one engine read guard. The immediate first sample,
  16 ms interval, nonblocking `try_read()`, and skipped contended ticks remain.
- [GUI publication](../crates/inputforge-gui-dx/src/bridge.rs) owns the signals,
  non-subscribing selection reads, and equality-gated publication. The renderer
  supplies its publication handle to an app callback once per mounted scope;
  Dioxus scope teardown cancels the app task. No extra thread, polling loop,
  timer, or snapshot channel was introduced. Tray dispatch keeps its existing
  cadence and uses the same app routing policy.
- [Prepared rendering contracts](../crates/inputforge-gui-dx/src/snapshots/mod.rs)
  hold the snapshot values. All engine projection constructors, action-tree
  traversal, device-usage preparation, and timestamp preparation moved into
  small app modules. Redundant profile-row adapters were removed.
- [The app metadata cache](../crates/inputforge-app/src/bridge/profile_metadata.rs)
  reads modification times for synthesized active-profile rows only after the
  engine guard is released. It refreshes immediately on path changes and once
  per second otherwise, including after failed reads. Library rows keep their
  core-provided metadata. Labels are formatted from cached timestamps each tick;
  file changes may take up to one second to appear. Leaving synthesized rows
  clears the cache.
- The existing app package now has a small
  [library target](../crates/inputforge-app/src/lib.rs), allowing GUI fixtures and
  the seeded demo to reuse the production projection. Test-only dependencies
  and canonical renderer-value aliases avoid duplicate projection implementations;
  production GUI dependencies still do not include the app. The demo publishes
  a prepared fixture once and owns no polling loop.

**Still open in #1:** raw engine reads and command senders used by remaining GUI
workflow handlers. Capture revalidation moved in the live-capture slice below. The later controller slice
below resolves basic selection/layout/Apply ownership in #2; the later axis slice
resolves advanced axis settings. Other configuration gates and findings #3–10
remain open. Finding #7's
polling metadata read is resolved; dialogs, clipboard, and other services remain.

Behavioral projection and capture-epoch tests moved to
[the app bridge tests](../crates/inputforge-app/src/bridge/tests/mod.rs).
New metadata regressions cover refresh, path changes, failure recovery, cache
invalidation, and engine write-lock availability during metadata reads. The
[renderer integration regression](../crates/inputforge-gui-dx/tests/editor_bridge.rs)
uses paused time to verify the immediate sample, 16 ms cadence, contention skips
and recovery, one installation across rerenders, teardown, and remount.
[GUI publication tests](../crates/inputforge-gui-dx/src/bridge_tests.rs) verify
unchanged-frame suppression and non-subscribing selection reads.

## Polling-slice verification

- `cargo test --workspace --locked --offline`: 2,059 passed, 0 failed, 11 ignored.
  The first sandbox run could not create local IPC sockets in five existing
  single-instance tests (`Operation not permitted`); the host-enabled rerun passed.
- Strict production Clippy passed for both affected crates and examples:
  `cargo clippy --locked --offline -p inputforge-app -p inputforge-gui-dx --lib --bins --examples -- -D warnings`.
- Workspace/all-target Clippy remains blocked by the existing Rust 1.99
  `clippy::assert_is_empty` baseline. It passed with only that lint excluded:
  `cargo clippy --workspace --all-targets --locked --offline -- -D warnings -A clippy::assert_is_empty`.
  Repository lint settings were not relaxed. The contention fixture deliberately
  holds a write guard through a tick and records that with a narrow test-only
  `#[expect(clippy::await_holding_lock)]`.
- Formatting, `git diff --check`, and audit local-link checks passed.
- An independent read-only review found no actionable code issues.
- Changes remain uncommitted. The user reported the polling-slice manual desktop
  checks working, with one timestamp discrepancy: 20:30 appeared instead of 22:30.
  The existing formatter displayed UTC; it now displays the system's local time
  with its UTC offset, while stored timestamps and sort order remain UTC-based.
  The regression reproduced that exact discrepancy before the fix, and passed
  under `Europe/Paris`, `America/New_York`, and `UTC`, covering winter dates,
  midnight rollover, and dropped sub-second precision. The GUI fixture also
  checks that the prepared local timestamp reaches the tooltip.
  After the correction, all 945 app/GUI library tests and strict production
  Clippy passed; formatting and diff checks also passed.
  Native acceptance of this timestamp correction and performance measurements
  remain pending.

## Controller-workflow slice — 2026-10-12

The controller selection, basic virtual-layout editing, and Apply portions of
findings **#1** and **#2** are resolved:

- [App controller policy](../crates/inputforge-app/src/controllers/policy.rs)
  prepares selection rows, requested versus fixed-driver layouts, editing locks,
  retained-output presentation, pending Apply, and action availability.
  Input readiness alone does not lock editing; Running/Starting, retained input
  capture, offline state, and a missing profile do.
- [App controller edits](../crates/inputforge-app/src/controllers/edit.rs) own
  numeric parsing, selection mutations, slot allocation/removal, button/hat/axis
  edits, and command construction from the current profile. Disconnected selected
  controllers can be deselected; disconnected unselected or unknown identities
  cannot be newly selected. Removed slots and obsolete Apply operations dispatch
  nothing. Layout edits and selection do not reconnect outputs or start routing.
- [App callbacks and dispatcher](../crates/inputforge-app/src/controllers/mod.rs)
  recheck current state on each callback, release the engine read guard before
  command delivery, and return app-prepared error feedback for the panel alert
  as well as logging validation or delivery failures. A successful subsequent
  operation clears that feedback. They do not optimistically mutate engine state; core execution owns persistence, binding
  acquisition, hardware operations, and execution-time validation.
- [Core capability validation](../crates/inputforge-core/src/output/validation.rs)
  is the existing engine validator extracted for reuse by the app and engine.
  `ControllerConfig::validate()` still owns structural validation, and
  `SessionState::requires_output_reconfiguration()` still owns exact configurable
  versus compatible fixed-layout comparison. No parallel GUI validation rules
  were introduced.
- [GUI controller contract](../crates/inputforge-gui-dx/src/controllers/mod.rs),
  [selection rendering](../crates/inputforge-gui-dx/src/frame/panel_slot/controller_session.rs),
  and [layout rendering](../crates/inputforge-gui-dx/src/frame/panel_slot/virtual_config.rs)
  retain prepared values, signals, markup, and callback wiring. These controller
  controls no longer read raw engine state, parse numeric fields, mutate controller
  configurations, or send engine commands. Repeated fields have slot-specific
  accessible names. Fixed layouts remain read-only while pending Apply stays
  separately available.

The existing app bridge prepares this model under the same engine guard as the
rest of the frame and publishes it after releasing the guard. GUI publication
uses the existing equality gate. The immediate sample, 16 ms cadence,
nonblocking contention skips, tray cadence, scope teardown, and remount behavior
remain unchanged. No extra poller, timer, thread, channel, dependency, or generic
workflow abstraction was introduced. The old GUI `update_config` helper was
removed, and the app shares one slot-range constant across projection and edits.

[App regressions](../crates/inputforge-app/tests/controllers.rs) cover state locks,
readiness, selection identity/disconnection, duplicate prevention, current-state
dispatch, slot reuse/full capacity, numeric and backend limits, axis toggles,
explicit Apply, fixed-driver presentation, and failed command delivery without
optimistic mutation. [Renderer event regressions](../crates/inputforge-gui-dx/src/frame/panel_slot/controller_events_tests.rs)
exercise the actual form/button handlers and verify app payloads, unparsed numeric
values, and no raw engine command delivery.
[Real-bindings feedback regressions](../crates/inputforge-gui-dx/tests/controller_feedback.rs)
verify visible alerts for rejected numeric edits, invalid Apply, and failed
command delivery, plus clearing on successful delivery and unchanged engine
configuration/retained outputs. This covers a review-discovered regression where
app prevalidation initially only logged errors that the engine previously made
visible. The missing-alert cases failed before the callback-feedback correction.
Existing controller rendering tests
now consume prepared values, including a regression where app availability
intentionally differs from the metadata snapshot. Publication tests cover
controller-only changes and unchanged-frame suppression. The renderer integration
fixture now opens Devices and verifies prepared native-slot updates on the existing
cadence, contention recovery, single installation, teardown, and remount.

**Still open after this controller slice:** keyboard capture, mappings/undo, device removal,
profiles/modes/settings, OS services, other configuration/readiness gates, and
other workflows in findings #1–10. Transitional raw handles remain for those
workflows. Advanced axis settings moved in the later axis slice below.
This controller slice has no new native acceptance evidence; the earlier
successful user-reported Apply timeout retest is retained below.

## Controller-slice verification

- Regression tests were written before controller implementation and observed
  failing for missing app behavior, ignored prepared availability/selection,
  and missing controller publication. The review-discovered missing alerts were
  separately reproduced through real app bindings before the feedback fix.
- `cargo test --workspace --locked --offline`: **2,072 passed, 0 failed,
  11 ignored**, including the existing explicit-Apply, invalid/running Apply,
  slow-destruction neutralization, and renderer polling/lifecycle regressions.
  The final run includes visible validation and failed-delivery feedback.
- Strict production Clippy passed:
  `cargo clippy --locked --offline -p inputforge-app -p inputforge-gui-dx -p inputforge-core --lib --bins --examples -- -D warnings`.
- Strict workspace/all-target Clippy remains blocked by the existing
  `clippy::assert_is_empty` baseline. The all-target check passed with only that
  lint excluded:
  `cargo clippy --workspace --all-targets --locked --offline -- -D warnings -A clippy::assert_is_empty`.
  New controller/test diagnostics were fixed; repository lint settings remain
  unchanged and unrelated baseline tests were not edited.
- Formatting, `git diff --check`, module-size checks, and audit local-link checks
  passed. The complete prior Apply timeout follow-up and native retest block was
  verified byte-for-byte against the pre-extraction audit.
- One independent read-only review found the visible-feedback regression above;
  it was corrected with failing-then-passing renderer regressions and the final
  green workspace run. It reported no other actionable issues.
- Changes are unstaged and uncommitted. Native Wayland/Windows acceptance of
  this controller slice and runtime performance measurements remain pending.

## Controller-slice manual acceptance

Use a disposable profile and run `dx run -p inputforge-app` on native Wayland;
repeat the applicable checks on Windows for separate platform acceptance.

1. In Devices while stopped, select/deselect connected controllers. Disconnect a
   selected controller and confirm it can be deselected; a disconnected unselected
   controller cannot be selected. Inspecting a device must not select it. Selection
   changes must preserve virtual layouts and mappings without reconnecting outputs.
2. On a configurable backend, add/remove slots and edit buttons, hats, and axes
   within the displayed driver limits. Confirm the first free slot is reused and
   other slots/selection are preserved. Invalid numeric/count values must not be
   persisted or applied and must show an explanatory alert; restore a valid value,
   confirm the alert clears, and reopen the profile to verify.
3. Start routing: selection and virtual-layout edits must lock. Stop: inputs remain
   visible and retained outputs remain connected and neutral. Change a layout:
   pending Apply appears and Start remains unavailable until review is completed.
4. With at least three virtual controllers, Apply changed layouts repeatedly.
   Confirm all outputs reconnect successfully, no slot-3 `packet write deadline`
   appears, pending review clears, and routing remains stopped until explicitly
   started. Unchanged layouts must not request another Apply.
5. On a fixed-driver backend, confirm native layouts and limits are displayed,
   editing/add/remove controls are unavailable, and extra compatible native slots
   do not create false pending review. A saved-profile layout change that requires
   reconfiguration must still allow Apply while capability fields stay read-only.
6. Switch panels/profiles, disconnect/reconnect inputs, and repeat Stop/edit/Apply/
   Start. Quit and relaunch: updates continue and no duplicate controller resources
   remain. This is manual lifecycle acceptance; automated fixtures do not prove
   native kernel timing, Windows behavior, compositor compatibility, or performance.

## Advanced-axis slice — 2026-10-12

The advanced axis settings portions of findings **#1** and **#2** are resolved:

- [App axis preparation and resolution](../crates/inputforge-app/src/controllers/axes.rs)
  select live session bindings first and saved profile bindings only for devices
  absent from the live table. Rows resolve persisted detection/override choices
  by native axis code, independently of logical display indices.
- The existing [controller contract](../crates/inputforge-gui-dx/src/controllers/mod.rs)
  now carries axis rows and raw select-value/redetection intents. Each intent
  includes device identity, logical index, and native code. The existing app
  dispatcher rechecks the current table and refuses obsolete identities, removed
  slots, offline operations, or unavailable redetection before delivering commands.
- Global overrides remain available online regardless of Running/Starting,
  profile presence, input readiness, retained capture, or virtual-layout ownership.
  Redetection additionally requires a monitored device and a resolving axis.
  These rules intentionally differ from the basic controller-editing lock.
- App preparation and dispatch reuse `DeviceBinding::validate()`; redetection
  reuses `DeviceBinding::resolves()`. Malformed tables produce no rendered rows,
  and attempted operations against them return core validation feedback.
  The core axis engine and adapters retain sampling, detection heuristics,
  automatic/manual interpretation, persistence/rollback, and cached-value updates.
- [Axis rendering](../crates/inputforge-gui-dx/src/frame/panel_slot/axis_settings.rs)
  retains shared controls, display labels/formatting, and callback wiring. It
  no longer selects binding tables, resolves settings, decides availability,
  interprets field events, or sends engine commands. The sole-consumer raw
  `ConfigSnapshot.axis_settings` map and its per-tick copy were removed.
- Axis operations reuse the existing visible controller alert and app callback
  feedback, including failed command delivery and clearing on successful delivery.
  Dispatch releases the engine guard before delivery and never optimistically
  changes persisted settings. Backend persistence failures retain core warnings.

Axis rows travel in the existing coherent controller projection and equality-gated
publication. No extra poller, timer, thread, channel, dependency, host API, or generic
workflow framework was introduced. Immediate sampling, the 16 ms cadence,
nonblocking contention skips, tray cadence, scope teardown, and remount are intact.
The existing no-profile editor shell still hides Devices; app policy and isolated
axis rendering support no-profile data without changing that shell workflow.

[App regressions](../crates/inputforge-app/tests/axes.rs) cover sparse native codes,
live precedence/saved fallback, separate devices sharing native codes, logical
reordering, stale callbacks, missing devices/indices, automatic/manual choices,
readiness/monitoring/offline/resolution gates, core rejection of malformed tables,
failed delivery, and absence of optimistic settings mutation.
[Renderer regressions](../crates/inputforge-gui-dx/src/frame/panel_slot/controller_session_tests.rs)
retain prepared values and shared controls and make prepared availability disagree
with metadata. [Actual event regressions](../crates/inputforge-gui-dx/src/frame/panel_slot/controller_events_tests.rs)
verify native identity and raw field values reach app callbacks with no raw engine
dispatch. [Real-bindings feedback](../crates/inputforge-gui-dx/tests/controller_feedback.rs)
covers invalid values, failed delivery, clearing, and unchanged settings.
[Publication](../crates/inputforge-gui-dx/src/bridge_tests.rs) covers axis-only updates
and unchanged rows; [the polling fixture](../crates/inputforge-gui-dx/tests/editor_bridge.rs)
now verifies axis detection updates on the existing cadence, contention recovery,
single installation, teardown, and remount.

**Still open:** capture, mappings/undo, device removal, profiles/modes/settings,
OS services, other configuration/readiness gates, and other workflows in findings
#1–10. Transitional raw handles remain for those workflows. This extraction has
no new native acceptance evidence; the previous Apply timeout/native retest block
is preserved below without changes.

## Advanced-axis verification

- Regression tests preceded the extraction. The new app contract first failed
  to compile for missing axis rows/actions; after the minimal contract was added,
  five app cases failed for missing projection, dispatch, and validation behavior.
  GUI regressions separately failed for ignored prepared availability and missing
  prepared controls/events/feedback. These cases now pass through real app bindings.
- `cargo test --workspace --locked --offline`: **2,082 passed, 0 failed,
  11 ignored**. The sandbox run could not create local IPC sockets in the five
  existing single-instance tests; the host-enabled rerun passed. This includes
  core detection/override/Auto/redetection persistence, save rollback, cached
  polarity metadata updates, controller/Apply, and slow-destruction regressions.
- Strict production Clippy passed:
  `cargo clippy --locked --offline -p inputforge-app -p inputforge-gui-dx -p inputforge-core --lib --bins --examples -- -D warnings`.
- Strict workspace/all-target Clippy remains blocked by the existing
  `clippy::assert_is_empty` baseline. The diagnostic check passed with only that
  lint excluded:
  `cargo clippy --workspace --all-targets --locked --offline -- -D warnings -A clippy::assert_is_empty`.
  New test diagnostics were fixed, the larger polling fixture setup was separated
  from lifecycle assertions, and the controller resolver now routes global actions
  before profile edits. Repository lint settings and unrelated tests are unchanged.
- Formatting, `git diff --check`, touched-module size checks, and audit local-link
  checks passed. The complete Apply timeout follow-up/native retest block was
  verified byte-for-byte against the pre-extraction audit.
- One independent read-only review found no actionable extraction regressions.
  It confirmed the existing no-profile shell limitation described above.
- Changes remain unstaged and uncommitted. Native Wayland/Windows acceptance and
  runtime performance measurements for this axis slice remain pending.

## Advanced-axis manual acceptance

Run `dx run -p inputforge-app` with a disposable profile on native Wayland.
Repeat applicable steps on Windows for separate platform acceptance.

1. Open Devices → Advanced axis settings. Verify device aliases/names, axis labels,
   saved choices, and detected/not-detected text. With a sparse native-axis device,
   confirm changing one row affects that physical axis and no other device/axis.
2. Leave controls at rest, choose Auto, then Detect again. Verify the resting
   classification updates. Cycle Centered → Rests at minimum → Auto and confirm
   live bars update even without movement. Redetection must preserve an explicit
   override; Auto must restore the detected interpretation.
3. Start/Stop routing and repeat axis choices/redetection. Global axis controls
   remain usable while their engine is online even when layout/selection controls
   lock. An unmonitored or unavailable axis cannot redetect. A saved disconnected
   axis retains its override editor but cannot redetect; reconnect and verify.
4. Switch profiles, close/reopen Devices, and disconnect/reconnect the device.
   Verify settings remain global, identities stay correct, and updates continue.
   Quit/relaunch and confirm persisted choices/detection and continued updates.
5. Check existing Stop/edit-layout/Apply/Start behavior, including multiple virtual
   controllers, still works. Preserve the previous native Apply retest as separate
   historical evidence; this new extraction still requires its own acceptance.

## Live-capture slice — 2026-10-13

The live-input portion of finding **#3** and its engine access in **#1** are
resolved. Keyboard capture, Add/duplicate collision decisions, mapping mutations,
command construction and undo in **#4**, and other workflows are deferred.

- [App capture controller](../crates/inputforge-app/src/live_capture/mod.rs) owns
  exclusive capture and replacement, detector state, Escape interception, native
  binding confirmation, cancellation and one-time delivery. A consumer's stale
  cancel/drop cannot cancel another owner. Rearming the same owner keeps its
  workflow open and starts a fresh baseline. Listener cleanup dispatches directly
  during teardown and wakes the waiting receive task; unique listener keys and
  capture serials prevent old Escape events from cancelling a newer request.
- [GUI capture contract](../crates/inputforge-gui-dx/src/capture.rs) and
  [hook wiring](../crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs) retain
  prepared targets, root/per-consumer listening signals, prompts, markup, and
  supplied callbacks. The four surfaces are Add, primary rebind, MergeAxis
  secondary input, and predicate input. Their existing completion handlers,
  explicit Add commit, no-op behavior, dispatch-before-undo and failure handling
  remain in the GUI pending their own extraction.
- [The existing app poller](../crates/inputforge-app/src/bridge/mod.rs) samples
  capture on every successful tick under its coherent engine read guard, then
  publishes and delivers after releasing the guard. Immediate sampling, the
  16 ms interval, nonblocking contention skips/recovery, root-scope cancellation,
  and tray cadence remain. Poller timestamps use Tokio's monotonic clock, matching
  its interval and allowing deterministic paused-time debounce tests. No second
  poller, timer, thread, dependency, or snapshot channel was introduced.
- The detector retains native cached addresses, first-tick baseline, the 0.15 axis
  deadband, 50 ms debounce, deterministic ties, and axis/button/hat filters.
  Unchanged held inputs still finish their debounce on polling ticks. Unready
  sessions wait and rebaseline; Running does not prohibit capture; offline
  sessions cancel. Generation and native-code layout changes rebaseline pending
  detection and reject candidates already awaiting delivery.
- [Current-state validation](../crates/inputforge-app/src/live_capture/policy.rs)
  checks stable profile identity, the live editing mode and selection, mapping
  existence, stored optional name and root actions, and rendered field identity.
  It repeats these checks plus readiness, epoch/native layout and captured-address
  presence immediately before delivery. Navigation changes are caught before
  consumer rerender. Engine guards are released before confirmation, callbacks
  or signal publication. Sparse native controls retain their original address.
- Mapping consumers retain same-address native confirmation before their no-op
  edit handlers. Failed confirmation cancels without completion. Add retains its
  existing explicit commit policy. Invalid MergeAxis paths are rejected from the
  prepared target before confirmation, preserving the previous rejection order.
  The new target-name lookup uses both mode and input and preserves `None`;
  display fallback names and another mode's name cannot invalidate capture.
  Existing mapping-save name policies are deferred with mapping edits.

[App detector regressions](../crates/inputforge-app/src/live_capture/tests/detection.rs)
move with the behavior. [Controller regressions](../crates/inputforge-app/tests/live_capture.rs)
cover ownership/rearm, delayed delivery, reentrant completion, profile/mode/name/
action/field invalidation, pre-rerender navigation, epoch/layout/readiness changes,
cache eviction, Running/offline rules and confirmation failure.
[Real renderer event regressions](../crates/inputforge-gui-dx/src/frame/mapping_editor/capture_events_tests/mod.rs)
click all four surfaces and preserve native addresses, existing commands/undo,
explicit Add commit, cancel controls, unnamed/cross-mode target names and invalid
stage rejection. The name and invalid-path cases failed before their corrections.
[Paused polling/lifecycle regressions](../crates/inputforge-gui-dx/tests/live_capture_polling.rs)
cover unchanged-input debounce, cadence, contention, exactly-once delivery,
stale/fresh Escape, root cleanup and remount with the real app bindings.

## Live-capture verification

- `cargo test --workspace --locked --offline`: 2,091 passed, 0 failed, 11 ignored.
  Five existing single-instance IPC tests could not create loopback sockets in
  the sandbox (`Operation not permitted`); the host-permission rerun passed.
- Focused checks passed: 68 app library tests, 10 app capture tests, 877 GUI
  library tests (including four real capture-event regressions), two capture
  polling/lifecycle tests, and the six existing bridge/controller-feedback tests.
- Strict production Clippy passed:
  `cargo clippy --locked --offline -p inputforge-app -p inputforge-gui-dx --lib --bins --examples -- -D warnings`.
  Strict workspace/all-target Clippy still encounters the existing Rust 1.99
  `clippy::assert_is_empty` baseline. The diagnostic check passed with only that
  lint excluded:
  `cargo clippy --workspace --all-targets --locked --offline -- -D warnings -A clippy::assert_is_empty`.
  New fixture warnings were corrected; repository lint configuration is unchanged.
- Formatting, `git diff --check`, new-module size checks, and local audit links
  passed. The full Apply timeout/native retest section below was checked
  byte-for-byte against the pre-extraction audit.
- Independent read-only review identified the unnamed/cross-mode target-name
  regressions and invalid-stage confirmation ordering; reproducing regressions
  passed after correction, and the follow-up review found no remaining issues.
- A Node self-check of the extracted production listener script verified
  capture-phase Escape, synchronous removal, shutdown acknowledgement and
  isolated listener keys. The Dioxus fake-document fixtures verify dispatched
  cleanup and receive-task release, not native WebView execution.
- Changes remain unstaged and uncommitted. Native Wayland/Windows hardware,
  WebView lifecycle acceptance and performance measurements remain pending.

## Live-capture manual acceptance

Run `dx run -p inputforge-app` with a disposable profile on native Wayland.
Repeat applicable checks on Windows for separate platform acceptance.

1. Add a mapping by pressing a button or moving an axis/hat. Confirm the prompt
   ends once, the source identifies the correct physical control, and the mapping
   is saved only by the existing explicit commit. Repeat with sparse native codes
   and multiple devices; no ordinal/device substitution may occur.
2. Rebind a named and an unnamed primary mapping, a MergeAxis secondary source,
   and a predicate source. Verify existing edit/undo behavior, axis-only filtering
   for MergeAxis, and that another mode using the same primary input does not
   cancel capture. Capture the same current source to verify its existing no-op
   behavior. For an ambiguous retained native control, verify deliberate rebind
   confirms only that control.
3. Arm one surface then another, rearm Add, press Escape, and use the Cancel
   buttons. Only the current owner may consume input; cancellation must produce
   no mapping edit/undo and must restore normal Escape behavior afterward.
4. While listening or during debounce, change editing mode/selection/profile,
   rename/edit/remove the target, disconnect/reconnect or refresh the input
   layout, and stop/start routing. No stale result may edit a different target.
   Unready capture waits for a fresh baseline; Running remains capturable.
5. Hold an input after its initial movement and verify completion still occurs.
   Close/reopen the editor and quit/relaunch while listening; verify no lingering
   Escape interception, double delivery or duplicate polling after remount.
6. Check the existing Stop/edit-layout/Apply/Start flow still works. The prior
   successful native Apply retest below remains historical evidence for that
   timeout fix; it is not acceptance of this capture extraction.

## Live-capture acceptance update — 2026-10-13

The user reported live capture working. This is user-reported native acceptance
of the live-input slice above, recorded separately from the previous successful
native Apply timeout retest below. It is not an independent hardware trace,
complete Windows/Wayland acceptance matrix, or performance measurement. It does
not establish native acceptance of the keyboard-capture extraction below.

## Keyboard-capture slice — 2026-10-13

Keyboard capture and translation in finding **#3** are now app-owned. Mapping
mutations, command construction, undo, and other editor workflows in **#4** remain
deferred.

- [App controller](../crates/inputforge-app/src/keyboard_capture/mod.rs) owns
  editor-scoped exclusive capture, replacement/rearm, cancellation and hint
  ownership. Stale consumer cancel/drop cannot cancel another owner. A request
  serial rejects events queued before replacement; pending commits ignore repeat
  events. One passive root listener is installed before editor shortcuts and
  reused across arms and rerenders.
- [Physical translation](../crates/inputforge-app/src/keyboard_capture/translation.rs)
  and [capture machine](../crates/inputforge-app/src/keyboard_capture/machine.rs)
  retain physical `code` identity rather than layout-dependent characters, numpad
  distinctions, left/right modifiers, unidentified Meta/Super location, AltGr's
  synthetic Ctrl suppression, fallback flags, modifier-only release, multi-modifier
  rejection and the swallowed-base-keydown fallback. Unsupported keys and Escape
  cancel without committing.
- [App listener](../crates/inputforge-app/src/keyboard_capture/listener.rs) and
  [production script](../crates/inputforge-app/assets/keyboard_capture_listener.js)
  own capture-phase interception, arming/disarming, synchronous root cleanup and
  receive-task release. While armed, default/propagation prevention includes
  immediate propagation so later same-window editor shortcut listeners cannot
  interpret captured keys. Unique listener keys isolate root replacement; an old
  shutdown cannot dispose the new listener. Idle listeners do not intercept keys.
- [Target policy](../crates/inputforge-app/src/keyboard_capture/policy.rs) checks
  stable profile identity, editing mode and selection, the mapping's stored
  optional name and root actions, and the exact typed edit/insertion path. The
  app validates on start, reactive target/navigation changes and immediately
  before delivery. Removed mappings, invalid branches, shifted stages and stale
  targets receive no completion. Another mode's name and display fallback names
  cannot invalidate capture. Keyboard capture adds no controller readiness,
  native-layout, Running, or hardware-confirmation gate.
- [App delivery](../crates/inputforge-app/src/keyboard_capture/delivery.rs) keeps
  the surface logically active through pending completion and until its existing
  commit handler returns. It releases engine and controller guards before the
  callback; completion may rearm the same or another owner. Cancel/replacement
  discards pending completion, and signal publication tolerates disposed consumers.
- [GUI contract](../crates/inputforge-gui-dx/src/keyboard_capture.rs) and
  [hook wiring](../crates/inputforge-gui-dx/src/patterns/keyboard_capture/mod.rs)
  retain prepared targets, display/pending signals, navigation subscriptions and
  supplied callbacks. The hooks forward target/navigation changes to app
  validation rather than owning cancellation policy. The existing
  [Add stage](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs)
  and [keyboard field](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs)
  retain action construction, dispatch and undo handlers. The keyboard field still
  publishes its local combo only after successful dispatch, ignores unchanged stale
  props, and accepts later external edits/undo. Existing mapping-save name lookup
  policy remains deferred with mapping edits.

The shared global owner counter, delivery sequence/wrapper, GUI state machine,
GUI browser translation and listener flags were removed. Add's separate armed
profile-name/path snapshot and comparison helper were replaced by the app's
stable target validation. These simplifications are covered by owner/rearm,
pending display, target invalidation and real-handler regressions. No new poller,
timer, thread, generic workflow framework or third-party dependency was added;
Serde and test JSON reuse existing workspace dependencies. The existing poller's
immediate sample, 16 ms cadence, contention behavior and teardown are unchanged.

[Moved behavioral tests](../crates/inputforge-app/src/keyboard_capture/tests.rs)
cover translation and modifiers. [App controller regressions](../crates/inputforge-app/tests/keyboard_capture.rs)
cover ownership, repeats, pending delivery/cancel, current-state rejection,
pre-rerender navigation, invalid destinations, reentrancy, exclusive hints and
root lifecycle. [Renderer frame regressions](../crates/inputforge-gui-dx/src/patterns/keyboard_capture/display_tests.rs)
cover pending-display ordering and cancellation on target/navigation changes
before any further key. [Actual control regressions](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/keyboard_events_tests.rs)
click both surfaces through real app bindings, verifying existing commands,
name preservation, dispatch-before-undo, no optimistic engine mutation, stale
props and failed-dispatch display. The previous unit coverage for later prop
changes/undo and failed dispatch remains in the keyboard field.

## Keyboard-capture verification

- Final host-enabled `cargo test --workspace --locked --offline -j 2`:
  **2,099 passed, 0 failed, 11 ignored**. The first sandbox run failed in the five
  existing single-instance IPC fixtures because loopback socket creation is denied
  there; the host rerun passed. Two build jobs bounded linker memory during the
  final run. No ignored native/hardware fixture was enabled.
- This includes **88 app library tests, 12 keyboard-controller tests and 853 GUI
  library tests**, plus existing controller/Apply, live-capture, feedback and
  paused polling/lifecycle coverage. Renderer tests exercise both actual keyboard
  surfaces, successful/failed dispatch and capture display frames.
- Strict production Clippy passed for both affected crates and examples:
  `cargo clippy --locked --offline -p inputforge-app -p inputforge-gui-dx --lib --bins --examples -- -D warnings`.
  Strict workspace/all-target Clippy still fails exclusively on the existing
  Rust 1.99 `clippy::assert_is_empty` baseline. Its diagnostic run passed with only
  that lint excluded:
  `cargo clippy --workspace --all-targets --locked --offline -- -D warnings -A clippy::assert_is_empty`.
  New fixture/style diagnostics were corrected; repository lint settings and
  unrelated baseline tests were not changed.
- Formatting, `git diff --check`, local audit links and app-module size checks
  passed. The complete Apply timeout/native-retest block was checked byte-for-byte
  against the pre-extraction audit. `use_combo_sync`, `commit_capture_combo`,
  `dispatch_keyboard` and Add's existing insertion/undo handler were unchanged at
  extraction completion. The label follow-up below fills the keyboard handler's
  presentation values without moving mapping/undo ownership.
- The executable production listener self-check passed:
  `node crates/inputforge-app/tests/keyboard_capture/listener.mjs crates/inputforge-app/assets/keyboard_capture_listener.js`.
  It verifies passive idle behavior, physical payloads, capture-phase/default/
  same-window shortcut interception, arming/disarming, isolated synchronous cleanup
  and shutdown acknowledgement. Fake Dioxus document tests verify dispatched
  lifecycle operations and receive-task release; they do not execute a native WebView.
- Independent read-only review found target-invalidation and exclusive-hint parity
  gaps. Both were reproduced before correction, and the follow-up found no
  remaining actionable issues.
- Changes were left unstaged and uncommitted at extraction completion. Subsequent
  user-reported app acceptance is recorded below; platform-specific checklist
  coverage and performance measurements remain unverified.

The controller tests first failed for the missing app contract and listening
behavior. The executable listener check reproduced missing synchronous cleanup; the
root-reuse, reinstall, target-invalidation and exclusive-hint cases
were separately reproduced before their corrections; independent review found
the latter two parity gaps and its follow-up found no remaining actionable issues.
The old source-text listener assertion and sequence-counter assertion were
replaced by executable listener and end-to-end delivery/lifecycle checks.

## Keyboard-capture manual acceptance

Run `dx run -p inputforge-app` with a disposable profile on native Wayland.
Repeat applicable checks on Windows for separate platform acceptance.

1. Capture a keyboard stage through Add, then recapture an existing keyboard
   field. Try a different physical layout (including AZERTY), main/numpad keys,
   right/left modifiers, AltGr and a modifier-only release. Verify physical
   identity, the displayed combo and saved output behavior; multi-modifier-only
   or unsupported input must cancel with the existing hint.
2. Hold/repeat a key or modifier and try Ctrl+S, Ctrl+Z, Alt+arrow and another
   available WebView shortcut while armed. Capture must finish once with one
   mapping edit/undo, and no unrelated shortcut action. After completion/cancel,
   normal shortcuts must work again. OS-reserved keys may never reach the WebView.
3. Arm one surface, another surface and the same surface again. Try Escape and
   the Add prompt's cancel control. Only the current owner may receive input;
   cancellation must produce no edit/undo. An unsupported-key hint from the first
   owner must disappear when another owner starts.
4. While listening, switch editing mode/selection/profile, change keyboard output
   behavior, or edit/remove/reorder the destination. Capture must disarm when the
   prepared target/navigation changes; no delayed result may edit another target.
   Verify a different mode using the same source does not invalidate a valid target.
5. Recapture F11 as F12 and watch the transition: the old key must not briefly
   return. Undo/redo must synchronize afterward. Verify rejected/failed delivery
   retains the saved key and produces no phantom undo. Close/reopen and quit/relaunch
   while armed; no lingering interception or duplicate listener may remain.
6. Repeat live input capture and Stop/edit-layout/Apply/Start. The earlier native
   Apply retest and separately reported live-capture success remain historical
   evidence for those slices; this extraction needs its own native acceptance.

## Keyboard undo-label follow-up — 2026-10-14

The user reported `Map to keyboard: key ? -> ?`. The existing GUI keyboard
handler supplied the stage and field names to the undo formatter but omitted
`before_after`, so the formatter used its `?` fallback. The extraction had kept
that handler unchanged; this is a separate presentation fix.

[The shared keyboard dispatcher](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs)
now resolves the saved keyboard action at the exact stage path and supplies old
and new values. Key changes reuse the field's combo formatter, including modifier
labels; behavior changes display Hold/Pulse values. Command construction and
successful-dispatch-before-undo ordering remain unchanged. Existing history
labels are not rewritten; subsequent edits create the corrected labels. Mapping
and undo workflow extraction remains deferred.

[Actual-control regressions](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/keyboard_events_tests.rs)
first reproduced `key ? -> ?` and `behavior ? -> ?`, then passed with
`Map to keyboard: key F11 -> F12` and
`Map to keyboard: behavior Hold -> Pulse`. The control selector was narrowed to
text within the exact button so Pulse tests cannot accidentally click a sibling.
Existing failed-dispatch, stale-props and undo-order coverage remains in place.

- `cargo test --workspace --locked --offline -j 2` with host IPC permissions:
  **2,100 passed, 0 failed, 11 ignored**. The 22 focused keyboard-related GUI tests
  include the reproducing label cases and both successful/failed delivery paths.
- Strict app/GUI production Clippy and examples passed. GUI all-target Clippy
  passed with only the existing `clippy::assert_is_empty` baseline excluded;
  repository lint configuration remains unchanged.
- Formatting, `git diff --check`, local audit links and byte-for-byte preservation
  of the previous Apply timeout/native-retest block passed.
- Changes were left unstaged and uncommitted at follow-up completion; subsequent
  user-reported app acceptance is recorded below.
For native acceptance, capture a new key combination and change Hold/Pulse;
check the new undo hints and confirm undo/redo still restores the expected mapping.
Earlier Apply and live-capture acceptance evidence remains separate and unchanged.

## User-reported app acceptance — 2026-10-14

After the keyboard-capture extraction and undo-label follow-up, the user reported:
“i tested the app. It works”. This records successful user-reported app acceptance
of the current changes. The report does not specify individual checklist cases,
platforms or explicit undo-label observations; those details and performance
measurements remain unverified. This is separate from the earlier native Apply
retest and live-capture success, whose evidence remains unchanged.

## Pre-commit Clippy follow-up — 2026-10-14

The user reported commit-time Clippy failures. The unchanged pre-commit command,
`cargo clippy --all-targets -- -D warnings`, reproduced 136
`clippy::assert_is_empty` diagnostics with Rust 1.99. Earlier all-target checks
excluded that baseline lint and therefore did not establish a passing commit gate.

The 136 flagged test assertions across 49 Rust files now compare collection/string
lengths with zero using `assert_eq!` or `assert_ne!`. The expected empty/nonempty
conditions remain the same, with actual lengths shown on failure. Only those
assertions and their formatting changed; production behavior, the hook and lint
configuration remain unchanged.

- `bash .githooks/pre-commit` passed formatting and strict all-target Clippy with
  no lint exclusions.
- `cargo test --workspace --locked --offline -j 2` with host IPC permissions:
  **2,100 passed, 0 failed, 11 ignored**.
- The previous native Apply evidence and separate user-reported acceptance remain
  unchanged. These follow-up changes are uncommitted; the user's staged hook
  permission changes were preserved.

## Shared mapping dispatch and history slice — 2026-10-14

This approved extraction partially resolves findings **#1**, **#4** and **#8**.
The dated capture sections above record ownership at their implementation time;
this section supersedes their deferral of shared mapping dispatch and undo. Batch
mapping, profile/mode/snapshot workflows and curve/deadzone algorithms remain open.

- [The app controller](../crates/inputforge-app/src/mapping_edits/mod.rs) owns
  editor-scoped history and lifecycle, current profile/mode/selection validation,
  dispatch, undo/redo, invalidation and the inclusive 250 ms nudge window. Grouping
  includes mapping, stage, arrow direction and focused handle; timeout, focus or
  navigation changes, intervening edits and restores end a group.
- [Preparation](../crates/inputforge-app/src/mapping_edits/prepare.rs) preserves
  exact optional names, uses core's existing action-tree validator, and rejects
  stale or malformed targets before delivery. Edits and restores reconcile private
  history under their validation read guard, so a polling contention skip cannot
  allow stale history to resurrect an external deletion. [Path edits](../crates/inputforge-app/src/mapping_edits/paths.rs)
  handle root-relative Conditional, Tap and Press branches. Queued edits use the
  latest delivered tree; known older renderer snapshots can support disjoint
  field edits across engine acknowledgements, but structural/restore boundaries
  block stale positional edits and parent replacement cannot erase newer children.
- [History](../crates/inputforge-app/src/mapping_edits/history/mod.rs) keeps restore
  snapshots private, with the existing 50-entry FIFO limit per mapping. Fresh
  edits clear redo. Undo restores the before snapshot and redo the delivered final
  snapshot, including a grouped burst. Empty mappings remain present with their
  optional names. Rebind preserves its existing forward SetMapping behavior;
  undo restores or removes the destination according to its original existence.
  Destination edits invalidate dependent history, and source deletion cannot
  restore stale rebind history. Stable profile identity, mappings and modes
  isolate histories; external replacements/deletions invalidate affected stacks.
- [Dispatch](../crates/inputforge-app/src/mapping_edits/dispatch.rs) releases
  engine/controller guards before command delivery and signal publication. History
  changes only after successful edit/restore delivery. A small
  [core queue barrier](../crates/inputforge-core/src/engine/command.rs) records an
  owner/serial marker in AppState after preceding commands. This distinguishes
  unfinished A→B→A command chains from confirmed round trips and external changes,
  including intermediate engine snapshots; equality alone cannot acknowledge them.
  It introduces no routing decision, dependency, thread or polling task.
- [The GUI contract](../crates/inputforge-gui-dx/src/mapping_edits.rs) contains
  prepared paths/intents, display labels, read-only history summaries, signals and
  callbacks. The old GUI mutable history and nudge timer were removed. Field
  action construction, labels, SVG/geometry, local capture display and callback
  wiring remain in the GUI. Keyboard capture still publishes the local combo
  only after successful dispatch, resists unchanged stale props and accepts
  external edits/undo. Gesture threshold/toggle local drafts remain intact.
- The existing 16 ms app bridge calls history reconciliation after publication;
  reconciliation skips contended state reads. GUI navigation changes forward the
  same callback. Teardown disposes history; remount starts empty while owner/serial
  identity prevents old queue acknowledgements from confirming new commands.
  Engine handles at the renderer entry now use the existing RawHandles bundle;
  they remain transitional for deferred workflows.

```mermaid
flowchart LR
    GUI["GUI: prepared targets, labels, signals"] -->|"edit / undo / redo callbacks"| App["App: validation, dispatch, private history"]
    App -->|"mapping command + serial barrier"| Core["Core: queued execution"]
    Core -->|"state and acknowledged serial"| App
    App -->|"prepared history labels"| GUI
```

All history callers were included:

| Callers | Result |
| --- | --- |
| Stage fields: ChangeMode, keyboard/mouse/vJoy, MergeAxis, predicate, Tap/Press | Shared app replacement dispatch; labels and typed capture targets remain GUI-prepared. |
| Add palette, stage menu remove/move, pipeline drop | App insert/remove/move dispatch; GUI expansion/hint invalidation and gap/drag wiring retained. |
| Curve/deadzone toolbars, pointer-release commits and keyboard nudges | Shared dispatch/history; one drag commit, nudge group metadata and focus callbacks retained. Algorithms deferred. |
| Editor header rename and primary rebind | App dispatch and complete destination restoration. |
| Editor keyboard undo/redo and history recap | App restore callbacks and prepared label summaries. |
| Rail inline rename | App obtains current actions and validates captured optional-name metadata even after config advances; no GUI engine-state read. |
| Rail add/duplicate/replace, duplicate to another mode and delete | Existing deferred forward workflow retained; successful delivery invokes app forget for the affected mapping, while failed delivery leaves history intact. |
| External profile/mode/mapping changes | App reconciliation detects stable profile identity changes and missing/replaced mappings, invalidating related history rather than resurrecting stale state. |
| Root unmount/remount and existing bridge publication | App disposal/reconciliation with GUI signal ownership; no new poller. |

Regression coverage was written before the extraction implementation. Additional
review regressions reproduced stale structural targets, core-rejected trees,
round-trip acknowledgement ambiguity, overlapping nested edits, write-contention
revalidation, delivery-before-
render timing and rebind invalidation before their controller fixes. Tree mutation
coverage moved to app-backed tests; real GUI keyboard/capture handlers, labels,
DnD callback delivery, stale rail metadata and root lifecycle are also covered.

## Mapping-slice verification

- `bash .githooks/pre-commit` passed unchanged: formatting and
  `cargo clippy --all-targets -- -D warnings`, with no lint exclusions.
- `cargo test --workspace --locked --offline -j 2` with host IPC permissions:
  **2,107 passed, 0 failed, 11 ignored**. The initial sandbox run could not create
  sockets for five existing single-instance tests; the host-enabled suite passed.
- The suite includes **43 app mapping regressions**, **817 GUI library tests**,
  app-backed renderer integration/polling/lifecycle tests and the core command-
  barrier regression. The added contention test covers both dispatch and undo.
- `git diff --check`, audit local-link checks and unchanged acceptance-section
  checks passed. All new app modules remain below 200 nonblank code lines.
  Independent read-only review found no further issues after regression fixes.
- The hook, lint configuration and dependencies remain unchanged. All changes
  remain uncommitted and unstaged.

The user subsequently reported this slice seems to work fine; see the acceptance
update below. Individual native checklist cases, platforms and performance remain
unverified. Earlier native Apply, live-capture and user-reported app acceptance
sections remain unchanged. No commit or staging is part of this work.

## Mapping-slice manual acceptance

Run `dx run -p inputforge-app` on native Wayland; repeat on Windows when available.

1. Edit named and unnamed mappings, including the same input in two modes. Change
   keyboard/mouse/vJoy targets, ChangeMode, MergeAxis and predicates. Confirm names,
   undo labels and input identities; switching mode/profile must isolate histories.
2. Capture a keyboard combo in the field and Add palette. It must update immediately,
   remain visible through unchanged stale props, and dispatch once. Check unsupported
   keys/Escape, modifier-only combos, typing in text fields, and Ctrl+Z/Y interception
   while listening. Undo/redo and external changes must restore the displayed combo.
3. Add, remove and reorder stages in every Conditional/Tap/Press branch, including
   cross-branch drops. Confirm expansion survives in unaffected branches, invalid or
   source-adjacent drops do nothing, and deleting the last stage keeps its named row.
   Quickly edit sibling stages and gesture threshold then toggle; no queued edit
   may disappear. Undo/redo repeatedly and confirm each complete before/after tree.
4. For both curve and deadzone plots, drag and release once, then undo/redo once.
   A same-arrow/same-handle burst within 250 ms must undo to its initial value and
   redo to its final value. Wait longer, change arrow/handle/stage, Tab/Home/End,
   focus elsewhere, navigate away/back or perform another edit; each must split
   the group. Existing plot geometry and calculations should behave identically.
5. Rename in the editor and rail, rebind a primary input to unused and existing
   targets, then undo/redo. Destination mappings must restore correctly. Use rail
   add/duplicate/replace, duplicate-to-mode and delete after making editor history;
   old history must not overwrite replacement work or resurrect a deleted source.
   Change/delete targets during capture and verify stale completion is rejected.
6. Close/reopen the editor and quit/relaunch. History must be fresh, listeners and
   polling must install once, and no stale undo/redo may affect a new session.
   Recheck Stop/edit-layout/Apply/Start; the previous successful native Apply retest
   is historical evidence for its timeout fix, independent of this slice.

## Mapping-slice acceptance update — 2026-10-14

After the shared mapping dispatch/history extraction, the user reported:
“seem to work fine”. This records user-reported acceptance of this slice. The
report does not specify individual manual checklist cases, platforms or timing
measurements; those remain unverified. This evidence is separate from the prior
native Apply retest, live-capture success and earlier user-reported app acceptance,
whose recorded evidence remains unchanged.

## Mapping-rail workflow slice — 2026-10-14

This approved bounded extraction partially resolves findings **#1**, **#3**, **#4**
and **#8**. It supersedes the earlier sections' deferral of rail Add/duplicate,
collision decisions, duplicate-to-mode, delete and row reorder. Batch mapping,
profile/mode workflows and curve/deadzone algorithms remain deferred.

- [The existing app mapping controller](../crates/inputforge-app/src/mapping_edits/mod.rs)
  supplies rail callbacks and retains editor-scoped history, navigation publication,
  reconciliation and disposal. No separate controller, poller, timer or dependency
  was introduced. Disposed rail callbacks reject saved targets; reinstall starts
  fresh while the existing queue owner/serial identity remains intact.
- [Target/menu preparation](../crates/inputforge-app/src/mapping_edits/rail/mod.rs)
  checks stable profile identity, editing mode, offline state, source existence and
  captured metadata. It prepares full mapping targets, other modes and reorder
  availability from current app state. Known queued edits supply current actions;
  missing or externally replaced sources cannot silently become empty duplicates.
- [Add preparation](../crates/inputforge-app/src/mapping_edits/rail/add.rs) owns
  opening, capture completion, refresh, cancellation, collision reconciliation,
  explicit commit and collision navigation. Blank names remain `None`; nonblank
  names are trimmed. Add capture still requires an explicit button/Enter commit.
  Both collision surfaces open the existing destination rather than overwriting it.
  Commit rechecks collisions, so a newly occupied destination cannot be silently
  replaced. Removed duplicate sources close the pad and release capture.
- [Dispatch](../crates/inputforge-app/src/mapping_edits/rail/dispatch.rs) releases
  engine/controller guards before delivery and publication. Successful mapping
  delivery plus the existing queue barrier precede affected-history invalidation,
  selection changes and closing the pad/dialog/menu. Failed delivery retains the
  draft or confirmation surface and publishes visible error feedback. Action-tree
  validation remains core-owned and is reused before dispatch.
- [Forward history invalidation](../crates/inputforge-app/src/mapping_edits/history/rail.rs)
  clears affected/dependent restore stacks while preserving the already delivered
  command chain. This retains acknowledgement of delete→re-add round trips.
  Unrelated mappings and modes keep their history. Delete still uses forward
  removal and history invalidation; it does not introduce a new delete-undo stack.
  The confirmation text no longer incorrectly promises undo for that operation.
- [Reorder preparation](../crates/inputforge-app/src/mapping_edits/rail/reorder.rs)
  validates source identity, mode, group, captured order and filtering. It handles
  Up/Down and pre-removal gaps; boundaries and source-adjacent gaps are no-ops.
  Outstanding reorders and queued membership changes gate further moves until
  the existing owner/serial barrier acknowledges them, preventing old positions
  from being reused. Reorder preserves edit history and ends an active nudge group.
- [The GUI contract](../crates/inputforge-gui-dx/src/mapping_rail.rs),
  [Add pad](../crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs),
  [rail handlers](../crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs) and
  [row wiring](../crates/inputforge-gui-dx/src/frame/mapping_list/row.rs) retain
  prepared data, signals, labels, markup and supplied callbacks. The rail no
  longer reads raw engine state or constructs/sends mapping commands. Drag start
  captures the app-prepared source and group order; a later rerender cannot
  substitute another row at the same index. Accessibility success announcements
  are formatted only from successful app reorder results.
- Existing rail keyboard and idle-Escape transports remain renderer wiring.
  They now remove listeners synchronously and send shutdown on scope teardown;
  controller/scope keys isolate mounts. The capture primitives, native target
  confirmation, keyboard pending display, stage dispatch, nudge grouping and
  core persistence/execution behavior remain unchanged. The existing bridge still
  samples immediately and every 16 ms, skips contended reads and owns one task.

All affected callers are included:

| Callers | Result |
| --- | --- |
| Add dashed row, forced open, pending Duplicate hook/effect; three rail mount branches | App prepares opening/duplicate state; GUI retains copy-name display and capture wiring. |
| Capture completion, Refresh, Cancel, idle Escape and collision-drift effect | App transitions/revalidation; existing capture owner cancellation is retained. |
| Add button and Enter; collision Edit existing | Shared app commit/navigation callbacks; failed dispatch retains the draft. |
| Context menu Duplicate to mode | Current source actions and destination collision decision in app; source mode history retained. |
| Delete confirmation | Current-target validation, forward removal and affected-history invalidation after delivery. |
| Alt+Up/Down, Move up/Move down menu items and sortable gap drop | Shared app reorder policy and successful-delivery results. Drag identity is captured at start. |
| Existing bridge/history reconciliation, navigation and teardown/remount | Same controller callbacks and polling cadence; rail guards and pending reorder acknowledgements share lifecycle. |

The desktop host, bridge demo, controller-feedback fixture, editor-bridge fixture
and live-capture polling fixture still call the same `render_editor` signature
with `Controller::bindings()`. No core command, dependency, hook or lint-setting
change is part of this slice.

## Mapping-rail verification

- `bash .githooks/pre-commit` passed unchanged: formatting and
  `cargo clippy --all-targets -- -D warnings`, with no lint exclusions.
- `cargo test --workspace --locked --offline -j 2` passed: **2,135 passed,
  0 failed, 11 existing ignored**, including 58 app mapping tests and 830 GUI
  library tests. The same command was rerun with host permissions after the
  sandbox denied loopback sockets in five existing single-instance IPC tests;
  all five passed on that rerun. Ignored native/platform checks were not enabled.
- Both executable listener checks passed:
  `node crates/inputforge-gui-dx/tests/mapping_rail_listener.mjs` and
  `node crates/inputforge-app/tests/keyboard_capture/listener.mjs crates/inputforge-app/assets/keyboard_capture_listener.js`.
- Audit local links and byte-for-byte preservation of the four prior acceptance
  sections (Apply timeout/native retest, live capture, keyboard/app acceptance
  and shared mapping-slice acceptance) passed. App mapping modules remain within
  200 nonblank code lines. The history file became `history/mod.rs` for the
  repository's required module-folder layout; its existing history behavior is
  retained and rail-specific pending-chain operations live in `history/rail.rs`.
- Independent read-only review identified the delete/re-add queue variant above;
  its reproducing regression passes after preserving the pending chain.
  Follow-up review found no actionable regressions in the final changes.
  Changes remain unstaged and uncommitted.

Regression tests preceded implementation of the app workflows. Actual Add-button
regressions reproduced missing queue acknowledgement and failed-dispatch pad closure.
Additional regressions reproduced queued reorder/deletion positions, delete→re-add
membership ambiguity, and disposed rail targets before their corrections. App tests
cover explicit commit, optional names, latest queued actions, source disappearance,
late/drifting collisions, stale profile/mode/source targets, delivery failure,
history isolation and group-local movement. Real renderer handlers cover Add/Enter,
collision navigation, menu duplicate/reorder, delete confirmation, keyboard delivery,
stale drag identity, pre-rerender duplicate-mode/filter changes, visible feedback
and listener teardown/remount. The old
source-text collision assertion is replaced by a rendered collision regression.

Earlier native Apply retest, live-capture success, keyboard/app acceptance and
mapping-slice acceptance blocks remain unchanged. This slice has no new native
acceptance evidence; automated fixtures do not establish native Wayland/Windows
WebView behavior, focus, hardware delivery or performance.

## Mapping-rail manual acceptance

Use a disposable profile and run `dx run -p inputforge-app` on native Wayland.
Repeat applicable checks on Windows for separate platform acceptance.

1. Add a named and unnamed button/axis/hat mapping. Capture must finish once, keep
   native identity and wait for Add/Enter. Refresh and Cancel/Escape must work;
   typing a name must not be consumed by rail shortcuts.
2. Duplicate a named and unnamed mapping to a fresh input, including nested stages.
   Check the copy name and complete action tree. Capture an occupied input and use
   Edit existing: the destination must open without being overwritten. Remove or
   change the source while the pad is open; no empty/stale duplicate may be saved.
3. Duplicate to another mode with and without an existing destination. Check exact
   optional names/actions, collision navigation and isolation of both mode histories.
   Delete after editing: confirmation is required, the row disappears after delivery,
   and unrelated mappings remain undoable. Old history must not resurrect the row.
4. Reorder axes, buttons and hats using menu Up/Down, Alt+Up/Down and drag gaps.
   Check boundaries, source-adjacent gaps and text/device filters. Change the order
   or mode during a drag; no different row may move. Rapid moves and delete/re-add
   must not reuse old positions; success announcements must match delivered moves.
5. Repeat live primary/MergeAxis/predicate capture and keyboard capture/undo. Check
   immediate combo display, unchanged stale props, labels and cancellation. Switch
   profile/mode and close/reopen while capturing; no stale delivery, lingering Escape
   interception, duplicate listener or old history may affect the new surface.
6. Quit/relaunch and repeat Stop/edit-layout/Apply/Start. The earlier successful
   native Apply retest remains separate historical evidence for its timeout fix.

## Native drag transport follow-up — 2026-10-14

This records the initial HTML5 transport correction. The approved pointer-gesture
follow-up below supersedes that gesture and expands the browser regression check.

The user reported a cancel cursor, missing drop hints and no working drop on Linux.
The [shared drag handle](../crates/inputforge-gui-dx/src/components/sortable/handle.rs)
initialized data through Dioxus's desktop `SerializedDataTransfer`; its setters
are no-ops in the installed 0.7.10 dependency. The browser therefore received no
drag payload, matching the [WebKitGTK empty-drag failure](https://bugs.webkit.org/show_bug.cgi?id=265857).

The handle now supplies a nonempty `text/plain` marker and `effectAllowed=move`
directly in its synchronous browser `ondragstart` attribute. Existing Rust handlers
still capture identity, produce gap hints, cancel, validate and dispatch through
app callbacks. No document listener, timer, dependency or native-platform workaround
was added. Both rail rows and editor pipeline stages use this shared primitive;
their reorder policies, boundaries and history ownership are unchanged.

The browser initializer regression reproduced the absent native payload, and the
renderer regression reproduced the absent synchronous attribute before the fix.
The executable [browser self-check](../crates/inputforge-gui-dx/tests/sortable_drag.mjs)
and [renderer drag regressions](../crates/inputforge-gui-dx/src/frame/mapping_list/workflow_events_tests/reorder.rs)
now pass, including hover hints, cancellation without delivery, successful drop
and rejection of order drift. The unchanged strict pre-commit hook passed.
`cargo test --workspace --locked --offline -j 2` passed with host permissions:
**2,137 passed, 0 failed, 11 existing ignored**. All three executable listener/drag
checks passed. Read-only review found no actionable issues; the browser attribute
and Rust event listener coexist, and both shared consumers retain their validation.
Changes remain unstaged and uncommitted.

Native pointer-gesture acceptance remains pending. With `dx run -p inputforge-app`
on native Wayland, drag the grip across nonadjacent gaps in one mapping group:
the valid gap should show a move cursor and cyan hint, and releasing should reorder
the captured row. Try the leading/trailing gaps, cross-group rejection, adjacent
no-op gaps, Escape/off-target cancellation and another drag afterwards. Repeat
within an editor pipeline and check undo. Repeat on Windows for separate platform
acceptance. Prior native acceptance sections remain unchanged; this report is
evidence of the defect, not a successful retest.

## Pointer drag targets and scrolling — 2026-10-14

The approved enhancement replaces native HTML5 dragging with a pointer gesture on
the existing grip, so normal wheel/trackpad scrolling remains available. The
[browser transport](../crates/inputforge-gui-dx/assets/components/sortable_drag.js)
forwards start, hover, leave, drop and end events to the existing Rust handlers.
For each insertion gap, the hit region spans the lower half of the previous card,
the entire gap and the upper half of the next card. Measurements are synchronous
and refreshed during scrolling; no invisible overlay intercepts ordinary controls.
The visible insertion hint stays on the gap. Nested pipeline groups and empty
branches retain their own destinations.

Near the containing scroll pane's top/bottom edge, a drag-only animation frame
ramps scrolling with proximity to the edge. It changes only that pane, clamps
elapsed time/scroll limits and updates the destination under a stationary pointer.
Drop, Escape, pointer cancellation/capture loss, blur, source removal and scope
teardown release capture, frame and gesture listeners. The existing immediate-first
sample/16 ms app bridge is unchanged; the browser frame does no engine polling.

[Mounted owner/handle cancellation](../crates/inputforge-gui-dx/src/components/sortable/interaction.rs)
uses the signals' generational identities to isolate remounts. The
[handle](../crates/inputforge-gui-dx/src/components/sortable/handle.rs) also clears
its active Rust drag state when removed while its list remains mounted. Prepared
context markers cancel a reused source when profile/mode/mapping changes; pipeline
markers additionally include the complete prepared action tree. They prevent a
reused index-based stage node from moving replacement content. No editor remount
is introduced on ordinary edits, preserving keyboard/capture display behavior.
Release clicks are suppressed after a drag; ordinary grip clicks remain available.

Both callers are wired: [mapping groups/rows](../crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs)
and [pipeline groups/stages](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs).
Their app-owned dispatch/history and target/order validation remain unchanged.
Rail/editor keyboard transports defer during a pointer gesture and retain their
ordinary behavior afterwards. No dependency, core algorithm, profile workflow,
batch workflow, hook or lint-setting change is part of this extension.

Regression tests preceded implementation. The missing pointer initializer and
DOM boundaries failed first; release-click/keyboard interaction and the two
reviewed source-removal/reuse cases reproduced failures before their fixes.
The executable [pointer checks](../crates/inputforge-gui-dx/tests/sortable_drag.mjs)
exercise unequal card halves, midpoint boundaries, wheel/edge scrolling, stationary
target refresh, cancellation/removal, disabled/foreign sources, release clicks,
nested branches and changing prepared context. Real renderer tests cover emission,
owned teardown/remount, source removal and unchanged app delivery/order rejection.
The existing four-stage summary test now inspects visible text rather than hidden
identity attributes. `bash .githooks/pre-commit` passed unchanged with no lint
exclusions. `cargo test --workspace --locked --offline -j 2` passed with host
permissions: **2,140 passed, 0 failed, 11 existing ignored**, including 835 GUI
library tests. All **11 executable pointer cases**, the rail/editor listener
check and the unchanged keyboard-capture listener check passed. Follow-up read-only
review found no remaining actionable findings after the source-removal/reuse fixes.
Audit links and byte-for-byte preservation of the four prior native acceptance
sections passed. Changes remain unstaged and uncommitted.

Native acceptance is pending. On native Wayland with `dx run -p inputforge-app`,
use a disposable profile long enough to overflow the rail, then repeat in a long
editor pipeline. Repeat on Windows for separate platform acceptance:

1. Drag the grip across either neighboring card half and the gap; the same insertion
   line must remain selected. Check different card heights and leading/trailing gaps.
2. Hold the grip while scrolling with the wheel/trackpad. Keep the pointer stationary
   as rows move underneath it, then release; the currently indicated slot must receive
   the captured item. Normal fields/buttons remain usable outside a gesture.
3. Hold near the top/bottom edge; the containing pane should scroll, accelerate nearer
   the edge and stop when the pointer leaves the edge band or the gesture ends.
4. Cancel with Escape, release outside a destination and close/reopen while dragging.
   Check no scrolling, lingering dimmed row/hint, accidental release click or duplicate
   handler remains, then start another drag and verify keyboard reorder still works.
5. Change/remove the source or switch profile/mode/selected mapping during a gesture.
   It must cancel rather than move replacement content. Check cross-group rejection,
   nested/empty branches, pipeline undo and isolation of other mappings' history.

The four prior native acceptance sections remain unchanged. Automated browser
geometry/renderer fixtures do not establish native pointer, wheel or focus behavior.

## Card ghost follow-up — 2026-10-14

The user described the pointer interaction as "almost perfect" and reported the
missing card ghost. This is user-reported feedback before ghost restoration;
it does not establish separate platform or per-case acceptance.

The existing [pointer transport](../crates/inputforge-gui-dx/assets/components/sortable_drag.js)
now snapshots the complete source card when the drag threshold is crossed, retaining
its measured size, pointer grab offset, inherited typography/color and canvas drawings.
The viewport-fixed translucent copy follows pointer movement while wheel/edge scrolling
moves only the underlying list. It is inert and hidden from accessibility, with pointer
events disabled throughout the clone. DOM/renderer/stage IDs, form bindings, inline
handlers and sortable metadata are stripped; the copy cannot register as another
interactive card or drop target. The shared finish path removes it before drop/end
delivery, including cancellation, context changes, source removal and unmount.

Only the browser presentation script, shared sortable CSS, browser fixtures/tests
and this audit changed in this follow-up. Reorder policy, target validation, command
delivery/history, capture, polling and lifecycle ownership remain unchanged.
Three new preview regressions failed before implementation; pointer-following,
non-interference and canvas snapshots now pass alongside the existing cleanup cases.
Validation: all 14 executable pointer regressions and the rail/editor listener and
keyboard-capture listener checks passed. The unchanged pre-commit hook passed with
no lint exclusions. `cargo test --workspace --locked --offline -j 2` passed on the
host: 2,140 passed, zero failed and 11 existing ignored tests. Read-only review found
no actionable issues. Changes remain unstaged and uncommitted.

Manual retest: drag a mapping card and an expanded pipeline stage with a graph;
confirm the complete translucent card follows the grip at the original grab point,
retains its content, leaves drop hints/wheel/edge scrolling functional and disappears
on drop, Escape, source/context changes and close/reopen. Native ghost appearance
remains pending; the earlier acceptance sections are preserved unchanged.

## Batch mapping slice — 2026-10-14

This approved bounded extraction resolves finding **#5** and its workflow engine
access in **#1**. Earlier dated sections describe ownership at their implementation
time. Profile/settings workflows and curve/deadzone algorithms remain deferred.

- [The existing app mapping controller](../crates/inputforge-app/src/mapping_edits/mod.rs)
  supplies batch callbacks and owns the mounted wizard alongside private history.
  [Batch lifecycle and validation](../crates/inputforge-app/src/mapping_edits/batch/mod.rs)
  use stable profile identity, editing navigation, selected mode/all-mode scope,
  source capabilities/native bindings and target capabilities. Mounted-owner and
  context-generation tokens reject obsolete callbacks, including before rerender;
  old disposal cannot close a newer mount. Profile/navigation changes reset drafts.
- [Row preparation](../crates/inputforge-app/src/mapping_edits/batch/rows.rs)
  owns automatic selection and positional target defaults, source/target fallback,
  capability validation and row reconciliation. Live native bindings take precedence
  over saved bindings; core binding validation and unavailable-control resolution
  preserve logical identities for sparse controls and retained tombstones. Overflow
  stays excluded. Valid row overrides and replace flags survive unrelated capacity
  changes; invalid choices are rebuilt from current capabilities. A stale Apply
  rejects rather than silently dispatching to a fallback source or target.
- [Policy](../crates/inputforge-app/src/mapping_edits/batch/policy.rs) prepares
  current collisions, mode choices, target choices, summary counts, contextual group
  controls, Reset and Apply availability. Collision checks use the existing private
  history's latest delivered mapping view, including queued additions/deletions.
  Create/replace/skip fan out per selected mode; excluded counts once per input row.
- [App transitions](../crates/inputforge-app/src/mapping_edits/batch/edit.rs) own
  raw form parsing, source/target/mode choices, All modes, row targets, replace,
  Include/Exclude, Skip/Replace conflicts and Reset. Malformed, wrong-kind and
  out-of-capability values show visible feedback without changing the draft.
- [Batch dispatch](../crates/inputforge-app/src/mapping_edits/batch/dispatch.rs)
  prepares entries and the alias-aware recovery label under the validation guard,
  then releases engine/controller guards before command delivery, publication or
  completion. Empty batches request no snapshot. Successful batch plus queue-barrier
  delivery precede affected/dependent history invalidation and navigation; unrelated
  mappings/modes retain history. The existing pending command chain survives
  delete→batch re-add and prevents stale membership or history restoration.
- [The GUI contract](../crates/inputforge-gui-dx/src/batch_mapping.rs) contains
  prepared rendering data, signal publication and callbacks. [The panel](../crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs),
  [metadata controls](../crates/inputforge-gui-dx/src/frame/bulk_map/metadata.rs),
  [rows](../crates/inputforge-gui-dx/src/frame/bulk_map/rows.rs) and
  [labels](../crates/inputforge-gui-dx/src/frame/bulk_map/labels.rs) retain markup,
  labels, signals, live readouts and callback wiring. The wizard no longer reads
  raw engine state, maintains reconciliation/collision policy or sends commands.
  The mirrored test-only Apply handler and redundant GUI policy modules were removed.

Failed command or barrier delivery leaves the wizard and choices visible, publishes
an alert, and invokes no completion/success/navigation callback or batch history
invalidation. Delivery feedback now says **Requested N mappings**, rather than
claiming mappings have been created. Core still owns pre-save → recovery snapshot →
bulk upserts → post-save and its warnings/abort behavior; queue delivery alone does
not acknowledge successful snapshot creation or persistence. Existing replacements
remain unnamed single MapToVJoy mappings, and recovery uses the existing snapshots.

```mermaid
flowchart LR
    GUI["GUI: prepared wizard, signals, labels"] -->|"captured owner/context + intent"| App["App: revalidation, wizard, collisions, entries"]
    App -->|"batch + existing serial barrier"| Core["Core: recovery snapshot, update, save"]
    App -->|"prepared model / delivery feedback"| GUI
```

All affected callers are wired: workspace/panel mount and empty states; metadata
selectors and All modes; axis/button/hat targets and Replace; all four group controls;
Reset and Apply; existing mapping reconciliation/disposal; batch/layout fixtures and
the real editor bridge. Desktop host, demo and other renderer fixtures retain the
existing render_editor signature and Controller::bindings(). No dependency, hook,
lint setting, core command, capture listener or drag implementation changed.

The same app poller still samples immediately and every 16 ms, skips contended
reads and publishes after releasing its guard. Batch reconciliation uses the existing
mapping callback; there is no additional polling loop, timer, thread or channel.
Capture fixes, drag scrolling/ghost cleanup, nudge grouping, history limits and
root teardown/remount remain intact.

### Batch verification

Real renderer regressions first reproduced failed-send navigation/success and
pre-rerender disconnected-source dispatch. App regressions were written before
implementation. An additional failing regression reproduced loss of a valid axis
choice/replace flag when button capacity shrank; reconciliation now preserves it.

[App workflow regressions](../crates/inputforge-app/tests/mapping_edits/batch/mod.rs)
cover defaults/overflow, invalid choices, summary/collisions, profile/source/target/
mode/native-layout drift, sparse controls, failed delivery, empty batches, queued
membership, affected history and mount ownership. [Real renderer handlers](../crates/inputforge-gui-dx/src/frame/bulk_map/events_tests/mod.rs)
cover metadata/row/group controls, Reset, feedback, snapshot payload and navigation.
[The real editor bridge](../crates/inputforge-gui-dx/tests/editor_bridge/batch.rs)
checks cadence, contention recovery, one installation and state release on teardown;
existing remount/capture/drag/history fixtures remain in the workspace suite.

Final verification:

- `bash .githooks/pre-commit` passed unchanged: formatting and strict all-target
  Clippy, with no lint exclusions.
- Host-enabled `cargo test --workspace --locked --offline -j 2` passed:
  **2,122 passed, 0 failed, 11 existing ignored**, across 25 reported suites.
  This includes 79 app mapping tests (21 batch cases), 795 GUI library tests
  (35 batch rendering/event cases), both editor-bridge tests, existing capture,
  history/drag/lifecycle checks and core bulk snapshot/save-failure coverage.
  The first workspace run identified a layout fixture missing the newly required
  toast context; its app callback/presentation contexts now match the runtime.
- All 14 executable pointer/ghost regressions and both rail/editor and keyboard
  listener checks passed. Native/hardware fixtures remain ignored.
- Diff integrity, audit local links, and touched production-module sizes passed;
  new batch modules and the mapping controller stay within 200 nonblank code lines.
  Seven prior acceptance/drag evidence sections were verified byte-for-byte unchanged.
- Independent read-only review found no actionable issues. The subsequent valid-row
  preservation correction was verified by its failing-then-passing regression and
  the final green workspace run. No second review was performed.
- Changes remain unstaged and uncommitted. No new native acceptance or runtime
  performance measurement is claimed. Prior Apply timeout, live capture,
  keyboard/app acceptance, mapping acceptance and drag evidence remain separate.

### Batch manual acceptance

Use a disposable profile and run `dx run -p inputforge-app` on native Wayland.
Repeat applicable checks on Windows for separate platform acceptance.

1. Open Batch map, choose different sources/targets and override axes/buttons/hats.
   Check positional defaults, sparse physical identities, target labels, overflow,
   Do not map, Include/Exclude and Reset; valid overrides must survive an unrelated
   capacity change. Invalid/unavailable choices must retain the draft and show an alert.
2. Create named and unnamed collisions in two modes. Exercise row Replace and group
   Skip/Replace, then All modes. Check counts and that skipped/excluded mappings remain
   unchanged; only chosen replacements lose their old pipeline/name as before.
3. Apply once and confirm a Before batch map recovery snapshot with the source alias
   and target device. Verify persisted results and restore the snapshot. The requested
   feedback is delivery feedback; backend snapshot/save failures must still warn.
4. Change profile/mode, disconnect the source, remove/change target capacity, and close/
   reopen the wizard. No stale click may act on another context. On rejected/failed
   delivery, the wizard must stay open without a success announcement/navigation.
5. Make editor history before a batch, including another mode and inputs that will be
   skipped. After Apply, unrelated/skipped history must remain usable and old affected
   history must not overwrite batch work. Repeat live/keyboard capture, undo/redo,
   pointer drag with ghost/wheel/edge scrolling, and Stop/edit-layout/Apply/Start.
6. Quit/relaunch and repeat. Check no duplicate updates or stale callback/listener
   behavior. Earlier successful native Apply/capture reports remain separate evidence;
   native WebView/hardware timing and performance are not established by these fixtures.

## Mode-management slice — 2026-10-16

This approved bounded extraction resolves the mode-management portion of finding
**#6**, its engine access in **#1**, and its selection/focus decisions in **#8**.
Profile, snapshot and settings workflows remain deferred. Earlier dated sections
record ownership and acceptance at their implementation time.

- [App mode workflows](../crates/inputforge-app/src/mapping_edits/modes/mod.rs)
  reuse the existing editor controller, navigation publication, queue serials and
  disposal. [Policy](../crates/inputforge-app/src/mapping_edits/modes/policy.rs)
  prepares row availability, the runtime marker and direct-mapping counts under
  the coherent engine frame guard. [Edit preparation](../crates/inputforge-app/src/mapping_edits/modes/edit.rs)
  owns add/rename Enter/blur/cancel decisions, deletion confirmation/guards/counts,
  Activate, startup mode and survivor focus. Tab selection changes editing mode;
  Activate and startup changes leave editing navigation alone.
- [Shared core validation](../crates/inputforge-core/src/engine/mode_validation.rs)
  is extracted from existing engine handlers. The app reuses the 64-grapheme name
  validator and `Modes` collision checks, including ASCII-case duplicates and
  case-only rename. First/startup/final-mode deletion guards are shared. Core still
  owns execution-time validation, reference/runtime cascades, held-output release
  and persistence; delivery is not a persistence acknowledgement.
- [Dispatch](../crates/inputforge-app/src/mapping_edits/modes/dispatch.rs) validates
  stable profile identity, captured mode revision, live editing navigation,
  existence, offline state and confirmation counts before delivering a command.
  The transient [core mode revision](../crates/inputforge-core/src/state/mod.rs)
  advances on add/rename/delete mutations, including an in-memory add whose save
  fails. It rejects an older draft/confirmation even after a mode is recreated
  with the same name and mapping count. It changes no persisted profile format.
  No new Running/readiness restriction is added.
- Commands and the existing queue barrier are delivered before history or
  navigation changes. Failed delivery retains drafts, menu or confirmation,
  publishes visible feedback and requests no success, closure, selection or focus
  change. The mode menu gates the shared menu primitive's automatic activation
  close on the app result. Cancellation remains available after offline/profile
  drift and requests no focus in an obsolete profile.
- [Mode history integration](../crates/inputforge-app/src/mapping_edits/history/modes.rs)
  prepares cascades using the latest delivered mapping view, including queued
  additions/deletions, and reuses core reference rewriting. It invalidates affected
  and dependent restores after delivery, including historical nested ChangeMode
  references, while preserving pending command chains and unrelated histories.
  Confirmation counts include mappings queued before the next poll. A changed
  count requires reopening the confirmation rather than approving extra mappings.
- Pending mode mutations gate stage edits/restores, rail writes and batch dispatch
  until the existing barrier acknowledges execution. This prevents a late duplicate
  or edit being saved into a mode queued for rename/deletion. Older snapshots cannot
  reset successful add/rename navigation while that mutation is pending.
- [The GUI contract](../crates/inputforge-gui-dx/src/mode_management.rs) and
  [mode surfaces](../crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/mod.rs)
  retain prepared rows/targets, labels, drafts, signals, menus, dialogs, alerts and
  callback/DOM focus wiring. The duplicate GUI validator, raw mode command sends,
  profile traversal for counts and survivor calculations are removed. The mode
  portions of [navigation reconciliation](../crates/inputforge-gui-dx/src/frame/view_state.rs)
  forward app callbacks; profile initialization/reconciliation remains deferred.

All affected callers are included: Add opening/input/Enter/blur/Escape; rename
opening/input/Enter/blur/Escape/no-op; keyboard and menu Delete; Confirm/Cancel/
Escape/click-away; right-click/Shift+F10; Activate/Set as default; tab selection,
menu-close focus and missing-mode fallback; editor reconciliation/disposal; and
all renderer fixture providers. The desktop host, demo, controller-feedback,
editor-bridge and capture-polling callers retain the same `render_editor` signature
and receive mode callbacks through the existing `Controller::bindings()`.

The immediate sample, 16 ms interval, nonblocking contention skips, equality-gated
publication, tray cadence and mounted cleanup remain. No second controller,
poller, timer, listener, thread, channel, dependency or generic workflow framework
was added. Capture translation/display/cleanup, pointer ghosts/scrolling, core
curve/deadzone algorithms and OS services are unchanged.

### Mode verification

Four real-handler regressions first reproduced lost add drafts, premature rename
closure, missing delete-delivery feedback and stale dispatch into a replacement
profile. App regressions preceded the missing workflow implementation. Later
regressions reproduced old-frame selection fallback, cancellation after profile
loss, queued deletion counts and stale editing-mode authorization before fixes.
The independent read-only review found pending-writer and same-name recreation
races; their reproducing regressions failed before the shared writer gate and
mode-revision checks, then passed. No second independent review was performed.

[App mode regressions](../crates/inputforge-app/tests/mapping_edits/modes/mod.rs)
cover names/graphemes, guards/counts, context drift, failed delivery, pending
mutations, selection decisions and affected/unrelated history. [Real mode handlers](../crates/inputforge-gui-dx/src/frame/top_bar/mode_tabs/events_tests/mod.rs)
cover inline failure retention, confirmation/menu feedback and focus, stale profile
identity and successful selection through older metadata. [Core revision tests](../crates/inputforge-core/src/engine/tests/mode_revision.rs)
cover delete/recreate, case-only rename and save failure. The existing
[editor bridge fixture](../crates/inputforge-gui-dx/tests/editor_bridge.rs) now also
checks prepared mode-row updates on the unchanged cadence, contention recovery,
one installation and teardown/remount. Existing capture/drag/history suites remain.

Final verification:

- `bash .githooks/pre-commit` passed unchanged: formatting and strict
  `cargo clippy --all-targets -- -D warnings`, with no lint exclusions.
- Host-enabled `cargo test --workspace --locked --offline -j 2` passed:
  **2,121 passed, 0 failed, 11 existing ignored**, across 25 reported suites.
  This includes 96 app mapping tests, 775 GUI library tests, 1,047 core tests and
  both editor-bridge tests. The sandbox run failed only in the five existing
  single-instance IPC tests because socket creation was denied; all five passed
  in the host run. No ignored native/hardware fixture was enabled.
- All 14 executable pointer/ghost regressions, the rail/editor listener check and
  the unchanged keyboard-capture listener check passed.
- Diff integrity, local audit links and module sizes passed. Seven historical
  acceptance/drag evidence sections were checked byte-for-byte unchanged.
- The review's two findings were reproduced and fixed, then covered by the final
  green workspace run. No additional independent review was performed.
- Changes remain unstaged and uncommitted. No new native acceptance or runtime
  performance measurement is claimed.

### Mode manual acceptance

Use a disposable profile and `dx run -p inputforge-app` on native Wayland. Repeat
applicable checks on Windows for separate platform acceptance.

1. Add modes with Enter and blur; try whitespace, duplicate/case variants, emoji
   and the length boundary. Rename selected and unselected modes, including a
   case-only rename and unchanged name. Escape and empty blur must cancel; invalid
   nonempty values must retain the draft and expose an accessible error.
2. Switch editing tabs, Activate through the menu and Set as default. Editing mode,
   runtime marker and startup mode must remain distinct. Check right-click,
   Shift+F10, arrow navigation and Tab/Escape/click-away focus behavior.
3. Delete using both menu and Delete key. First/startup modes must be protected;
   counts must describe the direct mappings. Cancel/Escape/click-away returns
   appropriate focus. Confirm removes the mode, uses the adjacent survivor for
   focus and falls back to startup editing only when deleting the edited mode.
4. Change/remove/recreate a target or switch profiles while a draft/confirmation
   is open. An obsolete action must not affect the new context; cancel/reopen to
   continue. If dispatch fails, the draft/menu/confirmation must remain visible
   with feedback and without a success announcement or selection/focus jump.
5. Make history in unrelated mappings and in pipelines with nested ChangeMode
   references. Rename/delete a referenced mode; affected old undo must not restore
   stale references, while unrelated undo remains usable. Repeat quick edits and
   mode changes; pending work must reject safely rather than create orphan mappings.
6. Repeat live/keyboard capture, stale-prop display, rail/pipeline drag with ghost,
   wheel/edge scrolling, undo/redo and Stop/edit-layout/Apply/Start. Close/reopen and
   quit/relaunch while editing/capturing; no stale listener, duplicate updates or
   inherited history should remain. Native focus, hardware timing, platform behavior
   and performance remain manual acceptance; previous reports remain separate evidence.

## Remaining responsibilities to move

The following areas are ordered by migration priority. Source links identify
the current implementations; line numbers describe the audited checkout and
may change during extraction.

| # | Area and evidence | App-owned responsibility |
| --- | --- | --- |
| 1 | **Engine access — partially resolved** — [render entry point](../crates/inputforge-gui-dx/src/lib.rs); [app polling bridge](../crates/inputforge-app/src/bridge/mod.rs); [GUI publication](../crates/inputforge-gui-dx/src/bridge.rs) | Polling and snapshot preparation are app-owned. Controller selection/layout/Apply, advanced axis settings, live capture, keyboard capture, shared mapping dispatch/history, mapping rail, batch and mode workflows now use app-owned handlers. Device inspector actions are app-owned in Part 6. The GUI per-window context no longer carries production engine state or a command sender after Part 7; transitional host handles close in Part 13. |
| 2 | **Controller editing policy — partially resolved** — [app controller policy](../crates/inputforge-app/src/controllers/policy.rs); [app axis settings](../crates/inputforge-app/src/controllers/axes.rs); [axis rendering](../crates/inputforge-gui-dx/src/frame/panel_slot/axis_settings.rs) | Selection, virtual-layout mutations, Apply, shared routing decisions, and advanced axis settings are app-owned. Device inspector selection, alias drafts and guarded removal are app-owned in Part 6. Other configuration-editing/readiness gates remain; see resolved status above. |
| 3 | **Capture workflows — partially resolved** — [app live capture](../crates/inputforge-app/src/live_capture/mod.rs); [GUI capture contract](../crates/inputforge-gui-dx/src/capture.rs); [app keyboard capture](../crates/inputforge-app/src/keyboard_capture/mod.rs); [GUI keyboard contract](../crates/inputforge-gui-dx/src/keyboard_capture.rs) | Live and keyboard capture, physical keyboard translation, ownership, target revalidation, cancellation, interception and delivery are app-owned; live input additionally owns epoch/native confirmation. Shared stage edits/undo and rail Add/collision decisions are app-owned in the later mapping slices. |
| 4 | **Mapping edits and instruments — resolved** — [app controller](../crates/inputforge-app/src/mapping_edits/mod.rs); [app paths](../crates/inputforge-app/src/mapping_edits/paths.rs); [app history](../crates/inputforge-app/src/mapping_edits/history/mod.rs); [GUI callback wiring](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs) | Shared mapping stage edits, command construction, restore snapshots, undo/redo, invalidation and nudge grouping are app-owned. Rail Add/duplicate/collision/delete/reorder workflows are app-owned. Curve/deadzone mutation, draft/drag decisions, keyboard and numeric behavior are app-owned in Part 2; screen/viewbox geometry stays with rendering. Profile workflows are app-owned in Part 3; batch mapping is app-owned in the later slice. |
| 5 | **Batch mapping — resolved** — [app wizard](../crates/inputforge-app/src/mapping_edits/batch/mod.rs); [app dispatch](../crates/inputforge-app/src/mapping_edits/batch/dispatch.rs); [GUI rendering](../crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs) | Target selection/validation, collisions, reconciliation, entry generation, snapshot requests and delivery are app-owned. Core retains snapshot/persistence execution. |
| 6 | **Settings, snapshots, profiles and modes — resolved** — [app settings](../crates/inputforge-app/src/settings/mod.rs); [settings policy](../crates/inputforge-app/src/settings/policy.rs); [app profiles](../crates/inputforge-app/src/profiles/mod.rs) | Mode CRUD, startup/activation, validation, confirmation/counts, dispatch and selection/focus decisions are app-owned. Profile workflows, name/collision policy, captured identities, file dialogs, confirmation and failed-delivery retention are app-owned. Snapshot actions, current identity checks, drawer state and restore confirmation are app-owned in Part 4. Settings validation, pending-value reconciliation, prune confirmation and guarded dispatch are app-owned in Part 5. Core retains persistence, native startup services and pruning execution. |
| 7 | **Operating-system services and filesystem access** — [app file picker](../crates/inputforge-app/src/profiles/dialog.rs); [app clipboard](../crates/inputforge-app/src/controllers/inspector/clipboard.rs); [cached file metadata](../crates/inputforge-app/src/bridge/profile_metadata.rs) | Native profile dialog invocation and obsolete-result rejection are app-owned. Part 6 moves clipboard invocation, native owner lifetime and write feedback to the app, with the resolved Linux Wayland feature enabled. Polling profile metadata is now app-owned, cached, and read after releasing the engine lock. Other filesystem-service ownership remains open. |
| 8 | **Event handling and UI state transitions — partially resolved** — [app navigation](../crates/inputforge-app/src/navigation/mod.rs); [navigation rendering](../crates/inputforge-gui-dx/src/frame/view_state.rs); [sortable drop handler](../crates/inputforge-gui-dx/src/components/sortable/gap.rs) | Snapshot Ctrl+S is app-owned in Part 4; device-inspector interactions in Part 6. Part 9 moves editor surfaces/panels, stable-profile/selection reconciliation, filtering, expansion invalidation and rail/editor shortcut interpretation and listener lifecycle into the app. Part 10 moves reusable form/tab behavior and Part 11 moves shared overlay/dismissal behavior and document listener ownership into app bindings. Sortable interaction remains in Part 12. |
| 9 | **Live readout analysis — resolved in Part 7** — [app analysis](../crates/inputforge-app/src/bridge/readout/analyze.rs); [app deadlines](../crates/inputforge-app/src/bridge/readout/activity.rs); [renderer](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs) | The existing app bridge prepares typed input/output values, merge/conditional/gesture chains, predicates, exact output ownership/polarity/activity and instrument tracking values. App callbacks own expansion/reset decisions. Preview expiry advances on the same 16 ms tick, including contended engine reads; the GUI renders completed values without engine access or per-render timers. |
| 10 | **Notifications — resolved in Part 8** — [app notification owner](../crates/inputforge-app/src/notifications/mod.rs); [queue deadlines](../crates/inputforge-app/src/notifications/state.rs); [warning tail](../crates/inputforge-app/src/notifications/warnings.rs); [renderer](../crates/inputforge-gui-dx/src/toast/viewport.rs) | The app owns deduplication, capacity/order, warning delivery, expiry, independent hover/focus pauses and dismissal/Escape. The existing bridge advances deadlines even during engine read contention. GUI contracts expose prepared visible values and supplied callbacks; inline workflow errors remain independent. Existing notification interactions are preserved; dormant undo controls are not added. |

## Native Wayland clipboard — build gap resolved in Part 6

The [app manifest](../crates/inputforge-app/Cargo.toml) enables the installed
`arboard` 3.6.1 `wayland-data-control` feature on Linux. The [GUI manifest](../crates/inputforge-gui-dx/Cargo.toml)
no longer depends on `arboard`. The [app clipboard owner](../crates/inputforge-app/src/controllers/inspector/clipboard.rs)
retains its initialized native handle across copy actions and panel unmounts;
initialization/write failures reach accessible inspector feedback without retrying
the write or claiming success.

`cargo tree --locked --offline -p inputforge-app -e features -i arboard`
resolves `wayland-data-control` and `wl-clipboard-rs`. Installed source confirms
that `arboard` selects the Wayland data-control backend when available. Its
Wayland provider serves offered data through the dependency's existing background
implementation; the app adds no polling task, thread or channel. Upstream
transitive X11 support remains; the app adds no direct X11 API or fallback command.
The previously disabled Wayland feature was a build configuration gap, and this
feature resolution closes that gap. It does not establish compositor compatibility
or independence from XWayland in a live desktop session. Native acceptance remains
manual on Wayland and separately Windows.

## Shared Windows and Linux architecture

Keep controllers and workflows in common app modules used by both operating
systems. Restrict platform differences to services that actually require them.
Existing core processing and validation should remain reusable.

The GUI can retain:

- Markup and component composition.
- CSS and styling.
- SVG drawing and visual geometry.
- Display formatting.
- Rendering mechanics such as portal placement.

Components should receive rendering state and app-provided callbacks. Markup
wires those callbacks; event decisions and state transitions are implemented
in the app. The renderer should not receive raw mutable engine state or an
engine command sender.

## Recommended extraction order

1. Polling/snapshot preparation, shared routing controls, and basic controller
   selection/virtual-layout/Apply and advanced axis settings are complete. Extract
   remaining engine access alongside each deferred workflow.
2. Live/keyboard capture, shared mapping dispatch/history, mapping rail, batch
   and mode workflows are complete. Extract deferred editor algorithms and profile workflows next.
3. Extract the remaining workflows, services, event handling, and timers.

Move existing behavioral tests with the responsibilities they verify. Retain
rendering tests in the GUI. Preserve capture delivery, cancellation, undo,
confirmation, accessibility, and renderer lifecycle behavior during migration.

## Routing-controls extraction verification

- `cargo test --workspace --locked --offline`: 2,049 passed, 0 failed,
  11 ignored (existing native/hardware fixtures and doctests remain ignored).
- Formatting and `git diff --check` passed.
- Strict Clippy passed for both affected production crates and examples:
  `cargo clippy --locked --offline -p inputforge-app -p inputforge-gui-dx --lib --bins --examples -- -D warnings`.
- An independent read-only review found no actionable issues.
- Strict workspace/all-target Clippy is still blocked by existing
  `clippy::assert_is_empty` test diagnostics under Rust 1.99. The same check
  passed with only that lint excluded for the diagnostic run:
  `cargo clippy --workspace --all-targets --locked --offline -- -D warnings -A clippy::assert_is_empty`.
  No repository lint configuration was relaxed and no unrelated tests were edited.

The timeout follow-up below records the state before the controller-workflow
extraction. Its native retest evidence is preserved; current ownership is
recorded in the controller slice above.

## Apply timeout follow-up — 2026-10-12

Native checks reported working routing controls but a `packet write deadline`
for slot 3 when changing a virtual-controller layout and clicking Apply.
The shared uinput teardown interleaved neutral writes and device destruction
under one 50 ms write deadline. Slow destruction of earlier controllers could
exhaust that budget before later controllers received neutral state.

[Controller teardown](../crates/inputforge-core/src/output/uinput/lifecycle.rs)
now attempts neutralization across the complete controller set before destroying
any handle. The shared write deadline, failed-slot exclusion, owner invalidation,
and cleanup-error collection remain in place; every handle is still closed even
if neutralization fails. No new abstraction or file was needed.

A [fake-clock regression](../crates/inputforge-core/src/output/uinput/tests/failures.rs)
with three controllers and 25 ms destruction per handle reproduced the exact
slot 3 timeout before the fix. It now verifies successful release, neutral output
for every controller, and closure of all handles.

- Focused uinput tests: 45 passed, 0 failed.
- `cargo test --workspace --locked --offline`: 2,050 passed, 0 failed, 11 ignored.
- Formatting and `git diff --check` passed.
- Strict uinput production Clippy passed. Workspace/all-target Clippy passed
  with only the existing `clippy::assert_is_empty` baseline diagnostics excluded,
  as described above; repository lint settings remain unchanged.
- An independent read-only review of teardown and its callers found no
  actionable issues.

**Native retest:** the user subsequently reported a successful native Apply
retest. This records user-reported native acceptance of the timeout follow-up;
live kernel-call timing was not independently traced. This backend fix does not
resolve the remaining GUI Apply ownership in finding #2. Finding #1 is partially
resolved by the polling slice above; other workflow responsibilities remain open.

## Verification limits

This was a static architecture audit of the GUI structure and principal
workflow implementations, with targeted dependency-feature verification.
The user performed desktop routing checks after the extraction, reported the
Apply timeout described above, and subsequently reported a successful native
Apply retest. The new polling slice has automated renderer/runtime and projection
coverage, and the user reported its manual desktop checks working apart from the
UTC timestamp display corrected above. The controller-workflow extraction has
new automated coverage and awaits its manual acceptance checklist above.
The advanced-axis slice has app, renderer-event/feedback, and polling/lifecycle
coverage; its native manual acceptance remains pending. The live-capture slice
has app detector/controller, real renderer-event, and paused polling/lifecycle
coverage, and the user separately reported live capture working as recorded above.
The keyboard slice has app/controller, real-handler, renderer-frame and executable
listener coverage. After its extraction and undo-label follow-up, the user reported
the app working as recorded above; individual native checklist cases, platform
coverage and timing measurements remain unverified.
The mapping dispatch/history slice has app regression and GUI callback/lifecycle
coverage, and the user subsequently reported it seems to work fine as recorded
above. Individual native checklist cases, platforms and timing remain unverified.
The mapping rail slice has app workflow/queue regressions, real renderer handlers
and listener cleanup checks; its native manual checklist remains pending.
The batch slice adds app, real-handler and existing-poller coverage; its native
manual acceptance remains pending. The previous desktop reports are user-reported native acceptance;
the timestamp correction, separate Windows acceptance, and runtime performance
measurements remain unverified on native hardware. The recommendations are ownership
changes; they are not evidence that every listed workflow currently fails at runtime.

The mode slice adds shared core validation, app/controller and real-handler tests
and mode-row polling coverage; native manual acceptance remains pending.

## Remaining editor edit intents and inline commits — 2026-10-16

Part 1 of the [approved plan](gui-rendering-boundary-plan.md) moves the remaining
mapping field decisions into the existing app mapping controller. The prior
shared-dispatch, rail, mode and native acceptance sections remain historical
and unchanged.

- The [editor preparation](../crates/inputforge-app/src/mapping_edits/editor/mod.rs)
  resolves field, palette-add and sibling-move intents against the effective queued
  mapping, then uses the existing prepare/dispatch/history/barrier path. Individual
  fields preserve unrelated queued fields, optional names and child branches.
  Insertion rechecks current keyboard/mouse availability. Gesture thresholds
  reuse the core validator; malformed numeric/picker values
  and stale profile/stage requests deliver nothing and create no history.
- [Field reconstruction](../crates/inputforge-app/src/mapping_edits/editor/fields.rs),
  [predicate operands](../crates/inputforge-app/src/mapping_edits/editor/predicate.rs)
  and [palette defaults](../crates/inputforge-app/src/mapping_edits/editor/defaults.rs)
  own validation and replacement/insertion decisions. Selecting the current
  predicate kind preserves its operands. A queued wheel target remains Pulse
  even if a previously rendered behavior control is activated afterwards.
- The [prepared GUI contract](../crates/inputforge-gui-dx/src/mapping_edits.rs)
  carries raw field intents, palette availability, sibling boundaries and explicit
  inline completion outcomes. Rename no-ops compare with the effective queued
  name after full target validation. Empty rename and cancellation also complete
  offline; same-address rebind completes without delivery. Failed commits retain
  their draft. Successful delivery alone does not establish
  engine persistence.
- [Header](../crates/inputforge-gui-dx/src/frame/mapping_editor/header.rs),
  [rail rename](../crates/inputforge-gui-dx/src/frame/mapping_list/rename_inline.rs),
  [palette](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs),
  [stage menu](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs)
  and all ChangeMode, keyboard/mouse/vJoy, MergeAxis, predicate and Tap/Press field
  handlers forward intents and retain rendering labels/signals. Rejected field
  edits publish accessible editor feedback; rail/menu errors render an alert.
  Capture display and field signals publish committed values only after delivery.
- [Dispatch wiring](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs)
  captures mounted profile identity before delayed events. The
  [profile/mapping content scope](../crates/inputforge-gui-dx/src/frame/mapping_editor/content.rs)
  remounts when that identity changes and remains stable across ordinary edits.
  Advancing metadata before rerender cannot redirect an old field handler into
  an identical replacement profile. Header rename also retains captured optional
  name metadata rather than treating the display fallback as a stored name.
  Both header and rail retain the full mapping target alongside the draft text;
  a same-key, same-name action replacement followed by a props rerender rejects
  the old draft with feedback. Rail reuses the existing app-prepared target.
  If initial target preparation is unavailable, the captured target remains
  absent for that draft. The inline contract carries that optional target plus
  the raw edit/cancel intent; the app rejects nonempty commits with feedback
  while still allowing empty/cancel dismissal offline. No mapping is fabricated.
- The existing anchored-menu activation gate retains a failed stage operation.
  The only shared menu extension is an optional bool-returning
  [activation callback](../crates/inputforge-gui-dx/src/components/menu/mod.rs) for
  palette items; false leaves the menu open. Existing onclick consumers keep
  their behavior, and disabled items invoke neither callback nor close.

Regression evidence: the real rail commit handler first failed by closing after
rejection; actual palette activation first failed by closing after delivery
failure; an old rendered keyboard behavior handler first edited an identical
replacement profile. App tests first reproduced resetting same-kind predicate
operands and turning a queued wheel target into Hold. These now have behavioral
regressions in [app editor tests](../crates/inputforge-app/tests/mapping_edits/editor.rs),
[rail rename tests](../crates/inputforge-gui-dx/src/frame/mapping_list/rename_tests.rs)
and [rendered editor tests](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/keyboard_events_tests.rs).
Review round 1 first reproduced Old→New→Old being dropped and Old→New→New
being rejected before acknowledgement through real inline bindings. Both now use
the effective queued name, producing two commands/history entries and one
command/history entry respectively. Real header/rail handlers first applied
retained text to replacement actions after publication and rerender; they now
reject safely. The [header draft tests](../crates/inputforge-gui-dx/src/frame/mapping_editor/capture_events_tests/rename.rs)
and rail fixture use updating production-like props. Review round 2 also first
reproduced an unavailable initial rail capture accepting a later empty replacement.
The real handler now rejects it after availability returns and props rerender;
there is no command/history and the retained text has an alert. Existing capture rearm,
stale-prop/undo synchronization, history, lifecycle and pointer-drag coverage
remains in the full gates.

Final automated gates: the unchanged `bash .githooks/pre-commit` passed with no
lint exclusions. `cargo test --workspace --locked --offline -j 2` passed with host
IPC access: **2,127 passed, 0 failed, 11 existing ignored**, including **108 app
mapping regressions** and **769 GUI library tests**. The sandbox attempt failed
five existing single-instance IPC tests; the identical host-enabled run passed
all of them. The keyboard-capture listener, rail/editor listener and all **14
pointer/ghost executable cases** passed. `git diff --check`, all audit links and
byte-for-byte preservation of the seven historical acceptance/drag sections
passed. New production modules are at most 138 nonblank/noncomment code lines;
existing large GUI rendering modules mostly shrink. The checkout remains on
`main`. Part 1 is complete and independent review approved both rename findings,
including unavailable-target handling. Later plan parts remain pending.

Native Wayland and Windows acceptance remains manual; no new hardware, visual,
WebView lifecycle or performance claim follows from the automated checks.
Use `dx run -p inputforge-app` with a disposable profile:

1. Rename named, empty-name and unnamed mappings from both header and rail.
   Check unchanged/empty Enter or blur, Escape, exact name preservation during
   other edits and undo/redo. Queue New then Old/New before acknowledgement;
   completion must follow the effective name. Replace a same-key mapping while
   a rename draft is open; after rerender, rejection must retain text and feedback.
2. Add processing/output/control stages into outer, nested and empty branches;
   move/delete through the stage menu and undo once. Failed delivery must retain
   the menu and expansion state without a success announcement or navigation.
3. Change mode strategy/target, mouse/keyboard behavior, vJoy output, MergeAxis
   and condition operands. Queue disjoint changes quickly and verify neither
   disappears. Switch a mouse target to wheel; it must remain Pulse.
4. Edit Tap/Press thresholds and toggles rapidly, then undo/redo. Capture a new
   keyboard shortcut, rearm immediately, and verify stale props do not restore
   the old display while later external edits/undo still synchronize.
5. Switch profile/mode/mapping during pending edits and capture. Old callbacks
   must not edit replacement content. Repeat on Windows for separate platform
   acceptance, and retain the earlier Stop/edit-layout/Apply/Start checklist.

Curve/deadzone algorithms, general keyboard/navigation/component workflows and
later plan parts remain deferred. Prior native Apply and drag evidence is
preserved verbatim.


## Curve and deadzone editor behavior — 2026-10-16

Part 2 of the [approved plan](gui-rendering-boundary-plan.md) extends the existing
[app mapping controller](../crates/inputforge-app/src/mapping_edits/mod.rs) with
[instrument sessions](../crates/inputforge-app/src/mapping_edits/instruments/mod.rs).
It supersedes the historical curve/deadzone deferrals above; previous native Apply,
capture and drag acceptance evidence remains unchanged.

- App-owned [curve transitions](../crates/inputforge-app/src/mapping_edits/instruments/curve.rs),
  [deadzone transitions](../crates/inputforge-app/src/mapping_edits/instruments/zone.rs),
  [curve mutations](../crates/inputforge-app/src/mapping_edits/instruments/response_curve/mutation/mod.rs)
  and keyboard/focus helpers own type conversion, symmetry, point insertion/removal,
  constrained movement, numeric validation, focus interpretation and commit/revert
  decisions. Curve mutation is split by point, Bezier, conversion, topology and
  symmetry responsibilities. Core evaluation, constructors and persisted formats
  are unchanged. App reconstruction rejects nonfinite curve points before delivery.
- [Private sessions](../crates/inputforge-app/src/mapping_edits/instruments/session.rs)
  capture profile/mapping/stage identity and retain working values. Each input uses
  effective queued history, existing dispatch/acknowledgement barriers and existing
  nudge grouping. A completed changed drag delivers one edit; unchanged clicks,
  repeated release and cancellation deliver none. External edits/undo, queued
  structural remove/reinsert, deleted stages and replacement profiles cancel stale
  gestures. A stale toolbar click refreshes/rejects before it can edit replacement
  content. Delivery failures retain the working draft with accessible feedback;
  explicit cancellation restores the saved/effective value.
- [GUI contracts](../crates/inputforge-gui-dx/src/instruments.rs) carry prepared
  drawing values, highlights, numeric bounds, feedback and callbacks. The
  [signal wiring](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/wiring.rs)
  publishes supplied results. [Geometry](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/geometry.rs),
  SVG layers, curve sampling, thumbnails and live-dot rendering stay in GUI.
  Live-axis analysis remains scheduled for Part 7. The curve and deadzone bodies
  and toolbars forward input observations/intents instead of constructing edits.
- The existing eval transport is now [app-owned](../crates/inputforge-app/src/mapping_edits/instruments/lifecycle.rs),
  with an [instrument pointer asset](../crates/inputforge-app/assets/instrument_pointer.js).
  It keeps live per-event rect measurement, outside-plot dragging and right-click
  release semantics. Escape, blur, pointer cancellation and capture loss cancel;
  close removes listeners synchronously and shuts down the eval receive task.
  Per-mount IDs and element-identity checks reject old elements after remount.
  There is no new engine poller, thread, channel, dependency or general controller.

Regression evidence: the missing app contract first failed the new real-binding
regressions. A nonfinite curve release first retained NaN; app reconstruction now
rejects it and reverts with feedback. A valid curve with sub-step point spacing
first panicked because clamp bounds inverted; neighboring bounds now preserve
its current coordinate when the nominal editing gap cannot fit. A stale Reset
first changed a replacement mapping; the event now refreshes/rejects without a
command. An ignored vertical deadzone key first merged separate horizontal nudge
bursts; it now ends the prior burst. These cases are in
[app binding tests](../crates/inputforge-app/tests/mapping_edits/instruments.rs),
alongside pointer cancel, failed-delivery retention, undo during drag, queued
numeric edits/nudges, stacked centers, structural replacement and close/remount.
Existing algorithm/key cases moved with their app owners; GUI geometry/drawing
checks remain. Independent review found stale focus after type/reset/point removal;
shared focus reconciliation now clamps every rebuilt anchor collection while
preserving an active drag baseline. Cancellation restores values through the same
anchor/focus path; a failed Bezier draft followed by End/Escape/nudge has its own
red/green regression. Real-binding type/reset/context-removal then
nudge cases first reproduced the issue and now pass. Idle hover and unrelated
keys retain the existing nudge burst, with a dedicated red/green regression. [Actual rendered handlers](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/events_tests/mod.rs)
exercise curve keys/type selection, numeric failure alerts, pointer observation,
synchronous close and rejection of a queued release after unmount.

Automated gates passed: unchanged `bash .githooks/pre-commit` with no new lint
exclusions; `cargo test --workspace --locked --offline -j 2` with host IPC access:
**2,143 passed, 0 failed, 11 existing ignored**, including **167 app library**,
**124 app mapping** and **690 GUI library** cases. The keyboard-capture and
rail/editor listener suites, all **14 sortable pointer/ghost cases**, and
**7 instrument listener cases** passed. `git diff --check`, all audit links and
byte-for-byte preservation of the seven historical acceptance/drag sections
passed. New production modules remain below 200 code lines; the existing shared
mapping controller stays near its earlier size. Independent review cleared after
the focus-reconciliation fixes; no Critical/Important issues remained.

Native Wayland/Windows acceptance and runtime performance remain manual. Use
`dx run -p inputforge-app` with a disposable profile:

1. Exercise curve types, symmetry, point add/remove, Bezier junctions, endpoint/
   center constraints and arrow/Shift-arrow focus/nudges. Drag once and undo/redo
   once; unchanged clicks and repeated release must not add history.
2. Drag stacked deadzone centers in both directions, edit all four numeric fields
   and steppers, reset and undo/redo. Check ordering and visual live tracking.
3. Cancel a gesture with Escape/blur/pointer loss; undo externally during a drag.
   Switch profile/mapping, replace/delete/collapse the stage and remount while
   dragging. No old release may edit current content. Repeat separately on Windows.

Part 3 and later parts remain pending. Earlier native Apply/drag evidence remains
historical evidence rather than acceptance of this instrument refactor.


## Profile workflows and file dialogs — 2026-10-16

Part 3 of the [approved plan](gui-rendering-boundary-plan.md) moves profile
selection, creation, copying, external-file choice/import, add-to-library, rename,
duplicate, deletion and reveal decisions into the [app controller](../crates/inputforge-app/src/profiles/mod.rs).
The [policy](../crates/inputforge-app/src/profiles/policy.rs) reuses core name
validation and filename sanitization, isolates the library namespace from active
external profiles, validates case/sanitized collisions and keeps the active row
visible under filtering. [Form preparation](../crates/inputforge-app/src/profiles/forms.rs)
and [dispatch](../crates/inputforge-app/src/profiles/dispatch.rs) revalidate the
captured native path, profile identity and workflow context before sending the
existing engine commands. They release the engine guard before delivery and only
publish accepted presentation transitions. Invalid or failed delivery retains the
form/rename/confirmation, with accessible feedback and no success announcement,
selection, focus or navigation change. Core retains filesystem execution and its
existing warning/error semantics; delivery does not prove that a file was saved.

The [native picker lifecycle](../crates/inputforge-app/src/profiles/dialog.rs)
rejects cancelled/obsolete results after Back, a newer request, profile replacement,
or panel close/remount. Obsolete ownership is checked before a potentially
contended engine read. Picker invocation and `rfd` ownership moved to the app;
the GUI no longer depends on `rfd`. No dependency package, engine poller, timer,
thread, channel, persisted profile format or direct X11 path was added. Existing
bridge reconciliation refreshes the mounted profile view through nonblocking
reads and equality-gated signal publication. Library metadata now carries an
optional persisted profile ID from its existing core load, so a same-name,
same-path replacement cannot receive an old row action or confirmation. Loaded
active and library-file identities are checked independently. Native paths remain
paired with their rows; lossless native-path tokens identify DOM rows and copy
sources even when filenames have identical display text. Existing cached
modification labels flow from the bridge into the profile view, survive local
input, and clear when the authoritative library metadata becomes unavailable.

The [GUI contract](../crates/inputforge-gui-dx/src/profiles/mod.rs) contains prepared
rows, availability, names/errors, mounted signals and app callbacks.
[The panel](../crates/inputforge-gui-dx/src/frame/profiles/mod.rs), forms and rows
render them and forward events. The app owns rename key/focus interpretation and
top-bar profile-panel navigation. A retained rename remains visibly associated
with its path after replacement while its original identity continues to reject
submission. Snapshots and Ctrl+S keep their existing path until Part 4; no dormant
snapshot controls or other product features were introduced.

Regression coverage uses [real app bindings](../crates/inputforge-app/tests/profiles.rs),
[real rendered form/rename/delete handlers](../crates/inputforge-gui-dx/src/frame/profiles/event_tests.rs)
and core temporary-directory lifecycle tests. Tests cover trimmed/illegal/case/
sanitized names, library/external namespaces, active-row filtering, every existing
profile command, cancelled/missing copy sources, native non-UTF8 external paths,
unchanged rename/cancelled delete, failed delivery retention, stale active/inactive
profile identities, stale form callbacks and picker replacement/teardown. Real
Create first lost accessible failure feedback; retained rename first disappeared
after same-path replacement. An obsolete picker under engine contention first
wrote feedback into a newer form. These cases have recorded red/green evidence.
App assertions run outside Dioxus rendering so its error boundary cannot absorb a
test failure.

**Validation:** the unchanged pre-commit hook passed. The final host run of
`cargo test --workspace --locked --offline -j 2` passed with **2,155 passed,
0 failed and 11 existing ignored**, including 22 app profile integration cases,
36 GUI profile/snapshot cases, core filesystem handling, and existing
mapping/history/capture/lifecycle suites. Host IPC access was required for the
existing single-instance socket tests; the sandbox failures were rerun and passed.
Keyboard capture, rail/editor, all 14 sortable and all 7 instrument pointer
JavaScript regressions passed. Diff whitespace and audit links passed; seven
historical evidence sections retain their saved hashes. New profile modules stay
below 200 production lines each.

One independent read-only review found two identity defects: active library rows
lost their separate on-disk profile identity, and lossy path lookup could merge
distinct native filenames. Regression cases reproduced both before the fixes.
Focused re-review cleared both fixes and the connected metadata preservation,
with no outstanding Critical or Important findings. The reviewed source hashes
match the final tested source.

Native Wayland and separate Windows desktop acceptance remain manual. Use
`dx run -p inputforge-app` and disposable profiles:

1. Create blank, copy-active and copy-library profiles. Try empty, illegal,
   case-only and duplicate names. Rename with Enter, blur and Escape; filter the
   library and confirm the active row stays visible.
2. Cancel the picker; choose an in-library file, then an external file for Load
   once and Add to library. Close/reopen the panel or switch profiles while a
   picker is open; an old result must not change the current workflow.
3. Keep same-named library and external profiles distinct. Duplicate/reveal a
   library row; cancel and confirm deletion, including the active-profile warning.
   Check snapshots/Ctrl+S still behave as before. Repeat separately on Windows.

Part 4 and later parts remain pending. Earlier native Apply/capture/drag evidence
is preserved verbatim and does not establish acceptance of these profile changes.


## Part 4 — Snapshot workflows and shortcut — 2026-10-16

The [app snapshot policy](../crates/inputforge-app/src/profiles/snapshots.rs)
now owns manual creation, restore/delete validation, protected session-start
availability, drawer transitions and destructive restore confirmation. It reuses
the existing profile controller and captured native-path/profile identity; each
row also captures the complete engine-projected snapshot identity. Missing or
replaced snapshots/profiles reject retained callbacks. Active-profile menu
availability follows current profile identity even while the library frame is old.

The [prepared contract](../crates/inputforge-gui-dx/src/profiles/snapshots.rs)
provides rows, availability and confirmation state. The [drawer](../crates/inputforge-gui-dx/src/frame/profiles/snapshot_drawer.rs),
active-profile menu and [confirmation renderer](../crates/inputforge-gui-dx/src/frame/profiles/snapshot_confirm.rs)
forward app callbacks. Restore now honors its previously declared destructive
confirmation through the existing dialog primitive. Cancellation never sends a
restore; failed delivery keeps the confirmation and accessible error. Delivered
requests never announce successful persistence or restore. A [narrow core execution guard](../crates/inputforge-core/src/engine/snapshots.rs)
checks captured profile ID/native path and snapshot row before any operation side
effect. This rejects a snapshot from A queued behind a load of B before B receives
manual/recovery creation, pruning, restore or deletion. It delegates accepted
requests to the existing core recovery, restore/rollback, filesystem and warning
handlers; the operation implementations remain unchanged. The shared mapping
barrier/acknowledgement is untouched. Dormant snapshot rename/pin/undo controls were not introduced;
superseded GUI action contracts, mirrored focus-policy tests and unused snapshot
chrome fields were removed.

The [root shortcut owner](../crates/inputforge-app/src/profiles/shortcut/mod.rs)
and [browser listener](../crates/inputforge-app/assets/snapshot_shortcut.js)
own Ctrl+S interpretation, current-profile generations and synchronous removal.
The listener installs after physical keyboard interception; editable/rename,
menu/dialog, repeated/intercepted/composing keys and stale native-picker-return
focus are suppressed. The existing bridge updates profile generations and rejects
queued events from obsolete contexts. Unmount removes listeners synchronously;
fresh mounts have separate owners. Errors use the existing accessible warning
queue. No extra engine poller, thread, channel, dependency or X11 path was added.

Regression evidence covers [real app bindings and root lifecycle](../crates/inputforge-app/tests/snapshots.rs),
[real drawer handlers](../crates/inputforge-gui-dx/src/frame/profiles/snapshot_events.rs)
and an [executable listener fixture](../crates/inputforge-app/tests/snapshots/listener.mjs)
that runs alongside the production capture interceptor. Missing-contract tests,
destructive confirmation bypass, missing listener and stale active-row availability
were observed failing before their respective implementation. The [core request regressions](../crates/inputforge-core/src/engine/snapshot_request_tests.rs)
cover queued loads followed by every snapshot operation, failed queued loads,
replacement/removal/changed rows, accepted operations and restore-failure warnings.
Real app drawer and root tests verify captured guarded payloads even before a
queued profile switch reaches the state frame. Separate executable regressions
cover ordinary element blur and modifier-then-Ctrl+S after native picker return.
Existing restore/rollback, profile/native-path, capture, history and drag
regressions remain covered by the workspace gate.

Automated verification:

- `cargo test --workspace --locked --offline -j 2`: **2,167 passed, 0 failed,
  11 existing ignored** on the host, including local IPC tests. The initial
  sandbox run denied the five existing single-instance socket tests; host
  verification included them without exclusions.
- Unchanged `bash .githooks/pre-commit` passed, including format and strict
  all-target Clippy. Keyboard-capture, rail/editor, sortable drag (14), instrument
  pointer (7) and snapshot shortcut JavaScript checks passed.
- Audit links, seven historical acceptance/drag hashes, touched production-module
  sizes and `git diff --check` were checked. Independent review cleared after
  native-return modifier handling and queued-profile execution guards were fixed
  regression-first. All new production modules stay below the 200-code-line
  baseline; existing engine orchestration/dispatch gained only the narrow wiring.

Manual acceptance remains with a disposable profile using `dx run -p inputforge-app`
on native Wayland and separately Windows:

1. Create through Ctrl+S, drawer + and the active-profile menu; toggle the drawer,
   delete a manual snapshot and confirm session-start deletion stays disabled.
2. Restore opens confirmation. Cancel/Escape sends nothing; confirm restores and
   preserves the pre-restore recovery snapshot. Exercise a failed restore and
   verify warning feedback without false success.
3. Check editable/rename/menu/dialog focus, held Ctrl+S, armed keyboard capture,
   OS-picker cancellation/refocus and close/reopen. Each ordinary keypress should
   create one snapshot and obsolete mounts must remain inactive.

Native desktop/hardware/visual acceptance and performance are unverified. Part 5
and later parts remain pending. The seven historical acceptance/drag sections
are preserved verbatim and do not establish acceptance of this snapshot refactor.


## Part 5 — Settings and prune confirmation — 2026-10-17

The settings portion of findings **#1**, **#2**, **#6** and workflow-local **#8** is resolved:

- [App settings controller](../crates/inputforge-app/src/settings/mod.rs) owns mounted callback identity, switch transitions, pending-value reconciliation and command delivery. [Settings policy](../crates/inputforge-app/src/settings/policy.rs) owns raw buffer validation, no-op decisions, gesture validation, prune calculation and confirmation/cancellation. [GUI settings contract](../crates/inputforge-gui-dx/src/settings.rs) contains prepared values, captured confirmation and callback wiring.
- [Settings rendering](../crates/inputforge-gui-dx/src/frame/settings_panel/mod.rs) and its snapshot/startup/gesture sections consume those values and forward observations. They no longer read engine state, build commands, decide toggles or announce optimistic removal success. Invalid input and rejected delivery expose accessible feedback; a failed confirmation retains the dialog. Prune Cancel and Escape deliver no settings command.
- [Core settings requests](../crates/inputforge-core/src/engine/settings.rs) apply field deltas through existing persistence, autostart and gesture handlers, preserving queued cross-field changes. A buffer change captures stable profile identity, lossless native path, current snapshot configuration and unpinned count; app confirmation and core execution both revalidate that context. Queued profile loads or count/settings changes cannot redirect a previously confirmed destructive operation.
- A dedicated settings acknowledgement in [AppState](../crates/inputforge-core/src/state/mod.rs) allows pending switch/field choices to survive old polling frames and reconcile after execution, including save rollback. It does not overwrite the mapping-edit barrier. Fresh mounted owner identities reject obsolete callbacks and acknowledgements. The existing immediate/16 ms nonblocking bridge drives reconciliation; no new polling task, timer, thread or channel was added.
- [SnapshotConfig validation](../crates/inputforge-core/src/snapshot/config.rs) now provides the shared 1–100 buffer rule before persistence. The existing core gesture validator, startup/autostart service behavior, pinned exclusions, save rollback and prune warnings remain authoritative. Delivery never announces that files were removed; backend save/prune failures remain warnings.

### Part 5 verification

- Regression-first app contract failures, the real GUI failed-prune retention failure, a remount acknowledgement failure and missing confirmation-generation contract were recorded before their fixes. A real invalid-gesture handler regression also failed before its feedback callback was added.
- [App binding tests](../crates/inputforge-app/tests/settings.rs) cover malformed/0/1/100/101 and unchanged buffers, gesture bounds/no-ops, rapid switches before polling, queued cross-field values, pinned counts, stale profile/native path/count/config/confirmation, delivery failure, cancellation, disposal and remount acknowledgements.
- [Real GUI handler tests](../crates/inputforge-gui-dx/src/frame/settings_panel/event_tests.rs) exercise buffer/gesture validation, failed switches and confirmation retention without a removal-success claim, plus Cancel/Escape. [Core request tests](../crates/inputforge-core/src/engine/settings_request_tests.rs) cover queued profile-switch rejection, count/config/bounds drift, delta ordering, independent mapping acknowledgement, pinned-safe pruning and backend save/prune failures.
- Unchanged `bash .githooks/pre-commit` passed. `cargo test --workspace --locked --offline -j 2` passed with host IPC access: **2,176 passed, 0 failed, 11 existing ignored**. The initial sandbox run encountered the same five existing single-instance socket permission failures; no test was excluded or weakened.
- All five required executable JavaScript suites passed: keyboard capture, mapping rail/editor, sortable pointer/ghost, instrument pointer and snapshot shortcut. `git diff --check`, audit links, new production-module sizes and all seven historical acceptance-section hashes passed.
- Independent read-only review cleared with no actionable Critical/Important findings. Checkpoint details live in the plan-specific scratch report; native desktop acceptance remains separate.

### Part 5 manual acceptance and remaining scope

Use `dx run -p inputforge-app` on native Wayland, then separately Windows. With disposable profiles, verify buffer edits through Enter/blur, invalid values and unchanged commits; toggle skip/startup/start-minimized rapidly and edit both gesture thresholds. Relaunch and check persisted settings and actual startup/tray behavior. Create enough unpinned snapshots plus pinned snapshots to require pruning: Cancel/Escape must not prune; Confirm must keep pinned snapshots and remove only the excess. Change profile or snapshot count while confirmation is open and verify rejection; confirm queued profile changes cannot prune the replacement profile. Reopen Settings and ensure no duplicate actions or obsolete callbacks. Verify backend warnings remain visible on failure.

Reusable numeric/tab behavior (Part 10), general overlay/listener policy (Part 11), notification scheduling (Part 8) and final raw-handle removal (Part 13) remain deferred. Native hardware/compositor/Windows acceptance, desktop focus and runtime performance were not verified by automated tests. Prior Apply and drag acceptance evidence is preserved verbatim.


## Part 6 — Device inspector and native clipboard — 2026-10-17

The inspector portion of findings **#1**, **#2**, **#7** and workflow-local **#8**
now belongs to the [app inspector](../crates/inputforge-app/src/controllers/inspector/mod.rs).
Its [policy](../crates/inputforge-app/src/controllers/inspector/policy.rs) chooses
the first connected device, falls back to remembered devices, reconciles removed
selections and prepares Save/removal availability. [Captured dispatch](../crates/inputforge-app/src/controllers/inspector/dispatch.rs)
owns alias input, trimmed/no-op/empty commits, Enter/Escape, cancellation, removal
and report-copy invocation/error feedback. Raw drafts remain after rejected
validation, delivery or persistence; delivery does not announce a successful save
or removal. Ordinary typing keeps native keyboard behavior even when Enter fails.

The [prepared GUI contract](../crates/inputforge-gui-dx/src/controllers/inspector.rs)
contains rows, selection, draft, availability, accessible feedback and mounted
callbacks. [Ledger rendering](../crates/inputforge-gui-dx/src/frame/panel_slot/device_panel/mod.rs)
and [inspector rendering](../crates/inputforge-gui-dx/src/frame/panel_slot/device_panel/inspector.rs)
forward observations; [plain-text report formatting](../crates/inputforge-gui-dx/src/frame/panel_slot/device_panel/formatting.rs)
remains pure rendering. No migrated handler reads engine state, constructs an
engine command or invokes the clipboard. The existing controller contract moved
to `controllers/mod.rs` to satisfy the unchanged module-layout lint. Existing
selection/layout/Apply and sparse-axis bindings retain their behavior.

The [core device guard](../crates/inputforge-core/src/engine/devices.rs) captures
stable profile ID, lossless native profile path, native device metadata and serial.
Mounted owners and observed row generations reject obsolete callbacks, including
same-ID native replacements and queued profile changes. Retained alias drafts
never adopt a replacement device. A fresh explicit selection resets the draft.
App removal rechecks current connection and primary/nested secondary references;
core execution retains its existing backend-connection and dependency validation.
A failed alias settings save now restores the prior in-memory aliases before
returning the existing error. Core warnings remain the persistence-failure signal;
no extra acknowledgement replaces the mapping barrier.

The existing bridge reconciles through nonblocking reads and publishes only changed
inspector values. Empty frames do not advance workflow generations. Mount/unmount
ownership rejects obsolete callbacks; the editor retains the clipboard handle
while the panel is closed. The Linux feature and installed-source evidence are
recorded above. No new application poller, timer, thread, channel, generic workflow
framework, direct X11 path, persisted format or processing algorithm was introduced.

### Part 6 verification

[App binding regressions](../crates/inputforge-app/tests/devices.rs) exercise
connected/remembered fallback, removed selection, native/profile replacement,
trimmed/empty/no-op aliases, cancellation, failed-delivery retention, fresh versus
obsolete row callbacks, ordinary keys and failed Enter. Private native-I/O boundary
cases exercise actual copy dispatch exactly once, lock release, failure feedback
and owner retention/reuse. [Real GUI event cases](../crates/inputforge-gui-dx/src/frame/panel_slot/device_panel/event_tests.rs)
exercise Save, Enter and Escape with accessible failure feedback. [Core guard cases](../crates/inputforge-core/src/engine/device_request_tests.rs)
exercise queued profile loads, replaced/missing devices, new nested references,
connection drift, accepted persistence and failed-save rollback. Missing app/core
contracts, an inaccessible real GUI error, stale ledger selection, repeated empty
publication and failed alias-save mutation were observed failing before fixes.

The unchanged `bash .githooks/pre-commit` passed (format and strict all-target
Clippy). `cargo test --workspace --locked --offline -j 2` passed with host IPC
access: **2,186 passed, 0 failed, 11 existing ignored**, including all single-instance
socket tests. All five executable JavaScript suites passed: keyboard capture,
mapping rail/editor, sortable drag (14), instrument pointer (7) and snapshot
shortcut. Resolved Linux Cargo features include `wayland-data-control` and
`wl-clipboard-rs`. Audit/plan links, seven saved historical evidence hashes,
production-module sizes and `git diff --check` passed. New production modules stay
below 200 code lines; larger pre-existing modules received only narrow integration
changes. One independent read-only review verified all 34 frozen path hashes and
cleared the complete connected change with no Critical or Important findings.

### Part 6 manual acceptance and remaining scope

Use `dx run -p inputforge-app` with disposable profiles on native Wayland and
separately Windows. Select connected and remembered devices, disconnect/remove the
selected device and verify fallback. Save aliases with Save/Enter, try trimmed,
empty and unchanged names, then Escape an unsaved draft. Switch profiles or replace
the native device while a draft remains; old actions must reject until reselected.
Try removal while connected and while primary/nested conditional/merge references
exist, then remove an unused disconnected device. Copy the report once, close and
reopen the panel and paste into a separate native application; repeat on native
Wayland without an XWayland clipboard dependency and separately Windows. Exercise
clipboard denial/unavailability and backend-save failure for visible feedback and
retained drafts. Repeat Stop/edit/Apply/Start with sparse native controls and
retained outputs to check unchanged controller behavior.

Native hardware/compositor/Windows acceptance, desktop focus and runtime performance
remain unverified. Live readout analysis, notification scheduling, remaining
navigation/component policies and final raw-handle removal remain Parts 7–13.
Prior Apply and drag acceptance evidence is preserved verbatim.


## Prepared live readouts and activity expiry — 2026-10-17

Part 7 resolves finding **#9**. [App analysis](../crates/inputforge-app/src/bridge/readout/analyze.rs),
[bounded traversal](../crates/inputforge-app/src/bridge/readout/walk.rs),
[predicates](../crates/inputforge-app/src/bridge/readout/predicates.rs) and
[terminal ownership](../crates/inputforge-app/src/bridge/readout/terminal.rs) prepare
selected-mapping input/output rows, nested conditional/gesture paths, independent
sibling/stacked merge chains and polarity. The existing branch-local merge-preview
semantics and depth cap remain. Core evaluation still owns scalar processing;
[prepared instrument values](../crates/inputforge-app/src/bridge/readout/mod.rs)
retain top-level, bound-axis and connected-device gates. No processing algorithm
or persisted format changed.

[Typed input display and rail intensity](../crates/inputforge-app/src/bridge/readout/inputs.rs)
are prepared with the existing coherent physical sample, including the mapping
rail and bulk-map consumers. Missing button/hat references keep their binary/hat
readout types; unbound previews stay neutral without reaching the core's Bound
primary invariant. Logical axis indexes remain independent of sparse native codes.
The [GUI contract](../crates/inputforge-gui-dx/src/readouts/mod.rs) carries completed
values. Labels, percentage formatting, bars/SVG geometry and stage drawing remain
rendering; the GUI per-window context has no production engine state/sender.
The transitional host `RawHandles` contract remains for Part 13.

[App activity deadlines](../crates/inputforge-app/src/bridge/readout/activity.rs)
replace the readout's recursive GUI timers. The first observation fixes the 150 ms
latch deadline; identical subsequent observations do not renew that latch. A
separate last-sample freshness deadline prevents long-observed transient activity
from sticking when engine reads become contended. The existing immediate/16 ms
nonblocking bridge advances both and publishes only changed prepared readouts on
contended ticks, without repeating capture/history reconciliation or reading the
engine again. Only the base selected readout is cached for expiry. Profile/path,
selection, actions and session-generation changes clear prior held state. Selection
changes during contended reads explicitly clear the published readout; renderers
also require the full mode/input key, so identical actions on the same input in
different modes cannot inherit a prior mode's pulse.
[App expansion callbacks](../crates/inputforge-app/src/mapping_edits/readouts.rs)
reset on prepared identity/output-count changes and reject obsolete toggle keys;
per-mount renderer signals hold the resulting expansion state.

### Part 7 verification

[Real app regressions](../crates/inputforge-app/tests/readouts.rs) compare instrument
and nested merge values with core evaluation, exercise exact branch ownership,
sparse native axes, disconnected/missing/unbound input and identity replacement.
Moved analyzer/walker/polarity tests preserve nested conditional/gesture traversal,
sibling/stacked merge isolation, predicates and depth limits.
[Bridge deadline cases](../crates/inputforge-app/src/bridge/readout/tests.rs) cover
unchanged observations, disappearance without new input, first and long-observed
activity during contended reads, selection reset and profile replacement.
[Real renderer events](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/events_tests.rs)
exercise chevron callbacks, live merge refresh while the engine write guard is
held, equal-output-count identity resets, and clean production remounts.
Missing prepared contracts, missing input type fallback, unbound preview panics
and stale long-observed activity were observed failing before their fixes.
Existing instrument fixtures now seed the real app projection rather than split
raw snapshots. The unchanged `bash .githooks/pre-commit` passed (format and strict
all-target Clippy). Final `cargo test --workspace --locked --offline -j 2` with
host IPC access passed: **2,192 passed,0 failed,11 existing ignored** across30
suites. All five executable JavaScript suites passed: keyboard capture, mapping
rail/editor, sortable drag(14), instrument pointer(7), and snapshot shortcut.
All234 audit/plan links, seven saved historical evidence hashes, new production
module sizes(under200 code lines) and `git diff --check` passed. One independent
read-only review verified all48 frozen raw paths and cleared the complete change
after its stale-selection finding was fixed regression-first. The fix's real
sampler, publication and renderer cases cover mode switches with identical input/
actions and contended deselect/reselect; obsolete readouts cannot revive.

### Part 7 manual acceptance and remaining scope

Use `dx run -p inputforge-app` with disposable profiles on native Wayland and
separately Windows. Move axes/buttons/hats and check IN rows, rail/bulk-map readouts,
curve/deadzone dots, predicates, stacked/nested merge previews and keyboard/mouse/
vJoy OUT values. Expand outputs, switch mappings/profiles/actions, disconnect and
reconnect sparse native controls, and check reset/fallback behavior. Fire short
single/double/long gesture outputs; pulses must clear without another input event.
Check Stop/edit/Apply/Start, capture, queued edits and undo/redo, then close/reopen
the editor with outputs expanded. Native hardware/compositor/Windows acceptance,
desktop focus and runtime performance remain unverified. Notification scheduling,
remaining navigation/component policy and final host-handle removal remain Parts
8–13. Prior Apply and drag evidence remains unchanged.

The user authorized automatic sequential continuation on 2026-10-17: each remaining
part still uses a fresh implementation subagent and one commit, reports its summary,
and then proceeds to the next part. Only material decision blockers stop execution.


## Notifications and warning delivery — 2026-10-17

Part 8 resolves finding **#10**. The [app notification owner](../crates/inputforge-app/src/notifications/mod.rs)
installs isolated mounted state and callback delivery; the [bounded queue](../crates/inputforge-app/src/notifications/state.rs)
owns latest-only `(level, message)` coalescing, five visible entries in insertion
order, FIFO overflow, eight effective seconds, expiry, independent hover/focus
pause accounting and Escape/dismiss. Expired duplicates receive fresh identities;
coalescing resets the lifetime while retaining an active hover/focus pause.
Obsolete IDs and disposed owners cannot change the current list. The app
[warning tail](../crates/inputforge-app/src/notifications/warnings.rs) baselines the
first coherent snapshot on each mount, delivers later appends once, handles
truncation/sliding tails and same-length replacement, and rebaselines resets.
Historic warnings remain available in engine metadata without replaying on remount.

The [existing app bridge](../crates/inputforge-app/src/bridge/mod.rs) advances
notification deadlines on its immediate/16 ms tick, including contended engine
reads. It installs no extra production poller, task, channel or thread and does not
repeat capture/history work for expiry. The [GUI contract](../crates/inputforge-gui-dx/src/toast/queue.rs)
contains visible values and supplied callbacks; [notification rendering](../crates/inputforge-gui-dx/src/toast/viewport.rs)
retains chronological markup, formatting, polite status delivery, assertive alert
roles and event wiring. Queue mutation, wall-clock tests, warning effects and the
GUI interval are removed. Bulk-map request notices and snapshot-shortcut feedback
use the same app-owned delivery; profile/snapshot/settings/editor inline feedback
is independent of notification expiry. Delivery never becomes a persistence-success
claim, and no dormant undo action is introduced.

The standalone [component gallery](../crates/inputforge-gui-dx/examples/component_gallery.rs)
uses the app's [preview deadline hook](../crates/inputforge-app/src/notifications/preview.rs),
sharing the production state/deadline calculations. Only that isolated gallery
scope installs a cancellable sleep to its next deadline; changing the deadline
replaces that sleep and teardown cancels it. The production editor uses only its
existing bridge. All hosts and fixtures now install the app callbacks atomically.
The app dev dependencies enable existing core/autostart mock features to exercise
a real delivered command and deterministic core persistence failure; no runtime
dependency, package version, native service or persisted format changes.

### Part 8 regression and verification evidence

[App callback regressions](../crates/inputforge-app/tests/notifications.rs) and
[injected-time state cases](../crates/inputforge-app/src/notifications/tests.rs)
cover latest-only/expired duplicates, chronological FIFO overflow, level isolation,
coalescing lifetime resets, interleaved hover/focus and duplicate resume/dismiss,
warning truncation/reset/replacement and no duplicate delivery. The missing app
contract was observed failing before implementation. [Actual viewport events](../crates/inputforge-gui-dx/tests/notifications.rs)
exercise hover/focus/Escape/click dismissal, preserved status/alert markup, preview
expiry/coalescing and teardown/fresh remount. An initial preview test exposed that
`use_future` does not subscribe/restart on deadline changes; the app now replaces
one deadline sleep from an effect, with no interval or polling loop.

The [production bridge lifecycle case](../crates/inputforge-gui-dx/tests/editor_bridge/notifications.rs)
verifies expiry while the engine writer is held, no warning replay/duplicate
polling and released handles on disposal/remount. A [real settings request](../crates/inputforge-app/tests/notifications/backend.rs)
is accepted before core save failure: its warning arrives once, the confirmed
setting rolls back and inline failure feedback remains after toast expiry.
Equal visible lists do not wake subscribers. Native focus, accessibility software,
Wayland/Windows acceptance and runtime performance remain manual.

Independent review identified one host omission: the bridge demo still published
only one static frame, leaving notification deadlines undriven after the GUI
interval moved. The [actual demo host](../crates/inputforge-gui-dx/examples/bridge_demo.rs)
now starts the same app bridge/capture owner as the editor, with no additional
scheduler. Its [paused-time regression](../crates/inputforge-gui-dx/examples/bridge_demo/notifications.rs)
failed at eight seconds before the fix, then passed through the real snapshot
shortcut listener/feedback, expiry, pending-close, cleanup and fresh remount. The
GUI manifest includes this example test in normal workspace validation.

Final host `cargo test --workspace --locked --offline -j 2` passed: **2,193
passed, 0 failed, 11 existing ignored** across 33 result blocks, including the
actual demo-host regression. The unchanged strict pre-commit hook (format and
all-target Clippy), all five current executable JavaScript suites, 251 local
audit/plan links, seven historical acceptance hashes, touched module sizes and
`git diff --check` passed. One independent read-only review verified the frozen
complete change, raised the demo deadline-driver finding above, then cleared its
regression-first fix in scoped re-review: Spec PASS / Quality SHIP. No hook,
lint or test exclusions changed.

### Part 8 manual acceptance and remaining scope

Use `dx run -p inputforge-app` with disposable profiles on native Wayland and
separately Windows. Trigger warnings and request feedback; verify chronological
order, latest-only count badges, five-entry overflow and eight effective seconds.
Hover, focus via Tab, move the pointer out while focus remains, then move focus
out: expiry resumes only after both sources leave. Repeat coalescing while paused,
Escape and close-button dismissal. Reopen the editor and verify historic warnings
do not replay. Trigger a disposable settings/profile save failure and confirm the
backend warning plus persistent inline feedback, with no false completion claim.
Check the standalone component gallery's expiry and close/reopen cleanup.

Remaining refactor scope is Parts 9–13: navigation/shortcuts, form/tab policies,
overlays, sortable interaction and final raw-handle removal/audit closure. Prior
Apply, live-capture and native drag acceptance evidence is preserved verbatim.

## Editor navigation and remaining shortcuts — 2026-10-18

Part 9 moves editor navigation and shortcut policy into the existing mapping
controller's [navigation modules](../crates/inputforge-app/src/navigation/mod.rs).
The [GUI contract](../crates/inputforge-gui-dx/src/navigation.rs) supplies rendering
signals and app callbacks. [View-state wiring](../crates/inputforge-gui-dx/src/frame/view_state.rs)
allocates those signals and forwards reconciliation; it no longer compares
profile display names or chooses fallback modes.

- [Reconciliation](../crates/inputforge-app/src/navigation/reconcile.rs) uses the
  actual native profile identity, effective queued mappings and the existing
  pending-mode guards. Different profiles with equal names reset authoring
  selection and expansion; renaming the same profile retains them. Deleted modes
  use the existing app fallback, and pending mode delivery survives old frames.
- App callbacks own primary surfaces, live panel toggles, profile-panel entry,
  rail row selection, filter/device transitions, stage expansion/context-menu
  entry and selection/structural-edit invalidation. Captured row and stage events
  cannot act on an identically named replacement profile. Successful insertions
  expand the inserted stage; index shifts preserve ancestors and other branches.
  Failure still retains editor drafts and expansion without success navigation.
- [Filter projection](../crates/inputforge-app/src/navigation/filter.rs) preserves
  source/name matching, aliases and disambiguated device labels. The app flattens
  visible rows in AXES/BUTTONS/HATS order, preserving order within each group, so
  keyboard traversal follows the displayed groups even when source mappings are
  interleaved. GUI source-label formatting and group headings remain drawing.
- [Rail listener handling](../crates/inputforge-app/src/navigation/rail.rs) and
  [editor listener handling](../crates/inputforge-app/src/navigation/editor.rs)
  own interpretation, mounted cleanup and app-selected focus targets. Undo/redo
  use the existing history; Alt+Up/Down now use the existing guarded relative
  stage edit, including nested paths, and Shift+F10 opens the actual stage menu.
  Focus execution stays in the renderer. Closed or obsolete listener payloads
  cannot dispatch edits or request focus.
- The production scripts defer to input/modal/menu focus, the existing capture
  interceptor and pointer drags. Ctrl+F remains available from editor chrome;
  Escape inside a nonempty rail filter clears text while the capture pad defers
  to that field. Input Ctrl+Z stays native; existing editor redo conventions stay.
- Stable profile identities key the mapping workspace and add-palette scope.
  Profile replacement remounts drafts/listeners, while same-profile renames do
  not invalidate active authoring. Host/tray Review remains on its existing
  app-owned navigation; the no-profile Devices gate and placeholder empty-state
  product scope remain unchanged. No additional poller, thread or channel exists.

Regression evidence: app-contract RED checks preceded implementation; the
interleaved-group regression failed with source-order keyboard rows before the
stable grouping fix. App binding and real document-transport tests exercise
same-name replacement, same-profile rename, live panel toggles, pending modes,
missing mapping/mode, expansion/menu/hint reset, stale row/stage callbacks,
nested stage command delivery/barriers and obsolete/closed focus requests.
Existing GUI events still cover failed delivery, retained drafts, actual rail
reorder, capture display and batch completion. Executed JavaScript covers real
production listener bodies, capture interception, input/modal/menu scopes,
filter Escape, pointer-drag deferral and isolated teardown/remount.

The independent review identified two stale-frame races. Regression-first fixes
bind positional stage events to the rendered action tree, rejecting old positions
both before engine execution and before DOM publication. Pointer stage headers
use the same captured-tree guard; undo/redo retain their existing queue semantics.
Row selection now requires the current editing mode and effective mapping, so a
retained old-mode row cannot override navigation during a queued mode rename.

Final host workspace gate: **2,196 passed, 0 failed, 11 existing ignored** across
34 suites. The unchanged strict pre-commit hook and all six JavaScript suites
passed, as did 258 local links, seven preserved historical evidence hashes and
new-module size checks. Independent review and scoped re-review cleared the
regression-first stale-event fixes: Spec PASS / Quality SHIP. The approved plan
records the completed automated checkpoint. Earlier native Apply/capture/drag evidence
above remains unchanged. Parts 10–12 still own reusable forms/tabs, overlays and
sortable primitives; Part 13 closes transitional host handles.

Manual acceptance remains: on native Wayland and separately Windows, use keyboard
navigation/filter/Escape, switch mapping and batch surfaces, toggle panels, rename
and switch equal-name profiles during drafts and capture, reorder nested stages,
open the stage menu with Shift+F10, undo/redo, and close/reopen the editor. Check
focus, draft cancellation and listener cleanup. Native desktop focus/compositor,
assistive technology, hardware and runtime performance are unverified.

## Reusable form and tab behavior — 2026-10-18

Part 10 moves reusable control policy to the app's engine-independent
[UI behavior modules](../crates/inputforge-app/src/ui/mod.rs). The
[renderer contract](../crates/inputforge-gui-dx/src/controls.rs) supplies raw text,
prepared focus instructions and application handlers. Desktop hosting, the seeded
editor demo, the component gallery and direct fixtures use these same callbacks;
the GUI has no production dependency on the app or fallback behavior implementation.

- [Numeric inputs](../crates/inputforge-app/src/ui/inputs.rs) and
  [integer inputs](../crates/inputforge-app/src/ui/integer.rs) own parsing,
  finite bounds, stepper values, Enter/blur commit deduplication, Escape restore,
  invalid feedback and raw draft synchronization. Typing survives unrelated
  external value updates. Failed delivery retains the draft; refocus permits
  retry without retyping. Enter followed by new input does not suppress that
  new edit's blur. Integer errors can be revalidated on later blur without
  duplicating Enter's immediately following blur.
- Text arrow/Home/End propagation and segmented-option disabled dispatch belong
  to app callbacks. Native-only Select, Slider, Checkbox and Switch forwarding
  remains unchanged. The renderer retains formatting, native attributes, labels,
  described-by links, ARIA and indeterminate drawing state.
- [Tab behavior](../crates/inputforge-app/src/ui/tabs.rs) owns orientation-aware
  wrapping, disabled-item policy, activation and focus-target selection.
  Keyboard focus waits for the accepted active value; failed activation cannot
  optimistically move focus. Explicit focus consumes pending keyboard focus,
  accepts repeated requests and skips disabled targets. The GUI retains mounted
  references and performs the supplied focus instruction. Root disabled changes
  now reach existing descendants instead of preserving a mount-time snapshot.

Regression-first evidence includes a missing app callback contract and actual
renderer failures for nonfinite dispatch, raw draft loss and disabled controls.
Focused follow-up regressions caught duplicate explicit/keyboard focus, impossible
finite bounds, the missing refocus observation and a new edit arriving between
Enter and blur. Existing parser tests moved to the app. The
[real renderer handlers](../crates/inputforge-gui-dx/tests/controls.rs) cover
numeric commit/cancel/step rules, disabled gates, text propagation and tab
traversal; [app tab tests](../crates/inputforge-app/tests/ui_tabs.rs) cover
acknowledged focus, repeated requests, removed targets and bounded empty or
all-disabled traversal. Independent review also identified same-value commits
leaving dirty numeric drafts behind; two actual-handler regressions reproduced
the stale text surviving later external updates. Same-value commits now clear
the dirty marker synchronously from the already-published value while retaining
consumer validation, including controls without an `oncommit` consumer. Steppers
replace older typed text only after their value is published; failed steps retain
the draft and newer typing supersedes a pending step. No extra poller or effect
loop was introduced.
Existing settings, instrument, profile, mode, capture,
history and listener regressions remain part of the workspace gate.

Final automated checkpoint: **2,216 passed, 0 failed, 11 existing ignored**
across 36 workspace suites. The unchanged strict pre-commit hook and all six
JavaScript suites passed, together with 265 local links, seven preserved
historical evidence hashes and production-module size checks. Independent
review and the same-seat regression-fix reviews cleared all findings
(Spec PASS / Quality SHIP). Product and test source stayed frozen for the final
review and full gate.
Parts 11–12 retain general overlays/dismissal and sortable interaction; Part 13
removes transitional host handles and closes the audit. Earlier native Apply,
live-capture and drag evidence above remains unchanged.

Manual acceptance: use `dx run -p inputforge-app` on native Wayland and separately
Windows. Check settings/gesture/deadzone typing, steppers, invalid errors,
Enter/blur once, Escape, external/undo changes during drafts and failed-edit
refocus retry. Check inline text navigation, curve/mode tab arrows/Home/End,
disabled tabs, add/rename focus, gallery controls and assistive-technology labels.
Native focus/compositor, accessibility, hardware and performance remain unverified.

## Menus, dialogs and dismissal behavior — 2026-10-18

Part 11 moves reusable overlay interpretation and native document ownership into
focused [app UI modules](../crates/inputforge-app/src/ui/overlays/mod.rs), supplied
through the existing engine-independent [controls provider](../crates/inputforge-app/src/ui/mod.rs).
Desktop hosting, demos/gallery and canonical renderer fixtures share the same
[overlay contracts](../crates/inputforge-gui-dx/src/overlays.rs); the GUI has no
production dependency on the app or duplicate default behavior.

- [Menu behavior](../crates/inputforge-app/src/ui/overlays/menu.rs) owns trigger
  toggles, dismissal reasons, keyboard interpretation and focus requests. The
  [focus script](../crates/inputforge-app/assets/menu_focus.js) wraps traversal,
  skips native and ARIA-disabled items, and safely ignores removed targets.
  Arrows/Home/End prevent page scrolling and propagation. Native Enter/Space
  remains button activation. Escape returns trigger focus; Tab preserves native
  traversal. Existing domain activation gates still run before closure: rejected
  actions retain their menu and accessible workflow feedback. Closed/disabled
  trigger-menu items reject stale synthetic activations.
- [Dialog behavior](../crates/inputforge-app/src/ui/overlays/dialog.rs) owns
  visibility, current dismissible/backdrop policy and cancellation. The native
  cancel listener prevents browser closure until the owning workflow accepts
  dismissal. [Destructive](../crates/inputforge-gui-dx/src/patterns/destructive_confirm.rs)
  and [dirty](../crates/inputforge-gui-dx/src/patterns/dirty_confirm.rs) confirmation
  patterns invoke bool-returning workflow callbacks before changing visibility;
  false retains the dialog, and closed dialogs cannot submit again. Production
  profile, restore, prune and mode cancellation returns the existing workflow
  result. Command delivery is not presented as persistence completion.
- [Click-away ownership](../crates/inputforge-app/src/ui/overlays/listener.rs)
  includes composed-path/nested/excluded-target classification, synchronous
  exactly-once disposal, request replacement and obsolete receiver rejection.
  Inspection also found the existing add-pad Escape transport in GUI AddInline.
  Its [app hook](../crates/inputforge-app/src/ui/overlays/escape.rs) and
  [script](../crates/inputforge-app/assets/rail_add_escape.js) preserve the filter
  exclusion and live-capture precedence. No extra engine listener, poller, thread
  or channel was introduced. The existing large add-pad markup was split into a
  bounded [rendering leaf](../crates/inputforge-gui-dx/src/frame/mapping_list/add_inline_rendering.rs).
- GUI [menus](../crates/inputforge-gui-dx/src/components/menu/mod.rs),
  [dialog markup](../crates/inputforge-gui-dx/src/components/dialog.rs) and
  [click-away wrapper](../crates/inputforge-gui-dx/src/components/click_away_listener.rs)
  retain signals, IDs, native event forwarding, attributes and rendering.
  Placement geometry and the stable outer [portal anchor](../crates/inputforge-gui-dx/src/components/portal.rs)
  are unchanged. [Drawer](../crates/inputforge-gui-dx/src/components/drawer.rs) is
  CSS/ARIA rendering only; its snapshot transition policy already moved in Part 4.

Regression-first real-handler failures reproduced premature confirmation closure,
menu arrow scrolling, stale closed-item activation and repeated closed-dialog Save.
Missing app focus/add-pad script and binding contracts were observed before their
extraction. [Real app/provider renderer tests](../crates/inputforge-gui-dx/tests/overlays.rs)
cover failure retention, action ordering, disabled/closed gates, native activation,
Tab/Escape focus, all dismissal/backdrop combinations, accessible ID relationships,
capture precedence, synchronous cleanup and obsolete owners across remount.
[Executable production scripts](../crates/inputforge-gui-dx/tests/overlays_listener.mjs)
cover disabled/removed DOM focus targets, nested/excluded/shadow click paths and
exactly-once listener cleanup/remount. Existing [portal lifecycle tests](../crates/inputforge-gui-dx/tests/portal_lifecycle.rs)
retain stable reopening anchors. Existing profile/mode/settings/snapshot/capture,
history, native-address and bridge lifecycle regressions remain in the full gate.
Fixtures now publish controlled-dialog readiness before simulating Escape and
account for the additional native cancel listener separately from keyboard owners;
no workflow-result or failed-delivery expectation was weakened.

Final automated checkpoint: **2,225 passed, 0 failed, 11 existing ignored** across
37 workspace suites. The unchanged strict pre-commit hook and all six existing
JavaScript suites plus the new overlay listener suite passed. Documentation links,
seven preserved historical evidence hashes and bounded production-module sizes
were checked. The default sandbox reproduced five existing single-instance IPC
permission failures; the complete host-enabled gate passed without exclusions.

Manual acceptance: run `dx run -p inputforge-app` on native Wayland and separately
Windows. Check profile/mode/stage menus with pointer/right-click/Shift+F10,
arrows/Home/End, disabled items, Enter/Space, Escape focus return and Tab traversal.
Check rejected menu actions and delete/restore/prune confirmations, Cancel/Escape,
backdrop policy, click-away exclusions and removed focus targets. Close/reopen
panels and menus; check add-pad live capture versus filter Escape and dirty/
destructive gallery dialogs. Native compositor focus, assistive technology,
hardware and performance acceptance remain unverified. Part 12 retains sortable
interaction; Part 13 removes transitional handles and performs the final ownership
inspection. Historical native Apply, capture and drag evidence above is unchanged.
