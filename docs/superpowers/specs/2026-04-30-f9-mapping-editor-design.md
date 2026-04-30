# F9 Mapping Editor (Pipeline Structure): Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-30
**Parent spec:** [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md), Core screens feature F9
**IA root:** [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md), Mapping editor (center) section
**Predecessors:** [F1](./2026-04-24-f1-dioxus-scaffold-state-bridge-design.md) (state bridge), [F2](./2026-04-25-f2-design-system-design.md) (design system), [F3](./2026-04-26-f3-app-shell-tray-bridge-design.md) (legacy shell), [F4](./2026-04-26-f4-toast-dialog-design.md) (toast and dialog), [F5](./2026-04-27-f5-architecture-ia-redesign-design.md) (IA), F6 (snapshot module + preferences core), F7 (chrome shell), [F8](./2026-04-30-f8-mapping-list-design.md) (mapping list, live-capture primitive)
**Brainstorm artefacts:** wireframes persisted under `.superpowers/brainstorm/1141-1777577665/content/` (`editor-anatomy.html`, `stage-vocabulary.html`, `editor-c-applied.html`, `readout-bars.html`, `copy-pass.html`, `editor-consolidated.html`, `layout-pass.html`, `harden-checklist.html`, `polish-pass.html`)
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Product brief:** [`/PRODUCT.md`](../../PRODUCT.md)

---

## Context

F9 builds the **mapping editor** in the center column of the new Dioxus GUI. It is the heaviest IA-level surface per the parent plan and the primary tool of the tuning session per PRODUCT.md ("live data is the contract"). The editor reads the mapping selected by F8's left rail, surfaces its identity (name, input, output) and live signal, and renders the action pipeline as a graph of category-colored stages with inline editing. F10 (curve editor) and F11 (deadzone editor) plug into F9 stage bodies as sub-features.

F9 owns the editor frame (header, name field, input field, live readout, inactive-runtime hint, pipeline graph, undo recap), the pipeline graph component (chain layout, Conditional branch rendering, MergeAxis stage with secondary-input picker, stage add/remove/reorder), the per-mapping session-undo log, and the live-input/output binding to F1's polling Signal. All edits commit live via `EngineCommand::SetMapping`. There are no save buttons.

This spec is approval-ready: every surface decision below was validated in a brainstorming loop spanning a `/impeccable audit` plus four follow-up `/impeccable shape | clarify | layout | harden | polish` rounds with text Q&A and visual mockups.

---

## Confirmed design choices

The decisions below are recorded in order of dependency, each surfaced and approved during brainstorming.

### Editor frame anatomy

**1. Editor sections in fixed vertical order.** Header (title plus subtitle), Name field, Input field with rebind action, Live readout, Inactive-runtime hint (when divergent), Pipeline graph, Undo recap footer. Order matches the F5 spec's mapping editor section. All sections are 1 px divider separated; consistency over rhythm because the product register favors predictable structure.

**2. Header is `<h2>` plus subtitle line.** Mapping name renders as a real `<h2>` (the chrome top bar is the page-level `<h1>`). Subtitle reads `<source-label>   →   <output-label>` as one line in JetBrains Mono. Source and output truncation: wrap to two lines when overflow, with the arrow leading the second line. Sim users need full identifiers; do not truncate device names with ellipsis.

**3. Subtitle output tail is omitted when no MapToVJoy stage exists.** Engine permits keyboard-only or mode-change-only mappings (F8 contract). When no `Action::MapToVJoy` is present in the action tree, the subtitle reads `<source-label>` only, no arrow, no output. Live-readout OUT row is also hidden in this state.

**4. Name field grows to fill available space, capped.** `width: 100%; max-width: 480px;`. Long names scroll horizontally inside the input. The h2 above always shows the full current name truncated to one line with an F2 `Tooltip` on hover for long names.

**5. Input field shows readonly source label plus rebind action.** `<source-label>` is JetBrains Mono 12 px. The `rebind` button is an F2 `Button` ghost variant (28 px tall) sitting to the right; click arms F8's live-capture primitive in `Any` filter mode. Captured input replaces the mapping's `InputAddress` via a fresh `SetMapping` dispatch with the same actions and name. Cross-mode collision uses F8's redirect UX. Esc cancels the live-capture per F8's primitive contract. (Note: `editor-consolidated.html` shows rebind as a styled link; the harden-pass promoted it to a Button ghost variant. This spec is the source of truth.)

