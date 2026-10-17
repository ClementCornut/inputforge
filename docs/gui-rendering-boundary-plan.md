# GUI Rendering Boundary Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development after user approval. Each numbered part has a fresh implementation subagent, one commit, and a commit summary. Steps use checkbox syntax for tracking.

**Goal:** Finish the rendering-only boundary in `inputforge-gui-dx` while preserving existing editor behavior, persistence, accessibility and native lifecycle guarantees.

**Architecture:** Keep prepared rendering contracts, signals, markup, formatting, SVG geometry and portal placement in the GUI. Move workflow policy, event interpretation, state transitions, services and scheduling into small shared app modules. Core remains authoritative for processing, validation, persistence and command execution; reuse the existing app controllers and queue barriers.

**Tech Stack:** Rust 2024 / MSRV 1.99, Dioxus 0.7.10, Tokio, existing core engine and native services.

**Spec:** [GUI rendering boundary audit](gui-rendering-boundary-audit.md), particularly “Remaining responsibilities to move” and “Shared Windows and Linux architecture.”

**Baseline:** Clean `main` at `7cc08de` on 2026-10-16. Mode management is complete. Older audit sections describe their historical implementation state; they do not reopen completed slices. That planning baseline preceded implementation; the part status entries record subsequent execution.

## Global Constraints

- Planning approval is required before implementation. This file is the only planning change; do not include it in an extra standalone commit.
- One numbered part = one implementation subagent = one commit containing tests, implementation, caller updates and audit evidence. Work sequentially because contracts and fixtures overlap.
- After every commit, report the hash, scope, verification, remaining limitations and a short manual checklist, then automatically dispatch the next part sequentially. The user authorized this continuation on 2026-10-17; only a material decision blocker stops progress. Fix review findings before that part's commit.
- Recheck the working tree and current code before every dispatch. Preserve unrelated work. Stage exact owned paths only; never force-add ignored files. No push, merge or deployment.
- All workers are told they share the checkout, must not revert others' changes, and own only the files/responsibility assigned for their part.
- Work in this requested checkout on `main`, as explicitly requested by the user after plan approval. Do not create or switch branches or worktrees. Preserve unrelated edits if they appear.
- Regression tests come first. Preserve existing passing behavior with characterization tests; observe a failing test before fixing a defect or adding a missing app contract. Do not intentionally introduce a runtime failure to manufacture a red test.
- Use existing core validators; extract a narrowly reusable validator only where inline prevalidation would otherwise duplicate engine rules. No processing algorithm rewrite or persisted-format change.
- Failed validation or command/barrier delivery retains the draft/dialog, exposes accessible feedback, and performs no success announcement or selection/focus/navigation change. Delivery alone never proves save, prune or restore completion.
- Windows and Linux share workflow code. Native Wayland is primary; no direct X11 API, command or workaround. Existing transitive X11 support is acceptable.
- Keep the immediate sample, 16 ms nonblocking polling cadence, skipped contended reads, equality-gated publication, tray cadence, capture ownership, queue acknowledgement and mounted cleanup. Do not add another engine poller, thread, channel or generic workflow framework.
- Event interpretation belongs to the app even inside reusable components. GUI hooks may create/publish signals, register rendering references and wire supplied callbacks. Pure drawing/measurement and portal placement can remain GUI rendering mechanics.
- Keep touched/new production modules near the repository's 200-line baseline, excluding tests. Split a large moved algorithm by responsibility; avoid unrelated file cleanup.
- Every implementation commit updates `docs/gui-rendering-boundary-audit.md` with its ownership, actual test evidence, manual acceptance and remaining work. Preserve prior acceptance/Apply/drag evidence verbatim. Include this plan in part 1 and update its progress in the owning commits.
- Do not introduce missing product features merely because unused enums, state fields or core commands exist. Snapshot rename/pin/undo controls and the hidden calibration UI are not feature requests.

## Review Focus

1. A same-name replacement profile/mapping must not receive a stale draft, picker result, confirmation, drag or delayed callback. Test captured stable identity and current context in parts 1–7 and 9–12.
2. A command may be delivered before the next engine frame, or an engine save may subsequently fail. Test queued edits, old-frame reconciliation and honest feedback in parts 1–6 and 8–9.
3. A mounted scope can disappear or be replaced while a listener, picker, timer or gesture is active. Test synchronous cleanup and rejection of obsolete owners in parts 2–4 and 6–12.
4. Keyboard capture, text editing, menus and drag shortcuts can overlap. Test interception, propagation, focus and exactly-once actions in parts 1–4 and 9–12.
5. Sparse native controls and retained/disconnected devices must keep their identities. Test current readiness, native axis/input identity and no unintended Apply/Start in parts 2, 6 and 7.

