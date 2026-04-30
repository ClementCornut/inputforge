# egui → Dioxus Rewrite, Master Plan

**Status:** Design approved, ready for per-feature planning
**Date:** 2026-04-24
**Scope:** Replace the immediate-mode egui configuration GUI with a declarative Dioxus Desktop GUI, incrementally, on `main`.

## Motivation

The current `inputforge-gui` crate (~6,200 lines across panels and widgets) uses `egui` in immediate mode. Working in it has four dominant pain points:

1. **Paradigm friction**, immediate mode tangles state and rendering, making it hard to reason about "what should be on screen for a given state".
2. **Styling**, fighting egui for visual polish.
3. **Layout**, widget positioning and composition are awkward for non-trivial compositions.
4. **LLM productivity**, AI-assisted edits to egui code produce incorrect APIs and patterns at high rates.

The GUI is a configuration/testing surface. It is closed when the app is running normally, so runtime performance is not a constraint, developer and LLM ergonomics dominate.

A secondary motivation: the existing UI has structural UX problems that are better addressed during a rewrite than retrofitted. The rewrite is also a UX redesign.

## Constraints & Principles

- **100% Rust.** No Node.js, npm, or JS frontend toolchain.
- **All work lands on `main`** as incremental, shippable merges. No long-lived branch.
- **egui stays the default GUI** until the Dioxus version reaches parity across all core and secondary surfaces. Per F5, the default feature flag flips at the end of **F13** (Profiles side panel + Snapshots + no-profile empty state), that is the point at which all core and secondary surfaces are in place and the GUI is shippable.
- **Per-feature focused plans** will be written separately (via `superpowers:writing-plans`). This document is a master plan, not an implementation plan.
- **UX-first per feature.** Each feature invokes `impeccable:frontend-design` as part of its focused plan. A dedicated audit/polish pass runs at F16.
- **Preserve the engine contract.** The Dioxus GUI reuses the existing `Arc<RwLock<AppState>>` read path and `mpsc::Sender<EngineCommand>` write path without changes to the engine side.

## Framework Choice: Dioxus Desktop

Dioxus Desktop was selected over Slint after weighing the four pain points:

| Pain | Dioxus Desktop | Slint |
|---|---|---|
| Declarative paradigm | RSX + Signals/Context | Declarative DSL |
| Styling | Real CSS (flexbox, grid, pseudo-classes) | Custom styling syntax |
| Layout | CSS flexbox/grid, heavily represented in LLM training | Slint layout primitives |
| LLM familiarity | JSX + CSS + SVG, excellent LLM fluency | `.slint` DSL is niche |

