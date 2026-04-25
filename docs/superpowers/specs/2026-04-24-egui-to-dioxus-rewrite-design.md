# egui → Dioxus Rewrite — Master Plan

**Status:** Design approved, ready for per-feature planning
**Date:** 2026-04-24
**Scope:** Replace the immediate-mode egui configuration GUI with a declarative Dioxus Desktop GUI, incrementally, on `main`.

## Motivation

The current `inputforge-gui` crate (~6,200 lines across panels and widgets) uses `egui` in immediate mode. Working in it has four dominant pain points:

1. **Paradigm friction** — immediate mode tangles state and rendering, making it hard to reason about "what should be on screen for a given state".
2. **Styling** — fighting egui for visual polish.
3. **Layout** — widget positioning and composition are awkward for non-trivial compositions.
4. **LLM productivity** — AI-assisted edits to egui code produce incorrect APIs and patterns at high rates.

The GUI is a configuration/testing surface. It is closed when the app is running normally, so runtime performance is not a constraint — developer and LLM ergonomics dominate.

A secondary motivation: the existing UI has structural UX problems that are better addressed during a rewrite than retrofitted. The rewrite is also a UX redesign.

## Constraints & Principles

- **100% Rust.** No Node.js, npm, or JS frontend toolchain.
- **All work lands on `main`** as incremental, shippable merges. No long-lived branch.
- **egui stays the default GUI** until the Dioxus version reaches parity across all core and secondary surfaces (end of F14).
- **Per-feature focused plans** will be written separately (via `superpowers:writing-plans`). This document is a master plan, not an implementation plan.
- **UX-first per feature.** Each feature invokes `impeccable:frontend-design` as part of its focused plan. A dedicated audit/polish pass runs at F16.
- **Preserve the engine contract.** The Dioxus GUI reuses the existing `Arc<RwLock<AppState>>` read path and `mpsc::Sender<EngineCommand>` write path without changes to the engine side.

## Framework Choice: Dioxus Desktop

Dioxus Desktop was selected over Slint after weighing the four pain points:

| Pain | Dioxus Desktop | Slint |
|---|---|---|
| Declarative paradigm | RSX + Signals/Context | Declarative DSL |
| Styling | Real CSS (flexbox, grid, pseudo-classes) | Custom styling syntax |
| Layout | CSS flexbox/grid — heavily represented in LLM training | Slint layout primitives |
| LLM familiarity | JSX + CSS + SVG — excellent LLM fluency | `.slint` DSL is niche |