## Common Wiring and Verification

The following paths are shared integration points, changed only when a part's contract requires them:

- App: `crates/inputforge-app/src/{lib.rs,desktop_host.rs,bridge/mod.rs}` and the relevant `bridge/{config,live,meta,settings}.rs` projections.
- GUI: `crates/inputforge-gui-dx/src/{lib.rs,app.rs,bridge.rs,context/mod.rs}` and prepared contracts beside `mapping_edits.rs`, `controllers.rs`, etc.
- Hosts/fixtures: `crates/inputforge-gui-dx/examples/{bridge_demo,component_gallery}.rs`, `tests/{editor_bridge,controller_feedback,live_capture_polling}.rs`, their support modules and affected inline fixture providers.
- Existing production dependency direction stays app → GUI contracts; GUI must not gain a production dependency on the app. Existing test-only canonical type aliases may be extended only as needed.

New workflow contracts follow the current `Bindings`/`View` and captured request/transition conventions in `mode_management.rs` and `batch_mapping.rs`. Extend the existing mapping `Controller` for mapping edits/instruments; use small domain modules for other workflows. Do not introduce an umbrella service bus or a second editor controller.

For every part:

- [ ] Read the assigned code and enumerate current call sites with Cartog before edits; report a material scope change before proceeding.
- [ ] Add behavioral regressions through the real app bindings and real GUI handlers. Run the focused cases; record expected failures before implementation where applicable.
- [ ] Move behavior and its tests, wire every caller/fixture, remove superseded GUI behavior, and simplify only within that responsibility.
- [ ] Run the focused tests and relevant existing lifecycle/capture/history/drag regressions.
- [ ] Run unchanged `bash .githooks/pre-commit`, `cargo test --workspace --locked --offline -j 2`, and `git diff --check` before the commit. Obtain host permissions if existing IPC tests are blocked by sandbox socket restrictions; do not silently exclude them or relax lints.
- [ ] Run `node crates/inputforge-app/tests/keyboard_capture/listener.mjs crates/inputforge-app/assets/keyboard_capture_listener.js`, `node crates/inputforge-gui-dx/tests/mapping_rail_listener.mjs`, and `node crates/inputforge-gui-dx/tests/sortable_drag.mjs`. Update paths together if ownership moves a script.
- [ ] Check audit links, prior evidence preservation and touched production-module sizes. Review the complete diff, including deleted/untracked files and manifest changes.
- [ ] Commit the exact owned paths using the scoped message below, then return the checkpoint so the next fresh implementation subagent can begin. Historical test totals are not current expectations.

No native hardware tests, desktop acceptance or performance claims follow from automated checks. Provide manual steps using `dx run -p inputforge-app`; the user performs native Wayland and separate Windows acceptance.

## Part 1: Remaining Editor Edit Intents and Inline Commits

**Status (2026-10-16):** Complete on `main`, with independent review approved after both rename findings were addressed. Final automated gates passed; native Wayland/Windows acceptance remains manual. Later parts have not started.

**Commit:** `refactor(editor): move remaining edit decisions into app`

**Files:** Extend `inputforge-app/src/mapping_edits/{prepare.rs,paths.rs,dispatch.rs}` with a focused `editor.rs` if needed; extend GUI `src/mapping_edits.rs`. Modify GUI `frame/mapping_editor/{header.rs,mod.rs}`, `pipeline/{add_palette.rs,stage_actions_menu.rs}`, `pipeline/stage_body/{change_mode.rs,map_to_keyboard.rs,map_to_mouse.rs,map_to_vjoy.rs,merge_axis.rs,predicate.rs,press_gesture.rs,gesture_body.rs,conditional.rs,branches.rs}`, and `frame/mapping_list/rename_inline.rs` where they interpret edits.

**Boundary:** App prepares action replacement/insertion, raw-field validation, no-op/commit/cancel decisions and draft completion. Retain display labels and rendering values in GUI. Reuse `Controller::dispatch`, the existing `Request`/`Edit`, typed stage paths, capture validation and private history; do not implement a parallel editing path.

**Callers:** Header rename/rebind; rail rename Enter/blur/Escape; Add-stage selection/capture; every stage-field handler; stage menu remove/move; pipeline dispatch adapters. Existing rail/mode/batch workflows remain the shared authority.

