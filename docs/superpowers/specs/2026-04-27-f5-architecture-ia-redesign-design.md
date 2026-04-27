# F5 — Architecture & IA Redesign Pass: Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-27
**Parent spec:** [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md) — Architecture pass feature F5
**Predecessors:** [F1](./2026-04-24-f1-dioxus-scaffold-state-bridge-design.md) (state bridge), [F2](./2026-04-25-f2-design-system-design.md) (design system), [F3](./2026-04-26-f3-app-shell-tray-bridge-design.md) (shell + tray), [F4](./2026-04-26-f4-toast-dialog-design.md) (toast + dialog)
**Brainstorm artefacts:** wireframes persisted under `.superpowers/brainstorm/1682-1777320356/content/` (`nav-root.html`, `mapping-list-detail.html`, `multi-input.html`, `chrome.html`, `chrome-a-refined.html`, `activate-mode.html`, `save-semantics.html`, `backup-snapshots.html`, `snapshot-core.html`, `devices-calibration.html`, `calibration-no-warning.html`, `profiles-surface.html`).
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Product brief:** [`/PRODUCT.md`](../../PRODUCT.md)

---

## Context

F5 is the architecture pass. Its job is to commit the new information architecture for the Dioxus rewrite — what surfaces exist, how they relate, what the navigation root is, where each function of today's egui GUI lands — and to rewrite the master plan's feature list (F6 onwards) in light of those decisions.

The brainstorming process took the master plan's prompt ("audit current IA at structural level only") at its strongest reading: a **clean-slate redesign** anchored in PRODUCT.md's two session shapes (authoring and tuning) and DESIGN.md's surface rules (inline-edit primitive, side-panel for multi-field, dialog rare). Today's egui IA was constrained by egui idioms; several of its choices — three floating windows for Calibration, Input Viewer, Profile management; device-first as the only navigation root; status bar doubling as engine toggle and profile picker — don't survive contact with the design rules already committed in DESIGN.md or with PRODUCT.md's anti-references (JoystickGremlin Tk-era dialog soup).

This spec is approval-ready: every surface decision below was validated section-by-section in a visual companion brainstorming loop. F5 implementation is therefore mostly a documentation deliverable plus a master-plan amendment; no code lands in F5 itself. F6 onwards is where this design becomes Dioxus components.

---

## Confirmed design choices

The decisions below are recorded in order of dependency, each surfaced and approved during brainstorming.

### Posture & navigation root

**1. Posture: clean-slate redesign.** The new IA does not preserve today's device-first, three-floating-window, save/discard mental model. It re-roots the GUI around mappings, replaces all floating windows with side panels and inline drill-ins, and removes save-state from the main loop.

**2. Navigation root: mapping-list, mode-scoped.** The left rail is the named mappings of the active *editing* mode, grouped by output category (Axes / Buttons / Hats), with each row showing name + physical source. Hardware is a property of mappings, not the navigation root. Engine model preserved: a `Mapping` is rooted in one `InputAddress`; multi-input behavior (MergeAxis, Conditional gates) is encoded inside the action pipeline.

**3. Multi-input rendering:** one mapping = one row, always. Source-label glyphs annotate secondary inputs — `+` (output gold) for MergeAxis, `⊕` (control violet) for Conditional gate. Devices side panel exposes per-input backref roles (`primary`, `merge 2nd`, `gate`) so a flat list still answers "what touches this axis?".

### Chrome, modes, and force semantics

**4. Chrome model: heavy top bar, thin status bar (variant A).** Top bar carries acts (engine pill, profile name, mode tabs, secondary tools); status bar carries glances (device count, warning count). No clickable engine toggle in two places; no chrome competing with the editor.

**5. Engine pill is clickable** — toggles activate/deactivate, mirrors the tray menu. Color follows `EngineStatus` (live / warning / error). `role="status"` + `aria-live="polite"` for transitions; `<button>` semantics for keyboard reachability.

**6. Mode tabs are editing scope.** They never tell the engine to change mode; that is driven by the profile's mode-change rules during play. Engine's runtime mode shows as a small **green dot** on whichever tab the engine is in. When editing tab and runtime mode diverge, an actionable banner appears: <em>"Editing Landing — engine is in Combat"</em> with **Activate Landing** button.

**7. Forced runtime mode (sticky override).** Activating a mode forces the engine into it via a new `AppState.mode_force: Option<ForcedMode>` field (engine-owned state, added in F6 — see `crates/inputforge-core/src/state/mod.rs`). The runtime marker dot turns **amber** to signal override. Mode-change rules are paused while `mode_force.is_some()`. **Release** button hands control back. Switching the editing tab while forced does not release the force. **Engine paused + Activate** = activate-and-resume in one click. Right-click on a mode tab is the secondary path: Activate · Rename · Delete · Set as default.

### Save model and snapshots

**8. Save semantics: auto-commit + session undo + on-disk snapshots.** Every mapping edit commits to the engine and disk immediately (engine's existing `SetMapping` already writes the TOML; the GUI's working-copy gating is removed). Session undo (`Ctrl+Z` / `Ctrl+Shift+Z`) is the in-memory safety net for drag-experiment-revert. Profile snapshots are the cross-session safety net. Profile-level text edits (profile rename, snapshot label/rename, `+ New profile` name commit) are also auto-commit on Enter or blur — Esc cancels uncommitted text — reusing the synchronous pattern in `inputforge_core::profile::manager` already used by today's profile rename UI.

**9. Snapshot machinery lives in `inputforge-core::snapshot`.** Stateless module next to `Profile`. Engine triggers `AutoSessionStart` once per launch (deduped by content hash) and `AutoBeforeRestore` as the first step of any restore. New `EngineCommand` variants — `CreateSnapshot`, `DeleteSnapshot`, `PinSnapshot`, `RenameSnapshot`, `RestoreSnapshot` — go through the existing command channel. GUI never touches `std::fs` for snapshot ops.

**10. Snapshot storage:** TOML files in `<profile>.snapshots/` co-located with the profile. `index.toml` sidecar carries metadata (kind, label, content hash, pinned flag). Default rolling buffer = 10 snapshots, configurable; FIFO eviction; pinned snapshots never evict. Manual snapshots are pinned by default.

**11. Manual snapshot keyboard shortcut: `Ctrl+S`.** In an auto-commit world there is no "save" gesture; the muscle memory of `Ctrl+S` maps onto "checkpoint this moment". Opens F4's dialog primitive with focus on an optional label field; Enter commits, Esc cancels.

### Side panels and calibration

**12. Devices side panel** (right-side) lists devices with connection status + coverage in profile. Drilling into a device shows its inputs with live values + role badges + the subtle `cal` pill where present. Drilling into an axis expands inline into the calibration editor.

**13. Calibration absence is silent.** InputForge calibration sits on top of whatever the device's own driver/companion software (VKB Configurator, TARGET, etc.) has done. Absence is not a deficit; only a positive `cal` pill on calibrated axes. Unmapped axes use a neutral em-dash, not value-loaded copy.

**14. Calibration uses explicit save** (the only explicit-save surface in the GUI). `SetCalibration` updates runtime live; `SaveCalibrations` persists. Amber dirty banner when unsaved. Reset triggers F4 destructive dialog.

**15. Top-bar Calibration item is sugar** — opens Devices already drilled into the first uncalibrated axis (or last-used device). Single underlying surface, two entry points.