Dioxus Desktop uses WebView2 under the hood. On Windows 10+ (the project's target), WebView2 is a system-provided component — bundle impact is minimal, and it is pre-installed on modern systems. This tradeoff was considered acceptable in exchange for the LLM-productivity and styling wins.

**Custom widgets** (curve editor, axis bars, hat indicator, calibration visualization, input viewer) will be implemented as **SVG** with event handlers. SVG is one of the strongest LLM output targets.

**State flow** maps cleanly:
- Read path: `Arc<RwLock<AppState>>` placed in Dioxus context; a background task snapshots state into a reactive Signal at ~60Hz for live visualizations.
- Write path: `mpsc::Sender<EngineCommand>` in context, invoked from event handlers.
- Per-frame egui snapshots become explicit Signal updates; no semantic change to the engine contract.

## Migration Strategy

**Parallel-crate, feature-flag switchover.**

1. Create `crates/inputforge-gui-dx` alongside the existing `crates/inputforge-gui`.
2. Add `gui-egui` (default) and `gui-dioxus` cargo features to `inputforge-app`. `launch_gui` dispatches to the selected crate at startup.
3. Each master-plan feature (F1–F14) lands as a merge to `main`. After each merge, `cargo build --features gui-dioxus` and `cargo build --features gui-egui` must both succeed; egui remains the default runtime behavior.
4. At **F14 completion** (all core + secondary surfaces built, tray integration already in place via F3), flip the default feature to `gui-dioxus`.
5. At **F16**, delete the `inputforge-gui` crate, rename `inputforge-gui-dx` → `inputforge-gui`, and remove egui dependencies from the workspace.

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
- Pluggable layout container sketched as today's three-panel + status bar structure — explicitly a placeholder expected to be reshaped by F5.
- Tab bar component usable for later view-switching.
- Menubar / toolbar region.
- **Tray integration:** bridge `muda::MenuEvent` from the tray thread into Dioxus context (channel + Signal or direct command-channel fanout). Wire Show / Hide / Quit actions to the Dioxus window lifecycle. Tray is infrastructure, not a feature — placing it here guarantees a production-viable default once it flips at F14.

**Acceptance:** a runnable shell frames empty panel regions with correct sizing/responsiveness; tray menu actions behave identically to the egui build; the shell is intentionally minimal so F5 can revise it cheaply.

#### F4. Toast & Dialog Infrastructure

- Global toast queue with level (info/success/warning/error), dedupe, and auto-dismiss.
- Modal dialog primitive with focus trap and ESC-to-dismiss.
- Dirty-state confirmation pattern (reusable component used when switching inputs/devices with unsaved changes).

**Acceptance:** test screen exercises all four toast levels, a modal dialog, and a dirty-state confirmation flow.

### Architecture Pass

#### F5. Architecture & IA Redesign Pass

- Audit the current egui GUI's information architecture: three-panel layout; Mappings/Modes tabs; floating Calibration / Input Viewer / Profile windows; device-first navigation; clickable status-bar profile name; the separation between mapping editing and live testing.
- Identify UX problems at the **structural** level only. Visual/styling problems are per-feature and F16.
- Answer concretely: should Mappings and Modes share a view? Should the Input Viewer be docked rather than floating? Is device-first the right navigation root, or should it be mapping-first or profile-first? Where does calibration belong in the flow? Does the status bar earn its space? Should profile management be a top-level surface or a menu action? Are any current floating windows actually modal workflows that would serve users better inline?
- Invoke `impeccable:critique` on the current design, then `impeccable:frontend-design` + `impeccable:distill` + `impeccable:arrange` to propose the new IA.
- Produce a layout-and-navigation spec: wireframes, screen inventory, navigation flow, and a revised feature decomposition.

**Acceptance:** a dated design doc in `docs/superpowers/specs/` describing the new IA and rewriting the F6–F14 feature list. After F5 merges, this master plan is updated to reflect the final feature set.

### Core Screens — *provisional pending F5*

The following features are the current sketch based on today's structure. Their scope, sequencing, and even existence may change once F5 completes. Their focused plans will not be written until after F5.

#### F6. Left Panel — Device List + Input Tree *(provisional)*

Device list with connection status; expandable input tree with mapping indicators; selection state for device + input.

#### F7. Mapping Editor — Shell + Action List *(provisional)*

Center panel for Mappings; action list / card layout; empty state; add/remove.

#### F8. Mapping Editor — Action Config Forms *(provisional)*

Mode-action form and vJoy-output form (axis / button / hat, target device, polarity); validation and dirty tracking.

#### F9. Mapping Editor — Curve Editor *(provisional)*

SVG-based bezier curve editor with interactive point/handle drag, symmetric mode, axis labels, live-value overlay. Heaviest widget in the rewrite — its focused plan will likely have multiple internal steps.

#### F10. Mapping Editor — Deadzone Editor *(provisional)*

Deadzone visualization with inner/outer controls; linked to the live axis value readout.

#### F11. Mode Editor *(provisional)*

Mode tree view; create/rename/delete; active-mode indicator.

### Secondary Surfaces — *provisional pending F5*

Floating-vs-docked status may change at F5.

#### F12. Input Viewer Surface *(provisional)*

Live per-device visualization: axis bars (SVG, polarity + percentage), button grid, hat indicator. This feature exercises the live-polling path most heavily.

#### F13. Calibration Surface *(provisional)*

Live range detection visualization; calibration parameter editor; apply/reset flow.

#### F14. Profile Surface *(provisional)*

Profile CRUD; load/save/switch; integration with the existing profile manager. **Default feature flag flips to `gui-dioxus` when this merges**, per the migration strategy — all core and secondary surfaces now exist and tray integration is already wired from F3.

### Integration + Finish

#### F15. UX Polish & Audit

- Run `impeccable:audit` against the built application.
- Apply `impeccable:polish`, `impeccable:typeset`, `impeccable:harden`, and `impeccable:animate` where motion earns its keep.
- Keyboard navigation / focus order review.
- Light-theme support deferred unless earlier features force a decision.

**Acceptance:** audit report items at or above "medium" severity are resolved; keyboard-only use of the app is possible end-to-end.

#### F16. Cutover & Cleanup

- Confirm production parity (Dioxus is already the default since F14).
- Delete `crates/inputforge-gui` (egui crate).
- Rename `crates/inputforge-gui-dx` → `crates/inputforge-gui`.
- Drop `eframe`, `egui`, `egui_extras`, `egui_plot`, `egui_kittest` from workspace dependencies.
- Remove the cargo feature flags (Dioxus is the only GUI now).
- Update README and any docs that reference egui.
- Remove `egui_kittest` snapshot tests (testing story replaced or accepted-as-reduced — see Open Questions).

**Acceptance:** a fresh `cargo build` and `cargo test --workspace` succeed with no egui references remaining in the tree.

## What Lives Outside the GUI Crate

- **Engine** (`inputforge-core`): untouched.
- **Tray icon** (`inputforge-app` + `tray-icon` + `muda`): unchanged except for the `MenuId` bridge into the Dioxus GUI (F3).
- **Settings, profiles, calibration data**: continue to live in `inputforge-core`.

## Open Questions (Resolve Per Feature, Not Here)

- **Testing story for Dioxus GUI.** `egui_kittest` snapshot tests are dropped at F16. Options include: (a) accept-reduced UI testing (rely on logic-layer tests in `inputforge-core`); (b) Playwright or similar against the WebView; (c) Dioxus-native renderer testing as that ecosystem matures. Decided per feature; F15 or F16 is the latest we should commit to an approach.
- **Hot-reload ergonomics in release workflow.** Determine whether a dev-only script is needed. Deferred to F1 implementation detail.
- **Light theme.** Out of scope until explicitly needed.
- **Localization / i18n.** Out of scope; not present today.

## Success Criteria (end-state after F16)

- `inputforge-gui` is a Dioxus Desktop crate.
- No egui dependencies in the workspace.
- All features from the current egui GUI have equivalent or improved Dioxus surfaces (subject to F5's redefinition).
- Adding a new configuration panel or widget takes materially less code and fewer LLM correction cycles than the egui equivalent.

## Risks

- **F5 changes downstream feature decomposition significantly.** Accepted — that is the point of F5. F6–F14 are explicitly provisional.
- **Curve editor (F9) porting is nontrivial.** SVG + pointer events are a well-trodden pattern, but correctness of bezier math and symmetric mode needs direct porting of the existing logic, not re-derivation.
- **Webview runtime on older Windows.** WebView2 is pre-installed on Windows 10 20H1+ and all Windows 11. Older systems may need the evergreen runtime installer. Acceptable for a configuration tool.
- **Loss of `egui_kittest` snapshot tests.** See Open Questions.