**Tests first:** Add `inputforge-app/tests/mapping_edits/editor.rs` through the existing test root; extend GUI `rename_tests.rs`, keyboard/capture event tests and editor handler tests. Cover unchanged/empty rename, exact optional-name preservation, a failed rail rename retaining its text and showing feedback, malformed numeric/gesture values, nested insertion paths, queued disjoint edits, stale stage/profile rejection, and no closure/local-key update/history on failed delivery. Preserve capture rearm and stale-prop/undo synchronization.

- [x] Complete the common test → implementation → validation → commit → checkpoint cycle.

## Part 2: Curve and Deadzone Editor Behavior

**Status (2026-10-16):** Complete on `main`; independent review cleared after focus-reconciliation fixes. Native Wayland/Windows manual acceptance remains pending.

**Commit:** `refactor(instruments): move curve and deadzone behavior into app`

**Files:** Create focused `inputforge-app/src/mapping_edits/instruments/` modules for curve edits, deadzone edits and interaction transitions. Modify GUI `frame/mapping_editor/pipeline/stage_body/{response_curve,deadzone}/{mod.rs,state.rs,mutation.rs,interaction.rs,keyboard.rs,toolbar.rs}` and `instruments/{bridge.rs,stage_dispatch.rs}`; keep drawing in their `rendering.rs`/`thumbnail.rs`. Add prepared instrument values/callbacks in GUI `src/mapping_edits.rs` or a small adjacent contract.

**Boundary:** Move curve mutation/symmetry/type conversion and deadzone constraints, drag draft lifecycle, keyboard interpretation, validation/revert and commit decisions. Keep core `ResponseCurve`/`DeadzoneConfig` validation and evaluation unchanged. Keep screen/viewbox transforms, SVG paths and hit-test measurements as rendering geometry; app callbacks consume their normalized observations and decide edits.

**Callers:** Both instrument bodies, toolbars, pointer bridge, keyboard nudges, stage previews and shared mapping dispatch/history. Use the existing controller for exactly one commit per completed drag and existing nudge grouping.

**Tests first:** Move mutation/interaction/keyboard behavior tests to app modules; retain drawing tests in GUI and add real handler regressions. Cover symmetric centers/endpoints, Bezier junctions, valid add/remove/type conversion, invalid/NaN values, deadzone ordering, pointer cancel and invalid-release revert, external undo during drag, stage deletion/replacement before release, failed dispatch preserving the working draft, nudge group boundaries, and unmount/remount with no late commit. Split the current large curve mutation module while moving it.

- [x] Automated common cycle: 2,143 workspace tests passed, 0 failed, 11 existing ignored; unchanged pre-commit and required JS suites passed. Independent review cleared.
- [ ] User acceptance: both graphs, numeric edits, symmetry, drag cancellation and undo on native Wayland and separately Windows.

## Part 3: Profiles and File Dialogs

**Status (2026-10-16):** Complete on `main`; independent review cleared after profile identity, native-path and metadata regressions were fixed. Final host workspace: 2,155 passed, 0 failed, 11 existing ignored. Native Wayland/Windows manual acceptance remains pending.

**Commit:** `refactor(profiles): move profile workflows and file dialogs into app`

**Files:** Create `inputforge-app/src/profiles/{mod.rs,policy.rs,dispatch.rs,dialog.rs}` as needed. Modify GUI `frame/profiles/{mod.rs,actions.rs,library.rs,new_profile.rs,new_profile_submode.rs,open_choice_submode.rs,no_profile.rs,delete_dialog.rs,projection.rs}`, profile-related `frame/view_state.rs`, and a new GUI `src/profiles.rs` contract. Add `rfd` to app dependencies and remove it from GUI when its final production caller moves. Reuse core `profile/{manager.rs,library.rs}` rather than moving persistence into GUI/app.

**Boundary:** App owns profile selection/create/open/import/add-to-library/rename/duplicate/delete/reveal decisions, inline validation and collision policy, filtering/selection policy, picker invocation and completion. Use current path/profile/origin identity, and the core's filename rules. Extract shared core name validation if necessary; remove the mirrored GUI illegal-character list.

**Callers:** Library row/menu/filter controls; New and Open-file affordances, including no-profile entry; create/duplicate/import forms; active external row; profile delete confirmation; top-bar profile entry. Keep snapshot actions working through their existing path until part 4.

**Tests first:** Add `inputforge-app/tests/profiles.rs` and real GUI profile-event cases alongside existing `frame/profiles/tests.rs`. Cover trimmed/illegal/duplicate/case-only names; external versus library namespaces; active row visibility under filtering; cancelled picker; in-library open versus external choice; picker completion after close/unmount/newer request; profile replacement and stale delete/rename; failed delivery preserving forms/confirmation/focus; and unchanged filesystem error handling. Keep existing core temporary-directory persistence tests.