Dioxus Desktop uses WebView2 under the hood. On Windows 10+ (the project's target), WebView2 is a system-provided component, bundle impact is minimal, and it is pre-installed on modern systems. This tradeoff was considered acceptable in exchange for the LLM-productivity and styling wins.

**Custom widgets** (curve editor, axis bars, hat indicator, calibration visualization, input viewer) will be implemented as **SVG** with event handlers. SVG is one of the strongest LLM output targets.

**State flow** maps cleanly:
- Read path: `Arc<RwLock<AppState>>` placed in Dioxus context; a background task snapshots state into a reactive Signal at ~60Hz for live visualizations.
- Write path: `mpsc::Sender<EngineCommand>` in context, invoked from event handlers.
- Per-frame egui snapshots become explicit Signal updates; no semantic change to the engine contract.

## Migration Strategy

**Parallel-crate, feature-flag switchover.**

1. Create `crates/inputforge-gui-dx` alongside the existing `crates/inputforge-gui`.
2. Add `gui-egui` (default) and `gui-dioxus` cargo features to `inputforge-app`. `launch_gui` dispatches to the selected crate at startup.
3. Each master-plan feature (F1-F17) lands as a merge to `main`. After each merge, `cargo build --features gui-dioxus` and `cargo build --features gui-egui` must both succeed; egui remains the default runtime behavior.
4. At **F13 completion** (all core + secondary surfaces built, tray integration already in place via F3), flip the default feature to `gui-dioxus`. F14 (mode editing extras) and F15 (settings UI) ship after the flip, on top of the now-default Dioxus GUI.
5. At **F17**, delete the `inputforge-gui` crate, rename `inputforge-gui-dx` → `inputforge-gui`, and remove egui dependencies from the workspace.

This is chosen over a full-cutover branch because "on `main`" is a stated constraint and solo-maintainer workflow benefits from always-shippable increments. It is chosen over running both frameworks in the same process because event-loop ownership conflicts would be costly to resolve.

## Master Plan

### Foundation (strict sequential order)

#### F1. Dioxus Crate Scaffold & State Bridge

- Create `crates/inputforge-gui-dx` with workspace inheritance and dependencies on `inputforge-core`, `dioxus`, `dioxus-desktop`.
- Launch an empty Dioxus Desktop window with title "InputForge" and matching default size (1280x800, min 800x500).
- Place `Arc<RwLock<AppState>>` and `mpsc::Sender<EngineCommand>` in Dioxus context.
- Establish the live-polling pattern: a background task periodically snapshots `AppState` into a reactive Signal at ~60Hz, so live visualizations re-render on data change without each component acquiring the read lock.
- Add `gui-egui` (default) and `gui-dioxus` cargo features to `inputforge-app`.
- Confirm Dioxus hot reload works in dev.
- Tray-menu and `muda::MenuId` wiring are stubbed at this stage (handled in F3 alongside the shell).

**Acceptance:** launching with `--features gui-dioxus` opens a Dioxus window and successfully reads engine state; launching with default features still runs the egui GUI identically.

#### F2. Design System & Theme

- Decide CSS architecture (recommended: single global stylesheet with design tokens as CSS variables; revisit if complexity demands modules).
- Define tokens: color (dark theme to start), spacing scale, typography scale, radii, elevation/shadow.
- Invoke `impeccable:frontend-design` to set the overall visual direction before screen work begins.
- Build atomic components: `Button`, `IconButton`, `TextInput`, `NumberInput`, `Select`, `Slider`, `Switch`, `Checkbox`, `Card`, `Badge`, `Tooltip`, `Menu`, `Separator`.
- Establish icon strategy (inline SVG symbols recommended).

**Acceptance:** a test harness screen renders all atomic components; visual direction is captured in the design-system source.

#### F3. Application Shell + Tray Bridge

- Window chrome.
- Pluggable layout container sketched as today's three-panel + status bar structure, explicitly a placeholder expected to be reshaped by F5.
- Tab bar component usable for later view-switching.
- Menubar / toolbar region.
- **Tray integration:** bridge `muda::MenuEvent` from the tray thread into Dioxus context (channel + Signal or direct command-channel fanout). Wire Show / Hide / Quit actions to the Dioxus window lifecycle. Tray is infrastructure, not a feature, placing it here guarantees a production-viable default once the feature flag flips at F13.

**Acceptance:** a runnable shell frames empty panel regions with correct sizing/responsiveness; tray menu actions behave identically to the egui build; the shell is intentionally minimal so F5 can revise it cheaply.

#### F4. Toast & Dialog Infrastructure

- Global toast queue with level (info/success/warning/error), dedupe, and auto-dismiss.
- Modal dialog primitive with focus trap and ESC-to-dismiss.
- Dirty-state confirmation pattern (reusable component used when switching inputs/devices with unsaved changes).

**Acceptance:** test screen exercises all four toast levels, a modal dialog, and a dirty-state confirmation flow.

### Architecture Pass

#### F5. Architecture & IA Redesign Pass, *Resolved*

Spec: [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md).

F5 audited the current egui IA at the structural level and committed a clean-slate redesign anchored in PRODUCT.md's authoring/tuning session shapes and DESIGN.md's surface rules. Outcomes that affect this plan:

- **Navigation root re-rooted from device-first to mapping-list, mode-scoped.**
- **Floating windows eliminated.** Calibration, Input Viewer, and Profile manager collapse into two right-side panels (Devices, Profiles) that share one slot via Replace discipline.
- **Save model becomes auto-commit + session undo + on-disk snapshots.** Working-copy gating is removed; calibration is the only explicit-save surface.
- **F6 onwards rewritten.** The provisional F6-F14 feature list in this plan is replaced by the F6-F17 sequence below. F1-F4 are unchanged and complete.
- **Default feature flag flip moves from F14 → F13.**
- **Feature count grows by one** (F1-F17 instead of F1-F16) to accommodate F15 (Settings UI) for the global preferences whose data layer F6 introduces.

**Acceptance:** the design doc above is committed; this master plan now incorporates the final feature set.

### Core Screens, *resolved by F5*

#### F6. Snapshot module + preferences core in `inputforge-core`

Core-only feature, no GUI. Owns `crates/inputforge-core/src/snapshot/` (Snapshot/SnapshotKind/SnapshotConfig types, ULID ids, BLAKE3 content hashing, atomic file writes, FIFO eviction respecting pinned flags, AutoSessionStart and AutoBeforeRestore triggers, `<profile>.snapshots/` storage with index recovery from snapshot file headers); the `AppState.mode_force: Option<ForcedMode>` field plus the `ForcedMode` enum and `EngineCommand::ForceMode | ReleaseMode` lifecycle commands that pause mode-change rules while forced; and the `inputforge_core::preferences` module (typed `Preferences` struct, OS-specific TOML location via `directories`, defaults-on-first-launch, `EngineCommand::ReloadPreferences`). Snapshot module reads `SnapshotConfig` from `Preferences` from day one, direct TOML edits are honored before any UI exists.

#### F7. Chrome shell, top bar, mode tabs, status bar, banner

Replaces F3's placeholder shell. Owns the top bar (engine pill, profile name, mode tabs, secondary tools cluster), the divergence/forced banner, the thin status bar, all chrome-level click handlers, and the side-panel **Replace discipline** (the shared right-side panel slot, F12 and F13 plug content in). Reads `mode_force` through the existing `pub(crate) MetaSnapshot` projection.

#### F8. Mapping list (left rail)

Mode-scoped mapping list grouped by output category (Axes / Buttons / Hats), filter input, group headers, row component (name + source + glyph annotations), `+ Add mapping` expanding row, empty state, mapping-list keyboard navigation. **Owns the live-capture primitive** (a GUI-only modal state that subscribes to `AppState.input_cache` and emits the next observed input event, mirrors today's calibration `Record range` pattern; no new engine command). The primitive is reused by F9 (`change input`, secondary-input picker), F10/F11 (any "press an input" affordance in stage editors), and F12 (axis drill-in `Record range` after migration).

#### F9. Mapping editor (pipeline structure)

Heaviest IA-level surface. Owns the editor frame (header, name field, input field, live readout, pipeline graph, undo recap, inactive-in-runtime hint); the pipeline-graph component (chain layout, Conditional branch rendering, MergeAxis stage with secondary-input picker, stage add/remove/reorder); the per-mapping session-undo log; and live-input/output binding to F1's polling Signal. All edits commit live via `EngineCommand::SetMapping`, no save buttons. F10 and F11 own the heavy widgets that live inside curve and deadzone stages.

#### F10. Curve editor (SVG inside curve stage), *signature feature*

The primary tool of the tuning session; flagged for visual ambition. Owns the SVG bezier curve editor: control-point and handle drag, symmetric mode, axis labels and ticks, live-value tracking dot, keyboard manipulation. Bezier math is a direct port of `crates/inputforge-gui/src/widgets/curve_editor/mutation.rs` (no re-derivation). Reference quality bar: synthesizer envelope editors and DAW LFO designers.

#### F11. Deadzone editor (SVG inside deadzone stage), *signature feature*

Coherent with F10's animation timing and precision feel. Owns the deadzone visualization (input axis vs deadzone-applied output), inner/outer threshold drag handles on the curve, numeric input fields, live overlay showing where the live signal currently sits on the input-output curve.

### Secondary Surfaces, *resolved by F5*

#### F12. Devices side panel + Calibration drill-in

Right-side panel that plugs into F7's Replace slot. Owns the device-list section (connection dot, name, coverage, disconnection display); the device drill-in section (axes/buttons/hats with live values + role badges + the subtle `cal` pill); the axis drill-in calibration editor (raw + calibrated bars, Min/Center−/Center+/Max number fields, Record range / Set center / Reset, amber dirty banner, calibration remains the only explicit-save surface in the GUI); used-by backref panel listing every mapping touching the axis.

#### F13. Profiles side panel + Snapshots + no-profile empty state, *feature flag flips here*

Right-side panel that plugs into F7's Replace slot. Owns the profile library + Snapshots sub-section; per-row hover-revealed actions; `+ New` inline expanding-row flow with template radio (Blank / Copy from active / Copy from selected); `Open file…` OS picker; per-snapshot actions (Pin/Unpin · Rename · Delete · Restore →) wired to F6's snapshot commands; and the no-profile workspace empty state (`+ New profile`, `Open file…`, library pointer; engine forced Stopped). Profile rename, snapshot rename, and `+ New` name commit on Enter or blur, Esc cancels. **Default feature flag flips to `gui-dioxus` when this merges**, all core and secondary surfaces are in place at this point; tray was wired in F3.

#### F14. Mode editing (beyond tab CRUD)

Downscoped from the old F11 mode editor. Most mode CRUD (create/rename/delete/select/activate, default-mode selection) lives in F7's mode-tab right-click menu. F14 owns what's left: the `ChangeMode { strategy }` action editor inside the pipeline (a mode-change is just a regular `Action`, so it gets pipeline-stage treatment); strategy picker (`SetMode` / `CycleModes`); and any mode-tree visualization if implementation discovers users need one (default plan: do not).

#### F15. Settings UI, preferences editor surface

Editor on top of F6's preferences core. Owns the settings surface (panel sub-section or dialog, F15 brainstorm decides), the form components that bind to F6's `Preferences` struct, and the commit flow (write TOML → dispatch `EngineCommand::ReloadPreferences`). Does **not** own schema, location, or defaults, those are F6's. Lands after the feature flag flip (F13), on top of the already-default Dioxus GUI.

### Integration + Finish

#### F16. UX Polish & Audit

- Run `impeccable:audit` against the built application.
- Apply `impeccable:polish`, `impeccable:typeset`, `impeccable:harden`, and `impeccable:animate` where motion earns its keep.
- Keyboard navigation / focus order review.
- Light-theme support deferred unless earlier features force a decision.

**Acceptance:** audit report items at or above "medium" severity are resolved; keyboard-only use of the app is possible end-to-end.

#### F17. Cutover & Cleanup

- Confirm production parity (Dioxus is already the default since F13).
- Delete `crates/inputforge-gui` (egui crate).
- Rename `crates/inputforge-gui-dx` → `crates/inputforge-gui`.
- Drop `eframe`, `egui`, `egui_extras`, `egui_plot`, `egui_kittest` from workspace dependencies.
- Remove the cargo feature flags (Dioxus is the only GUI now).
- Update README and any docs that reference egui.
- Remove `egui_kittest` snapshot tests (testing story replaced or accepted-as-reduced, see Open Questions).

**Acceptance:** a fresh `cargo build` and `cargo test --workspace` succeed with no egui references remaining in the tree.

## What Lives Outside the GUI Crate

- **Engine** (`inputforge-core`): untouched.
- **Tray icon** (`inputforge-app` + `tray-icon` + `muda`): unchanged except for the `MenuId` bridge into the Dioxus GUI (F3).
- **Settings, profiles, calibration data**: continue to live in `inputforge-core`.

## Open Questions (Resolve Per Feature, Not Here)

- **Testing story for Dioxus GUI.** `egui_kittest` snapshot tests are dropped at F17. Options include: (a) accept-reduced UI testing (rely on logic-layer tests in `inputforge-core`); (b) Playwright or similar against the WebView; (c) Dioxus-native renderer testing as that ecosystem matures. Decided per feature; F16 or F17 is the latest we should commit to an approach.
- **Hot-reload ergonomics in release workflow.** Determine whether a dev-only script is needed. Deferred to F1 implementation detail.
- **Light theme.** Out of scope until explicitly needed.
- **Localization / i18n.** Out of scope; not present today.

## Success Criteria (end-state after F17)

- `inputforge-gui` is a Dioxus Desktop crate.
- No egui dependencies in the workspace.
- All features from the current egui GUI have equivalent or improved Dioxus surfaces (subject to F5's redefinition).
- Adding a new configuration panel or widget takes materially less code and fewer LLM correction cycles than the egui equivalent.

## Risks

- **F5 reshaped the downstream feature decomposition.** Accepted, that was the point of F5. The provisional F6-F14 sketch was replaced by the resolved F6-F17 sequence above. See the F5 spec for full traceability.
- **Curve editor (F10) porting is nontrivial.** SVG + pointer events are a well-trodden pattern, but correctness of bezier math and symmetric mode needs direct porting of the existing logic at `crates/inputforge-gui/src/widgets/curve_editor/mutation.rs`, not re-derivation.
- **Webview runtime on older Windows.** WebView2 is pre-installed on Windows 10 20H1+ and all Windows 11. Older systems may need the evergreen runtime installer. Acceptable for a configuration tool.
- **Loss of `egui_kittest` snapshot tests.** See Open Questions.
