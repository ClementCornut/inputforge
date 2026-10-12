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

The audit was read-only. This document records findings and recommendations;
it does not implement the extractions below.

## Remaining responsibilities to move

The following areas are ordered by migration priority. Source links identify
the current implementations; line numbers describe the audited checkout and
may change during extraction.

| # | Area and evidence | App-owned responsibility |
| --- | --- | --- |
| 1 | **Engine access and polling** — [render entry point](../crates/inputforge-gui-dx/src/lib.rs), line 34; [bridge](../crates/inputforge-gui-dx/src/bridge.rs), line 16 | The renderer currently receives mutable engine state and a command sender, then polls every 16 ms. The app should supply prepared rendering data and callbacks. |
| 2 | **Routing and controller policy** — [engine button](../crates/inputforge-gui-dx/src/frame/top_bar/engine_pill/mod.rs), line 65; [controller session](../crates/inputforge-gui-dx/src/frame/panel_slot/controller_session.rs), line 16 | Start/Stop decisions, controller configuration changes, readiness gates, and pending-layout checks. Share these decisions with the tray controller so both entry points follow one policy. |
| 3 | **Capture workflows** — [live capture](../crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs), line 83; [keyboard capture](../crates/inputforge-gui-dx/src/patterns/keyboard_capture/mod.rs), line 323 | Capture ownership, state machines, input selection, keyboard translation, cancellation, and delivery. The GUI should render prompts, captured values, and errors. |
| 4 | **Mapping edits and undo** — [stage dispatch](../crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs), line 28; [undo history](../crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs), line 90 | Action-tree mutations, command construction, undo/redo, drag commits, and keyboard-nudge grouping. Curve/deadzone mutation algorithms also belong outside rendering. |
| 5 | **Batch mapping** — [entry generation](../crates/inputforge-gui-dx/src/frame/bulk_map/apply.rs), line 19; [Apply handler](../crates/inputforge-gui-dx/src/frame/bulk_map/mod.rs), line 335 | Automatic target selection, conflict decisions, wizard reconciliation, batch command construction, and snapshot creation requests. |
| 6 | **Profiles, modes, snapshots, and settings** — [profile actions](../crates/inputforge-gui-dx/src/frame/profiles/actions.rs), line 114; [snapshot settings](../crates/inputforge-gui-dx/src/frame/settings_panel/snapshots_section.rs), line 85 | CRUD workflows, validation, destructive-confirmation decisions, pruning calculations, optimistic state, and command dispatch. Reuse existing core validation rather than maintaining separate GUI rules. |
| 7 | **Operating-system services and filesystem access** — [file picker](../crates/inputforge-gui-dx/src/frame/profiles/mod.rs), line 30; [clipboard](../crates/inputforge-gui-dx/src/frame/panel_slot/device_panel/mod.rs), line 239; [file metadata](../crates/inputforge-gui-dx/src/context.rs), line 392 | Dialog invocation, clipboard writes, and metadata reads. External-profile metadata currently runs inside the polling loop while the engine read lock is held; move it to an app-owned service and cache it. |
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

1. Extract the engine bridge and shared routing policy.
2. Extract capture and editor controllers.
3. Extract the remaining workflows, services, event handling, and timers.

Move existing behavioral tests with the responsibilities they verify. Retain
rendering tests in the GUI. Preserve capture delivery, cancellation, undo,
confirmation, accessibility, and renderer lifecycle behavior during migration.

## Verification limits

This was a static architecture audit of the GUI structure and principal
workflow implementations, with targeted dependency-feature verification.
Live desktop behavior and runtime performance were not checked during this
audit. The recommendations are ownership changes; they are not evidence that
every listed workflow currently fails at runtime.
