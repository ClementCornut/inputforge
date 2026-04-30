# F8, Mapping List (Left Rail): Design Spec

**Status:** Design approved, ready for implementation plan
**Date:** 2026-04-30
**Parent spec:** [`2026-04-24-egui-to-dioxus-rewrite-design.md`](./2026-04-24-egui-to-dioxus-rewrite-design.md), Core screens feature F8
**IA root:** [`2026-04-27-f5-architecture-ia-redesign-design.md`](./2026-04-27-f5-architecture-ia-redesign-design.md), left-rail IA decisions
**Predecessors:** [F1](./2026-04-24-f1-dioxus-scaffold-state-bridge-design.md) (state bridge), [F2](./2026-04-25-f2-design-system-design.md) (design system), [F3](./2026-04-26-f3-app-shell-tray-bridge-design.md) (legacy shell), [F4](./2026-04-26-f4-toast-dialog-design.md) (toast + dialog), [F5](./2026-04-27-f5-architecture-ia-redesign-design.md) (IA), [F7 chrome shell] (shipped, top bar, mode tabs, banner, status bar)
**Brainstorm artefacts:** wireframes persisted under `.superpowers/brainstorm/1770-1777539306/content/` (`rail-resting.html`, `rail-active-states.html`, `rail-empty-states.html`).
**Design system:** [`/DESIGN.md`](../../DESIGN.md)
**Product brief:** [`/PRODUCT.md`](../../PRODUCT.md)

---

## Context

F8 builds the **mapping list (left rail)**, the navigation root of the new Dioxus GUI per F5. The rail replaces the egui device-tree left panel and re-roots navigation around mode-scoped mappings; hardware becomes a property of mappings rather than the navigation root.

F8 also owns one piece of shared infrastructure: the **live-capture primitive**, a GUI-only modal state that subscribes to `AppState.input_cache` and emits the next observed input event. F9, F10, F11, and F12 all reuse it.

This spec is approval-ready: every surface decision below was validated section-by-section in a brainstorming loop with text Q&A and visual mockups.

---

## Confirmed design choices

The decisions below are recorded in order of dependency, each surfaced and approved during brainstorming.

### Posture & data shape

**1. Group bucketing by *input kind*.** Rows group by the input kind of the mapping's root `InputAddress` (axis → AXES, button → BUTTONS, hat → HATS). Engine permits multi-output pipelines, mappings without a vJoy terminal (keyboard-only, mode-change-only), and incomplete mappings during construction; bucketing by input kind always works and matches the wireframe in F5. F5's "by terminal vJoy stage output kind" is superseded by this rule.

**2. Selection key = `Option<(mode, InputAddress)>`.** Mode-scoped literally. On editing-mode tab switch, the rail repopulates and selection clears (user must click again). Survives live engine churn, `InputAddress` is stable across `SetMapping` updates.

**3. Snapshot extension, not new Signal.** `ConfigSnapshot` gains a per-mapping summary list `mappings: Vec<MappingSummary>` populated once per polling tick from `Profile::mappings()`. Glyph state (MergeAxis present, input-Conditional present) is derived during the snapshot pass, not at render time. Mode filtering happens at render time inside the rail component via `use_memo` over `(config, editing_mode)`.

**4. Selection lives on `ViewState`.** A new `selected_mapping: Signal<Option<(String, InputAddress)>>` field on `ViewState`. Initialized to `None`. Reconciliation extends `use_view_state_provider`'s existing `use_effect` with a new branch: a sibling `last_editing_mode: Signal<String>` shadow (mirroring the existing `last_profile_name` peek-init pattern at `view_state.rs:75-76`) detects mode flips and clears `selected_mapping`. The existing profile-switch branch also clears `selected_mapping`. See §"`ViewState` extension" below for the full effect ordering.

### Live-capture primitive

**5. Module location: `src/patterns/live_capture/`.** Companion to `patterns/dirty_confirm.rs`. Behavioral primitive, not a component. F8 ships as the first consumer; F9, F10, F11, F12 import.

**6. Single-instance pattern.** Provided once via context in the `app_root` fn (`crates/inputforge-gui-dx/src/app.rs:17`), sibling of `ToastQueue`. Each consumer reads it via `use_context::<LiveCapture>()`. Starting a new capture cancels any in-flight one, there is exactly one capture at a time across the entire GUI.

**7. Baseline-and-edge detection.** `start()` snapshots the current input cache as a baseline; the polling effect compares *current vs baseline* (not current vs zero). This handles two real-world cases:
   - **Joystick stick already at 0.3 on X.** No false capture at start; capture fires only when X moves further (delta > deadband against baseline).
   - **Switch-style buttons that are mechanically always pressed.** Baseline records the always-on state; capture fires on toggle in either direction.

**8. Multi-axis simultaneous nudge handled.** When multiple axes cross the deadband within a `~50ms` debounce window, the primitive picks the one with the **largest absolute delta**. Defeats sympathetic stick movement.

**9. Esc takes top priority while armed.** A document-level keydown listener mounted by the primitive while `active == true` captures Esc, calls `cancel()`, and `stopPropagation()`. Esc does not bubble to enclosing inline rows, dialogs, or rename inputs while capture is armed. When capture is not armed, Esc reverts to normal scope.

### Mapping-list interactions