**6. Live readout is `IN` plus `OUT` rows.** Per source input touched by the mapping, one row: source-label tag (uppercase mono caption) + axis bar (live-green fill) + percentage (mono tabular). Output row appears only when `Action::MapToVJoy` is present. Bars are separated by a 1 px dashed divider in `--color-border-strong`. Both bars use `--color-live` (#2EE0A0) for the fill; output-gold is reserved for the MapToVJoy stage tint, never for the bar fill. Color does one job: live signal.

**7. Merge-mapping live readout shows two source rows + dashed divider + merged result row.** `IN 1` and `IN 2` for the two MergeAxis source inputs, then dashed divider, then a row labelled `IN` showing the merged value the rest of the pipeline operates on. Output row continues to follow as usual when MapToVJoy is present. Mirrors F5 spec's merge-mapping treatment.

**8. Inactive-runtime hint copy: `Engine is in <runtime>. Mapping fires only in <editing>.`** Two short sentences. Drops the original "Inactive in current runtime mode" preamble (the user is looking at the editor; the context is implicit). Keeps "fires" as the engine-domain verb. Copy is fixed; placeholder substitution at runtime. **F5 spec line 177 is amended to match this revision.**

**9. Inactive-runtime hint banner is a tinted card with no side stripe.** Background: 8 % violet tint (`rgba(154, 120, 214, 0.08)`). Border: 1 px violet at 22 % opacity. No `border-left` accent. Font: 12 px Inter, color `--color-control-badge-text` (#B89BEA). Renders between the live readout and the pipeline graph; visible only when `editing_mode != runtime_mode`. Treated as `role="status" aria-live="polite"`.

**10. Undo recap footer is the last committed change with keyboard hint.** Format: `<change-summary> · ⌃Z to undo`. The `⌃Z` glyph renders as a styled `<kbd>` with subtle bg tint (matches F2 keyboard hint patterns). No "Last change:" preamble. No engine-status dot. The chrome top bar's engine pill already carries that signal; the editor footer carries undo only.

#### Typography scale

Pinned by polish-pass (validated against `polish-pass.html` row e). Tokens consumed from F2's design system; the table below is the editor-side reference.

| Element | Family | Size | Weight | Line height | Notes |
|---|---|---|---|---|---|
| Mapping name (h2) | Inter | 20 px | 600 | 28 px | one-line; truncate with `Tooltip` on overflow |
| Subtitle | JetBrains Mono | 12 px | 400 | 18 px | wraps to 2 lines on overflow |
| Section labels (`IN`, `OUT`, `IN N`) | JetBrains Mono | 11 px | 500 (uppercase) | 14 px | `--color-text-subtle` |
| Stage title | Inter | 12 px | 500 | 16 px | category-tinted card |
| Stage summary | JetBrains Mono | 12 px | 400 | 16 px | `--color-text-muted`; right-align |
| Body field labels | Inter | 14 px | 500 | 20 px | F2 input labels |
| Body field values | JetBrains Mono (numeric) / Inter (text) | 14 px | 400 | 20 px | F2 input scale |
| Caption (branch labels, hints) | JetBrains Mono | 11 px | 500 (uppercase) | 14 px | `--color-control-badge-text` |
| Footer recap | Inter | 11 px | 400 | 14 px | `--color-text-muted`; `<kbd>` inline |

### Pipeline graph

**11. Pipeline is an ordered list of category-tinted stages.** `<ol>` semantically. Each stage is an `<li>`. Render order matches `Mapping::actions` declaration order. Stage card category color uses **Option C: soft category tint as the card background**, neutral border, no side-stripe accents. Per-category tint percentages: processing teal `--color-stage-tint-processing: 6%`, output gold `--color-stage-tint-output: 7%`, control violet `--color-stage-tint-control: 6%`. Tokens are committed in `assets/tokens/colors.css` as part of F9 implementation (CSS `color-mix` from existing category tokens). Categories: processing teal `#3FB8B0` (Deadzone, ResponseCurve, Invert), output gold `#C99846` (MergeAxis, MapToVJoy, MapToKeyboard), control violet `#9A78D6` (Conditional, ChangeMode).

**12. Stages click to expand inline, summary stays bound when expanded.** Stage header carries title, summary text, and an F2 `IconButton` chevron (rotated -90 deg when collapsed). Clicking anywhere on the header row toggles expand, not just the chevron (larger hit area without changing visual density). Summary text is **live-bound to the stage parameters** and updates in place as the user edits the body; the header reads as the at-a-glance readout, the body as the controls. Stage body opens below with a 1 px top border separator.

**13. Stage body editor is per-variant.** Each `Action` variant gets a body component. F9 owns the bodies for `Action::Invert` (no body, header only), `Action::MapToVJoy`, `Action::MapToKeyboard`, `Action::MergeAxis`, `Action::Conditional`. F10 owns `Action::ResponseCurve`. F11 owns `Action::Deadzone`. F14 owns `Action::ChangeMode`. F9 commits the stage frame plus the bodies it owns; F9 spec only includes the surface API for the deferred bodies.

**14. Conditional renders as collapsible card with indented sub-pipelines.** Predicate editor at the top of the body (condition kind picker plus operand fields). Below the predicate: `if true` and `if false` branches, each a 16 px-indented sub-pipeline. Branch labels are 11 px mono uppercase (caption scale) in `--color-control-badge-text`. Each branch is a nested `<ol>` with `aria-label="if true branch"` / `"if false branch"` at depth 1, and depth-qualified labels at depth >= 2 (`"if true branch (depth 2)"`, etc.); path qualification (`"if true -> if false branch"`) is deferred unless screen-reader testing surfaces ambiguity. No side stripes on branches; indent + label is the only visual cue.

**15. MergeAxis renders inline with operation picker plus secondary-input picker.** Stage body shows: operation picker (`Bidirectional` / `Average` / `Maximum`) as F2 `Select`, plus secondary-input picker (a row mirroring the editor's main Input field: `<source-label>` + `rebind` button that arms live capture). Secondary input is also surfaced in the source-label glyph (gold +) on F8's mapping-list row, per F8 spec. The merged result feeds the rest of the pipeline; live readout shows two source rows for merge mappings (per choice 7).

**16. Stage add affordances follow option 2A (end-only with louder empty-branch).** Each pipeline (outer plus each branch) gets a single small `+` at its end. Visual treatment: monospace `+`, `--color-border-strong`, no border, 14 px line height. **Empty branches** show a louder affordance: `+ Add first stage` button with violet tint (`rgba(154, 120, 214, 0.04)`) bg and violet dashed border at `rgba(184, 155, 234, 0.32)` (32 % opacity from `#B89BEA`), padded 8 px / 12 px. No between-stage `+` gutter; insertion happens via right-click menu (choice 18), drag-and-drop (see §"Stage drag-and-drop"), or palette.

**17. Add-stage palette categorizes the action variants.** Click `+` opens an F2 `Menu` anchored to the button. Sections: **Processing** (Response curve, Deadzone, Invert), **Output** (Map to vJoy, Map to keyboard, Merge axis), **Control** (Conditional, Change mode). Section labels in 11 px mono caps with category color. Click an item to append a default-configured action and dispatch `SetMapping`. Pattern mirrors today's egui `card_list::show_add_action_dropdown`.

**18. Right-click on a stage opens an action menu.** Items: Insert before, Insert after, Move up, Move down, Duplicate, Delete. Move up is disabled at index 0; Move down is disabled at the last index. Shift+F10 or the context-menu key is the keyboard equivalent on a focused stage header. Insert before/after open the same add palette as the `+` button but anchored to the stage. Delete dispatches `SetMapping` with the stage removed; F4 destructive confirm only when the stage is the last `MapToVJoy` (i.e., removing it strips the output) and the mapping has live editor presence. Destructive confirm composes F4's `Dialog` primitives directly (`Dialog` with two-button layout); the `ConfirmDestructive` pattern stub at `crates/inputforge-gui-dx/src/patterns/mod.rs` is a future home, F9 does not block on it.

### Empty state and error states

**19. Empty state composition: typographic, two lines, no count.** When `view.selected_mapping` is `None` and a profile is loaded, the editor renders a centered empty state. Title: `Select a mapping` (16 px Inter weight 600). Helper: `Pick a row in the rail, or click + Add mapping below the list to start one.` (12 px caption, color `--color-text-muted`). No SVG illustrations, no marketing softeners. Layout: vertically centered in the center column with min-height matching the editor frame.

**20. Engine offline: sticky banner inside the editor.** When the engine command channel is disconnected (engine torn down or crashed mid-edit), the editor surfaces a sticky banner above the pipeline graph in the same vocabulary as the inactive-runtime hint, but error-tinted: bg `rgba(242, 85, 85, 0.08)`, border `rgba(242, 85, 85, 0.22)`, text `--color-error`. Copy: `Engine offline. Edits not applied.` plus a `Restart engine` action (F2 Button ghost variant, error variant). Edits to fields stay locally responsive; SetMapping dispatches are dropped (mirrors F8 behavior) but the banner makes the dropping visible. The toast bridge already raises a Warning; the banner is the actionable surface.

**21. Malformed action: subtle red title plus error glyph in summary slot.** When a stage has invalid params (MergeAxis with cleared `second_input`, Conditional whose condition fails `validate_depth`, etc.), the stage title flips to `--color-error` and the summary slot reads a one-line fix hint (`Pick a secondary input`, `Predicate exceeds nesting limit`, etc.). No border or background change; the stage keeps its category tint. Reads as a fixable issue, not a system failure. The editor does not refuse to render; the user can expand the stage and supply the missing data.

### Per-mapping session-undo log

**22. Undo log is per-mapping, in-memory, cleared on profile flip.** Data shape: `HashMap<MappingKey, MappingHistory>` where `MappingKey = (String, InputAddress)` is declared in `frame/view_state.rs` and reused by `view.selected_mapping`, `ConfigSnapshot.selected_mapping_key`, and all editor key passing. `UndoEntry { kind: UndoKind, mapping_before: Mapping, label: String }` carries a full `Mapping` snapshot (cheap; bounded by stage count). Stack caps at 50 entries per mapping with FIFO eviction beyond cap; no settings UI in F9. Cleared on profile flip via the existing F4 dirty-confirm dialog when the stack is non-empty. Editing-mode flip does NOT clear the log (different scope; the log lives across mode tabs because mappings are mode-scoped).

**23. Undo and redo are keyboard primary, scoped to editor focus, with native textfield precedence for `Ctrl+Z` inside inputs.** `Ctrl+Z` pops the most recent entry, restores `mapping_before` via `SetMapping`, and pushes the popped entry to a redo stack. `Ctrl+Shift+Z` (Mac convention) or `Ctrl+Y` (Windows convention) reverses; both Redo bindings are active everywhere `Ctrl+Z` is. Shortcuts capture only when focus is inside the editor; outside the editor the shortcuts are unbound for now. **Inside a focused `<input>`** (name field, numeric stage-body field, KeyCombo capture, etc.), `Ctrl+Z` falls through to the browser's native textfield undo (keystroke-level). Once the field blurs (committing the edit to engine + undo log), `Ctrl+Z` drives the editor's per-mapping undo stack. `Ctrl+Shift+Z` and `Ctrl+Y` are unaffected by this rule: browsers do not bind either shortcut inside text inputs by default, so the editor's Redo handler captures them whether or not a field is focused. This matches OS conventions and aligns with choice 29's commit-on-blur dispatch model. The footer recap text reads the last entry from the active mapping's stack (`Undo: deadzone outer 92% → 95%` shows the inverse of what would happen if Ctrl+Z fires next).

### Keyboard, accessibility, and motion

**24. Tab order matches DOM order, no `tabindex` hacks.** Name field, Input rebind button, first stage header chevron, body fields when expanded, next stage header, add-stage button, undo recap (skip; non-interactive). DOM order matches visual order so default tab works.

**25. Stage chevron uses Space and Enter, not arrow keys.** Stage chevron is an F2 `IconButton` (32 px hit area). `Space` and `Enter` toggle expand. Arrow keys do NOT navigate stages by default (would conflict with body field input). Right-click or Shift+F10 opens the action menu.

**26. Esc semantics defer to F8's live-capture when armed.** F8's primitive owns Esc when capture is active (rebind flow, MergeAxis secondary picker). When capture is not armed, Esc on a focused stage is a no-op; collapsing happens via the chevron only.

**27. Focus rings use F2 token (`--color-border-focus` #5AB0FF) at 2 px outline + 2 px offset.** Verified visible against the 6/7/6 % category-tinted stage backgrounds (per choice 11). Implementation MUST use F2's focus styles, not override.

**28. Reduced motion: stage expand and collapse becomes instant. Live bars never animate.** Stage expand transition: 180 ms ease-out-quart by default; 0 ms under `@media (prefers-reduced-motion: reduce)`. Live readout bar fill: instant always (live data tracking does not get easing because it would lie about engine state). Inactive-hint banner appearance: 150 ms opacity fade by default, instant under reduced motion; never slides.

### Edit dispatch and conflict handling

**29. Frame-level edits dispatch `SetMapping` directly. Stage body edits use a local working copy committed on Enter or blur.** Frame-level: name change, stage add, stage remove, stage reorder (right-click Move up/Move down OR drag-and-drop, see §"Stage drag-and-drop"), rebind. All emit `SetMapping` with the full updated actions vector. Stage body edits: each stage body holds local state for its parameters (e.g., curve points, deadzone thresholds, condition predicate, merge secondary input). On commit (Enter on a numeric field, blur on a text field, selection change on a `Select`, drag-end on a slider, drag-end on an F10 curve handle), the stage body dispatches `SetMapping` with the full actions vector (the body knows its own index in the pipeline). This avoids visual flicker during drag while keeping all edits flowing through the engine.

**30. Cross-window conflict: trust the engine's last ack with focused-edit preservation.** No optimistic UI. The polling Signal projects engine state; the editor re-renders when the Signal changes. If another window edits the selected mapping while F9 has it open: **(a)** if no body field has focus and no drag is active, local working copies in stage bodies reset to engine state on the next polling tick and a Warning toast surfaces (`Mapping was edited externally`); **(b)** if a body field has focus or a drag is active, the reset is deferred, the toast surfaces immediately, but local state is preserved until blur or drag-end, at which point the edit commits and the next poll reconciles. This trades a single-window invariant (always-fresh) for a no-data-loss invariant during active edit, which the live-tuning workflow requires.

**31. Selected mapping deleted externally: fall back to empty state.** When `view.selected_mapping` is `Some(key)` but `key` is no longer in `ctx.config.read().mappings`, the editor reverts to the `Select a mapping` empty state. No special toast; the rail's deletion already communicates what happened.

## Non-goals (out of scope for this spec)

- **Pixel-perfect curve editor and deadzone editor.** F10 and F11 own those. F9 commits the stage frame plus the API the bodies plug into.
- **`ChangeMode` action editor.** F14 owns the `ChangeMode { strategy }` editor inside the pipeline. F9 commits the stage placeholder; the body comes from F14.
- **Stage header preview thumbnail visual decisions.** F9 commits the right-slot API (see §"Stage card anatomy" and §"F10 / F11 / F14 handoff"); the actual thumbnail visuals stay deferred to F10 and F11 brainstorms. F9 implementation ships chevron-only.
- **Mode-tree visualization.** F14's possible scope.
- **Slash-command palette (Cmd-K).** Out of scope; the chrome does not have a global command palette today. May be added later.
- **Light theme.** Out of scope per parent plan.

---

## IA architecture

### Module structure

```
crates/inputforge-gui-dx/src/
├── frame/
│   ├── mapping_editor/                          # F9 NEW, editor component tree
│   │   ├── mod.rs                               # Component<MappingEditor>, orchestrates sections
│   │   ├── header.rs                            # h2 title + subtitle line
│   │   ├── name_field.rs                        # name input + h2 binding
│   │   ├── input_field.rs                       # source label + rebind button (F8 live-capture consumer)
│   │   ├── live_readout.rs                      # IN / OUT bars, merge-mapping layout
│   │   ├── inactive_hint.rs                     # divergence banner (Engine is in X. Mapping fires only in Y.)
│   │   ├── empty_state.rs                       # Select a mapping
│   │   ├── engine_offline_banner.rs             # sticky error banner (channel-disconnected)
│   │   ├── undo_log.rs                          # UndoStore + Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y handlers
│   │   ├── pipeline/
│   │   │   ├── mod.rs                           # Component<Pipeline>, ordered list of stages
│   │   │   ├── stage.rs                         # Component<Stage>, header + body container
│   │   │   ├── stage_header.rs                  # title + summary + IconButton chevron + right-click menu
│   │   │   ├── stage_actions_menu.rs            # F2 Menu: Insert before, Insert after, Duplicate, Delete
│   │   │   ├── add_palette.rs                   # F2 Menu opened by + button: categorized action picker
│   │   │   ├── stage_body/
│   │   │   │   ├── mod.rs                       # variant dispatcher
│   │   │   │   ├── invert.rs                    # no-op (header only)
│   │   │   │   ├── map_to_vjoy.rs               # output device + axis pickers
│   │   │   │   ├── map_to_keyboard.rs           # KeyCombo editor
│   │   │   │   ├── merge_axis.rs                # operation picker + secondary input picker
│   │   │   │   ├── conditional.rs               # predicate editor + branch sub-pipelines
│   │   │   │   ├── response_curve.rs            # F10 placeholder (header only in F9)
│   │   │   │   ├── deadzone.rs                  # F11 placeholder (header only in F9)
│   │   │   │   └── change_mode.rs               # F14 placeholder (header only in F9)
│   │   │   └── tests.rs
│   │   └── tests.rs
│   └── layout/                                  # MODIFIED: wires <MappingEditor /> into if-layout__center slot
│       └── mod.rs
└── context.rs                                   # MODIFIED: extends ConfigSnapshot with selected_mapping_actions
```

CSS lives at `assets/frame/mapping_editor.css` keyed off the `.if-editor` class; tokens only, no raw color literals.

### Engine surface change

**One read-only helper added.** F9 dispatches `EngineCommand::SetMapping` (already shipped) for every edit; the engine handler at `crates/inputforge-core/src/engine/run.rs` is unchanged. F9 ships one new pure helper in `inputforge-core`:

```rust
/// Re-runs the action pipeline up to (but not including) `stop_at` against
/// the current engine state and returns the projected input value at that
/// point. Read-only; no engine command added. Used by F10's live-tracking
/// dot (and any future feature needing a per-stage live signal) without
/// duplicating action evaluation logic in the GUI.
pub fn evaluate_actions_through(
    actions: &[Action],
    state: &EngineState,
    stop_at: usize,
) -> InputValue { /* ... */ }
```

`stop_at = 0` returns the unprocessed input. `stop_at = actions.len()` returns the full pipeline output. F10 and F11 consume this helper from their stage bodies via the existing `inputforge-core` crate import; no new commands cross the engine command boundary.

### Data architecture

#### `ConfigSnapshot` extension

The editor needs the full `Vec<Action>` of the currently-selected mapping. Cloning every mapping's actions per polling tick is wasteful; cloning only the selected one is cheap. Pattern:

```rust
// context.rs

pub(crate) struct ConfigSnapshot {
    pub devices: Vec<DeviceState>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    pub mapped_inputs: HashSet<InputAddress>,
    pub mapping_names: HashMap<InputAddress, String>,
    pub mappings: Vec<MappingSummary>,
    pub selected_mapping_actions: Option<Vec<Action>>,   // NEW
    pub selected_mapping_key: Option<(String, InputAddress)>,  // NEW, paired sentinel
}
```

`ConfigSnapshot::from_state` takes an additional `&Option<(String, InputAddress)>` parameter (the active selection from `ViewState.selected_mapping`) and clones the matching mapping's actions when present. The `selected_mapping_key` field is the matching key recorded at the same tick; the editor compares its local view against this pair to detect cross-window conflicts.

The polling task (F1's snapshotter) reads `view.selected_mapping.peek()` once per tick and passes it into `ConfigSnapshot::from_state`. `ConfigSnapshot` derives `PartialEq` so Dioxus only re-renders when the snapshot actually changes.

#### Editor-internal state: `EditorState` provider

GUI-only state for the editor lives in a new context provider parallel to `ViewState` and the F8 `LiveCapture` provider:

```rust
// frame/mapping_editor/mod.rs

#[derive(Clone, Copy)]
pub(crate) struct EditorState {
    /// Per-mapping undo stacks. Cleared on profile flip.
    pub undo_log: Signal<UndoLog>,
    /// Stage IDs that are currently expanded. Resets on selection change.
    pub expanded_stages: Signal<HashSet<StageId>>,
    /// Right-click menu state (anchor + target stage).
    pub stage_menu: Signal<Option<StageMenuState>>,
    /// Per-stage validation hints surfaced in the stage header summary slot
    /// per choice 21. Variant bodies (F10/F11/F14 + F9-owned) compute the
    /// hint string on render and write it here; the stage header reads it.
    pub malformed_hints: Signal<HashMap<StageId, String>>,
}

pub(crate) fn use_editor_state_provider() -> EditorState { ... }
```

The `app_root` fn (`crates/inputforge-gui-dx/src/app.rs:17`) installs the provider via `use_context_provider` after the existing `ViewState` and `LiveCapture` providers.

#### `UndoLog` data shape

```rust
pub(crate) type MappingKey = (String, InputAddress);

pub(crate) struct UndoLog {
    /// Per-mapping undo stacks.
    stacks: HashMap<MappingKey, MappingHistory>,
}

pub(crate) struct MappingHistory {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
}

pub(crate) struct UndoEntry {
    pub kind: UndoKind,            // StageEdit / StageAdd / StageRemove / StageReorder / Rename / Rebind
    pub mapping_before: Mapping,   // full snapshot for restore
    pub label: String,             // e.g. "deadzone outer 92% -> 95%"
}

impl UndoLog {
    /// Push an edit entry. Appends to the mapping's undo stack and clears its redo stack.
    /// Enforces the F9 label convention; F10/F11/F14 bodies MUST go through this helper.
    pub(crate) fn push_edit(
        &mut self,
        key: MappingKey,
        before: Mapping,
        kind: UndoKind,
        label: String,
    ) { /* ... */ }
}
```

`UndoLog::push_edit` appends to `key`'s undo stack and clears its redo stack (standard undo semantics). `UndoLog::undo(key)` pops, dispatches `SetMapping` with `entry.mapping_before`, and pushes to redo. `redo` reverses. Default cap: 50 entries per mapping; FIFO eviction when exceeded.

**Label format convention** (enforced by `push_edit`'s callers, validated in tests):

| `UndoKind` | Label format | Example |
|---|---|---|
| `StageEdit` | `<stage-name>: <field> <before> -> <after>` | `deadzone outer: 92% -> 95%` |
| `StageAdd` | `add stage: <variant> at index <i>` | `add stage: ResponseCurve at index 2` |
| `StageRemove` | `remove stage: <variant> at index <i>` | `remove stage: Deadzone at index 0` |
| `StageReorder` | `move stage <variant> from <i> to <j>` | `move stage MergeAxis from 1 to 0` |
| `Rename` | `rename: '<old>' -> '<new>'` | `rename: 'X axis' -> 'Yaw'` |
| `Rebind` | `rebind: <old-source> -> <new-source>` | `rebind: VPC Stick X -> VKB Pedals Y` |

### Action surface coverage

| Action variant | Body owner | Body summary text |
|---|---|---|
| `Action::Invert` | F9 | None (no parameters) |
| `Action::Deadzone { config }` | F11 | `inner X% · outer Y%` |
| `Action::ResponseCurve { curve }` | F10 | `N points · symmetric` or `N points` |
| `Action::MapToVJoy { output }` | F9 | `vJoy <id> · <axis>` |
| `Action::MapToKeyboard { key }` | F9 | rendered KeyCombo (e.g., `Ctrl + Shift + Q`) |
| `Action::MergeAxis { second_input, operation }` | F9 | `<op> with <secondary-source-label>` |
| `Action::ChangeMode { strategy }` | F14 | strategy-dependent (e.g., `set Combat`, `cycle Combat → Landing`) |
| `Action::Conditional { condition, if_true, if_false }` | F9 | predicate summary (e.g., `if Btn 12 pressed`) |

F10, F11, and F14 each plug their body component into the variant dispatcher in `pipeline/stage_body/mod.rs`. F9 ships placeholders for those three variants that render the header only with a "F10 / F11 / F14 owns this body" caption inside the body slot, plus a chevron that toggles the placeholder.

---

## Pipeline graph

### Stage card anatomy

Per the approved `editor-consolidated.html` mockup, with polish-pass adjustments to spacing and typography. Where the consolidated mockup and this spec body diverge, the spec body is the source of truth (the mockup may be stale relative to subsequent harden/polish passes).

```text
┌───────────────────────────────────────────────────────────────┐
│ <bg = per-category tint>                                      │
│ <padding 8px 12px>                                            │
│                                                               │
│ <stage-title>  <stage-summary mono right-aligned>  <right-slot>│
│                                                               │
│ <if expanded:>                                                │
│   <1px top border in --color-border>                          │
│   <body>                                                      │
└───────────────────────────────────────────────────────────────┘
```

**Toggle target (a11y resolution).** The F2 `IconButton` wrapping the entire header row is the only interactive element in the header. The chevron (or thumbnail) inside the IconButton is a visual-cue child element, not its own `<button>`. Clicking anywhere on the row fires the IconButton's `onclick`, which toggles expand. This satisfies "click anywhere on header toggles" without nesting interactives.

**Right-slot prop.** The header exposes a `right_slot: Element` prop. Default: chevron-down SVG. F10/F11 may pass a 28x14 inline SVG preview thumbnail; the IconButton's 32x32 hit area, `aria-expanded`, and `aria-controls` are invariant. Preview thumbnails render *inside* the IconButton's 32x32 box.

**Chevron motion.** Chevron SVG: 16 px, `currentColor`, transform `rotate(-90deg)` when collapsed -> `rotate(0deg)` when expanded. CSS transition: `transform 180 ms ease-out-quart`; reduced-motion: instant.

**Typography.** Title: Inter 12 px weight 500, line-height 16 px. Summary: JetBrains Mono 12 px regular, color `--color-text-muted`, right-aligned (margin-left auto), line-height 16 px. See §"Typography scale" for the full table covering the editor frame.

**Stage body (when expanded):** 8 px top padding, 1 px top border in `--color-border` separating header from body. Body contents are variant-specific.

### Conditional stage body

**Engine shape (verified in `crates/inputforge-core/src/action/mod.rs:51-55`):**

```rust
Action::Conditional {
    condition: Condition,
    if_true: Vec<Action>,            // mandatory
    if_false: Option<Vec<Action>>,   // optional
}
```

`if_true` is mandatory and always present. `if_false` is optional; `None` means "do nothing on the false branch". The branch UIs never disappear; only their contents change.

Body layout, top to bottom:

1. **Predicate editor.** Condition kind picker (F2 `Select` with options `ButtonPressed`, `ButtonReleased`, `AxisInRange`, `HatDirection`, `All`, `Any`, `Not`). Operand fields render based on the selected kind. For input-bearing kinds: an Input field row mirroring the editor's main Input field (source label + rebind button). For `AxisInRange`: input row plus min/max numeric inputs. For `HatDirection`: input row plus a multi-select for direction set. For `All`, `Any`: a nested list of sub-conditions, each rendered as a card with a kind picker, recursively. For `Not`: a single nested condition card.

2. **`if true` branch.** Indented 16 px. Header: `if true` 11 px mono uppercase caption in `--color-control-badge-text`. Body: nested ordered list of stages (recursive; the same `<Pipeline>` component used by the outer pipeline). Empty branch shows `+ Add first stage` louder affordance per choice 16. Non-empty branch shows tiny `+` at the end. Round-trip: emptying `if_true` leaves an empty `Vec<Action>` (the field is mandatory, never set to `None`). The branch UI continues to render with the louder add-affordance.

3. **`if false` branch.** Same structure as `if true`. Always rendered. Round-trip: when `if_false` is `None` (engine default), F9 surfaces an empty branch with `+ Add first stage`. When the user adds a first stage, the editor sets `if_false: Some(vec![...])`. When the user deletes the last stage in `if_false`, the editor sets `if_false: None` (reset to "do nothing"). The branch UI never disappears; only its content does.

Nested Conditional stages can be added inside a branch like any other action; F9's recursion handles arbitrary depth up to `MAX_CONDITION_DEPTH = 32` (engine constant). The visual layout indents an additional 16 px per nesting level. **Default ship behaviour:** render through depth 32 with linear 16 px indent. The "(N more levels)" placeholder for branches deeper than 5 is deferred polish; impeccable:layout in implementation phase will validate the default or revise.

### MergeAxis stage body

Body layout:

1. **Operation picker.** F2 `Select` with options `Bidirectional`, `Average`, `Maximum`. 14 px label `Operation`, 14 px select width 200 px. Changing the operation dispatches `SetMapping` immediately.

2. **Secondary input picker.** Mirrors the editor's main Input field. Source label + `rebind` F2 Button ghost. Click rebind arms F8's `LiveCapture::start(CaptureFilter::AxesOnly)` (axes only because MergeAxis only makes sense between axes). Captured input writes to `Action::MergeAxis::second_input` and dispatches `SetMapping`.

The summary in the stage header reads `<op> with <secondary-source-label>` (e.g., `Average with VKB Pedals · Brake bal`). Long secondary labels truncate with ellipsis in the summary (the body shows the full label).

### MapToVJoy stage body

Two pickers stacked: device picker (F2 `Select` listing `VirtualDeviceConfig` ids from `ConfigSnapshot.virtual_devices`) and axis/button/hat picker (F2 `Select` listing the chosen device's available outputs). Output kind (Axis vs Button vs Hat) is implied by the picker pair. Changing either dispatches `SetMapping`.

### MapToKeyboard stage body

KeyCombo editor: a row of modifier toggles (Ctrl, Alt, Shift, Win/Meta) plus a key input field. Click in the key input and press a key to capture; Esc cancels. Backed by F2's existing keyboard input infrastructure.

### Invert stage body

No body. The header summary is empty (no parameters). Click-to-expand is allowed but the body just shows a placeholder caption (`Inverts the input value: x becomes -x.`).

### ChangeMode stage body (F14)

Placeholder. F9 ships an "F14 owns this body" caption. Header summary reads the strategy: `set Combat`, `cycle Combat → Landing`.

### Stage drag-and-drop

Cross-pipeline drag-and-drop. Stages drag between any pipeline (outer or any Conditional branch). Implementation uses native HTML5 drag-and-drop (Dioxus exposes `ondragstart`, `ondragover`, `ondrop` event props); no third-party library.

**Drop indicators.** During `dragover`, a 2 px horizontal accent bar in `--color-border-focus` renders between the target stages, marking the drop position.

**Validation.** Dropping a Conditional (with nested actions) into one of its own descendant branches is rejected (cycle prevention; the engine's `validate_depth` would also catch it on commit). Visual feedback on rejection: drop indicator turns `--color-error` for 200 ms, then disappears; no state change dispatched.

**Affordances.** A 6-dot drag handle renders on the left edge of each stage card on `:hover` (hidden otherwise). Cursor switches to `grab` / `grabbing` over the handle and during the drag. Move up / Move down right-click menu items remain (keyboard-accessible alternative to drag, see choice 18).

**Keyboard equivalent.** With a stage focused, `Alt+Up` / `Alt+Down` move the stage within its current pipeline (between siblings). `Alt+Left` / `Alt+Right` enter / exit the parent Conditional branch (this cross-pipeline keyboard move is an open question: if usability proves confusing during impeccable:harden, drop the cross-pipeline keyboard direction and rely on Move up/down + cut-paste via right-click).

**Commit.** Drop dispatches `SetMapping` with the new actions vector and pushes to `UndoLog` with `UndoKind::StageReorder`, label format per the helper convention (`move stage <variant> from <i> to <j>`).

---

## Live readout

Layout:

```
┌────┬─────────────────────────────────────────────┬───────┐
│ IN │ ────────████████████████░░░░░░░░░░░░        │ +0.64 │
├────┼─────────────────────────────────────────────┼───────┤   (1 px dashed)
│ OUT│ ────────█████████████░░░░░░░░░░░░░░░        │ +0.58 │
└────┴─────────────────────────────────────────────┴───────┘
```

Grid: 60 px label column, flexible bar column, 50 px percentage column. Gap: 12 px. Row vertical padding: 4 px.

Label column: 11 px JetBrains Mono uppercase caps (`IN`, `OUT`, `IN 1`, `IN 2`), color `--color-text-subtle`. Bar: 8 px tall, bg `--color-bg-sunken`, fill `--color-live` (#2EE0A0). Bipolar axes: fill anchored at 50 % center, extends left or right. Unipolar axes: fill anchored at 0 %. Polarity comes from `DeviceInfo.axis_polarities` for axis inputs; button rows show a binary fill (full or empty), hat rows show a directional indicator.

Percentage column: 12 px JetBrains Mono with `font-variant-numeric: tabular-nums`, right-aligned. Format: signed for bipolar (e.g., `+0.64`, `-0.32`), unsigned for unipolar.

Merge mappings render two `IN N` rows for the two source inputs, then a 1 px dashed `--color-border-strong` divider, then a row labelled `IN` for the merged value (this is what the rest of the pipeline operates on). Output row appears after the merged row.

OUT row is hidden when no `Action::MapToVJoy` is in the action tree.

**Accessibility.** Live readout is visual-only (no `aria-live`). Rationale: bar values update on every polling tick (~16 ms); announcing them would flood screen readers. Users relying on AT can read percentages from the percentage column on demand. Future work: an optional throttled `role="status"` summary if user research surfaces demand.

---

## Empty state

When `view.selected_mapping` is `None` and a profile is loaded:

```rsx
div { class: "if-editor__empty",
    div { class: "if-editor__empty-title", "Select a mapping" }
    div { class: "if-editor__empty-helper",
        "Pick a row in the rail, or click "
        kbd { "+ Add mapping" }
        " below the list to start one."
    }
}
```

Layout: vertically centered in the center column. Title 16 px Inter weight 600. Helper 12 px caption `--color-text-muted`. The `kbd` for `+ Add mapping` uses the same styled-kbd treatment as the undo recap footer.

When no profile is loaded, the layout's `EmptyState` (F13's responsibility) takes over the workspace and the editor is hidden behind it; F9 does not render the empty state in that case.

---

## Engine command surface

### Commands F9 dispatches

| Command | Trigger | Payload |
|---|---|---|
| `SetMapping` | name change, rebind, stage add, stage remove, stage reorder, stage body commit (Enter / blur / drag-end), undo, redo | `{ input, mode, name, actions }` |

F9 does not introduce new engine commands. The full mutation surface is `SetMapping`.

### Edit dispatch flow

1. User mutates a frame-level field (name, input via rebind, or pipeline structure via add/remove/reorder).
2. Editor builds the new full `Vec<Action>` plus name and dispatches `SetMapping`.
3. Editor pushes an `UndoEntry` into the active mapping's undo stack with a label matching the change.
4. Engine acks via the next polling tick; `ConfigSnapshot.selected_mapping_actions` updates; F9 re-renders.

Stage body edits follow the same flow but with a local-working-copy step:

1. User drags a slider or types in a field inside the stage body.
2. The body holds local state; the visual updates immediately without engine round-trip.
3. On commit (Enter, blur, or drag-end), the body builds the new actions vector (using its index in the pipeline plus the new parameters) and dispatches `SetMapping`.
4. Same undo and re-render flow as above.

Drag interactions (F10 curve handle, F11 deadzone threshold) coalesce intermediate positions within the body and dispatch only on drag-end. F9 commits this contract; F10 and F11 implement it.

### Error handling

- **Channel disconnected.** Engine torn down. SetMapping silently drops (mirrors F8). Engine-offline banner (choice 20) becomes visible, surfacing the offline state.
- **Engine-side IO errors.** Surface through `AppState.warnings`. The toast bridge (already shipped) emits a Warning toast.
- **Validation errors.** Engine rejects malformed actions (e.g., `Condition::validate_depth` failure). Engine emits a warning; toast surfaces; the stage card flips to malformed-action treatment (choice 21).

### Observability

Each dispatch emits a `tracing::info!` event:
`info!(target: "f9::mapping_editor", action = "rename" | "rebind" | "stage_add" | "stage_remove" | "stage_reorder" | "stage_edit" | "undo" | "redo", ?input, %mode, ?stage_index, ?action_kind)`.

Live capture starts and cancels through F8's primitive emit `debug!` events; F9 inherits.

---

## Testing strategy

Three tiers, mirroring F8's pattern.

### 1. Pure logic (Rust unit tests)

- `UndoLog::push_edit` / `undo` / `redo` round-trip with the full Mapping shape, including the 50-entry FIFO cap.
- Label-format convention enforced: each `UndoKind` produces the documented label shape.
- `selected_mapping_actions` projection in `ConfigSnapshot::from_state`: present when selection matches a mapping, None otherwise.
- Conditional condition editor's predicate-validation passes through `validate_depth`.
- Conditional `if_true` empty round-trip leaves `Vec::new()`; `if_false` empty round-trip sets `None`. (Verifies the engine-shape asymmetry.)
- MergeAxis with cleared `second_input` produces a malformed-state Mapping (used by the malformed-action visual treatment).
- `evaluate_actions_through(actions, state, 0)` returns the input untouched; `stop_at = actions.len()` returns the full pipeline output.

### 2. Component tests via `dioxus_ssr::render`

- Editor with seeded `ConfigSnapshot` containing a 4-stage pipeline (Deadzone, ResponseCurve, MergeAxis, MapToVJoy) renders all four stages with correct category tints, summary text, and chevron states.
- Editor with selected mapping that has no MapToVJoy: subtitle output tail absent, OUT row absent.
- Editor with no selected mapping: empty state renders.
- Editor with `editing_mode != runtime_mode`: inactive-runtime banner appears with the tighter copy.
- Engine offline (mocked channel disconnect): banner appears.
- Conditional with both branches non-empty: nested ordered lists render in correct DOM order with the right indents.
- Conditional with empty `if_false`: branch label appears, "+ Add first stage" affordance appears, no nested list.
- Stage with malformed action (MergeAxis with `second_input.device.0 == ""`): title color flips to error, summary slot reads fix hint.
- Stage summary text re-renders in place when the user edits a body parameter (live-binding per choice 12).
- Footer recap text rebinds after Ctrl+Z. Redo stack clears on a fresh edit.
- Channel disconnected: edits to fields stay locally responsive (input value updates), `SetMapping` dispatches drop, banner appears.
- Conditional with `validate_depth` failure: stage title color flips to error, summary slot reads `Predicate exceeds nesting limit`.
- External-edit reset: with no field focused, the local working copy resets and a `Mapping was edited externally` toast surfaces. With the name field focused, only the toast surfaces; the field value is preserved until blur.

### 3. Integration

The harness validates `frame::Layout` mounts. F9 plugs into the center slot. Unit + SSR tests above suffice; full integration is exercised during `impeccable:audit` at F16.

---

## Acceptance criteria

1. Editor renders inside `if-layout__center` when a profile is loaded and a mapping is selected.
2. Editor reverts to `Select a mapping` empty state when `view.selected_mapping` is `None`.
3. Header shows mapping name as `<h2>`. Subtitle reads `<source-label>   →   <output-label>` on one line, wrapping to two lines on overflow with the arrow leading the second line. Output tail and OUT readout row hide when no `MapToVJoy` stage exists.
4. Name field grows to fill column up to 480 px max-width; long names scroll horizontally.
5. Input field shows readonly source label plus an F2 `Button` ghost variant labelled `rebind`. Clicking arms F8's live-capture in `Any` mode. Captured input dispatches `SetMapping` and surfaces via the polling Signal.
6. Live readout: IN row uses live-green fill, percentage in mono tabular. OUT row uses same fill color (no output-gold in the bar fill). Bipolar axes anchor at 50 %, unipolar at 0 %.
7. Merge mappings render two source rows + dashed divider + merged result row. The merged row feeds downstream stages.
8. Inactive-runtime hint reads `Engine is in <runtime>. Mapping fires only in <editing>.` and renders only when `editing_mode != runtime_mode`. Tinted card vocabulary, no side stripe.
9. Pipeline stages render with Option C category tint backgrounds at the pinned per-category percentages (processing 6 %, output 7 %, control 6 %). Borders neutral. No side-stripe accents anywhere.
10. Each stage has a header with title, live-bound summary, and an F2 IconButton chevron. Header row is the toggle target. Stage body opens below with a 1 px top divider.
11. Conditional stage body renders a predicate editor plus two indented sub-pipelines (`if true`, `if false`). Empty branch shows `+ Add first stage` louder affordance; non-empty branch shows a tiny `+` at the end.
12. MergeAxis stage body renders an operation picker plus a secondary-input picker that arms F8 live-capture in `AxesOnly` mode. Stage summary reads `<op> with <secondary-source-label>`.
13. Right-click on a stage header opens an F2 Menu with Insert before, Insert after, Move up, Move down, Duplicate, Delete. Move up is disabled at the first stage; Move down is disabled at the last. Shift+F10 (or the OS context-menu key) on a focused stage header opens the same menu. Drag-and-drop reorder is also available (see §"Stage drag-and-drop").
14. `+ Add stage` button at the end of every pipeline opens an F2 Menu categorized into Processing, Output, Control. Clicking an item appends a default-configured action and dispatches `SetMapping`.
15. Per-mapping session-undo stack records every commit. `Ctrl+Z`, `Ctrl+Shift+Z`, and `Ctrl+Y` capture only when focus is inside the editor. Stack is cleared on profile flip via the existing F4 dirty-confirm dialog.
16. Footer shows the most recent undo entry's label plus a styled `<kbd>⌃Z</kbd>` shortcut. No engine-status dot.
17. Stage with malformed parameters shows the title in error red plus a fix-hint summary; no border or background change.
18. Engine offline (channel disconnected) surfaces a sticky banner above the pipeline reading `Engine offline. Edits not applied.` plus a `Restart engine` action. Edits to fields stay locally responsive.
19. Selected mapping deleted from another window: editor reverts to empty state silently.
20. Profile flipped while editor was open: editor resets per F8's `ViewState` reconciliation; scroll position resets.
21. Tab order matches DOM order. Stage chevron Space and Enter both toggle expand. Esc on a focused stage is a no-op when capture is not armed. Ctrl+Z, Ctrl+Shift+Z, and Ctrl+Y drive the editor undo log only when focus is outside text inputs; inside an input, Ctrl+Z falls through to native field undo until blur (Ctrl+Shift+Z and Ctrl+Y have no native textfield handler so they always drive editor redo).
22. Focus rings use F2's `--color-border-focus` (#5AB0FF) at 2 px outline + 2 px offset; visible against tinted stage backgrounds.
23. Stage expand and collapse transition is 180 ms ease-out-quart by default and 0 ms under `prefers-reduced-motion`. Live readout bars never animate. Inactive-hint banner uses 150 ms opacity fade by default and instant under reduced motion; never slides.
24. Live readout bars (IN, OUT, IN N, merged IN) all use `--color-live` (#2EE0A0) for fill. Output-gold is reserved for the MapToVJoy stage card tint and never appears in any bar fill.
25. Per-mapping undo stack caps at 50 entries; FIFO eviction beyond cap. No settings UI in F9.
26. Editing-mode tab flip preserves the per-mapping undo log. Only profile flip clears it (via F4 confirm).
27. External edit to the selected mapping surfaces a `Mapping was edited externally` Warning toast immediately. If a body field has focus or a drag is active, the local working-copy reset is deferred to blur or drag-end; otherwise the reset fires on the next polling tick.
28. Drag-and-drop reorder works between any pipeline (outer or any Conditional branch). Drop indicator: 2 px accent bar in `--color-border-focus`. Cycle-creating drops (a Conditional dropped into its own descendant branch) are rejected with a 200 ms error-red indicator and no state change.
29. F10's live-tracking dot reads `inputforge-core::evaluate_actions_through(actions, &state, stop_at: usize) -> InputValue`; F9 ships this helper.

---

## Impeccable command invocations (per F5 spec)

- `impeccable:audit` is partially resolved by this brainstorm. Remaining audit items address the broader app at F16.
- `impeccable:shape` resolved here for the editor frame, the pipeline graph layout, the Conditional branch rendering, and the MergeAxis secondary-input picker placement.
- `impeccable:layout` resolved here for stage header density (live-bound summary), add-stage placement (end-only with louder empty-branch affordance), and section spacing rhythm.
- `impeccable:clarify` resolved here for IN/OUT label caps, inactive-runtime hint copy, rebind verb, undo recap, label-format convention for `UndoLog::push_edit`.
- `impeccable:harden` resolved here for semantic markup, keyboard navigation, focus rings on tinted backgrounds, reduced-motion behavior, error states, edge cases.
- `impeccable:polish` resolved here for typography scale, spacing scale, empty state composition, header subtitle truncation, name-field width, engine-status dot removal, stage chevron implementation, section divider rhythm.
- `impeccable:frontend-design` recommended during F9 implementation to apply the chosen visual direction concretely (color, hover, active, focus states).
- `impeccable:animate` recommended during F9 implementation for selection transitions, stage expand/collapse, undo recap fade.

---

## Open questions and deferred items

- **Deeply nested Conditional branch collapse threshold.** Default ship behaviour: render through depth 32 with linear 16 px indent (per §"Conditional stage body"). The "(N more levels)" placeholder for branches deeper than 5 is deferred polish; impeccable:layout in implementation phase will validate or revise.
- **Cross-pipeline keyboard drag.** `Alt+Left` / `Alt+Right` (per §"Stage drag-and-drop") moves a stage out of or into a Conditional branch via keyboard. If usability proves confusing during impeccable:harden, drop the cross-pipeline keyboard direction and rely on Move up/down + cut-paste via right-click.
- **Path-qualified aria-labels for deeply nested Conditional branches.** Choice 14 commits depth-qualified labels at depth >= 2; full path qualification (`"if true -> if false branch"`) is deferred unless screen-reader testing surfaces ambiguity.
- **Slash-command palette.** Out of scope. May be added later as a global GUI feature.
- **F10/F11 preview thumbnail visuals.** Right-slot API committed in F9 (see §"Stage card anatomy"). Whether F10 and F11 ship thumbnails (and the exact SVG shape) is decided in their respective brainstorms.

---

## F10 / F11 / F14 handoff

This spec commits boundaries that downstream features must respect.

### Shared contracts (apply to all three)

- **Stage header right-slot API.** The stage header component exposes a `right_slot: Element` prop. Default: chevron-down SVG. F10/F11 may pass a 28x14 inline SVG preview thumbnail instead. The IconButton's 32 px hit area, `aria-expanded`, and `aria-controls` are invariant. Preview thumbnails render *inside* the IconButton's 32x32 box, not as a replacement element. The chevron's role transfers to the IconButton's `aria-label` ("Toggle stage body") when a thumbnail occupies the slot.
- **EditorState consumption.** Each body component reads `EditorState` via `use_context::<EditorState>()` and uses: `expanded_stages` (read), `undo_log` (write via `push_edit`, see below), `malformed_hints` (write).
- **Undo dispatch.** Each commit MUST call `editor_state.undo_log.write().push_edit(key, mapping_before, kind, label)` with `label` matching the F9 convention (see §"`UndoLog` data shape"). The `push_edit` helper enforces label format; do not bypass.
- **Malformed-action contract.** Validation lives in `inputforge-core` variant validators (`ResponseCurve::validate_points`, `Deadzone::validate_thresholds`, `ChangeMode::validate_strategy`). Each body computes its hint string on render and writes to `editor_state.malformed_hints[stage_id]`. The stage header reads this map and surfaces the hint in the summary slot per choice 21.

### F10 (curve editor) handoff

- **Stage body slot.** F10 owns `pipeline/stage_body/response_curve.rs`. F9 ships a placeholder body that F10 replaces.
- **Stage header summary.** F9 commits the format `N points · symmetric` or `N points`. F10 may refine the summary copy during its brainstorm.
- **Preview slot.** F10 decides during its brainstorm whether to ship a 28x14 inline SVG preview of the response curve in the right-slot.
- **Commit semantics.** Drag interactions (curve handle drag) coalesce intermediate positions in local body state and dispatch `SetMapping` only on drag-end, per choice 29. Numeric/text fields commit on Enter or blur. Each commit also pushes to `UndoLog`.
- **Live tracking.** F10's live-tracking dot reads `inputforge-core::evaluate_actions_through(actions, &state, stop_at: usize) -> InputValue` (F9 ships this helper, see §"Engine command surface"). The helper re-runs the pipeline up to the stage's index without duplicating engine logic in the GUI.

### F11 (deadzone editor) handoff

- **Stage body slot.** F11 owns `pipeline/stage_body/deadzone.rs`.
- **Stage header summary.** Format `inner X% · outer Y%`.
- **Preview slot.** F11 may ship a deadzone-curve thumbnail in the right-slot (same shared API as F10).
- **Commit semantics.** Drag interactions (inner/outer threshold drag) coalesce in local body state and dispatch on drag-end. Numeric fields commit on Enter or blur.
- **Live tracking.** Same `evaluate_actions_through` helper available; F11 may surface a live-input dot if its brainstorm calls for one.

### F14 (mode editing) handoff

- **Stage body slot.** F14 owns `pipeline/stage_body/change_mode.rs`.
- **Stage header summary.** Strategy-dependent (e.g., `set Combat`, `cycle Combat -> Landing`).
- **Preview slot.** F14 does not surface a preview; the right-slot stays as the default chevron.
- **Commit semantics.** F14's body is form-based, no drag interactions. Numeric/text fields commit on Enter or blur; `Select` widgets commit on selection change (mirroring MergeAxis operation picker, §"MergeAxis stage body"). No debouncing, no Apply button.
- **Live tracking.** Not applicable. ChangeMode emits a side-effect, no continuous projected value.

---

## F5 spec amendment

F5 spec line 177 currently reads:

> Inactive-in-runtime hint: rendered when `editing_mode != runtime_mode`. Copy is fixed: "Inactive in current runtime mode. Engine is in *<runtime>*; this mapping fires only in *<editing>*."

F9 brainstorm tightened this to two short sentences. F9 implementation phase will amend F5 line 177 to:

> Inactive-in-runtime hint: rendered when `editing_mode != runtime_mode`. Copy is fixed: "Engine is in *<runtime>*. Mapping fires only in *<editing>*."

This amendment lands as part of the F9 implementation PR.

---

## Em-dash project policy

The em-dash sweep originally folded into F9 has been split into its own precondition PR series (commits `ed0860a`, `b3b1c2b`, `8d99478` on `main`). Scope: PRODUCT.md, DESIGN.md, CLAUDE.md, all `docs/superpowers/specs/*.md` (F9 spec excluded; written em-dash-free), all `docs/superpowers/plans/*.md`, all `docs/superpowers/notes/*.md`, all `.css` and `.rs` under `crates/` (comments + UI copy), `crates/inputforge-gui-dx/README.md`. Replacement convention: comma > colon > semicolon > period > parens; en-dash numeric ranges -> hyphen. F9 implementation begins from this swept baseline; no further em-dash cleanup is part of F9.

---

## Net summary

| Component | F9 status | Notes |
|---|---|---|
| `frame/mapping_editor/` (12 files plus 8 stage_body files) | new | editor component tree |
| `frame/layout/mod.rs` | modified | wires `<MappingEditor />` into `if-layout__center` |
| `context.rs` | modified | extends `ConfigSnapshot` with `selected_mapping_actions` and `selected_mapping_key` |
| `assets/frame/mapping_editor.css` | new | editor styling |
| `assets/tokens/colors.css` | modified | adds `--color-stage-tint-{processing,output,control}` tokens |
| `inputforge-core::evaluate_actions_through` | new | read-only pipeline projection helper for F10 live-tracking |
| `frame/view_state.rs` | modified | declares `MappingKey` type alias |
| F5 spec line 177 | amended | hint copy tightened |

---

## Next steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce the focused plan for F9.
3. F9 implementation invokes the impeccable commands listed above during execution.