- [x] Complete the common automated validation, review and commit cycle. Manual acceptance remains: create/import/load once/library rename/duplicate/delete and picker cancellation with disposable profiles.

## Part 4: Snapshot Workflows and Snapshot Shortcut

**Status (2026-10-16):** Complete on `main`; independent review cleared after native-focus and queued-context safeguards were added. Native Wayland/Windows restore safety and shortcut acceptance remain manual.

**Commit:** `refactor(snapshots): move snapshot actions and shortcuts into app`

**Files:** Add `inputforge-app/src/profiles/snapshots.rs` and a small shortcut/listener module/asset. Modify GUI `frame/profiles/{snapshot_drawer.rs,actions.rs}`, snapshot fields in `frame/view_state.rs`, `app.rs`, the profiles contract and associated tests. A narrow core captured-snapshot request and pre-operation identity guard cover queued profile switches while delegating to existing snapshot/recovery handlers. Keep core `snapshot/` and engine recovery execution authoritative.

**Boundary:** App owns manual snapshot, restore/delete policy and available existing snapshot actions, active-profile/snapshot revalidation, confirmation decisions, drawer transitions and Ctrl+S interpretation/listener lifecycle. GUI renders prepared rows/availability/dialog state and wires callbacks. Preserve declared destructive confirmation semantics with existing confirmation primitives; do not treat dormant rename/pin/undo contracts as permission to add new controls.

**Callers:** Snapshot-now button, active-profile snapshot entry, drawer toggle, each Restore/Delete row, current confirmation/cancel paths and root Ctrl+S listener. Core recovery snapshots from batch/restore stay in core.

**Tests first:** Add `inputforge-app/tests/snapshots.rs`, real drawer-handler cases and an executable listener test. Cover no active profile, protected session-start rows, missing/replaced snapshot and profile, restore confirmation, cancellation, failed send without closure/success, and core restore failure warnings. Exercise Ctrl+S outside versus inside text inputs/rename/menu/dialog/OS-picker return, repeat behavior and keyboard-capture interception; verify one listener, synchronous teardown and fresh remount.

- [x] Automated gates: 2,167 workspace tests passed, 0 failed, 11 existing ignored; unchanged pre-commit and all five required JavaScript suites passed.
- [x] Independent review cleared; single Part 4 commit checkpoint.
- [ ] User acceptance: restore safety/recovery, drawer actions, focus/capture and Ctrl+S on native Wayland and separately Windows.

## Part 5: Settings and Prune Confirmation

**Status (2026-10-17):** Complete on `main`; independent read-only review cleared with no actionable Critical/Important findings. Native Wayland/Windows acceptance remains manual.

**Commit:** `refactor(settings): move settings validation and pruning into app`

**Files:** Add `inputforge-app/src/settings/{mod.rs,policy.rs,dispatch.rs}` as needed and GUI `src/settings.rs` contract. Modify GUI `frame/settings_panel/{snapshots_section.rs,startup_section.rs,gestures_section.rs,prune_confirm.rs,mod.rs}` and the settings bridge projection. Reuse `SnapshotConfig`, gesture validators and existing autostart engine commands/services.

**Boundary:** App owns parsing/validation, no-op detection, switch state transitions, pending settings reconciliation, prune calculation/confirmation and dispatch. Confirm against current profile identity, unpinned count and current settings; stale confirmation must not authorize a changed destructive operation. Preserve snapshot limits 1–100 and existing gesture policies. A delivered config request must not announce that files were removed before execution establishes it.

**Callers:** Snapshot buffer Enter/blur/invalid, skip-unchanged switch, prune Confirm/Cancel/Escape, startup/autostart switches and both gesture threshold controls.

**Tests first:** Add `inputforge-app/tests/settings.rs` plus real settings/prune handlers. Cover 0/1/100/101, malformed text, unchanged commits, pinned exclusions, changed counts/profile during confirmation, rapid toggles before polling, cross-field updates without lost values, failed send retaining state/feedback, cancelled prune, and backend save/prune failures without false success. Keep autostart and start-minimized behavior unchanged.

- [x] Automated gates: 2,176 workspace tests passed, 0 failed, 11 existing ignored; unchanged pre-commit, all five required JavaScript suites, audit links and seven historical evidence hashes passed.
- [x] Independent review cleared; single Part 5 commit checkpoint.
- [ ] User acceptance: saved settings after relaunch, startup/tray switches and pruning only in disposable profiles on native Wayland and separately Windows.

## Part 6: Device Inspector and Native Clipboard

**Status (2026-10-17):** Complete on `main`; independent read-only review cleared with no Critical/Important findings. Native Wayland/Windows acceptance remains manual.