**16. Side-panel discipline: Replace.** Devices and Profiles are right-side panels sharing one slot; opening one closes the other. Top bar shows which is open via the active-tab style. Stacked panels and tabbed panels both rejected. F7 owns the shared slot and Replace logic; F12 and F13 plug content in.

**17. Profiles side panel** contains the profile library + Snapshots sub-section. Active profile pinned at top with `active` pill. Hover-revealed actions per row: active row gets `Snapshots · Rename · Duplicate · Reveal · Delete`; non-active rows add `Open` as primary. Snapshots sub-section is bound to the active profile (or last-clicked Snapshots row).

### Empty state, primitives, and editor framing

**18. No-profile empty state.** Workspace center shows Display-typography "No profile loaded" + two primary actions (`+ New profile`, `Open file…`) + library pointer. Profiles panel auto-opens. Engine forced Stopped. Modes / Devices / Calibration top-bar items disabled. No SVG illustrations, no marketing copy.

**19. + New profile flow** uses inline expanding row (per Expanding-row primitive): name + template radio (Blank / Copy from active / Copy from selected library entry).

**20. F4 dirty-confirm pattern repurposed.** Auto-commit eliminates the original "switch input" trigger. New triggers: delete mapping, delete profile, delete snapshot, restore snapshot, switch profile when undo stack non-empty, reset calibration.

**21. Mapping-list filter** scopes to name + source; reduces the list, doesn't reorder.

**22. Pipeline editor is a graph, not a chain.** Conditional renders as indented branches with collapsible `if_true` / `if_false` sub-pipelines; MergeAxis renders inline with an editable secondary-input picker inside the merge stage. F5 commits this requirement; F9 owns the structural design.

## Non-goals (out of scope for this spec)

- **Pixel-level visual treatment** of any surface. F5 commits IA, not aesthetics. Each F6+ feature invokes `impeccable:frontend-design` for its visual direction.
- **Curve and deadzone editor internals.** F10 and F11 own those; F5 only commits where they live (inline inside pipeline stages).
- **Mode-change action editor.** F14 owns the editor for `ChangeMode { strategy }` actions and any mode-tree visualization. F5 commits the boundary: "most mode CRUD lives in the tab right-click; what doesn't is F14".
- **Light theme.** Out of scope for the whole rewrite per parent plan; deferred unless a feature forces a decision.
- **Localization / i18n.** Not present today; out of scope for the rewrite.
- **Force-mode UX beyond Activate / Release.** No "force this mode for N seconds", no mode-pinning, no per-device mode override. Single sticky override; release is explicit.

---

## IA architecture

### Surface inventory

The Dioxus GUI has exactly these surfaces:

| Surface | Region | Status | Owner feature |
|---|---|---|---|
| Top bar | top, full width | always present | F7 |
| Mapping list (left rail) | left, fixed-width column | always present (empty when no mappings) | F8 |
| Mapping editor | center, takes remaining space | always present (empty when no mapping selected) | F9 |
| Status bar | bottom, full width, thin | always present | F7 |
| Devices side panel | right-side panel | togglable; replaces Profiles when opened | F12 |
| Profiles side panel | right-side panel | togglable; replaces Devices when opened | F13 |
| Workspace empty state | center overlay | shown only when no profile loaded | F13 |
| Toast viewport | top-right floating | F4 (already built) | F4 |
| Dialog backdrop | full-window overlay | F4 (already built) | F4 |

There are **no floating windows** in the new IA. Calibration, Input Viewer, and Profile Manager from today's egui have all collapsed into Devices and Profiles side panels.

### Top bar

Layout, left-to-right:

```
[engine pill] | [profile name] | [Editing] [Default] [Combat●] [Landing] [+] | [Devices] [Calibration] [Profiles]
```

- **Engine pill** — clickable. Click toggles activate/deactivate. Hover reveals state-aware hint ("click to pause" / "click to activate"). Color and label follow `EngineStatus`. ARIA: `role="status" aria-live="polite"` plus `<button>` semantics.
- **Profile name** — clickable. Click opens Profiles side panel. Display the bare profile name (no path). When no profile loaded: italicized muted "no profile loaded".
- **Mode tabs** — editing-scope only. Active editing tab gets focus-cyan underline. Engine's runtime mode shows as small dot on whichever tab the engine is in (green = natural, amber = forced override). "+" creates a new mode inline. Right-click menu: Activate · Rename · Delete · Set as default. Keyboard equivalent: with focus on a mode tab, **Shift+F10** (or the dedicated context-menu key) opens the same menu.
- **Secondary tools cluster** — Devices, Calibration, Profiles. Active tool gets focus-cyan-tinted background. Calibration is sugar for "Devices drilled into first uncalibrated axis or last-used device".

When **diverged** (editing mode ≠ runtime mode), a banner appears between the top bar and the mapping list: *"Editing Landing — engine is in Combat"* with `Activate Landing` button. When **forced**, the banner says *"Engine override: Landing. Mode-change rules paused."* with `Release` button. When aligned (or no profile), no banner.

### Status bar

Thin (28px). Three slots:

- **start** — dirty-or-warning state (e.g., warning count). No engine status here — engine state lives in the top-bar pill.
- **middle** — device count "3/3 devices" (truncates middle slot first when narrow).
- **end** — profile path or em-dash when no profile loaded.

Status bar text never raises to bright — per `DESIGN.md` (status bar section), consumers raise specific badges or pills to bright text via their own component, never through the status-bar text color. Click targets are the top bar and the side panels, not the status bar. The status bar exists to fill peripheral vision during tuning sessions, not to be acted on.

### Mapping list (left rail)

Always present, fixed width (revisit at F8 implementation). When no profile loaded the rail is hidden behind the workspace empty state.

```
[⌕ Filter mappings…]
AXES (4)
  Throttle           ← active
    TFM Throttle · Z
  Pitch
    VKB Gladiator · Y
  Yaw
    VKB Pedals · Brake bal + VKB Gladiator · Twist
BUTTONS (3)
  Boost
    TFM · Btn 4 ⊕ while Btn 12 held
  Gear
    TFM · Btn 7
+ Add mapping
```

- **Filter input** — narrows visible rows to name+source matches; doesn't reorder.
- **Group headers** — Axes / Buttons / Hats, derived from each mapping's terminal vJoy stage output kind.
- **Row** — name (body, 12px) + source (caption, 10px, muted). Active row uses focus-cyan left-border + tinted background (per existing F2 patterns).
- **Annotation glyphs** in source label:
  - `+` (output gold `#C99846`) — has at least one MergeAxis stage. Secondary input shown in italic after the glyph. Two merges → `+ A + B`.
  - `⊕` (control violet `#9A78D6`) — has a Conditional whose condition references another input. Tooltip carries the predicate.
  - Both can coexist on one row.
- **+ Add mapping** — bottom of the list. Click expands inline (per Expanding-row primitive). Two fields: name + "press an input to bind" (live capture from any device). Commit when both filled; new row becomes the active selection.
- **Empty state** (profile loaded, zero mappings): Display "No mappings yet" + `+ Add mapping` primary button + helper "Pick an input on a device to start binding."

### Mapping editor (center)

Always-present surface. When no mapping selected, shows empty state ("Select a mapping" or the no-profile workspace state). When a mapping is selected:

