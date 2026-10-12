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
implemented routing and polling extractions; the remaining findings are still open.

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

**Still open in #2:** controller selection, configuration edits, virtual-layout
Apply, and configuration-editing/readiness gates outside these routing controls.
**#1 is partially resolved:** polling and snapshot preparation now belong to
the app; see the first slice below. GUI workflow handlers still receive engine
handles and commands while their ownership remains to be extracted.
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
workflow handlers, including capture revalidation. This slice does not complete
finding #2's controller editing/Apply ownership or findings #3–10. Finding #7's
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

## Remaining responsibilities to move

The following areas are ordered by migration priority. Source links identify
the current implementations; line numbers describe the audited checkout and
may change during extraction.

| # | Area and evidence | App-owned responsibility |
| --- | --- | --- |
| 1 | **Engine access — partially resolved** — [render entry point](../crates/inputforge-gui-dx/src/lib.rs); [app polling bridge](../crates/inputforge-app/src/bridge/mod.rs); [GUI publication](../crates/inputforge-gui-dx/src/bridge.rs) | Polling and snapshot preparation are app-owned. Remaining GUI workflow handlers still receive raw engine state and a command sender; extract those with their respective workflows. |
| 2 | **Controller editing policy — partially resolved** — [controller session](../crates/inputforge-gui-dx/src/frame/panel_slot/controller_session.rs); [virtual configuration](../crates/inputforge-gui-dx/src/frame/panel_slot/virtual_config.rs) | Remaining: controller selection and configuration mutations, Apply, and configuration-editing/readiness gates. Shared Start/Stop/Retry/Refresh decisions and routing pending-layout review are app-owned; see resolved status above. |
| 3 | **Capture workflows** — [live capture](../crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs), line 83; [keyboard capture](../crates/inputforge-gui-dx/src/patterns/keyboard_capture/mod.rs), line 323 | Capture ownership, state machines, input selection, keyboard translation, cancellation, and delivery. The GUI should render prompts, captured values, and errors. |
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

1. Polling/snapshot preparation and shared routing controls are complete.
   Extract remaining workflow engine access alongside controller
   selection/configuration/Apply policy as a separate part.
2. Extract capture and editor controllers.
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
UTC timestamp display corrected above. That is user-reported native acceptance;
the timestamp correction, separate Windows acceptance, and runtime performance
measurements remain unverified on native hardware. The recommendations are ownership
changes; they are not evidence that every listed workflow currently fails at runtime.