**Commit:** `refactor(devices): move inspector workflows and clipboard into app`

**Files:** Extend `inputforge-app/src/controllers/` with a focused device-inspector module and clipboard service; extend GUI `src/controllers/{mod.rs,inspector.rs}` (moving the former `controllers.rs` to satisfy the existing module-layout lint). Modify GUI `frame/panel_slot/device_panel/{mod.rs,inspector.rs}`, its selection/draft tests, both crate manifests, workspace `Cargo.toml` and `Cargo.lock` only if feature resolution changes.

**Boundary:** App owns initial/changed device selection, alias draft commit/cancel, removal guards and dispatch, report-copy invocation and feedback. Pure report display formatting may stay in GUI; pass the current device identity and rendered report text to the app callback. Move `arboard` ownership to the app and enable the installed dependency's native Wayland path (`wayland-data-control`) for Linux; keep platform differences at this service. Inspect installed source/features first and test error/clipboard-lifetime behavior; do not add an X11 fallback command.

**Callers:** Device ledger selection, alias input/Save/Enter/Escape, Remove device and Copy report. Selection/layout/Apply and advanced-axis controller bindings remain shared and unchanged in behavior.

**Tests first:** Add app/controller device cases and real inspector handlers. Cover connected-first fallback, removed selection, stale/recreated device/profile, alias trim/no-op/cancel, failed save retaining the draft, disconnected-only removal with primary/nested secondary references, reference changes before click, clipboard success/failure and exactly one write. Verify the resolved Linux Wayland feature with Cargo; native compositor acceptance remains separate.

- [x] Automated gates: 2,186 workspace tests passed, 0 failed, 11 existing ignored; unchanged pre-commit, all five JavaScript suites, resolved Linux Wayland feature, audit links and seven historical evidence hashes passed.
- [x] Independent read-only review cleared; single Part 6 commit checkpoint.
- [ ] User acceptance: native copy/paste without an XWayland clipboard dependency, separately Windows, alias/removal workflows and Stop/edit/Apply/Start.

## Part 7: Prepared Live Readouts and Activity Expiry

**Status (2026-10-17):** Complete on `main`; independent review cleared after the contended selection/reset finding was addressed regression-first. Final host workspace:2,192 passed,0 failed,11 existing ignored; unchanged strict precommit and all five JS suites passed. Native Wayland/Windows acceptance remains manual. User authorized automatic sequential continuation after commit summaries.

**Commit:** `refactor(readouts): prepare live analysis and activity in app`

**Files:** Add focused `inputforge-app/src/bridge/readout/` modules and tests. Move analysis behavior from GUI `frame/mapping_editor/live_readout/{analyzer.rs,value_helpers.rs}` and stage live evaluation from `pipeline/stage_body/instruments/live_axis.rs`; modify GUI `live_readout/{mod.rs,in_block.rs,out_block.rs,out_chain.rs,predicate.rs}` and `snapshots/{config.rs,live.rs}` to consume prepared values.

**Boundary:** App computes merge/conditional/gesture chains, predicates, output ownership/polarity/observed activity and instrument tracking values under the existing coherent engine read. GUI draws/formats the completed model. Move latch/expiry and expansion-reset decisions to the app; preserve existing durations/reset keys and expiry when no new activity arrives. Use existing bridge ticks for deadlines without adding a second engine read or a per-render timer; keep expiry progressing during contended engine ticks.

**Callers:** Selected mapping readout, expanded merge chains, keyboard/mouse/vJoy destinations, predicate rows, response-curve/deadzone tracking dots, and any current scalar evaluation consumer discovered by the caller pass.

**Tests first:** Move analyzer/walker and latch behavior tests; add paused-time bridge/renderer cases for nested branches, stacked/sibling merges, depth limits, disconnected/unbound/sparse controls, unchanged activity not extending expiry, expiry without new input/under contention, profile/mapping/action-path replacement, selection/expansion reset, and no duplicate schedules after remount. Compare actual values with existing core evaluation fixtures.

- [x] Common automated validation, independent review and single-commit cycle complete.
- [ ] User acceptance: live dots, predicate/merge rows, mode/profile resets and short output pulses on native Wayland and separately Windows.

## Part 8: Notifications and Warning Delivery

**Status (2026-10-17):** Complete on `main`; independent review cleared after fixing the bridge-demo deadline driver. Native Wayland/Windows acceptance remains manual.

**Commit:** `refactor(notifications): move toast behavior and scheduling into app`