- **Header** — name (Title typography). Below: subtitle "<source> → <output>" line.
- **Name field** — text input, inline.
- **Input field** — readonly source path, with `[change]` action that opens a live-capture picker.
- **Live readout** — for each source input touched by the mapping, a row: source label + axis bar + percentage. For merge mappings, two source rows + a merged-result row separated by a dashed divider. Output row when applicable.
- **Pipeline graph** — chain of stages, plus indented branches under any Conditional. Stages use category colors (curve / deadzone in processing teal; merge / vJoy in output gold; conditional in control violet). Click a stage to expand its editor inline (curve editor in F10, deadzone editor in F11; merge stage shows operation picker + secondary input picker; conditional stage shows condition editor + nested branches).
- **Inactive-in-runtime hint** — when the editing tab differs from the engine's current runtime mode, a one-line hint below the live readout: *"Inactive in current runtime mode. Engine is in Combat; this mapping fires only in Landing."* Live input values still display; output is greyed.
- **Undo / redo affordance** — keyboard (`Ctrl+Z` / `Ctrl+Shift+Z`) primary; small undo-recap line in the editor footer ("Last change: deadzone outer 12% → 14%").

No Save / Discard buttons. No dirty marker on the mapping. Edits commit live.

### Devices side panel

Right-side panel. Replace discipline with Profiles. Slides in from the right; doesn't dim the workspace; doesn't trap focus from the mapping list or editor.

**Top section — device list:**
- Each row: connection dot (●/○), device name, "X/Y inputs in profile" coverage. Disconnected devices: red dot, italicized muted name, "K mappings affected".
- Active drill-in row uses focus-cyan left-border tint.

**Drill-in section — selected device:**
- Section headers per kind: Axes / Buttons / Hats.
- Axis row: short label + small `cal` pill if InputForge calibration applied + small role pill (`primary` / `merge 2nd` / `gate` / nothing) → live bar → percentage → mapping name (or em-dash if unmapped).
- Button grid: compact 8-column grid with mapped/unmapped/pressed states.
- Hat tile: classic 8-direction indicator.
- Click an axis row to expand it inline into the calibration editor (next section).

**Drill-in section — selected axis (calibration):**
- Header crumb: `Devices › TFM Throttle › Z`.
- Raw range readout: `0..65535` + detected polarity.
- Live signal bar with two fills: raw (focus-cyan, 50% opacity) + calibrated (live-green). Vertical center indicator.
- Number fields: Min · Center− · Center+ · Max (mono font, tabular figures).
- Action buttons: `● Record range` (toggle; latches min/max while user moves the stick), `Set center`, `Reset` (destructive — F4 confirm).
- Amber dirty banner when changes unsaved: `Unsaved calibration · Save · Discard`.
- **Used by** backref panel: every mapping that touches this axis, with role badge. Click to jump left-rail.

**Disconnection in the panel:** disconnected devices stay in the list (drilled-in if they were active), so the user can see what's missing. The drill-in body shows "Device disconnected — N mappings affected" + a list of those mappings.

### Profiles side panel

Right-side panel. Replace discipline with Devices.

**Header:** `+ New` (primary), `Open file…` (OS file picker for profiles outside the standard directory).

**Library list:**
- Each row: name (Title typography), `active` pill if active, meta line `N mappings · M modes · last edited Xh ago`.
- Hover-revealed actions: active row gets `Snapshots · Rename · Duplicate · Reveal · Delete`; non-active rows add `Open` as primary.
- Active profile pinned at top.

**Snapshots sub-section:**
- Bound to the active profile by default; clicking `Snapshots` on a non-active row swaps the binding.
- Header: `Snapshots · <profile-name>` + `+ Snapshot now` button (also `Ctrl+S`).
- Each row: kind icon (● auto / ★ manual), timestamp + relative-time, label or "Auto · session start", meta (mappings + modes count).
- Hover-revealed actions: auto rows get `Pin · Delete · Restore →`; manual/pinned rows get `Unpin · Rename · Delete · Restore →`. Right-click context menu mirrors.
- Restore confirmation: F4 dialog. On confirm, engine takes auto-before-restore snapshot, then loads the snapshot file as the active profile. Toast: "Restored to *<label>*. Previous state saved as *before restore*."
- Delete confirmation: F4 dialog with destructive-action shape. Pinned snapshots get distinct copy: "This snapshot is pinned and cannot be auto-evicted. Deleting removes it permanently."

**Empty / no-profile state** (see workspace empty state below): Profiles panel auto-opens, the library is the only useful surface.

### Workspace empty state (no profile loaded)

When no profile is loaded:
- Engine is forced Stopped (red indicator).
- Top-bar profile name slot reads "no profile loaded" (italic, muted).
- Mode tabs: only "Default" shown, all greyed.
- Devices and Calibration top-bar items: disabled.
- Profiles top-bar item: active.
- Profiles panel auto-opens.
- Mapping list (left rail) hides; the workspace center plus the area where the rail would be merge into one empty-state region.
- Empty-state content (Display typography 32px): "No profile loaded" + body copy "InputForge needs a profile to bind inputs and emit to vJoy. Open one from your library, or start a new profile from scratch." + two primary buttons (`+ New profile`, `Open file…`) + secondary text pointer to the auto-opened library panel.
- No SVG illustration, no marketing copy. Pure typographic hierarchy.

### F4 dirty-confirm pattern repurposing

The Dialog primitive built in F4 plus its dirty-confirm pattern stay in active use, but the trigger set shifts entirely. Original F4 trigger ("switch input with unsaved mapping changes") is **deleted** — auto-commit eliminates the unsaved state. New trigger set:

| Trigger | Dialog shape | Action verb |
|---|---|---|
| Delete mapping | Destructive | "Delete *N*? Undo available this session only." |
| Delete profile | Destructive | "Delete *<profile>.profile*? This cannot be undone." |
| Delete snapshot | Destructive | "Delete this snapshot?" |
| Restore snapshot | Confirmation (non-destructive — auto-snapshot taken first) | "Restore to *<label>*? Current state will be saved as a snapshot first." |
| Switch profile (undo stack non-empty) | Confirmation | "Switch to *<other>*? Recent edits to *<active>* can no longer be undone." |
| Reset calibration | Destructive | "Reset *<axis>* calibration? Live mappings using this axis will switch to raw range immediately." |

All shapes use the existing F4 Dialog primitive; only copy varies.

### Snapshot module (`inputforge-core::snapshot`)

The module is part of F5's commitment because it determines the engine command surface that F6+ features rely on. The module itself is implemented in F6 (the first feature in the rewritten plan).

**Public types:**

```rust
pub struct Snapshot {
    pub id:           SnapshotId,            // ULID — monotonic, sortable; generated via the `ulid` crate at create()
    pub kind:         SnapshotKind,
    pub label:        Option<String>,
    pub taken_at:     DateTime<Utc>,
    pub content_hash: [u8; 32],              // BLAKE3 of the profile TOML at snapshot time
    pub pinned:       bool,
}

pub enum SnapshotKind {
    AutoSessionStart,
    AutoBeforeRestore,
    Manual,
}

// Owned by F6 — read from the global app preferences TOML loaded by F6's
// `inputforge_core::preferences` module. Configurable via direct TOML edit from
// day one; F15 ships the editor UI on top of this same data.
pub struct SnapshotConfig {
    pub max_count:         usize,            // default 10
    pub skip_if_unchanged: bool,             // default true
}
```