**10. Filter is name + source substring, case-insensitive, single-substring.** Reduces visible rows; doesn't reorder. Empty groups (post-filter) are omitted entirely (no `BUTTONS (0)` header).

**11. `+ Add mapping` collision: detect during capture, redirect.** As soon as the captured input matches an existing mapping in the **current editing-mode**, the capture pad becomes a redirect strip: *"Btn 4 already mapped to **Boost**. [Edit existing →]"*. `[Edit existing →]` selects the existing row. Cross-mode "collisions" are explicitly **not** collisions, engine permits the same `InputAddress` mapped in multiple modes.

**12. Right-click row menu has four items.** Rename (inline) · Duplicate (in-mode rebind, requires fresh capture) · Duplicate to mode… (submenu listing modes minus active; disabled on single-mode profiles) · Delete (F4 destructive confirm). Cross-mode duplicate target-mode collision reuses the redirect UX: `[Edit existing →]` switches `editing_mode` to the target and selects the existing mapping.

**13. Inline rename mirrors F7's mode-tab rename.** Row name turns into a focused `<input>`. Enter dispatches `EngineCommand::SetMapping` with the same actions and the new name. Esc cancels.

**14. Delete dispatches a new `EngineCommand::RemoveMapping { input, mode }`.** F8 adds this variant to `crates/inputforge-core/src/engine/command.rs` and a new public `Profile::remove_mapping(&mut self, input, mode) -> bool` method (`crates/inputforge-core/src/profile/mod.rs`, sibling of `set_mapping`). Engine handler, a method on the running engine type, calls `profile.remove_mapping(...)` and, if it returned `true`, persists via `Profile::save(&self, path)` (the same method `SetMapping`'s handler uses). Round-trip test (Set → assert present → Remove → assert gone → reload from disk → assert still gone). See §"Engine command surface" for the rationale on why this is a dedicated command rather than `SetMapping(_, _, None, vec![])`.

**15. Filter empty result UI: title quotes query, "Clear filter" link.** *"No mappings match `<query>`"* + helper *"Filter searches name and source label."* + ghost-link button to clear.

**16. Zero-mappings empty state at rail-width-appropriate ~18px Title scale.** F5's "Display 32px" target applies to the workspace (no-profile) empty state owned by F13; the rail's 280px column requires a smaller size. Anatomy: Title + helper line + primary `+ Add mapping` button that expands directly into `CapturingArmed` (skips the dashed-row click). Filter input is gated on `total_in_mode > 0` and is therefore not rendered in this state.

**17. Rail width: fixed 280px.** Not resizable. Per F5; not user-tweakable to avoid forcing a settings surface or persistence story.

### Keyboard

**18. Up/Down move selection within visible (filtered) rows; wrap at boundaries.** If `selected_mapping == None`, Down selects first, Up selects last. Disabled while live-capture is armed.

**19. Enter dispatches focus to F9.** Selector `[data-editor-focus]` (F9 owns the attached element). F8 only emits the focus event.

**20. Cmd-F / Ctrl-F focuses the filter input.** Esc on the filter (with non-empty query) clears the query and unfocuses. Esc on the rail with empty filter is a no-op.

## Non-goals (out of scope for this spec)

- **Pixel-level visual treatment.** Group-header rhythm, row vertical spacing, source-line truncation, scroll affordance. F8's `impeccable:layout` and `impeccable:polish` passes during implementation.
- **Group-header collapsibility.** Structurally cheap (one `Signal<bool>` per group). Deferred to `impeccable:layout`, invoked during implementation.
- **Multi-token filter search.** Current spec is single-substring; multi-token (space-separated AND) is a possible follow-up.
- **Drag-reorder within group.** Declaration order from `Profile::mappings()` is the contract.
- **Bulk select / multi-delete.** Out of scope.
- **Right-click on group header** (e.g., "Delete all in group"). Out of scope.
- **Cross-mode "promote mapping"** beyond the existing "Duplicate to mode…". If user demand emerges, a future feature.
- **Light theme.** Out of scope per parent plan.

---

## IA architecture

### Module structure

```
crates/inputforge-gui-dx/src/
├── frame/
│   └── mapping_list/                # F8 NEW, rail component tree
│       ├── mod.rs                   # Component<MappingList>; orchestrates filter/empty/rows
│       ├── source_label.rs          # InputAddress → "TFM Throttle · Z" formatter
│       ├── group.rs                 # GroupKind enum + bucketing logic
│       ├── row.rs                   # Row component, selection, glyphs, right-click menu
│       ├── filter.rs                # Filter input + memoized filtered rows
│       ├── add_inline.rs            # + Add mapping expanding-row state machine
│       ├── rename_inline.rs         # Inline rename component
│       ├── empty.rs                 # Both empty states (0 mappings, 0 filter results)
│       ├── keyboard.rs              # Up/Down/Enter/Cmd-F/Esc handling
│       └── tests.rs
│
├── patterns/
│   └── live_capture/                # F8 NEW, shared GUI primitive
│       ├── mod.rs                   # use_live_capture_provider + LiveCapture handle
│       ├── core.rs                  # LiveCaptureCore (pure baseline-and-edge logic)
│       └── tests.rs
│
├── frame/
│   ├── layout/
│   │   └── mod.rs                   # MODIFIED, wires <MappingList /> into if-layout__rail slot
│   └── view_state.rs                # MODIFIED, adds selected_mapping field + reconciliation
│
└── context.rs                       # MODIFIED, extends ConfigSnapshot with mappings Vec<MappingSummary>
```

CSS lives at `assets/frame/mapping_list.css` keyed off the `.if-rail` class; tokens only, no raw color literals.

### Engine surface change

```
crates/inputforge-core/src/engine/
├── command.rs                       # MODIFIED, adds EngineCommand::RemoveMapping
├── run.rs                           # MODIFIED, adds handler dispatching to remove + persist
└── tests.rs                         # MODIFIED, adds round-trip test for SetMapping/RemoveMapping
```

### Data architecture

#### `ConfigSnapshot` extension

```rust
// context.rs

pub(crate) struct ConfigSnapshot {
    pub devices: Vec<DeviceState>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    pub mapped_inputs: HashSet<InputAddress>,        // unchanged, mode-agnostic
    pub mapping_names: HashMap<InputAddress, String>, // unchanged, mode-agnostic
    pub mappings: Vec<MappingSummary>,                // NEW, one per (input, mode) pair
}

pub(crate) struct MappingSummary {
    pub input: InputAddress,
    pub mode: String,
    pub name: Option<String>,
    pub glyphs: GlyphFlags,
}

pub(crate) struct GlyphFlags {
    pub merge_secondary: Option<InputAddress>,
    pub conditional_input_predicate: Option<String>,
}
```

`ConfigSnapshot::from_state` is extended to walk `profile.mappings()` once and populate `mappings`. Glyph derivation walks each mapping's `actions` tree depth-first into `Conditional.if_true` and into `Conditional.if_false` (when `Some`), recording the **first** `MergeAxis.second_input` (gold `+` glyph) and the **first** input-referencing `Condition` reachable inside any `Conditional.condition` (violet `⊕` glyph). The condition walker recurses through `All`, `Any`, `Not` composites until it finds a `ButtonPressed | ButtonReleased | AxisInRange | HatDirection`. Stops early per glyph (we render at most one of each per row). See §"Action surface assumptions" below for the exact variants.

`mapped_inputs` and `mapping_names` are mode-agnostic in current code (see `context.rs::ConfigSnapshot::from_state`); F8 preserves that contract for any future cross-mode features (e.g., a "Used by" backref in F12).

#### Action surface assumptions

Source: `crates/inputforge-core/src/action/mod.rs:22-56`, `crates/inputforge-core/src/action/condition.rs:14-42`.

```text
Action variants relevant to F8 glyphs:
  Action::MergeAxis { second_input: InputAddress, operation: MergeOp }
  Action::Conditional {
      condition: Condition,
      if_true: Vec<Action>,
      if_false: Option<Vec<Action>>,
  }

Condition variants carrying an InputAddress (terminate the glyph search):
  Condition::ButtonPressed  { input: InputAddress }
  Condition::ButtonReleased { input: InputAddress }
  Condition::AxisInRange    { input: InputAddress, .. }
  Condition::HatDirection   { input: InputAddress, .. }

Condition variants the walker must recurse through (no input themselves):
  Condition::All  { conditions: Vec<Condition> }
  Condition::Any  { conditions: Vec<Condition> }
  Condition::Not  { condition: Box<Condition> }
```

The glyph walker is read-only and stops on first match per glyph kind. Other `Action` and `Condition` variants are inspected but contribute nothing to F8 row state, they are surfaced inside F9-F12.

#### `ViewState` extension

```rust
// frame/view_state.rs

pub(crate) struct ViewState {
    pub editing_mode: Signal<String>,
    pub panel_slot: Signal<PanelSlot>,
    pub via_calibration: Signal<bool>,
    pub selected_mapping: Signal<Option<(String, InputAddress)>>,   // NEW
}
```

`use_view_state_provider` initializes `selected_mapping` to `None`. A new `last_editing_mode: Signal<String>` shadow signal is added, peek-initialized from `editing_mode.peek().clone()` so the first `use_effect` run is a no-op for the editing-mode branch (matching the `last_profile_name` peek-init at `view_state.rs:75-76`).

The existing reconciliation `use_effect` body is extended with branches in this order:

1. **Profile flip** (existing, `*last_profile_name.peek() != m.profile_name`). Update `last_profile_name`, reset `editing_mode` to the new `startup_mode`, and **also** clear `selected_mapping`. Returns early.
2. **Editing-mode flip** (NEW, `*last_editing_mode.peek() != *em.peek()`). Update `last_editing_mode`, clear `selected_mapping`. Returns early. (`em` is the `editing_mode` signal, same mutable handle the existing branches use.)
3. **Modes-list drift fallback** (existing, current `editing_mode` no longer present in `m.modes`). Re-points `editing_mode` to a valid value; `selected_mapping` clears via branch 2 on the next effect tick.

Branch 2 fires both when the user clicks a different mode tab (F7's `editing_mode.set(...)`) and when branch 3 re-points `editing_mode` automatically, both paths converge through the same shadow-signal check, so no duplicate clear logic is required.

#### Source-label formatter

```rust
// frame/mapping_list/source_label.rs

pub(crate) fn format(addr: &InputAddress, cfg: &ConfigSnapshot) -> String;
```

Walks `cfg.devices` to find the device by `addr.device`, returns `"<device.name> · <input-label>"`:
- Axis index `i` → `axis_label(i)` ported from `crates/inputforge-gui/src/panels/device_view.rs::axis_label`. The legacy helper has signature `axis_label(index: usize) -> Cow<'static, str>`; F8's `format()` returns `String`, so the implementation calls `.into_owned()` on the `Cow` (or interpolates directly via `Display`, which works for `Cow<str>`).
- Button index `i` → `"Btn {i+1}"`.
- Hat index `i` → `"Hat {i}"`.
- Missing device (disconnected, never seen) → `"<DeviceId> · <input-label>"`, caller's CSS italicizes.

## Live-capture primitive

### API

```rust
// patterns/live_capture/mod.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureFilter { Any, AxesOnly, ButtonsOnly }

#[derive(Clone, Copy)]
pub(crate) struct LiveCapture {
    pub active: Signal<bool>,
    pub captured: Signal<Option<InputAddress>>,
    pub start: Callback<CaptureFilter>,
    pub cancel: Callback<()>,
}

pub(crate) fn use_live_capture_provider() -> LiveCapture;
```

The `app_root` fn (`crates/inputforge-gui-dx/src/app.rs:17`) calls `use_live_capture_provider()` and installs the result via `use_context_provider`, same pattern as the `ToastQueue` provider at `app.rs:43` and the `ViewState` provider at `app.rs:36`. `LiveCapture` is `Copy` (all four fields are `Signal`/`Callback`, both `Copy` in Dioxus 0.6+) so consumers `use_context::<LiveCapture>()` without an explicit `.clone()`.

Consumer pattern:
```rust
let cap = use_context::<LiveCapture>();
cap.start.call(CaptureFilter::Any);    // arm

use_effect(move || {                    // observe
    if let Some(addr) = cap.captured.read().clone() {
        // consume...
        cap.cancel.call(());
    }
});
```

### Internal mechanics

`use_live_capture_provider` allocates four signals exposed on the `LiveCapture` handle (`active`, `captured`, plus the two `Callback`s) plus an internal state struct held in a `Signal<CoreState>`:

```rust
pub(crate) struct CoreState {
    pub baseline: Option<InputCacheCompact>,
    pub pending: Option<(InputAddress, Instant)>,   // debounce-window winner so far
    pub filter: CaptureFilter,
}
```

`pending` carries the current best candidate within the open debounce window. The previous "four signals" framing was incomplete, debouncing is **stateful across ticks**.

A `use_effect` watches `ctx.live` (the polling Signal). On every tick where `active.read() == true`:

1. Clone the current input cache via a new `InputCacheStore::clone_compact()` helper that returns a sortable list of `(InputAddress, InputKind, Value)` tuples.
2. Call `LiveCaptureCore::step(prev_state, snapshot, now)`, see signature below, which returns `(new_state, Option<InputAddress>)`.
3. Write `new_state` back. If the returned option is `Some(addr)`, write `addr` to `captured` and set `active = false`.

`LiveCaptureCore::step` semantics:

```rust
pub(crate) fn step(
    prev: CoreState,
    snapshot: InputCacheCompact,
    now: Instant,
) -> (CoreState, Option<InputAddress>);
```

- **First tick after `start()`** (`prev.baseline` is `None`): record `baseline = Some(snapshot)`, return `(new_state, None)`, never fires on the same tick that armed the capture.
- **Subsequent ticks:** diff `snapshot` against `prev.baseline`. Per filter:
  - **Axis (Any, AxesOnly):** `(current - baseline).abs() > AXIS_DEADBAND` (`0.15`).
  - **Button (Any, ButtonsOnly):** `current != baseline` (toggle either direction).
  - **Hat (Any):** `current != baseline` (any non-baseline direction).
  Collect crossing inputs. If empty and `prev.pending.is_none()`, return `(prev, None)`.
- **Debounce-window logic:**
  - If `prev.pending` is `None` and at least one input crosses, set `pending = Some((winner, now))` where `winner` is the largest-absolute-delta candidate (axes) or the first crossing (buttons/hats).
  - If `prev.pending` is `Some((addr, t0))` and `now - t0 < DEBOUNCE_MS`, evaluate any newly-crossing input: if its absolute delta exceeds the pending one's, replace `pending` with the new candidate (keeping `t0` so the window doesn't slide).
  - If `prev.pending` is `Some((addr, t0))` and `now - t0 >= DEBOUNCE_MS`, return `(state with pending=None and baseline=None, Some(addr))`, fire the winner.

`AXIS_DEADBAND = 0.15`, `DEBOUNCE_MS = 50` are module constants, tunable, but no settings UI in F8.

`cancel()` writes `CoreState { baseline: None, pending: None, filter }` and sets `active = false`, and **also** clears `captured` to `None`. Without the `captured` reset, a consumer's `use_effect(captured)` would re-fire on subsequent mounts. Consumers must call `cancel()` after they consume a `Some(addr)` value to reset for the next capture cycle.

### Esc priority while armed

A second `use_effect` watches the `active` signal. While `active == true`, a window-level `keydown` listener is registered whose handler checks `ev.key() == "Escape"`, calls `cancel()`, and `stopPropagation()`. The listener is removed on the `active == true` → `false` transition (whether triggered by capture completing, by the consumer calling `cancel()`, or by Esc itself).

This means inline-row Esc (Cancel), rename Esc, and dialog Esc only fire when capture is **not** armed. Same applies to F9's `change input`, F10/F11's stage editors, and F12's `Record range`.

### `LiveCaptureCore` testability split

Logic lives in `core.rs` as `LiveCaptureCore::step(prev, snapshot, now) -> (CoreState, Option<InputAddress>)`, a state-transition function (see signature above). The hook in `mod.rs` is a thin signal adapter that calls into `step` per tick, threading `CoreState` through a `Signal<CoreState>`. Unit tests target `step` directly without Dioxus runtime by feeding hand-crafted snapshot sequences and `Instant`s.

Test enumeration (in `core.rs::tests`):

- **First-tick baseline.** `prev.baseline == None` + non-zero snapshot → `(new_baseline, None)`. No false fire on the arming tick.
- **Joystick stick-already-displaced (AC #11a).** Baseline `X = 0.3`, subsequent tick `X = 0.32` (delta < deadband) → `(prev, None)`. Same baseline, tick `X = 0.6` (delta > deadband) → opens debounce window with `pending = Some(X-axis, t0)`.
- **Always-on switch (AC #11b).** Baseline `BtnN = pressed`, subsequent tick `BtnN = pressed` → `(prev, None)`. Tick `BtnN = released` → opens debounce window.
- **Multi-axis nudge, largest delta wins after debounce (AC #12).** Tick T₀: stick X crosses (`delta = 0.2`), `pending = Some(X, T₀)`. Tick T₀+30ms: Y crosses (`delta = 0.4`), `pending = Some(Y, T₀)` (larger delta replaces, t₀ preserved). Tick T₀+60ms: window expired, fires `Some(Y)`.
- **Filter rejects mismatched kind.** Filter `AxesOnly`, button toggles → `(prev, None)`.
- **Cancel mid-window.** Open window, then a `cancel()` resets `pending` and `baseline` to `None`; subsequent ticks behave as if just-armed.

---

## Mapping-list rendering

### Row anatomy

Per the approved `rail-resting.html` mockup. Two-line: **name** (12px, body, `--color-text`) + **source line** (10px, `--color-text-muted`, single-line, ellipsis-clipped). Source line format:

- Plain mapping: `<source-label>`
- MergeAxis present: `<source-label> <span class="glyph-merge">+</span> <em>{secondary-label}</em>` (gold `--color-output` `#C99846`)
- Conditional with input predicate: `<source-label> <span class="glyph-cond">⊕</span> <em>{predicate-summary}</em>` (violet `--color-control-badge-text` `#B89BEA`, the AA-tuned badge variant per `DESIGN.md` §control-badge-text)

Both glyphs may coexist; render `+` first, `⊕` second. Tooltip on the glyph carries the full predicate (HTML `title=` attribute or F2's `Tooltip` component).

**Active row:** `is-active` class adds 3px focus-cyan left border + 10% primary tint background. Driven by `view.selected_mapping == Some((m.mode, m.input))`.

**Click handlers:**
- LMB: `selected_mapping.set(Some((m.mode, m.input)))`.
- RMB: `oncontextmenu` with `preventDefault()`; opens F2 `Menu` at cursor coordinates.

### Group bucketing

```rust
pub(crate) enum GroupKind { Axes, Buttons, Hats }

fn group_of(addr: &InputAddress) -> GroupKind {
    match addr.input {
        InputId::Axis { .. } => GroupKind::Axes,
        InputId::Button { .. } => GroupKind::Buttons,
        InputId::Hat { .. } => GroupKind::Hats,
    }
}
```

Render order fixed AXES → BUTTONS → HATS. Within a group, mappings appear in the order they sit in `Profile::mappings()` (declaration order); filter narrows but preserves order. Empty groups (zero matching mappings, including post-filter) are skipped, no `BUTTONS (0)` header.

### `+ Add mapping` state machine

The state machine lives in `add_inline.rs`. The component holds two top-level signals: `state: Signal<AddState>` and `name: Signal<String>` (a single name field across the lifetime of the inline form, regardless of state).

```rust
enum AddState {
    Resting,
    CapturingArmed,                                     // capture is live; primitive owns Esc
    CapturingDisarmed,                                  // capture cancelled; pad re-armable on click
    Captured  { addr: InputAddress },
    Collision { existing_name: String, existing: InputAddress },
}
```

Transitions:

| From | Trigger | To |
|---|---|---|
| Resting | Click on dashed-row | CapturingArmed, `LiveCapture::start(Any)`, name autofocus, `name` cleared |
| CapturingArmed | `cap.captured == Some(addr)`, addr **not** in current-mode mapping list | Captured, `cap.cancel()` |
| CapturingArmed | `cap.captured == Some(addr)`, addr **is** in current-mode mapping list | Collision, `cap.cancel()` |
| CapturingArmed | Esc (intercepted by primitive, `cap.cancel()` runs, `active = false`, listener removed) | CapturingDisarmed |
| CapturingDisarmed | Click on capture pad | CapturingArmed, `LiveCapture::start(Any)` re-armed, `name` preserved |
| CapturingDisarmed | Esc (row-level handler now active, primitive's listener was removed on `active → false`) | Resting |
| Captured | Enter / click Add (with non-empty `name`) | dispatch `SetMapping {input, mode, name, actions: vec![]}`, set `selected_mapping`, → Resting |
| Captured | Esc | Resting |
| Collision | `[Edit existing →]` | set `selected_mapping`, → Resting |
| Collision | Esc | Resting |
| Collision | Polling-tick re-validation: `existing` no longer in current-mode mapping list | Captured `{ addr: existing }` (carry the captured `InputAddress` forward, the collision strip flips to name+Add inline) |

**Visual treatment of `CapturingDisarmed`** (separate from `CapturingArmed`): the capture pad shows "Cancelled, click to capture again", the name field remains focused (its content preserved). Visually muted relative to the active-armed pad. The second Esc tears down the inline form back to `Resting`.

**Two-stage Esc cross-reference.** The two Esc rows (`CapturingArmed → CapturingDisarmed` and `CapturingDisarmed → Resting`) are made possible by choice 9 (Esc priority while armed): the primitive's window-level keydown listener is mounted **only while `active == true`** and removed on `active → false`. The first Esc reaches the primitive, cancels capture, and tears down the listener; the second Esc bubbles to the inline-row's Esc handler, which transitions `CapturingDisarmed → Resting`.

**Collision drift via polling re-validation.** The `Collision` row is checked once per polling tick: if `existing` is no longer a mapped input in the active editing-mode (e.g., because an external profile edit removed it, or another window deleted it), the state transitions to `Captured { addr: existing }` automatically. The user observes the collision redirect strip flip back to the standard "name + Add" Captured affordance, no manual intervention needed.

Cross-mode "collision" is **not** a collision: an `InputAddress` mapped in another mode but not the active editing-mode transitions to **Captured** normally.

### Right-click row menu

| Item | Action | Disabled when |
|---|---|---|
| Rename | Inline rename (turn name into focused input; Enter dispatches `SetMapping` with same actions, new name) |, |
| Duplicate | Open Add inline pre-filled `{name: "{name} (copy)", actions: cloned}`; require fresh capture; Enter dispatches `SetMapping` in same mode |, |
| Duplicate to mode… | Submenu listing `meta.modes` minus current `editing_mode`. Click target mode → dispatch `SetMapping {input, mode: target, name, actions}`. Target-mode collision reuses Q4 redirect, `[Edit existing →]` switches `editing_mode` and selects existing | `meta.modes.len() <= 1` (tooltip: *"Profile has only one mode."*) |
| Delete | F4 destructive `Dialog`: *"Delete `<name>`? Undo available this session only."* On confirm, dispatch `RemoveMapping {input, mode}` |, |

### Empty states

State A, **0 mappings overall** (profile loaded, mode has zero mappings):
- Title `No mappings yet`, Title scale (~18px), `--color-text`, weight 600.
- Helper `Pick an input on a device to start binding. Or click below to name one first.`, `--color-text-muted`, 12px.
- Primary button `+ Add mapping`, F2's `Button` primary variant. Click expands `add_inline.rs` directly into **CapturingArmed** (skips Resting → click).
- Filter input is **not** rendered (gated on `total_in_mode > 0`); Cmd-F is a no-op in this state.

State B, **0 filter results**:
- Title `No mappings match "<query>"`, same scale; `<query>` wrapped in `<span class="muted">`.
- Helper `Filter searches name and source label.`, same muted style.
- Ghost-link button `Clear filter`, clears `filter_query`, refocuses input.

Discrimination logic:
```rust
let show_filter = total_in_mode > 0;
match (total_in_mode, query_empty, filtered.len()) {
    (0, _,    _)  => Empty::ZeroMappings,         // filter not mounted
    (_, false, 0) => Empty::ZeroFilterResults,    // filter mounted + active
    _             => RenderRows,                   // filter mounted
}
```

### Keyboard navigation

A document-level `keydown` listener installed by `MappingList` (via `use_effect` with cleanup on unmount). Active **only when the rail or one of its descendants holds focus**.

- **Up/Down**: move `selected_mapping` to the previous/next row in the visible (filtered) list. Wraps at boundaries. If `selected_mapping == None`, Down selects first, Up selects last. **Disabled while `cap.active.read() == true`**, short-circuits before processing.
- **Enter**: dispatch focus to F9's editor mount point (selector `[data-editor-focus]`, F9 attaches this; F8 only emits).
- **Cmd-F / Ctrl-F**: focus the filter input.
- **Esc** (filter focused, query non-empty): clear filter and unfocus. (Esc on rail with empty filter: no-op.)

### Scroll

Native vertical scroll inside the rail; CSS `overflow-y: auto`. No virtualization, 200-mapping rails are fine for a configuration tool.

---

## Engine command surface

### Commands F8 dispatches

| Command | Trigger | Payload |
|---|---|---|
| `SetMapping` | Inline rename Enter; Add Enter; Duplicate Enter; Duplicate-to-mode submenu click | `{ input, mode, name, actions }` |
| `RemoveMapping` *(NEW)* | Right-click → Delete → F4 confirm | `{ input, mode }` |

### Profile-side change

A new public method on `Profile` in `crates/inputforge-core/src/profile/mod.rs` (sibling of `set_mapping` at lines 197-224):

```rust
impl Profile {
    /// Remove the mapping for `(input, mode)`. Returns `true` if a mapping
    /// was removed, `false` if no matching mapping existed.
    ///
    /// Distinct from `set_mapping(_, _, None, vec![])` which can also remove -
    /// `remove_mapping` is the explicit API for the F8 delete flow.
    pub fn remove_mapping(&mut self, input: &InputAddress, mode: &str) -> bool {
        let before = self.mappings.len();
        self.mappings
            .retain(|m| !(m.input == *input && m.mode == mode));
        self.mappings.len() != before
    }
}
```

The `mappings` field is private (`profile/mod.rs:45`); F8 must go through this method rather than mutating directly.

### `RemoveMapping` engine-side change

Added to `crates/inputforge-core/src/engine/command.rs`:

```rust
EngineCommand::RemoveMapping { input: InputAddress, mode: String }
```

Engine handler, a method on the running engine type, mirroring the existing `set_mapping` handler shape (`crates/inputforge-core/src/engine/run.rs`):

```rust
// inside RunningEngine impl

fn remove_mapping(&self, input: &InputAddress, mode: &str) {
    let mut state = self.state.write();
    let Some(path) = state.profile_path.clone() else { return };
    let Some(profile) = state.active_profile.as_mut() else { return };
    if !profile.remove_mapping(input, mode) {
        return;                                          // no-op fast path, nothing changed
    }
    if let Err(e) = profile.save(&path) {
        tracing::warn!(target: "f8::mapping_list", error = %e,
                       "failed to persist after RemoveMapping");
        // surface via state.warnings, same path SetMapping uses
    }
}
```

Persistence reuses `Profile::save(&self, path: &Path) -> Result<()>` (`profile/mod.rs:138`), the same method `set_mapping`'s handler calls. F8 adds no new IO concern.

### Why a dedicated `RemoveMapping` command (not `SetMapping(_: _, None, vec![])`)

`Profile::set_mapping` already removes a mapping when `actions` is empty (`profile/mod.rs:204-207`), so a `SetMapping { actions: vec![] }` dispatch would also work. F8 deliberately introduces both `EngineCommand::RemoveMapping` and `Profile::remove_mapping` instead. Rationale:

1. **Dispatch-site clarity.** "Delete" reads as the user's intent at the call site, rather than "set with empty actions", the latter is an implementation accident, not an API.
2. **Observability.** Tracing events split cleanly into `action = "remove"` vs. `action = "set" | "rename" | "duplicate" | "duplicate_to_mode"`. Filtering "remove" events is one predicate, not a payload-shape inspection.
3. **No-op fast path.** `Profile::remove_mapping`'s boolean return enables the engine handler to skip `profile.save` when nothing changed (e.g., a stale dispatch races a prior remove). Detecting the same condition off `SetMapping` would require comparing `mappings.len()` before-and-after.

### Round-trip test

Added to `crates/inputforge-core/src/engine/tests.rs`:

The test dispatches `EngineCommand::SetMapping { input, mode, name, actions: <non-empty> }`, asserts `Profile::find_mapping(&input, &mode).is_some()` after the engine processes the command, then dispatches `EngineCommand::RemoveMapping { input, mode }`, asserts `find_mapping` returns `None`, reloads the profile from disk via `Profile::load(&path)`, and asserts `find_mapping` is still `None` post-reload (proves persistence). Symmetric to the existing `SetMapping` round-trip pattern.

### Error handling

- **Channel disconnected** (engine torn down): `mpsc::Sender::send` returns Err. F8 logs at `tracing::warn!` and silently drops the action. No toast.
- **Engine-side IO errors** during profile save: surface through `AppState.warnings`. The toast bridge installed in the `app_root` fn (`crates/inputforge-gui-dx/src/app.rs:17`), `install_warnings_bridge`, emits a Warning toast for any new tail entry. F8 inherits this, no F8-specific warning UI.
- **No optimistic UI.** F8 does not pre-write changes to its local view. The rail re-renders once the engine has applied the change and the polling task projects the new state. Avoids divergence; matches F7's mode-CRUD.

### Observability

Each dispatch emits a `tracing` event:
- `info!(target: "f8::mapping_list", action = "rename" | "add" | "duplicate" | "duplicate_to_mode" | "remove", ?input, %mode, ?name)`.
- Live-capture's `start`/`captured`/`cancel` emit `debug!` events with the active filter and outcome.

No metrics or counters.

---

## Testing strategy

Three tiers, mirroring F7's pattern.

### 1: Pure logic (Rust unit tests in each module's `#[cfg(test)] mod tests`)

- `source_label::format` round-trips for axis/button/hat, missing-device case.
- `group::group_of` discriminates input kinds.
- `MappingSummary` glyph derivation: walk `Action` trees with MergeAxis, with input-Conditional, with both, with neither.
- `matches_filter` substring matching, case-insensitive.
- `LiveCaptureCore`: axis baseline + delta detection; button toggle from already-pressed (always-on switch); multi-axis nudge picks max-delta within debounce; filter rejects mismatched kinds.

### 2: Component tests via `dioxus_ssr::render`

Mirrors `app.rs:161` pattern.

- Rail with seeded `ConfigSnapshot` of 4 mappings (3 axes / 1 button) → assert HTML contains both group headers, 4 rows, glyph spans for the right rows.
- Empty State A (zero mappings) renders title + button.
- Empty State B (filtered to zero) renders title with quoted query.
- Active row carries `is-active` class when `selected_mapping` matches.
- Inline rename state renders `input.row-rename` instead of `div.row__name`.

### 3: Integration

Not required for F8. The harness already validates `frame::Layout` mounts; F8 plugs into the rail slot. Unit + SSR tests above suffice.

### Live-capture testability

`LiveCaptureCore` is a pure struct unit-tested directly. The Dioxus hook gets a smoke test asserting mount/unmount don't panic and `start/cancel` flip the signals. The window keydown listener is exercised manually during F16's `impeccable:audit`.

---

## Acceptance criteria

1. Rail renders inside `if-layout__rail` when a profile is loaded; hidden when no profile (F13's empty state replaces).
2. Active mode tab toggle clears `selected_mapping` and re-renders the rail with the new mode's mappings.
3. Filter narrows visible rows by name + source-label substring; case-insensitive; doesn't reorder.
4. Group headers render in fixed order AXES → BUTTONS → HATS; empty groups omitted.
5. MergeAxis mappings show gold `+` glyph + italic secondary input; Conditional-with-input-predicate mappings show violet `⊕` glyph + italic predicate summary.
6. `+ Add mapping` `CapturingArmed` state arms `LiveCapture(Any)`; first edge-detected input becomes `addr`; collision in active mode redirects to `[Edit existing →]`; cross-mode is not a collision. Esc in `CapturingArmed` transitions to `CapturingDisarmed` (capture cancels, name preserved); a click on the pad re-arms; a second Esc returns to `Resting`. If the conflicting mapping is removed while in `Collision`, the next polling tick transitions to `Captured`.
7. Right-click menu: Rename (inline) · Duplicate (in-mode rebind) · Duplicate to mode… (submenu, disabled on single-mode profiles, with target-mode collision UX) · Delete (F4 destructive → `RemoveMapping`).
8. Up/Down navigate selection within filtered rows; Enter dispatches focus to `[data-editor-focus]`; Cmd-F focuses filter; Esc clears filter when focused with non-empty query.
9. Live-capture takes Esc priority while armed: Esc cancels capture, does not bubble. Up/Down disabled while capture is armed.
10. `EngineCommand::RemoveMapping` round-trips correctly and persists profile to disk.
11a. Joystick stick-already-displaced (e.g., X axis at 0.3 at capture start) does not produce a false capture; capture fires only when the stick moves further (delta against baseline > deadband).
11b. Always-on switch buttons (mechanically always pressed) baseline correctly; capture fires on toggle in either direction.
12. Multi-axis simultaneous nudge picks the largest-delta axis within the 50ms debounce window.

---

## Impeccable command invocations (per F5 spec)

- `impeccable:shape`, most resolved by this brainstorm; remaining shape work is row-density and group-header rhythm.
- `impeccable:frontend-design`, primary visual treatment.
- `impeccable:layout`, row vertical rhythm, group-header spacing, source-line indent. **Group-header collapsibility decision made here.**
- `impeccable:typeset`, name vs source typography contrast in the dense range.
- `impeccable:clarify`, empty-state copy, filter placeholder, capture-pad copy ("Press an input on any device…"), collision redirect copy, "Duplicate to mode…" submenu copy.
- `impeccable:polish`, final pass.

---

## Open questions / deferred items

- **Group-header collapsibility.** Structurally cheap (one `Signal<bool>` per group). Decided at `impeccable:layout`.
- **Rail width persistence.** Settled at fixed 280px. If user demand emerges later, switching to resizable is a CSS-only change.
- **Filter multi-token search.** Single-substring today; multi-token AND is a possible follow-up.
- **Cross-mode "promote mapping" UI.** Out of scope; today only "Duplicate to mode…" exists.

---

## Net summary

| Component | F8 status | Notes |
|---|---|---|
| `frame/mapping_list/` (9 files) | new | rail component tree |
| `patterns/live_capture/` (3 files) | new | shared GUI primitive |
| `frame/layout/mod.rs` | modified | wires `<MappingList />` into `if-layout__rail` |
| `frame/view_state.rs` | modified | adds `selected_mapping` + reconciliation |
| `context.rs` | modified | extends `ConfigSnapshot` with `mappings: Vec<MappingSummary>` |
| `assets/frame/mapping_list.css` | new | rail styling |
| `crates/inputforge-core/src/profile/mod.rs` | modified | adds public `Profile::remove_mapping(&mut self, input, mode) -> bool` method |
| `crates/inputforge-core/src/engine/command.rs` | modified | adds `EngineCommand::RemoveMapping` |
| `crates/inputforge-core/src/engine/run.rs` | modified | adds `RunningEngine::remove_mapping` handler |
| `crates/inputforge-core/src/engine/tests.rs` | modified | adds Set→Remove round-trip test (in-memory + disk reload) |
| `crates/inputforge-core/src/state/cache.rs` | modified | adds `InputCacheStore::clone_compact()` |

---

## Next steps

1. Commit this spec to git.
2. Invoke `superpowers:writing-plans` to produce the focused plan for F8.
3. F8 implementation invokes the impeccable commands listed above during execution.