**Files:** Add `inputforge-app/src/notifications/{mod.rs,state.rs}` and focused tests; modify GUI `toast/{mod.rs,state.rs,queue.rs,viewport.rs,warnings_bridge.rs}`, `app.rs`, current toast producer call sites and host/gallery fixtures. Keep rendered notification values/callback contracts in GUI, with app-owned queue state.

**Boundary:** Move deduplication, capacity/ordering, warning-tail tracking, pause/resume/dismiss/Escape, expiry and existing action delivery. Replace GUI queue mutation/timer behavior with supplied callbacks and a prepared visible list. Reuse app deadline handling introduced in part 7/existing bridge rather than another polling loop. Preserve five visible notifications, eight-second effective lifetime, chronological order, coalescing, pause accounting and alert/status accessibility.

**Callers:** Every existing toast producer, snapshot/settings feedback, warning bridge, notification hover/focus/Escape/dismiss and any actually wired notification action. Keep inline workflow feedback available independently of notification expiry.

**Tests first:** Move state/queue behavior tests and replace wall-clock sleeps with injected/paused time. Cover latest-only dedupe, expired duplicates, FIFO cap, interleaved hover/focus pause, resume/dismiss idempotence, warning list truncation/reset/remount, backend failures after delivered requests, stale action targets, no duplicate warning delivery, and no notifications/timer updates after disposal. Preserve rendered ARIA roles and callback delivery tests.

- [x] Automated gates: 2,193 workspace tests passed, 0 failed, 11 existing ignored; unchanged strict pre-commit, all five JavaScript suites, 251 local audit/plan links and seven historical evidence hashes passed.
- [x] Independent review cleared after the actual demo-host expiry regression and fix; single Part 8 commit checkpoint.
- [ ] User acceptance: warning delivery, chronological coalescing/overflow, keyboard dismissal, paused expiry, persistent inline feedback and gallery/demo teardown on native Wayland and separately Windows.

## Part 9: Editor Navigation and Remaining Shortcuts

**Commit:** `refactor(navigation): move editor transitions and shortcuts into app`

**Files:** Add focused `inputforge-app/src/navigation/` modules; modify GUI `frame/view_state.rs`, `frame/mapping_editor/{mod.rs,keyboard.rs}`, `frame/mapping_list/{keyboard.rs,filter.rs,group.rs,row.rs}`, `frame/top_bar/{primary_nav.rs,tools_cluster/mod.rs,tools_cluster/logic.rs,profile_name.rs}`, `frame/layout/` and navigation-related `frame/panel_slot/mod.rs` handlers. Include current mapping/pipeline expansion, selection, menus, focus-request and draft invalidation callers found during the caller pass.

**Boundary:** App owns panel/surface/filter transitions, profile initialization/reconciliation, selected mapping and editor expansion/draft reset policy, shortcut interpretation and listener lifecycle. Use stable profile identity rather than profile display name (current `reconcile_pure` uses the name). Extend the current navigation publication/mapping bindings; GUI retains signals and DOM focus execution for app-selected targets. Do not duplicate mode navigation authority or capture interception.

**Callers:** Mapping/batch tabs, rail selection/filter and keyboard navigation, editor undo/redo/reorder shortcuts, side-panel tools and profile entry, mode reconciliation, empty-state navigation, host/tray Review navigation and teardown.

**Tests first:** Add app navigation transitions and real DOM/listener cases. Cover same-name different profiles, pending mode mutation through old frames, missing selected mapping/mode, active-panel toggle, input/modal/capture focus scopes, nested stage shortcuts, Escape precedence, pointer-drag keyboard deferral, teardown/remount, and disposed/obsolete focus requests. Preserve the existing no-profile Devices gate; do not bundle its product redesign.

**Status:** Complete on `main` — 2026-10-18. Final workspace gate: **2,196 passed, 0 failed, 11 existing ignored** across 34 suites. The unchanged strict pre-commit hook, all six executable JavaScript suites, 258 local links, seven historical evidence hashes and new-module size checks passed. Independent review found two stale-event defects; regression-first fixes and same-seat re-review cleared both (Spec PASS / Quality SHIP). Native keyboard/focus, drafts/capture, Wayland/Windows and performance acceptance remain manual.

- [x] Complete the automated common cycle and independent review; hand off keyboard-only navigation and profile switching during drafts/capture for native manual acceptance.

## Part 10: Form and Tab Component Behavior

**Commit:** `refactor(controls): move input and tab behavior into app`