**Public functions** (all stateless, take a profile path):

```rust
pub fn create(profile: &Path, kind: SnapshotKind, label: Option<String>)
    -> Result<Snapshot>;
pub fn list(profile: &Path) -> Result<Vec<Snapshot>>;
pub fn delete(profile: &Path, id: SnapshotId) -> Result<()>;
pub fn pin(profile: &Path, id: SnapshotId, pinned: bool) -> Result<()>;
pub fn rename(profile: &Path, id: SnapshotId, label: Option<String>) -> Result<()>;
pub fn restore(profile: &Path, id: SnapshotId) -> Result<()>;
pub fn prune(profile: &Path, cfg: &SnapshotConfig) -> Result<usize>;
```

**Engine command extensions** (added in F6, dispatched in F12/F13):

```rust
EngineCommand::CreateSnapshot { kind: SnapshotKind, label: Option<String> }
EngineCommand::DeleteSnapshot { id: SnapshotId }
EngineCommand::PinSnapshot    { id: SnapshotId, pinned: bool }
EngineCommand::RenameSnapshot { id: SnapshotId, label: Option<String> }
EngineCommand::RestoreSnapshot{ id: SnapshotId }
```

**Engine triggers AutoSessionStart** at the end of `LoadProfile` (after the profile is loaded into runtime state, before any user edits). Skipped when `skip_if_unchanged && current_content_hash == latest_snapshot.content_hash`.

**Engine triggers AutoBeforeRestore** as the first step of `RestoreSnapshot` — takes a snapshot of current state, then performs the restore equivalent of `LoadProfile` from the snapshot's TOML content.

**Storage layout:**

```
<profile-dir>/
├── TFM_Throttle.profile.toml          # the live profile
└── TFM_Throttle.profile.snapshots/
    ├── index.toml                     # metadata: id → kind/label/timestamp/hash/pinned
    ├── 01H8ZK0M9Q5R3WVT8XEN1GS2HF.toml   # full profile TOML at snapshot time
    └── ...
```

Co-located with the profile so move/copy/delete carries snapshots as a single atom.

**Index recovery.** Snapshot files store `kind`, `label`, `taken_at`, `content_hash`, `pinned` in their own TOML header for resilience. If `index.toml` is missing or out of sync with the snapshot files on disk, `snapshot::list` rebuilds the index from the headers; orphaned files become re-indexed entries, and stale index entries that point at a missing file are dropped silently. This preserves the move/copy/delete-as-one-atom property of the `<profile>.snapshots/` folder.

---

## Navigation flow

Primary user flows in the new IA, rendered as state transitions:

**Authoring flow (engine paused or stopped):**
1. App start → no last-used profile → workspace empty state, Profiles panel auto-open.
2. Click `+ New` → expanding row → name + template → submit → new profile is active, mapping list empty, Profiles panel stays open.
3. Click `+ Add mapping` → expanding row → name + capture input → row commits, becomes active selection.
4. Editor displays the new mapping. Add pipeline stages, configure each inline. All edits auto-commit.
5. Repeat (3) and (4) until profile is complete.
6. (Optionally) `Ctrl+S` → labelled snapshot → "before testing in game".

**Tuning flow (engine running, game in another window):**
1. App start → last-used profile auto-loaded → workspace shows live state.
2. AutoSessionStart snapshot fires unless skipped by hash dedup.
3. User clicks a mapping in the left rail → editor loads it.
4. User drags deadzone outer threshold → engine state updates live → game response updates → user feels difference.
5. (Optionally) `Ctrl+Z` if the change made it worse.
6. Engine is in Combat (button held); user wants to tweak Landing's curve. User clicks Landing tab → banner appears → click `Activate Landing` → engine forces to Landing → user tweaks → click `Release` when satisfied.
7. (Optionally) `Ctrl+S` with label → snapshot for cross-session safety.

