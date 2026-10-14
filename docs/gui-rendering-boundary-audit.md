# GUI rendering boundary audit

Date: 2026-10-12

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
implemented routing, polling, controller-workflow, advanced-axis, live-capture, and keyboard-capture extractions;
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
- [GUI controller contract](../crates/inputforge-gui-dx/src/controllers.rs),
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
- The existing [controller contract](../crates/inputforge-gui-dx/src/controllers.rs)
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

## Remaining responsibilities to move

The following areas are ordered by migration priority. Source links identify
the current implementations; line numbers describe the audited checkout and
may change during extraction.

| # | Area and evidence | App-owned responsibility |
| --- | --- | --- |
| 1 | **Engine access — partially resolved** — [render entry point](../crates/inputforge-gui-dx/src/lib.rs); [app polling bridge](../crates/inputforge-app/src/bridge/mod.rs); [GUI publication](../crates/inputforge-gui-dx/src/bridge.rs) | Polling and snapshot preparation are app-owned. Controller selection/layout/Apply, advanced axis settings, live capture, and keyboard capture now use app-owned handlers. Other GUI workflow handlers still receive raw engine state and a command sender; extract those with their respective workflows. |
| 2 | **Controller editing policy — partially resolved** — [app controller policy](../crates/inputforge-app/src/controllers/policy.rs); [app axis settings](../crates/inputforge-app/src/controllers/axes.rs); [axis rendering](../crates/inputforge-gui-dx/src/frame/panel_slot/axis_settings.rs) | Selection, virtual-layout mutations, Apply, shared routing decisions, and advanced axis settings are app-owned. Other configuration-editing/readiness gates remain; see resolved status above. |
| 3 | **Capture workflows — partially resolved** — [app live capture](../crates/inputforge-app/src/live_capture/mod.rs); [GUI capture contract](../crates/inputforge-gui-dx/src/capture.rs); [app keyboard capture](../crates/inputforge-app/src/keyboard_capture/mod.rs); [GUI keyboard contract](../crates/inputforge-gui-dx/src/keyboard_capture.rs) | Live and keyboard capture, physical keyboard translation, ownership, target revalidation, cancellation, interception and delivery are app-owned; live input additionally owns epoch/native confirmation. Add/collision decisions and mapping edits/undo remain with their deferred workflows. |
| 4 | **Mapping edits and undo** — [stage dispatch](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs), line 28; [undo history](../crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs), line 90 | Action-tree mutations, command construction, undo/redo, drag commits, and keyboard-nudge grouping. Curve/deadzone mutation algorithms also belong outside rendering. |
| 5 | **Batch mapping** — [entry generation](../crates/inputforge-gui-dx/src/frame/bulk_map/apply.rs), line 19; [Apply handler](../crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs), line 335 | Automatic target selection, conflict decisions, wizard reconciliation, batch command construction, and snapshot creation requests. |
| 6 | **Profiles, modes, snapshots, and settings** — [profile actions](../crates/inputforge-gui-dx/src/frame/profiles/actions.rs), line 114; [snapshot settings](../crates/inputforge-gui-dx/src/frame/settings_panel/snapshots_section.rs), line 85 | CRUD workflows, validation, destructive-confirmation decisions, pruning calculations, optimistic state, and command dispatch. Reuse existing core validation rather than maintaining separate GUI rules. |
| 7 | **Operating-system services and filesystem access** — [file picker](../crates/inputforge-gui-dx/src/frame/profiles/mod.rs), line 30; [clipboard](../crates/inputforge-gui-dx/src/frame/panel_slot/device_panel/mod.rs), line 239; [cached file metadata](../crates/inputforge-app/src/bridge/profile_metadata.rs) | Dialog invocation, clipboard writes, and other OS services remain. Polling profile metadata is now app-owned, cached, and read after releasing the engine lock. Other filesystem-service ownership remains open. |
| 8 | **Event handling and UI state transitions** — [snapshot shortcut](../crates/inputforge-gui-dx/src/frame/profiles/snapshot_drawer.rs), line 306; [navigation reconciliation](../crates/inputforge-gui-dx/src/frame/view_state.rs), line 244; [sortable drop handler](../crates/inputforge-gui-dx/src/components/sortable/gap.rs), line 163 | Document listeners, shortcut interpretation, focus navigation, click-away behavior, drag/drop decisions, draft commits, and selection reconciliation. This includes reusable component behavior under the strict rendering-only boundary. |
| 9 | **Live readout analysis** — [analyzer](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/analyzer.rs), line 146; [activity timer](../crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout/mod.rs), line 100 | Pipeline interpretation, activity calculations, and expiry scheduling. Supply a completed readout model to the renderer. |
| 10 | **Notifications** — [toast queue](../crates/inputforge-gui-dx/src/toast/queue.rs), line 26; [toast timer and handlers](../crates/inputforge-gui-dx/src/toast/viewport.rs), line 27 | Warning delivery, notification expiry, pause/resume, dismissal, and undo actions. The GUI should display the current notifications and wire supplied callbacks. |

## Concrete Wayland parity gap

Clipboard support currently uses `arboard` with its Wayland feature disabled.
The audit checked the app's resolved Cargo features and the installed dependency
source: this configuration selects its X11 backend. Copying a device report
therefore requires X11 clipboard access.

Evidence:

- [Workspace dependency configuration](../Cargo.toml), line 95, specifies
  `arboard` 3.6.1 with `default-features = false` and no Wayland feature.
- [GUI dependency configuration](../crates/inputforge-gui-dx/Cargo.toml)
  inherits that dependency without enabling Wayland support.
- The device-report copy handler constructs `arboard::Clipboard` directly.
- `cargo tree --locked --offline -p inputforge-app -e features -i arboard`
  showed no enabled Wayland feature.
- Installed `arboard` 3.6.1 gates its Wayland backend behind
  `wayland-data-control`; without that feature, its Linux constructor selects
  the X11 backend.

An app-owned clipboard service needs a Wayland-capable path. This does not
require direct X11 code. Enabling a dependency feature alone does not establish
compositor compatibility; native Wayland acceptance remains necessary.

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
2. Live and keyboard capture primitives are complete. Extract the remaining mapping/editor controllers and undo workflows next.
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
The previous desktop reports are user-reported native acceptance;
the timestamp correction, separate Windows acceptance, and runtime performance
measurements remain unverified on native hardware. The recommendations are ownership
changes; they are not evidence that every listed workflow currently fails at runtime.