**Files:** Add small app-owned `ui/inputs.rs` and `ui/tabs.rs` behavior modules and a GUI callback contract beside `components/mod.rs`. Modify GUI `components/{integer_input.rs,number_input.rs,text_input.rs,select.rs,segmented_control.rs,slider.rs,checkbox.rs,switch.rs,tabs.rs,tabs_root.rs,tabs_list.rs,tab_button.rs}`, affected consumers and gallery providers. Components that only forward native events need no behavioral rewrite.

**Boundary:** Move parsing/clamping, stepper/Enter/blur commit rules, raw draft synchronization, propagation decisions, tab keyboard coordination, disabled-item policy and focus-target selection. Keep DOM attributes, formatting and mounted references in GUI. App supplies behavior callbacks through the existing host/provider pattern; do not leave a second default implementation in GUI or add a production GUI → app dependency.

**Callers:** Settings, controller layouts, editor numeric/gesture fields, profile forms, mode tabs, primary tabs, reusable component gallery and all direct component fixtures. Update each caller in the same commit so the app remains usable.

**Tests first:** Move meaningful parser/control/tab tests and add actual-handler cases for malformed/empty/nonfinite input, boundary steps, Enter followed by blur dispatching once, Escape restore, external value updates without draft loss, disabled controls, text arrow/Home/End isolation with Enter/Escape bubbling, tab wrapping/disabled/removed items and repeated focus requests. Keep labels, described-by links and ARIA tests in GUI.

**Status:** Complete on `main` — 2026-10-18. Final workspace gate: **2,216 passed, 0 failed, 11 existing ignored** across 36 suites. The unchanged strict pre-commit hook, all six executable JavaScript suites, 265 local links, seven historical evidence hashes and new-module size checks passed. Independent review and regression-first fixes cleared numeric acknowledgement, optional callback and step/commit ordering cases (Spec PASS / Quality SHIP). Native text editing, keyboard/focus, Wayland/Windows, accessibility and performance acceptance remain manual.

- [x] Complete the automated common cycle and independent review; hand off native text editing, keyboard tabs and numeric controls for manual acceptance.

## Part 11: Menus, Dialogs and Dismissal Behavior

**Status (2026-10-18):** Implemented on `main`; final host workspace: 2,225 passed, 0 failed, 11 existing ignored across 37 suites. Strict pre-commit, six existing JS suites and the new overlay listener suite pass. Native Wayland/Windows acceptance remains manual.

**Commit:** `refactor(overlays): move menu and dialog behavior into app`

**Files:** Add focused app `ui/overlays.rs`/listener modules; modify GUI `components/{menu/mod.rs,menu/focus_walker.rs,dialog.rs,click_away_listener.rs,drawer.rs}`, `patterns/{destructive_confirm.rs,dirty_confirm.rs}`, affected consumers, `tests/portal_lifecycle.rs` and gallery fixtures. Leave `components/portal.rs` placement mechanics in GUI unless its own audit finds a behavioral decision.

**Boundary:** Move open/close/dismissal policy, menu keyboard interpretation, focus walking/return decisions, click-away classification and document listener ownership. GUI executes supplied focus/visibility instructions and retains portal geometry. Keep the existing stable outer portal anchor and failure-aware menu activation-close gate.

**Callers:** Profile/mode/stage menus, delete/restore/prune confirmations, drawers, click-away wrappers and component gallery. Dismissal must follow workflow results rather than closing before a rejected/failed action.

**Tests first:** Add app overlay transitions and real-handler/listener lifecycle cases for arrows/Home/End, disabled items, Enter/Space/Escape/Tab, right-click/Shift+F10, nested/excluded click-away targets, backdrop/dismissible combinations, failed action retaining overlay, removed focus target, stable portal anchoring and exactly-once listener cleanup/remount. Preserve accessible relationships and keyboard-only focus return.

- [x] Regression-first common cycle: app overlay hooks and real-handler/listener tests, failure-aware activation/dismissal, bounded render leaves and caller/gallery updates. Inspection also moved the existing add-pad Escape transport into the app while preserving capture/filter precedence. Drawer and stable portal anchoring remain pure rendering. Independent review is recorded in the owning checkpoint.
- [ ] User acceptance: menus/dialogs across Profiles, Modes, Settings and the editor on native Wayland and separately Windows.

## Part 12: Shared Sortable Interaction

**Commit:** `refactor(sortable): move drag interaction policy into app`

**Files:** Add focused app `ui/sortable/` behavior/lifecycle modules; modify GUI `components/sortable/{state.rs,interaction.rs,handle.rs,gap.rs,live_region.rs}`, `frame/mapping_list/` and `frame/mapping_editor/pipeline/{dnd.rs,mod.rs,stage.rs}` callers. Move behavior-bearing browser transport ownership from `assets/components/sortable_drag.js` to app assets, updating fixtures/asset references together; keep visual geometry/ghost drawing with rendering where practical.