**Recovery flow (last week's curve was better):**
1. Click profile name in top bar (or click Profiles tool) → Profiles panel opens.
2. Find active profile, hover to reveal actions, click `Snapshots`.
3. Snapshot list shows in the sub-section. Find the date.
4. Click `Restore →` on the row → F4 confirmation → confirm.
5. Engine takes auto-before-restore snapshot, loads the snapshot file as the active profile, mapping list refreshes, editor refreshes. Toast confirms.

**Discovery flow ("what touches this axis?"):**
1. Click Devices in top bar → panel opens.
2. Click the device → drill-in expands.
3. Find the axis. Read the role badge and mapping name. Or click the axis to expand into calibration + Used-by panel for full backref list.

---

## Master plan rewrite (F6–F14)

The current master plan (`2026-04-24-egui-to-dioxus-rewrite-design.md`) declares F6–F14 provisional pending F5. This section is the rewrite. **After F5 merges, the parent master plan is updated to incorporate the changes below.**

Each feature lists the impeccable commands its focused plan should invoke. Commands are advisory — F6 implementation may decide to skip ones that don't apply.

### F6 — Snapshot module in `inputforge-core`

**Type:** core-only (no GUI, no shell changes)
**Replaces:** new feature
**Owns:**
- `crates/inputforge-core/src/snapshot/` module per the public surface in this spec; engine command extensions; serialization for `index.toml`; atomic file writes; content hashing (BLAKE3); FIFO eviction respecting pinned flags; AutoSessionStart trigger at end of `LoadProfile`; AutoBeforeRestore trigger at start of `RestoreSnapshot`.
- Engine `AppState.mode_force: Option<ForcedMode>` field plus the `ForcedMode` enum and `EngineCommand::ForceMode { mode } | ReleaseMode` lifecycle commands (these power F7's Activate/Release banner buttons, and pause mode-change rules while `mode_force.is_some()`).
- `crates/inputforge-core/src/preferences/` module — typed `Preferences` struct (currently `{ snapshot: SnapshotConfig }`, designed for additive extension), TOML schema, OS-specific on-disk location resolved via the `directories` crate (`%APPDATA%\inputforge\preferences.toml` on Windows; `$XDG_CONFIG_HOME/inputforge/preferences.toml` or `~/.config/inputforge/preferences.toml` on Linux; `~/Library/Application Support/inputforge/preferences.toml` on macOS — F6 commits the exact paths), serde-driven read/write with defaults written on first launch, and `Preferences::load() -> Result<Preferences>` called once during engine startup. The snapshot module reads `SnapshotConfig` from `Preferences` rather than hard-coding defaults — snapshot prefs are configurable via direct TOML edit from the moment F6 ships, even before any UI exists. New `EngineCommand::ReloadPreferences` re-reads the file on demand (wired in F6 even though no caller exists yet; F15 is the first consumer).
**Depends on:** F1–F4 (no GUI dependency, but lands after F4 to keep ordering linear); existing `Profile::load` / `Profile::save` infrastructure.
**Blocks:** F12 (calibration save flow uses no snapshot affordance, but file-system layout assumed); F13 (Profiles surface displays snapshots).

**Acceptance:**
- Public API matches the surface in this spec.
- Round-trip tests: create + list + restore + delete + prune.
- Concurrent-safety test: engine writing profile + concurrent snapshot create — no data loss, well-defined behavior.
- Pruning honors pinned flag.
- AutoSessionStart skipped when content hash matches latest.
- AutoBeforeRestore always fires.
- Index-recovery test: delete `index.toml` and verify `list()` rebuilds it from snapshot file headers; verify orphaned files are re-indexed and stale entries pointing at missing files are dropped.
- `ForceMode` / `ReleaseMode` commands flip `AppState.mode_force` and pause/resume mode-change rules; round-trip test asserts rules fire in normal mode and are skipped while forced.
- Preferences round-trip: defaults written on first launch round-trip byte-identical when re-read with no changes.
- Snapshot module honors `Preferences.snapshot.max_count` and `skip_if_unchanged` — override defaults via the TOML and verify the snapshot module observes the change.
- `EngineCommand::ReloadPreferences` re-reads the file and updates the in-memory `Preferences`; subsequent snapshot ops use the new values.
- Hand-edited TOML with valid schema is honored on next engine launch (no UI required to configure).
- All ops emit structured tracing events for observability.

**Impeccable commands:** none — this feature has no UI surface. Quality bar lives in code review.

---

### F7 — Chrome shell (top bar, mode tabs, status bar, banner)

**Type:** GUI shell upgrade
**Replaces:** F3's `PlaceholderShell` + parts of F3's `StatusBarView`
**Owns:** the top bar (engine pill, profile name, mode tabs, secondary tools cluster), the divergence/forced banner, the thin status bar, all chrome typography and color application, the chrome-level click handlers (engine toggle, profile-name → Profiles panel, mode tab activate/select, mode tab right-click menu, tool-cluster open/close); the side-panel **Replace discipline** — the shared right-side panel slot, the toggle wiring, and the invariant that opening either Devices or Profiles closes the other. F12 and F13 plug content into this slot rather than each defining their own toggle.
**Depends on:** F2 design system, F3 shell scaffold, F4 (banner uses no Dialog but copy patterns may share).
**Blocks:** F8 (left-rail content), F12 (Devices toggle), F13 (Profiles toggle).

**Key requirements:**
- Engine pill clickable; aria-live transitions; click sends `EngineCommand::Activate` or `Deactivate`. Visual states: Running (live green), Paused (warning amber), Stopped (error red).
- Mode tabs render editing scope; active tab gets focus-cyan underline; runtime marker dot is computed from `MetaSnapshot.current_mode`. Marker color is green when natural, amber when forced. Forced state is read from `AppState.mode_force` (added in F6) via the existing `pub(crate) MetaSnapshot` projection at `crates/inputforge-gui-dx/src/context.rs:42`; F7 extends `MetaSnapshot` with a `mode_force: Option<ForcedMode>` projection of the engine field — no new public engine API beyond F6's field/commands.
- Mode tab right-click menu uses F2 Menu primitive; items: Activate · Rename · Delete · Set as default. Active mode's Activate item is disabled. Keyboard equivalent: focus a mode tab and press **Shift+F10** (or the context-menu key) to open the same menu.
- Banner appears between top bar and main grid only when divergent or forced. Uses existing color tokens (`control` violet for diverge, `warning` amber for forced).
- Status bar: device count + warning count + profile path. Glance-only — no clickable elements.
- Profile name click opens Profiles panel; Profile name disabled-style when no profile loaded.

**Impeccable commands (recommended invocations):**
- `impeccable:shape` — at planning time, before implementation: shape the chrome layout including spacing, alignment of pill / name / tabs / tools / banner.
- `impeccable:frontend-design` — primary visual treatment of chrome (gutters, borders, typography, hover/active states).
- `impeccable:layout` — rhythm of the top bar; gap between pill / name / mode tabs / tools; banner placement.
- `impeccable:typeset` — chrome typography hierarchy (profile name vs mode-tab label vs tool label vs status bar text).
- `impeccable:clarify` — Activate / Release / "Editing X — engine is in Y" copy; engine pill hover hint copy; mode tab right-click labels.
- `impeccable:polish` — final pass.

---

### F8 — Mapping list (left rail)

**Type:** core screen
**Replaces:** old F6 (Left Panel — Device List + Input Tree). The left rail is no longer device-rooted.
**Owns:** the mapping-list component, filter input, group headers, row component (name + source + glyph annotations), `+ Add mapping` expanding row, empty state, mapping-list-level keyboard navigation (Up/Down to move selection, Enter to focus editor, Cmd-F to focus filter); **the live-capture primitive** — a GUI-only modal state that subscribes to the existing `AppState.input_cache` and emits the next observed input event. Mirrors today's calibration `Record range` pattern at `crates/inputforge-gui/src/panels/calibration_window.rs:74-142` (GUI-only `RecordingMode` enum, no new engine command).
**Depends on:** F7 chrome (so the rail has a shell to live in), F2 components, F4 dialog (used for delete confirmation triggered from a row's context menu).
**Blocks:** F9 (editor renders from the rail's selection); the live-capture primitive is reused by F9 (`change input` and the merge-stage secondary-input picker), F10 / F11 (any "press an input" affordance in their stage editors), and F12 (axis drill-in `Record range` after migration to the new shared primitive).

**Key requirements:**
- Rail width fixed (revisit during implementation; ~280px proposed).
- Rows ordered by group then by mapping declaration order in the active mode.
- Filter is name-and-source substring match, case-insensitive.
- Annotation glyphs rendered with the documented colors (output gold for `+`, control violet for `⊕`).
- Active row uses left-border focus-cyan + tinted background.
- `+ Add mapping` expanding row implements the live-capture flow (listens for an input on any device, populates source field on first event).
- Empty state: Display 32px "No mappings yet" + `+ Add mapping` primary button + helper.
- Right-click on a row: Rename · Duplicate · Delete (Delete triggers F4 destructive confirm).

**Impeccable commands:**
- `impeccable:shape` — rail structure including group hierarchy, row anatomy, expanding-row position.
- `impeccable:frontend-design` — primary visual.
- `impeccable:layout` — row vertical rhythm, group-header spacing, indent for source line.
- `impeccable:typeset` — name vs source typography contrast in the dense range.
- `impeccable:clarify` — empty-state copy, filter placeholder.
- `impeccable:polish` — final pass.

---

### F9 — Mapping editor (pipeline structure)

**Type:** core screen — the heaviest IA-level surface
**Replaces:** old F7 (Mapping Editor — Shell + Action List) + old F8 (Action Config Forms), merged because the new IA collapses the action-list and per-action-config into one inline-edit pipeline view.
**Owns:** the editor component framing (header, name field, input field, live readout, pipeline graph, undo recap, inactive-in-runtime hint); the pipeline-graph component including chain layout, Conditional branch rendering, MergeAxis stage with secondary-input picker, stage add/remove/reorder; session-undo log per mapping; live-input/output binding to the F1 polling Signal.
**Defers to:** F10 (curve editor inside the curve stage), F11 (deadzone editor inside the deadzone stage). F9's job is the *frame* and the graph; the heavy widget editors are sub-features.
**Depends on:** F8 (selection source), F7 (shell), F1 polling.
**Blocks:** F10, F11.

**Key requirements:**
- Selection-driven: when the active mapping in F8's rail changes, the editor reloads.
- All edits commit live via `EngineCommand::SetMapping`. No save buttons, no working copy.
- Undo log: per-mapping in-memory stack, cleared on profile switch (via F4 confirm if non-empty). Affordance: keyboard primary; small recap line in the editor footer ("Last change: …").
- Pipeline graph: SVG + Dioxus components. Each stage is a node; the chain is drawn left-to-right; Conditional opens a vertical branch with `if_true` / `if_false` columns indented and labeled.
- MergeAxis stage editor inline: operation picker (Bidirectional / Average / Maximum) + secondary input picker (live-capture or device drilldown). Secondary input shown in the stage label and the row's source annotation.
- Conditional condition editor inline: condition kind picker + operand fields (`ButtonPressed { input }` shows input picker).
- Inactive-in-runtime hint: rendered when `editing_mode != runtime_mode`. Copy is fixed: "Inactive in current runtime mode. Engine is in *<runtime>*; this mapping fires only in *<editing>*."
- Live readout layout for merge mappings: two source rows + dashed divider + merged result row.

**Impeccable commands:**
- `impeccable:shape` — heavy structural design: graph editor layout strategy, branch visualization, secondary-input picker placement.
- `impeccable:frontend-design` — primary visual; this surface is the user's home base.
- `impeccable:layout` — pipeline-stage spacing, branch indentation, live-bar placement.
- `impeccable:typeset` — labels (Name, Input, Live), pipeline stage labels (curve / deadzone / merge / etc.).
- `impeccable:animate` — selection state transitions, stage expansion, undo recap fade.
- `impeccable:clarify` — Inactive-in-runtime copy, undo affordance copy, condition editor copy.
- `impeccable:harden` — error states (engine crash / unreachable, mapping with disconnected source, malformed action), keyboard reachability.
- `impeccable:polish` — final pass.

---

### F10 — Curve editor (SVG inside curve stage) — *signature feature*

**Type:** core screen — heavy widget
**Replaces:** old F9 (Mapping Editor — Curve Editor)
**Owns:** the SVG bezier curve editor; control-point and handle drag interactions; symmetric mode logic; axis labels and tick marks; live-value overlay; keyboard manipulation of points and handles for accessibility; the inline expansion within the curve pipeline stage.
**Depends on:** F9 (lives inside a pipeline stage); F1 polling (live-value overlay).

**Special-care notes** (the user explicitly flagged this feature for visual ambition):

- The curve editor is the **primary tool of the tuning session**. A power user will spend more time inside this editor than any other surface in the GUI. Visual quality and interaction quality both directly determine whether the rewrite delivers on PRODUCT.md's promise.
- Reference quality bar: synthesizer envelope editors (Bitwig, Ableton's tools), DAW LFO designers, color-grading curve tools (DaVinci Resolve, Lightroom). These are precision instruments that *feel* good to use.
- The editor should **earn distinctive treatment**. Most surfaces in the GUI are restrained — the curve editor is permitted to push past safe defaults: micro-grid lines, optional tick guides, a live-value tracking dot that follows the input, a faint trail of recent positions, snap-to-quarter behavior with visual snap feedback.
- Keyboard accessibility is non-negotiable. Tab through points, arrow keys to nudge by 1% / 10% with shift, Tab into handles, Enter to add a point at cursor.

**Acceptance:**
- All today's egui curve-editor behavior preserved (drag points, drag handles, symmetric mode, add/remove points, reset).
- Bezier math is direct port of `crates/inputforge-gui/src/widgets/curve_editor/mutation.rs` — re-derivation forbidden (parent plan risk).
- Live-value tracking dot visible at all times when editor is open.
- Keyboard navigation reaches every control.
- Reduced motion honored: position transitions instant, no easing on tracking dot.

**Impeccable commands:**
- `impeccable:shape` — interaction model deep-design (drag mechanics, snap behavior, symmetric-mode UX, keyboard nav).
- `impeccable:frontend-design` — primary visual treatment; this is the surface where ambition shows.
- `impeccable:bolder` — push past safe defaults toward instrument-like richness (optional micro-grid, position trails, snap feedback animations).
- `impeccable:delight` — purposeful character moments (live tracking dot, smooth handle motion).
- `impeccable:animate` — drag interactions, hover responsiveness, value-tracking animations under prefers-reduced-motion-respecting easing.
- `impeccable:live` — variant exploration: try multiple visual treatments side-by-side in the browser before committing.
- `impeccable:typeset` — axis tick labels, value readouts.
- `impeccable:audit` — keyboard accessibility, focus rings on dark background, color-blind-safe live-value contrast against curve.
- `impeccable:polish` — final pass.

---

### F11 — Deadzone editor (SVG inside deadzone stage) — *signature feature*

**Type:** core screen — heavy widget
**Replaces:** old F10 (Mapping Editor — Deadzone Editor)
**Owns:** the deadzone visualization; inner / outer threshold editors; live-axis-value overlay showing input vs deadzone-applied output; the inline expansion within the deadzone pipeline stage.

**Special-care notes** (also flagged for visual ambition):

- Like the curve editor, the deadzone editor is a primary tuning instrument. Its aesthetic should be coherent with the curve editor: same animation timing, same precision feel, same instrumented-ness.
- The visualization should make the **transformation legible** — input on one axis, output on a perpendicular axis, with the deadzone curve drawn between. Threshold drag handles sit on the curve.
- Live overlay: a moving point on the input-output curve showing where the live signal currently is. This is the answer to PRODUCT.md's "live data is the contract" — the user sees exactly what the deadzone is doing to their input.

**Acceptance:**
- Inner / outer thresholds editable both via drag-handles on the curve and via numeric input fields.
- Live overlay always visible.
- Reset returns to default thresholds (no deadzone).
- Keyboard reachable.

**Impeccable commands:**
- `impeccable:shape` — interaction model + visualization geometry.
- `impeccable:frontend-design` — primary visual treatment; coherent with F10's curve editor.
- `impeccable:bolder` — push past flat-rectangle deadzone visualization toward something instrumented.
- `impeccable:delight` — live tracking point, smooth threshold drag.
- `impeccable:animate` — coherent with F10.
- `impeccable:live` — variant exploration alongside F10.
- `impeccable:typeset` — threshold value labels.
- `impeccable:audit` — keyboard accessibility.
- `impeccable:polish` — final pass.

**Coordination:** F10 and F11 should share an early `impeccable:shape` invocation that sets a coherent visual language for both editors. Inconsistency between them would feel cheap.

---

### F12 — Devices side panel + Calibration drill-in

**Type:** secondary surface
**Replaces:** old F12 (Input Viewer Surface) + old F13 (Calibration Surface), merged because the new IA puts calibration inside Devices via axis drill-in.
**Owns:** the Devices side panel component; the device-list section; the device drill-in section (axes/buttons/hats with live values + role badges + `cal` pill); the axis drill-in calibration editor (raw + calibrated bars, number fields, Record/Set center/Reset, dirty banner, used-by backref); panel open/close mechanics; replace discipline coordination with F13.
**Depends on:** F7 chrome (panel toggle button), F1 polling (live values), F2 components, F4 dialog (Reset confirmation), F6 (no direct dep, but engine command surface for `SetCalibration`/`SaveCalibrations` is unchanged), F11 (the deadzone editor doesn't appear here, but axis drill-in's calibration save UX is the only explicit-save surface in the GUI — quality bar must match).
**Blocks:** none. The shared right-side panel slot and Replace logic are owned by F7; F12 only registers as a slot consumer.

**Key requirements:**
- Side panel slides from the right; doesn't dim workspace; doesn't trap focus from the mapping list / editor.
- Replace discipline: opening Devices closes Profiles, and vice versa.
- Disconnection: red dot, italic muted name, "K mappings affected"; drill-in body shows affected mappings.
- Calibration drill: raw bar + calibrated bar same width; number fields use mono with tabular figures; Record range toggle latches min/max while user moves the stick; Reset is destructive (F4 confirm).
- `cal` pill rendered in CRT-green outline only on axes with InputForge calibration; absence is silent.
- Used-by backref reuses the multi-input-pattern role badges (`primary` / `merge 2nd` / `gate`).
- Top-bar Calibration item routes to Devices drilled into the first uncalibrated axis (or last-used device).

**Impeccable commands:**
- `impeccable:shape` — drill-in mechanics, panel structure, calibration editor layout.
- `impeccable:frontend-design` — primary visual.
- `impeccable:layout` — list rhythm, drill-in transitions, calibration field grouping.
- `impeccable:typeset` — device names, input labels, live values, calibration numbers.
- `impeccable:animate` — drill-in expand/collapse motion, live signal smoothness.
- `impeccable:clarify` — Record / Set center / Reset labels, dirty banner copy, Reset confirmation copy.
- `impeccable:harden` — disconnection edge cases, mid-calibration disconnection, calibration with missing device.
- `impeccable:onboard` — first-time Record range flow, "what does Record range do?" hint.
- `impeccable:polish` — final pass.

---

### F13 — Profiles side panel + Snapshots + no-profile empty state

**Type:** secondary surface — also owns the no-profile workspace state.
**Replaces:** old F14 (Profile Surface), expanded to absorb snapshot UI.
**Owns:** the Profiles side panel (header + library + Snapshots sub-section); profile library row component; per-row hover-revealed action cluster; `+ New` inline expanding-row flow; `Open file…` OS picker; Snapshots sub-section bound to active or selected profile; per-snapshot action cluster (Pin/Unpin · Rename · Delete · Restore →); workspace empty state when no profile loaded; auto-open behavior.
**Depends on:** F7 chrome (panel toggle, profile-name click target, shared panel slot, Replace discipline), F4 dialog (multiple destructive ops), F6 (snapshot module + engine commands).
**Blocks:** none.
**Triggers:** default feature flag flips to `gui-dioxus` when this merges, per the migration strategy. All core and secondary surfaces are in place at this point.

**Key requirements:**
- Plug into F7's shared right-side panel slot — opening Profiles closes Devices via F7's Replace logic.
- Library row: name, active pill (active row only, pinned at top), meta (mappings + modes + last-edited).
- Hover-revealed actions per row; right-click context menu mirrors.
- `+ New` expanding row: name + template radio (Blank / Copy from active / Copy from selected); active when at least one library row is selected. Name commits on Enter or blur (no dialog); Esc cancels uncommitted text. Profile rename and snapshot rename use the same sync-on-Enter pattern, reusing `inputforge_core::profile::manager`'s synchronous file ops.
- Snapshots sub-section tied to active profile by default; clicking a non-active row's `Snapshots` action swaps the binding without opening the profile.
- `Ctrl+S` from anywhere in the GUI opens the manual-snapshot dialog (F4 dialog with optional label field; Enter commits, Esc cancels).
- Restore: F4 confirm → engine command (`RestoreSnapshot`) which auto-snapshots first then loads.
- Delete profile / snapshot / mapping all use F4 destructive shape with profile-specific copy.
- Switch profile: if undo stack non-empty, F4 confirm; else direct.
- Empty state: workspace + auto-open, Display typography heading, two primary actions, library pointer, no SVG, no marketing copy. Copy is fixed and enumerated in this spec.

**Impeccable commands:**
- `impeccable:shape` — library + snapshots structure, expanding-row New flow, empty-state composition.
- `impeccable:frontend-design` — primary visual.
- `impeccable:layout` — library rhythm, sub-section header weight, snapshot row density.
- `impeccable:typeset` — profile name vs meta vs snapshot timestamp typography.
- `impeccable:clarify` — empty-state copy is critical (this is the user's first impression on first launch); action labels; snapshot timestamp/relative-time formatting; destructive-confirmation copy variants.
- `impeccable:onboard` — the no-profile empty state is the heaviest onboard moment in the GUI; design it as such.
- `impeccable:harden` — file errors (missing profile file, corrupt index.toml, snapshot file with old schema version), backup recovery from corrupt index.
- `impeccable:polish` — final pass.

---

### F14 — Mode editing (beyond tab CRUD)

**Type:** core feature, downscoped from old F11 (Mode Editor)
**Replaces:** old F11. Most mode CRUD (create/rename/delete/select/activate) lives in the chrome's mode tabs and right-click menu (delivered in F7). F14 is what's left.
**Owns:** the `ChangeMode { strategy }` action editor inside the pipeline (a mode-change is a regular `Action`, so it gets pipeline-stage treatment); mode-tree visualization for profiles with many modes (if implementation finds it warranted); default-mode selector UI; mode-change rule UX patterns.
**Depends on:** F9 (lives inside a pipeline stage, like other actions).
**Blocks:** none.

**Key requirements:**
- `ChangeMode` editor: strategy picker (`SetMode` / `CycleModes`); `SetMode` sub-form: target mode picker; `CycleModes` sub-form: list of mode names + cycle behavior (forward / wrap / etc., per existing engine semantics).
- Mode-change predicates appear inside Conditional editors per F9 — no separate surface for "mode change rules", they're just conditional pipelines.
- Profile-level "default mode" selector: lives inside the mode tab right-click menu (F7) — F14 just verifies the wiring.
- If implementation discovers users need a separate mode-tree visualization, F14 may add a `Modes` section inside the Profiles side panel (parallel to Snapshots). Default plan: do not.

**Impeccable commands:**
- `impeccable:shape` — `ChangeMode` action editor structure.
- `impeccable:frontend-design` — visual treatment for `ChangeMode` stage (control-violet category, per design system).
- `impeccable:clarify` — strategy labels, cycle semantics description.
- `impeccable:polish` — final pass.

---

### F15 — Settings UI (preferences editor surface)

**Type:** secondary surface — editor on top of F6's preferences core.
**Replaces:** new feature
**Owns:** the settings surface itself (panel sub-section or dialog — F15 brainstorm decides); the form components that bind to F6's `Preferences` struct fields; the in-memory edit-clone and the commit flow (write TOML → dispatch `EngineCommand::ReloadPreferences`); cancel-discards-clone flow; any first-time-launch onboarding hint that points users to the settings entry-point. F15 does **not** own the TOML schema, the on-disk location, default values, or the read/write code — those are F6's responsibility (`inputforge_core::preferences`).
**Depends on:** F4 dialog primitive, F6 (the `Preferences` struct, on-disk TOML, and `ReloadPreferences` command must all exist), F13 (entry-point integration — the settings surface is reached from the Profiles panel header or the top bar's secondary tools cluster; F15 brainstorm picks one).
**Blocks:** F16 (UX polish & audit) — the settings surface should exist before the global polish sweep so it gets audited alongside the rest of the GUI.

**Key requirements:**
- Editor surface mutates an in-memory clone of `Preferences`; on commit, writes the TOML through F6's serializer and dispatches `EngineCommand::ReloadPreferences`.
- Cancel discards the in-memory clone without writing.
- Form is keyboard-reachable end-to-end; field validation surfaces inline.
- Settings surface invokes the F4 dialog primitive (or a Profiles-panel sub-section — F15 chooses). Either way, no new chrome modal pattern.
- After commit, the surface reflects the post-`ReloadPreferences` state (handles the case where another writer changed the file mid-edit by surfacing a non-destructive warning + reload action).

**Acceptance:**
- Changing `max_count` (snapshot prefs) from 10 to 5 prunes the active profile's snapshots immediately after commit, and the toast/log surfaces the eviction.
- Cancel after edits leaves the on-disk TOML byte-identical to its pre-edit state.
- `skip_if_unchanged` toggle round-trips through the editor and is honored on the next AutoSessionStart.
- Field labels, descriptions, and validation copy committed during F15 brainstorm.

**Impeccable commands:** `shape`, `frontend-design`, `layout`, `clarify`, `polish`.

---

### F16 — UX polish & audit (renamed; structurally unchanged)

**Type:** integration / finish
**Replaces:** old F15. Same shape, applied to the new IA.
**Owns:** the cross-feature polish pass; audit-driven fixes; keyboard navigation completeness; focus-ring sweep on dark background; light-theme gate (deferred unless earlier features forced a decision — F5 commits no light-theme work).

**Impeccable commands:**
- `impeccable:audit` — full sweep across all surfaces.
- `impeccable:polish` — full sweep, surface by surface.
- `impeccable:typeset` — global typography pass.
- `impeccable:harden` — global edge-case pass (engine restart mid-edit, profile delete while panel open, etc.).
- `impeccable:animate` — global motion pass; verify motion only confirms causality.
- `impeccable:adapt` — verify all surfaces survive the documented window-size range.
- `impeccable:critique` — final director-level review before F17.

**Acceptance:** audit findings at "medium" severity or higher resolved; keyboard-only end-to-end traversal possible.

---

### F17 — Cutover & cleanup (unchanged)

**Type:** integration / finish
**Replaces:** old F16, identical scope. Delete egui crate, rename Dioxus crate, drop feature flags, drop egui dependencies, drop `egui_kittest` snapshot tests, update README.

**Impeccable commands:** none — code-only feature.

---

## Net summary of changes vs old master plan

| Old | New | Status |
|---|---|---|
| F1 Dioxus scaffold | F1 unchanged | ✓ done |
| F2 Design system | F2 unchanged | ✓ done |
| F3 App shell + tray | F3 unchanged (placeholder shell remains until F7 replaces) | ✓ done |
| F4 Toast & dialog | F4 unchanged (dirty-confirm trigger set updates per this spec) | ✓ done |
| F5 IA redesign | this spec | ✓ this work |
| — | **F6 — Snapshot module in core** | new |
| F6 Left panel device list | **F8** (mapping list, mode-scoped) — re-rooted | reshaped |
| F7 Mapping editor shell | merged into **F9** | merged |
| F8 Mapping editor action config | merged into **F9** | merged |
| F9 Curve editor | **F10** — flagged for ambition | renumbered |
| F10 Deadzone editor | **F11** — flagged for ambition | renumbered |
| F11 Mode editor | downscoped to **F14** (rest absorbed by F7 chrome) | reshaped |
| F12 Input viewer | merged into **F12** (Devices) | merged |
| F13 Calibration | merged into **F12** (Devices) | merged |
| F14 Profile surface | **F13** (Profiles + Snapshots), feature flag flips here | reshaped |
| — | **F7 — Chrome shell upgrade** (top bar + status + banner) | new feature splitting from F3 placeholder |
| — | **F15 — Settings UI** (global app preferences) | new feature for snapshot prefs et al. |
| F15 UX polish | **F16** — UX polish & audit | renumbered |
| F16 Cutover | **F17** — Cutover & cleanup | renumbered |

**Counts:** parent plan had F1–F16 (16 features, 4 done, 1 in progress, 11 provisional). New plan: F1–F17 (17 features — adds Settings UI as F15; 4 done, this spec is F5, 12 onwards rewritten). One more feature; significantly different scope distribution.

**Default feature flag flip moves from F14 → F13.** All core surfaces and secondary surfaces are in place at F13; tray was wired in F3.

---

## Open questions

- **Pipeline-stage drag-reorder UX.** Today's egui reorders actions via up/down arrow buttons on each card. The new pipeline graph might want drag-to-reorder, or might keep arrow buttons. F9 decides; F5 commits no preference.
- **Mode-tree visualization for many-mode profiles.** F14 may surface this if needed; out of scope here.
- **Force-mode keyboard shortcut.** Right-click is documented; whether to add a keyboard shortcut for "Activate currently-edited mode" is open. Default: no shortcut. F7 may add if compelling.
- **Snapshot pruning visibility.** Auto-pruning happens silently; user might want a toast when an auto-snapshot is evicted. Default: no toast (would be noisy). F13 may revisit.
- **Deadzone handles.** Symmetric vs. independent handles per side. F11 commits one during its brainstorm; today's egui behavior is the floor.
- **Profile schema migrations** when restoring an old snapshot whose schema differs from the current. Default: best-effort migrate; if migration fails, surface an error toast. F13 owns the policy.
- **Testing story for the new IA.** The parent plan's open question on Dioxus testing remains. F16 / F17 commits an approach.

---

## Next steps

1. Commit this spec to git.
2. Update the parent master plan (`2026-04-24-egui-to-dioxus-rewrite-design.md`) to incorporate the F6–F17 rewrite (one more feature than the original sequence — F15 Settings UI is new). Replace the "Core Screens — *provisional pending F5*" section with the new feature list. Add a "Resolved by F5" status note where relevant.
3. Invoke `superpowers:writing-plans` to produce the focused plan for **F6** (snapshot module in core) — the first feature in the new sequence.
4. F7 onwards each get their own brainstorm + spec + plan cycle as they come up. F10 and F11 in particular should each get an `impeccable:shape` invocation early in their brainstorm to set the visual ambition before implementation.

---

## Appendix — wireframes referenced

The following HTML wireframes from the brainstorming session are persisted at `.superpowers/brainstorm/1682-1777320356/content/`. They are not the spec — this document is — but they make the visual decisions concrete and are linkable from per-feature plans:

- `nav-root.html` — four navigation root options.
- `mapping-list-detail.html` — mapping-list IA fidelity.
- `multi-input.html` — MergeAxis and Conditional rendering in a flat list.
- `chrome.html` — three chrome-model variants.
- `chrome-a-refined.html` — chosen variant with engine-toggle + editing-vs-runtime mode.
- `activate-mode.html` — Activate / Release banner states.
- `save-semantics.html` — working-copy vs auto-commit comparison.
- `backup-snapshots.html` — three layers of revert + Snapshots panel mockup.
- `snapshot-core.html` — core/GUI module split + per-row delete affordance.
- `devices-calibration.html` — Devices side panel + axis drill-in calibration.
- `calibration-no-warning.html` — calibration absence-warning removal.
- `profiles-surface.html` — Profiles panel + no-profile empty state.

---

## Follow-ups beyond F5 scope

- **`DESIGN.md` `error-active` token mismatch.** Front-matter declares `error-active: "#DD4846"` (DESIGN.md line 23) while the prose body cites the active state as `#D43F3F` (DESIGN.md line 199). The 4.6× contrast claim at DESIGN.md line 189 only matches `#DD4846`. Reconciled in a separate F2-owned PR; F5 ships unchanged and continues to use whichever token is currently in CSS via the design-system layer.