**Boundary:** App owns gesture state, identity/context cancellation, drop eligibility/no-op decisions, keyboard/pointer coordination, listener lifecycle and announcements. Existing app rail/pipeline dispatch remains the final revalidation authority. Preserve hit geometry, ghost appearance, wheel/edge scrolling and browser animation used for drawing; it is not a new engine poller. Prefer moving the existing script intact over rewriting proven transport.

**Callers:** Both shared consumers: mapping groups/cards and nested/empty pipeline stage groups, including their keyboard transports and live announcements. Update gallery sortables and test providers in the same commit.

**Tests first:** Keep and run the executable pointer/ghost suite before changes; add failing ownership/cleanup cases where needed and move state-transition tests. Cover unequal card halves/gaps, first/last and adjacent no-op gaps, cross-group rejection, stationary target updates during wheel/edge scrolling, release-click suppression, Escape/blur/capture loss, source removal/reuse, profile/mode/action-tree replacement, shutdown/remount, ghost sanitization and exactly-once dispatch/undo.

- [ ] Complete the common cycle; manually repeat the audit's full pointer/ghost/scroll checklist on Wayland and separately Windows.

## Part 13: Remove Transitional Engine Access and Close the Audit

**Commit:** `refactor(gui): remove transitional engine handles from renderer`

**Files:** Modify GUI `src/{lib.rs,app.rs,context/mod.rs,bridge.rs}` and its README, app `src/{desktop_host.rs,bridge/mod.rs}`, every `render_editor` caller/example/integration fixture, both crate manifests as justified by actual remaining uses, this plan and `docs/gui-rendering-boundary-audit.md`.

**Boundary:** Remove `RawHandles`, `AppContext.state`, `AppContext.commands` and their renderer API/context plumbing. Remaining GUI context consists of prepared rendering data/signals and supplied callbacks. Remove unused GUI service/runtime dependencies only after confirming their final use moved; retain core value types and Dioxus rendering support. No compatibility raw-handle escape hatch or dormant engine-dispatch fallback.

**Callers:** Production desktop host; bridge demo; controller-feedback, editor-bridge and live-capture-polling fixtures; all root/inline providers and gallery setup. Refresh Cartog before final ownership inspection if its tools report stale data.

**Tests first:** Change the real renderer integration fixture to construct the editor using prepared values and bindings without any engine state/sender input, observing the missing API before removal. Extend the existing bridge lifecycle test to cover the assembled contracts: immediate/16 ms samples, contention skip/recovery, equality suppression, one installation, scope disposal and remount. Retain the complete capture/history/mode/batch/drag suite.

**Closure check:** Inspect all production GUI modules and browser assets for engine reads/commands, processing/workflow policy, direct OS/filesystem services, listeners, state-transition decisions and scheduling. Classify remaining geometry/formatting/portal behavior explicitly; do not mark #8 complete while reusable component policy remains. Any undiscovered substantive responsibility needs a reported plan adjustment rather than being hidden in this cleanup commit.

- [ ] Complete the common cycle and full ownership review. Mark each audit finding resolved only with its evidence; preserve native/manual/performance limitations and historical acceptance text.
- [ ] Final checkpoint: report all 13 hashes, actual checks, native acceptance still pending and the resulting Git state. Stop without pushing or merging.

## Audit Coverage

| Audit finding | Planned closure |
| --- | --- |
| #1 Engine access | Each workflow removes its own reads/sends; part 13 removes the transitional contract. |
| #2 Controller/configuration policy | Parts 1, 5, 6 and 9 finish remaining gates; completed selection/layout/Apply/axes stay shared. |
| #3 Capture | Completed capture controllers remain; parts 1, 2 and 9–12 finish their consumer event/draft behavior. |
| #4 Editor/mapping behavior | Parts 1–2, 7 and 9–12. |
| #5 Batch mapping | Already complete; preserve it across navigation/component changes. |
| #6 Profiles/snapshots/settings | Parts 3–5; modes already complete. |
| #7 OS/filesystem services | Profile dialogs in part 3, clipboard/Wayland in part 6; existing core persistence and app metadata cache remain authoritative. |
| #8 Event handling/state transitions | Workflow-local events in parts 1–8, navigation in 9, reusable controls/overlays/sortable in 10–12. |
| #9 Readout analysis | Part 7. |
| #10 Notifications | Part 8. |

The intended simplification is deletion of duplicate GUI policy and transitional plumbing. New features, renderer redesign, core processing changes, speculative abstraction, packaging work and runtime performance tuning are outside this plan.
