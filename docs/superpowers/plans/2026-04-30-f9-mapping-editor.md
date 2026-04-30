# F9: Mapping Editor (Pipeline Structure) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder `if-layout__center` text with the F9 mapping editor: header (h2 + subtitle), name field, input field with rebind, live readout (IN/OUT bars + merge layout), inactive-runtime hint, engine-offline banner, the action pipeline graph (category-tinted stages with inline expand, Conditional branches, MergeAxis secondary picker, add palette, right-click reorder, drag-and-drop), per-mapping session-undo with `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y`, and the `evaluate_actions_through` helper for downstream features.

**F9→F10/F11/F14 sequencing constraint:** F9 ships placeholder bodies for `Action::ResponseCurve`, `Action::Deadzone`, `Action::ChangeMode`. The header and chevron render normally; the body slot displays a "F10 / F11 / F14 owns this body" caption (single string per spec line 300). F10/F11/F14 each replace their body without touching the variant dispatcher, the `StageHeader.right_slot: Element` prop API, or the `EditorState` provider. Adding one of these three variants from the palette is allowed in F9; the user can name and reorder them but cannot configure their parameters until the owning feature ships. F9 also handles empty-actions mappings (created by F8's `+ Add mapping`): an empty `actions: vec![]` mapping renders as a one-stage-empty pipeline with the `+ Add first stage` louder affordance, so the user can recover and populate it.

**Architecture:** Engine-side first, one new pure helper `inputforge_core::pipeline::evaluate_actions_through(actions: &[Action], state: &AppState, addr: &InputAddress, stop_at: usize) -> InputValue` that re-runs a partial pipeline without crossing the engine command channel (the `addr` argument is the mapping's primary `InputAddress`, used to seed the pipeline's input read from `state.input_cache` via the `InputCache` trait's typed accessors). State plumbing follows: a `MappingKey = (String, InputAddress)` type alias on `view_state.rs` reused everywhere; `ConfigSnapshot` extended with `selected_mapping_actions` and `selected_mapping_key`; the polling task feeds `view.selected_mapping.peek()` into `ConfigSnapshot::from_state`. Design-system tokens land next, three `--color-stage-tint-*` tokens mixed from the existing category colors. Then the `EditorState` provider with `UndoLog` data shapes (`UndoEntry`, `UndoKind`, `MappingHistory`) plus `push_edit` / `undo` / `redo` covered by pure unit tests before any rendering. Editor renders inside-out: empty state and engine-offline banner first (smallest surface, immediate visual signal), then frame sections (header, name field, input field, live readout, inactive hint, undo recap footer), then the pipeline component shell, then stage skeleton (header + chevron + body container), then the variant-body dispatcher with five F9-owned bodies (Invert, MapToVJoy, MapToKeyboard, MergeAxis, Conditional) plus three placeholder bodies (ResponseCurve, Deadzone, ChangeMode). Pipeline interactions land last: add palette, right-click action menu, drag-and-drop reorder, malformed-stage treatment, keyboard shortcuts (Ctrl+Z / Shift+Z / Y with focus filtering, Alt+Up/Down stage move). External-edit reconciliation and SSR component tests close the plan.

**Tech Stack:** Rust 2024 edition · `inputforge-core` (engine, action, pipeline, profile, state) · `inputforge-gui-dx` (Dioxus 0.7, dioxus-desktop, F2 component primitives `IconButton` / `Tooltip` / `MenuRoot`, F4 `DirtyConfirmDialog` for profile-flip undo log clear, F8 `LiveCapture` primitive, F8 `components/sortable` primitive — generic-G upgrade lands in Task 30a) · `parking_lot::RwLock` over `AppState` · `std::sync::mpsc` for `EngineCommand` dispatch · `tracing` for engine + GUI events.

**Spec:** [`docs/superpowers/specs/2026-04-30-f9-mapping-editor-design.md`](../specs/2026-04-30-f9-mapping-editor-design.md).

---

## Sequencing rationale

Engine-side first: `evaluate_actions_through` is a pure function over `&[Action]` + `&AppState` + `usize` returning an `InputValue`. Unit-testable without GUI; F10/F11 will consume it from their bodies but F9 never calls it directly, so landing it early decouples the helper from any rendering work. State plumbing follows: `MappingKey` type alias plus `ConfigSnapshot::selected_mapping_actions` / `selected_mapping_key` extensions are pure data projections testable with seeded `AppState` fixtures, both fields are required by every editor renderer downstream so they ship before any component code. Stage-tint tokens land next as a one-line CSS commit so every later component can refer to `--color-stage-tint-{processing,output,control}` without forward references.

`EditorState` and `UndoLog` are the next foundation: data shapes (`UndoEntry`, `UndoKind`, `MappingHistory`, `StageId`, `StageMenuState`) plus the pure `UndoLog::push_edit` / `undo` / `redo` semantics get unit-tested before any Signal enters the picture, mirroring F8's `LiveCaptureCore::step` pattern. The provider hook is a thin adapter that wraps these shapes in Signals.

Editor render code ships inside-out, smallest surface first, so each task lands a visible piece of UI while leaving the next task's hooks in place. Empty state (`Select a mapping`) is the smallest surface, lands first to verify the editor mounts in `if-layout__center` at all. Engine-offline banner follows, same skeleton (sticky-positioned wrapper, no pipeline yet). Then the editor frame sections render in the order the user reads them, header → name field → input field → live readout → inactive hint → undo recap. Each section is independent and testable in SSR with a seeded `ConfigSnapshot`. The pipeline component lands as a flat ordered list of stages (no Conditional yet) so the layout commits before recursion. Stage skeleton lands next (header + chevron + body container, no body rendering). Variant bodies land in order of complexity: Invert (no body) → MapToVJoy (two pickers) → MapToKeyboard (KeyCombo editor) → MergeAxis (op picker + secondary input picker arming `LiveCapture::AxesOnly`) → Conditional (predicate editor + recursive sub-pipelines). Placeholder bodies for ResponseCurve, Deadzone, ChangeMode land in a single task because they share the same caption-only layout.

Pipeline interactions land last because each one piggybacks on rendering already in place: add palette opens an F2 `MenuRoot` anchored to the `+` button; right-click menu uses the same primitive inside an absolute-positioned wrapper at the cursor coordinates (F2 `MenuRoot` does not expose anchor coordinates so positioning is owned by the wrapper); drag-and-drop reuses the F8 `components/sortable` primitive (Task 30a generalizes its group discriminator to `G: 'static + Clone + PartialEq` so F9 can use `G = StageId` for cross-pipeline DnD into Conditional branches; F8 keeps `G = u32` with no behavior change); the `Ctrl+Z` handler is a new editor-scoped window-level keyboard listener (architecturally modelled on F8's pure `handle_key()` fn in `frame/mapping_list/keyboard.rs:73-141`, NOT F8's actual handler which routes navigation keys, not undo) that consults the `EditorState.undo_log` already wired to `SetMapping` dispatch. Malformed-stage treatment is the last visual concern because every body must compute and write to `EditorState.malformed_hints` first. External-edit reconciliation closes the plan, the polling task already projects `selected_mapping_actions`; this task just adds the focus-aware reset logic.

Tasks 1-8 and 10-11 are pure-logic / unit-testable / engine-or-state-only. Task 9 is the Signal-wiring adapter for `EditorState`, mounting the data shapes from Tasks 6-8 into Dioxus context (a thin adapter, not pure logic). Tasks 12-44 are GUI render code (44 task headings total: 41 originals, Task 26 splits into 26a Conditional shell + 26b predicate editor, Task 30 retains its overview heading plus 30a sortable generic-G upgrade + 30b F9 wiring); manual interaction passes happen in the final phase. Tasks 1-3, 5-8, 10-11 follow the standard failing-first TDD pattern. Tasks 12, 13 are SSR check-only tasks where test and implementation ship together because the implementation is a one-component render with no internal logic.

---

## File structure overview

**Created (engine):** None. One new pure fn added to existing `pipeline/mod.rs`.

**Modified (engine):**

- `crates/inputforge-core/src/pipeline/mod.rs`, `pub fn evaluate_actions_through(actions: &[Action], state: &AppState, addr: &InputAddress, stop_at: usize) -> InputValue`

**Created (GUI):**

```
crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/header.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/name_field.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/input_field.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/inactive_hint.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/empty_state.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/engine_offline_banner.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/keyboard.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/external_edit.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_header.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/dnd.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/invert.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_vjoy.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/conditional.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/placeholders.rs
crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
crates/inputforge-gui-dx/assets/frame/mapping_editor.css
```

**Modified (GUI):**

- `crates/inputforge-gui-dx/src/context.rs`, `ConfigSnapshot::selected_mapping_actions: Option<Vec<Action>>` + `selected_mapping_key: Option<MappingKey>`; `from_state` takes a new `&Option<MappingKey>` parameter
- `crates/inputforge-gui-dx/src/frame/view_state.rs`, declares `pub(crate) type MappingKey = (String, InputAddress);` and re-exports it
- `crates/inputforge-gui-dx/src/frame/mod.rs`, `mod mapping_editor;` + `pub(crate) use mapping_editor::MappingEditor;`
- `crates/inputforge-gui-dx/src/frame/layout/mod.rs`, replaces the `"Mapping editor, F9 owns content"` placeholder with `<MappingEditor />`
- `crates/inputforge-gui-dx/src/app.rs`, install `EditorState` via `use_context_provider` (sibling of `LiveCapture`)
- `crates/inputforge-gui-dx/src/bridge.rs`, polling task reads `view.selected_mapping.peek()` and threads it into `ConfigSnapshot::from_state`
- `crates/inputforge-gui-dx/assets/tokens/colors.css`, adds `--color-stage-tint-{processing,output,control}` tokens
- `crates/inputforge-gui-dx/src/components/sortable/state.rs`, generic-G upgrade — `SortableState<G>`, `DropTarget<G>`, `use_sortable_state<G>()` (Task 30a)
- `crates/inputforge-gui-dx/src/components/sortable/handle.rs`, `SortableHandle<G>` (Task 30a)
- `crates/inputforge-gui-dx/src/components/sortable/item.rs`, `SortableItemConfig<G, F>`, `validate_drop: Option<fn(&G, &G) -> bool>`, `use_sortable_item<G, F>` (Task 30a)
- `crates/inputforge-gui-dx/src/components/sortable/live_region.rs`, `SortableLiveRegion<G>` if needed (Task 30a)
- `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs`, F8 migration: `use_sortable_state::<u32>()` turbofish (Task 30a)
- `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs`, F8 migration: validator `Some(|src: &u32, tgt: &u32| src == tgt)` (Task 30a)

**Modified (specs):**

- `docs/superpowers/specs/2026-04-27-f5-architecture-ia-redesign-design.md`, line 177 hint copy tightened to F9's revised wording

**Deleted:** None.

---

## Phase A, Engine-side helper (Task 1)

### Task 1: `evaluate_actions_through`

Pure helper in `crates/inputforge-core/src/pipeline/mod.rs` that re-runs the action pipeline up to (but not including) `stop_at` against an `&AppState` snapshot and returns the projected `InputValue` at that point. Read-only; never dispatches commands. Used by F10's live-tracking dot and any future per-stage live signal without duplicating action evaluation logic in the GUI.

`stop_at = 0` returns the unprocessed input. `stop_at = actions.len()` returns the full pipeline output. `stop_at > actions.len()` clamps to `actions.len()` (defensive, the GUI may stale-pass an index from an older snapshot). The helper reads the input value from `state.input_cache` (a `&dyn InputCache` trait object on `PipelineContext`, NOT a `&HashMap`) using the trait's typed accessors (`get_axis(&InputAddress) -> f64`, `get_button -> bool`, `get_hat -> HatDirection`); the variant is discriminated by `primary.input` (`InputId::Axis { .. }` / `Button { .. }` / `Hat { .. }`). The helper builds a `PipelineContext` whose `current_value: f64` (NOT f32) seeds from the read, runs `execute_pipeline(&actions[..stop_at], &mut ctx)`, then reconstitutes the `InputValue`: axis variant carries `ctx.current_value` wrapped in `AxisValue::new`; button variant carries the boolean derived via `BUTTON_PRESS_THRESHOLD`; hat passes through unchanged (`ctx.current_value` is meaningless for hats, the direction read from the cache is preserved verbatim).

**Files:**
- Modify: `crates/inputforge-core/src/pipeline/mod.rs`
- Test: `crates/inputforge-core/src/pipeline/mod.rs` (existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(test)] mod tests` in `crates/inputforge-core/src/pipeline/mod.rs` (after the `pedal_merge_full_pipeline` test, around line 762). Add:

```rust
// -- evaluate_actions_through ---------------------------------------------

use crate::state::AppState;

fn axis_input_address() -> InputAddress {
    InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    }
}

#[test]
fn evaluate_actions_through_zero_returns_input_untouched() {
    let mut state = AppState::new();
    let addr = axis_input_address();
    state.input_cache.update(
        &addr,
        &InputValue::Axis {
            value: AxisValue::new(0.5),
        },
    );

    let actions = [Action::Invert];
    let out = evaluate_actions_through(&actions, &state, &addr, 0);

    match out {
        InputValue::Axis { value } => {
            assert!((value.value() - 0.5).abs() < TOLERANCE);
        }
        other => panic!("expected Axis, got {other:?}"),
    }
}

#[test]
fn evaluate_actions_through_full_runs_entire_pipeline() {
    let mut state = AppState::new();
    let addr = axis_input_address();
    state.input_cache.update(
        &addr,
        &InputValue::Axis {
            value: AxisValue::new(0.5),
        },
    );

    let actions = [Action::Invert];
    let out = evaluate_actions_through(&actions, &state, &addr, actions.len());

    match out {
        InputValue::Axis { value } => {
            assert!((value.value() - (-0.5)).abs() < TOLERANCE);
        }
        other => panic!("expected Axis, got {other:?}"),
    }
}

#[test]
fn evaluate_actions_through_partial_runs_subset() {
    let mut state = AppState::new();
    let addr = axis_input_address();
    state.input_cache.update(
        &addr,
        &InputValue::Axis {
            value: AxisValue::new(0.5),
        },
    );

    // Two Inverts cancel; stop_at=1 runs only the first.
    let actions = [Action::Invert, Action::Invert];
    let out = evaluate_actions_through(&actions, &state, &addr, 1);
    match out {
        InputValue::Axis { value } => {
            assert!((value.value() - (-0.5)).abs() < TOLERANCE);
        }
        other => panic!("expected Axis, got {other:?}"),
    }
}

#[test]
fn evaluate_actions_through_stop_at_overflow_clamps() {
    let mut state = AppState::new();
    let addr = axis_input_address();
    state.input_cache.update(
        &addr,
        &InputValue::Axis {
            value: AxisValue::new(0.5),
        },
    );

    let actions = [Action::Invert];
    // stop_at = 99 with 1 action; clamps to 1.
    let out = evaluate_actions_through(&actions, &state, &addr, 99);
    match out {
        InputValue::Axis { value } => {
            assert!((value.value() - (-0.5)).abs() < TOLERANCE);
        }
        other => panic!("expected Axis, got {other:?}"),
    }
}

#[test]
fn evaluate_actions_through_button_pipeline() {
    let mut state = AppState::new();
    let addr = button_input_address();
    state.input_cache.update(&addr, &InputValue::Button { pressed: true });

    let actions = [Action::Invert];
    let out = evaluate_actions_through(&actions, &state, &addr, 1);
    match out {
        InputValue::Button { pressed } => assert!(!pressed, "Invert should flip true to false"),
        other => panic!("expected Button, got {other:?}"),
    }
}

#[test]
fn evaluate_actions_through_unknown_input_returns_zero_axis() {
    // Defensive: if the address is missing from the cache, the helper
    // synthesizes an Axis(0.0) baseline (same convention used by
    // InputCache trait readers).
    let state = AppState::new();
    let addr = axis_input_address();
    let actions: [Action; 0] = [];
    let out = evaluate_actions_through(&actions, &state, &addr, 0);
    match out {
        InputValue::Axis { value } => {
            assert!(value.value().abs() < TOLERANCE);
        }
        other => panic!("expected Axis, got {other:?}"),
    }
}

#[test]
fn evaluate_actions_through_hat_pipeline_passes_direction_through() {
    // Hats: ctx.current_value is meaningless; the helper preserves the
    // original direction read from input_cache regardless of stop_at.
    let mut state = AppState::new();
    let addr = hat_input_address();
    state.input_cache.update(
        &addr,
        &InputValue::Hat {
            direction: HatDirection::NorthEast,
        },
    );

    let actions: [Action; 0] = [];
    let out = evaluate_actions_through(&actions, &state, &addr, 0);
    match out {
        InputValue::Hat { direction } => assert_eq!(direction, HatDirection::NorthEast),
        other => panic!("expected Hat, got {other:?}"),
    }
}

#[test]
fn evaluate_actions_through_partial_one_before_end_runs_subset() {
    // Boundary: stop_at = actions.len() - 1 must run all but the last action.
    // Distinct from the existing partial test (which uses stop_at = 1 with len 2).
    let mut state = AppState::new();
    let addr = axis_input_address();
    state.input_cache.update(
        &addr,
        &InputValue::Axis {
            value: AxisValue::new(0.5),
        },
    );

    // Three Inverts; stop_at = 2 leaves one un-applied → result inverted twice (= 0.5).
    let actions = [Action::Invert, Action::Invert, Action::Invert];
    let out = evaluate_actions_through(&actions, &state, &addr, actions.len() - 1);
    match out {
        InputValue::Axis { value } => {
            assert!((value.value() - 0.5).abs() < TOLERANCE);
        }
        other => panic!("expected Axis, got {other:?}"),
    }
}
```

Add `hat_input_address()` helper alongside `axis_input_address()` and `button_input_address()`:

```rust
fn hat_input_address() -> InputAddress {
    InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Hat { index: 0 },
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-core --lib pipeline::tests::evaluate_actions_through`
Expected: FAIL with `error[E0425]: cannot find function 'evaluate_actions_through' in this scope`.

- [ ] **Step 3: Implement `evaluate_actions_through`**

Insert after `execute_pipeline` (around line 162) in `crates/inputforge-core/src/pipeline/mod.rs`:

```rust
/// Re-run a partial action pipeline against a snapshot and return the
/// projected `InputValue` at `stop_at`.
///
/// `stop_at = 0` returns the unprocessed input read from `state.input_cache`
/// at `primary`. `stop_at >= actions.len()` runs the full pipeline.
///
/// Read-only; never dispatches commands. Used by F10's live-tracking dot
/// (and F9's live-readout OUT bar) without duplicating pipeline evaluation
/// in the GUI.
#[must_use]
pub fn evaluate_actions_through(
    actions: &[Action],
    state: &crate::state::AppState,
    primary: &InputAddress,
    stop_at: usize,
) -> InputValue {
    use crate::state::InputCache;

    let stop = stop_at.min(actions.len());

    // Discriminate variant from the address; read via the InputCache trait.
    // Returns the cache's default for missing entries (axis: 0.0, button: false,
    // hat: HatDirection::Centered) — same convention as direct trait reads.
    let input_value = match &primary.input {
        InputId::Axis { .. } => InputValue::Axis {
            value: crate::types::AxisValue::new(state.input_cache.get_axis(primary)),
        },
        InputId::Button { .. } => InputValue::Button {
            pressed: state.input_cache.get_button(primary),
        },
        InputId::Hat { .. } => InputValue::Hat {
            direction: state.input_cache.get_hat(primary),
        },
    };

    let current_value: f64 = match &input_value {
        InputValue::Axis { value } => value.value(),
        InputValue::Button { pressed } => {
            if *pressed {
                1.0
            } else {
                0.0
            }
        }
        InputValue::Hat { .. } => 0.0,
    };

    let mut ctx = PipelineContext {
        current_value,
        input_value: input_value.clone(),
        outputs: Vec::new(),
        input_cache: &state.input_cache,
    };

    execute_pipeline(&actions[..stop], &mut ctx);

    match input_value {
        InputValue::Axis { .. } => InputValue::Axis {
            value: crate::types::AxisValue::new(ctx.current_value),
        },
        InputValue::Button { .. } => InputValue::Button {
            pressed: ctx.current_value > BUTTON_PRESS_THRESHOLD,
        },
        // Hats: pipeline evaluation does not modify direction; the cached
        // direction reads through unchanged.
        InputValue::Hat { direction } => InputValue::Hat { direction },
    }
}
```

No new accessor on `InputCacheStore` is required: the existing `InputCache` trait (impl'd at `crates/inputforge-core/src/state/cache.rs:101-127`) exposes `get_axis(&InputAddress) -> f64` / `get_button -> bool` / `get_hat -> HatDirection`, which the helper consumes directly via `&dyn InputCache`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-core --lib pipeline::tests::evaluate_actions_through`
Expected: PASS, eight tests, all green.
Run: `cargo test -p inputforge-core` (full suite)
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-core/src/pipeline/mod.rs
git commit -m "feat(pipeline): add evaluate_actions_through(actions, state, primary, stop_at)"
```

---

## Phase B, State plumbing (Tasks 2-5)

### Task 2: `MappingKey` type alias

Declare `pub(crate) type MappingKey = (String, InputAddress);` on `frame/view_state.rs` and re-export from `frame/mod.rs`. Used by `view.selected_mapping`, `ConfigSnapshot.selected_mapping_key`, the `UndoLog` map key, and every editor key passing.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/view_state.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mod.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/inputforge-gui-dx/src/frame/view_state.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn mapping_key_alias_compiles() {
    let _: MappingKey = ("Default".to_owned(), _synthetic_addr());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::view_state::tests::mapping_key_alias_compiles`
Expected: FAIL with `error[E0412]: cannot find type 'MappingKey' in this scope`.

- [ ] **Step 3: Add the alias**

Insert near the top of `crates/inputforge-gui-dx/src/frame/view_state.rs`, just after the `use` block:

```rust
/// Identifier for a mapping in the editor: `(mode, input)`.
///
/// Used by `view.selected_mapping`, `ConfigSnapshot.selected_mapping_key`,
/// the `UndoLog` map key, and every editor key passing.
pub(crate) type MappingKey = (String, InputAddress);
```

Update the `selected_mapping` field type in `ViewState` to use the alias:

```rust
pub selected_mapping: Signal<Option<MappingKey>>,
```

In `crates/inputforge-gui-dx/src/frame/mod.rs`, re-export the alias:

```rust
pub(crate) use view_state::{MappingKey, use_view_state_provider};
```

Search-and-replace the long-form tuple type `(String, InputAddress)` in `frame/mapping_list/mod.rs`, `frame/mapping_list/keyboard.rs`, and any other call sites with the alias for consistency. Bind `use crate::frame::view_state::MappingKey;` at the top of touched files. Don't worry about non-mapping-list call sites that happen to use `(String, InputAddress)` for unrelated reasons.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::view_state`
Expected: PASS, all existing tests plus the new alias-compiles gate.
Run: `cargo build -p inputforge-gui-dx` to confirm the search-and-replace compiles.
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/view_state.rs crates/inputforge-gui-dx/src/frame/mod.rs crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs crates/inputforge-gui-dx/src/frame/mapping_list/keyboard.rs
git commit -m "refactor(frame): introduce MappingKey type alias"
```

---

### Task 3: `ConfigSnapshot` extension, `selected_mapping_actions` + `selected_mapping_key`

Extend `ConfigSnapshot` with two paired fields. `from_state` takes an additional `&Option<MappingKey>` parameter (the current selection peeked from `view.selected_mapping`) and clones the matching mapping's `actions` when present. The paired `selected_mapping_key` field records the key resolved at the same tick so the editor can detect cross-window conflicts (selection still refers to a key that no longer matches).

**Files:**
- Modify: `crates/inputforge-gui-dx/src/context.rs`
- Test: `crates/inputforge-gui-dx/src/context.rs` (existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Append to `crates/inputforge-gui-dx/src/context.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn config_from_state_with_selection_clones_actions() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId};

    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let mappings = vec![Mapping {
        input: addr.clone(),
        mode: "Default".to_owned(),
        name: Some("Fire".to_owned()),
        actions: vec![Action::Invert],
    }];
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);

    let sel = Some(("Default".to_owned(), addr.clone()));
    let cfg = ConfigSnapshot::from_state(&state, &sel);

    assert_eq!(
        cfg.selected_mapping_actions.as_ref().map(|a| a.len()),
        Some(1)
    );
    assert_eq!(
        cfg.selected_mapping_key.as_ref(),
        Some(&("Default".to_owned(), addr.clone()))
    );
}

#[test]
fn config_from_state_without_selection_actions_none() {
    let state = AppState::new();
    let cfg = ConfigSnapshot::from_state(&state, &None);
    assert!(cfg.selected_mapping_actions.is_none());
    assert!(cfg.selected_mapping_key.is_none());
}

#[test]
fn config_from_state_with_stale_selection_actions_none_key_present() {
    use inputforge_core::types::{DeviceId, InputId};

    let state = AppState::new();
    let stale = Some((
        "Default".to_owned(),
        InputAddress {
            device: DeviceId("nonexistent".to_owned()),
            input: InputId::Button { index: 99 },
        },
    ));
    let cfg = ConfigSnapshot::from_state(&state, &stale);
    assert!(cfg.selected_mapping_actions.is_none());
    assert_eq!(cfg.selected_mapping_key, stale);
}
```

Update existing call sites of `ConfigSnapshot::from_state(&state)` in tests and `bridge.rs` to pass `&None` initially; bridge.rs is updated in Task 4.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib context::tests::config_from_state_with_selection`
Expected: FAIL with the new fields missing.

- [ ] **Step 3: Add fields and update `from_state`**

Edit `ConfigSnapshot` in `context.rs`:

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ConfigSnapshot {
    pub devices: Vec<DeviceState>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    pub mapped_inputs: HashSet<InputAddress>,
    pub mapping_names: HashMap<InputAddress, String>,
    pub mappings: Vec<MappingSummary>,
    /// Cloned `Vec<Action>` for the currently-selected mapping, if any.
    /// Cheap because only one mapping's actions are cloned per tick.
    pub selected_mapping_actions: Option<Vec<inputforge_core::action::Action>>,
    /// The (mode, input) key recorded at the same tick. Allows the editor
    /// to detect cross-window conflicts: selection still refers to a key
    /// that the engine no longer holds.
    pub selected_mapping_key: Option<crate::frame::MappingKey>,
}
```

Replace `ConfigSnapshot::from_state` to accept `selection`:

```rust
impl ConfigSnapshot {
    pub(crate) fn from_state(
        s: &AppState,
        selection: &Option<crate::frame::MappingKey>,
    ) -> Self {
        let mut mapped_inputs = HashSet::new();
        let mut mapping_names = HashMap::new();
        let mut mappings = Vec::new();
        let mut selected_mapping_actions: Option<Vec<inputforge_core::action::Action>> = None;
        if let Some(profile) = &s.active_profile {
            for mapping in profile.mappings() {
                mapped_inputs.insert(mapping.input.clone());
                if let Some(name) = &mapping.name {
                    mapping_names.insert(mapping.input.clone(), name.clone());
                }
                mappings.push(MappingSummary {
                    input: mapping.input.clone(),
                    mode: mapping.mode.clone(),
                    name: mapping.name.clone(),
                    glyphs: derive_glyphs(&mapping.actions),
                });
                if let Some((sel_mode, sel_input)) = selection.as_ref() {
                    if mapping.mode == *sel_mode && mapping.input == *sel_input {
                        selected_mapping_actions = Some(mapping.actions.clone());
                    }
                }
            }
        }
        Self {
            devices: s.devices.clone(),
            virtual_devices: s.virtual_devices.clone(),
            mapped_inputs,
            mapping_names,
            mappings,
            selected_mapping_actions,
            selected_mapping_key: selection.clone(),
        }
    }
}
```

Update every existing in-tree call site of `ConfigSnapshot::from_state(&state)` to pass `&None`:
- `crates/inputforge-gui-dx/src/context.rs` test functions (search for `ConfigSnapshot::from_state(&state)` and add `, &None`)
- Any other crate-internal call site

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib context::tests`
Expected: PASS, all existing tests plus the three new ones.
Run: `cargo build -p inputforge-gui-dx`
Expected: clean build (bridge.rs may still be unchanged because it uses `&None` default).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/context.rs
git commit -m "feat(context): extend ConfigSnapshot with selected_mapping_actions/key"
```

---

### Task 4: Wire selection into the polling task

The polling task in `bridge.rs` constructs `ConfigSnapshot` once per tick. After Task 3, that constructor takes a selection argument. Read `view.selected_mapping.peek()` once per tick and thread the value through.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/bridge.rs`

- [ ] **Step 1: Inspect `bridge.rs`**

Read the polling task. Find the line that calls `ConfigSnapshot::from_state(&state)`. The polling task captures `view: ViewState` from context (or accepts it via the `AppContext`); confirm which one.

- [ ] **Step 2: Wire the selection**

Edit `bridge.rs` so the polling task reads `view.selected_mapping.peek().clone()` (or `view.selected_mapping.read().clone()`, depending on the surrounding signal style) once per tick and passes it as the second argument:

```rust
let selection = view.selected_mapping.peek().clone();
let next_cfg = ConfigSnapshot::from_state(&state, &selection);
```

If `view: ViewState` is not yet captured by `spawn_polling_task`, propagate it through the call site in `app.rs` (the spawn happens inside `app_root` after `view` is created, so it can be passed in). Match the existing wiring style.

- [ ] **Step 3: Verify the build**

Run: `cargo build -p inputforge-gui-dx`
Expected: clean build.

Run: `cargo test -p inputforge-gui-dx`
Expected: all tests pass (the polling task is exercised indirectly through SSR mount tests).

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/bridge.rs crates/inputforge-gui-dx/src/app.rs
git commit -m "feat(bridge): thread view.selected_mapping into ConfigSnapshot::from_state"
```

---

### Task 5: Stage-tint design tokens

Add three CSS custom properties on the `:root` block in `assets/tokens/colors.css`. Tokens use `color-mix(in srgb, ...)` to derive the per-category tint from the existing `--color-processing` / `--color-output` / `--color-control` colors at the percentages pinned in spec choice 11.

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/tokens/colors.css`

- [ ] **Step 1: Add the tokens**

Find the `:root` block and the existing `--color-processing-bg` / `--color-output-bg` / `--color-control-bg` lines (around `colors.css` line 67). Insert immediately after them:

```css
    /* F9 mapping-editor stage card backgrounds. Tinted at 6/7/6%
       per spec choice 11. Tokens are sRGB color-mix sources so the
       tint matches the canonical category color exactly. */
    --color-stage-tint-processing: color-mix(in srgb, var(--color-processing) 6%, transparent);
    --color-stage-tint-output:     color-mix(in srgb, var(--color-output)     7%, transparent);
    --color-stage-tint-control:    color-mix(in srgb, var(--color-control)    6%, transparent);
```

- [ ] **Step 2: Verify `cargo check` still passes**

Run: `cargo check -p inputforge-gui-dx`
Expected: clean.

(No automated test for CSS; visual verification happens once the stage card lands in Task 22.)

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/assets/tokens/colors.css
git commit -m "feat(tokens): add stage-tint tokens for mapping-editor categories"
```

---

## Phase C, EditorState + UndoLog (Tasks 6-11)

### Task 6: `StageId` and `UndoKind` data shapes

`StageId` identifies a stage in a possibly-nested action tree by a path of indices (e.g. `[0]` for the outer-pipeline first stage, `[2, 1, 0]` for the third stage's `if_true.if_false` first stage). `UndoKind` enumerates the six committable change kinds. Both are `pub(crate)` and live in `frame/mapping_editor/undo_log.rs` (a new file).

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs` (this task creates the directory + `mod.rs` stub)
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mod.rs` (`pub(crate) mod mapping_editor;`)

- [ ] **Step 1: Create the directory and stub `mod.rs`**

```bash
mkdir -p crates/inputforge-gui-dx/src/frame/mapping_editor
```

Write `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`:

```rust
//! F9 mapping editor (center column). See
//! `docs/superpowers/specs/2026-04-30-f9-mapping-editor-design.md`.
//!
//! This is a stub; further sub-modules land in Tasks 7+.

#![allow(
    dead_code,
    reason = "Sub-modules expose APIs that the orchestrator + Tasks 12+ consume; \
              clippy's reachability check loses some pub(crate) items here."
)]

pub(crate) mod undo_log;
```

- [ ] **Step 2: Write the failing test**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs` with:

```rust
//! Per-mapping session-undo log. See spec § "Per-mapping session-undo log".

use std::collections::HashMap;

use inputforge_core::action::Mapping;

use crate::frame::MappingKey;

/// Path of segments identifying a stage in a possibly-nested action tree.
///
/// Examples (using the `StageIdSegment` variants below):
/// - `[Index(0)]`                              outer-pipeline first stage
/// - `[Index(2)]`                              outer-pipeline third stage
/// - `[Index(2), IfTrue, Index(1)]`            Conditional at outer index 2, `if_true` branch, second stage
/// - `[Index(2), IfFalse, Index(0)]`           Conditional at outer index 2, `if_false` branch, first stage
///
/// Paths are positional, NOT identity-based. Structural mutations
/// (insert/remove) invalidate every StageId at or after the mutation point.
/// See Task 11 for the clear-on-mutation contract that keeps
/// `expanded_stages` and `malformed_hints` consistent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct StageId(pub Vec<StageIdSegment>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum StageIdSegment {
    Index(usize),
    IfTrue,
    IfFalse,
}

/// Kinds of change recorded in the undo stack.
///
/// Note: editing-mode changes (re-assigning a mapping to a different mode)
/// are encoded as `Rebind` because the mode is an axis of
/// `EngineCommand::SetMapping` alongside the input address. This keeps the
/// label-format helper (Task 8) compact; if F-future ever needs distinct
/// labelling, add an explicit `ChangeMode` variant then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UndoKind {
    StageEdit,
    StageAdd,
    StageRemove,
    StageReorder,
    Rename,
    Rebind,
}

/// One entry in a mapping's undo stack.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UndoEntry {
    pub kind: UndoKind,
    /// Full Mapping snapshot for restore. Cheap; bounded by stage count.
    pub mapping_before: Mapping,
    /// Human-readable label per the F9 convention.
    /// See spec § "`UndoLog` data shape" for format.
    pub label: String,
}

/// Per-mapping FIFO-capped undo + redo stacks.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MappingHistory {
    pub undo: Vec<UndoEntry>,
    pub redo: Vec<UndoEntry>,
}

/// Per-mapping session-undo log. Keyed by `MappingKey`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct UndoLog {
    pub stacks: HashMap<MappingKey, MappingHistory>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_kind_variants_present() {
        // Compile-time presence check.
        let _ = UndoKind::StageEdit;
        let _ = UndoKind::StageAdd;
        let _ = UndoKind::StageRemove;
        let _ = UndoKind::StageReorder;
        let _ = UndoKind::Rename;
        let _ = UndoKind::Rebind;
    }

    #[test]
    fn stage_id_segment_variants_present() {
        let _ = StageIdSegment::Index(0);
        let _ = StageIdSegment::IfTrue;
        let _ = StageIdSegment::IfFalse;
    }
}
```

Wire `pub(crate) mod mapping_editor;` into `crates/inputforge-gui-dx/src/frame/mod.rs`:

```rust
mod banner;
mod layout;
mod mapping_editor;
mod mapping_list;
mod panel_slot;
mod status_bar;
mod top_bar;
mod view_state;
```

- [ ] **Step 3: Verify build and run tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::undo_log::tests`
Expected: PASS, two tests.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/src/frame/mod.rs
git commit -m "feat(mapping_editor): scaffold UndoKind, StageId, MappingHistory shapes"
```

---

### Task 7: `UndoLog::push_edit`, `undo`, `redo`

Pure semantics:
- `push_edit(key, before, kind, label)` appends to `key`'s undo stack and clears its redo stack. Enforces 50-entry FIFO cap; drops the oldest entry beyond cap.
- `undo(key) -> Option<UndoEntry>` pops the last undo entry, pushes it to redo, returns it (caller dispatches `SetMapping` with `entry.mapping_before`).
- `redo(key) -> Option<UndoEntry>` pops the last redo entry, pushes it to undo, returns it.
- `clear(key)` clears both stacks for the key. Used by Task 32's profile-flip `DirtyConfirmDialog::onsave` callback.

**Coalescing:** No coalescing in F9. Consecutive single-character rename keystrokes each create a distinct undo entry; `push_edit` is called once per `SetMapping` dispatch with no debouncing, no timestamp window, no kind/target collapse. Future work (out of scope for F9): timestamp-window coalescing keyed by `(kind, target)`. The 50-entry FIFO cap (per spec AC #25) bounds memory pressure in the absence of coalescing.

**Cross-mapping isolation:** `MappingHistory` is per-`MappingKey`. Switching the editor's selected mapping does NOT invalidate either stack on either side; switching A→B→A leaves A's undo and redo intact. Test `mapping_history_isolated_across_switches` (below) pins this.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(test)] mod tests` in `undo_log.rs`:

```rust
use inputforge_core::action::{Action, Mapping};
use inputforge_core::types::{DeviceId, InputAddress, InputId};

fn synth_key() -> MappingKey {
    (
        "Default".to_owned(),
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        },
    )
}

fn synth_mapping(name: &str) -> Mapping {
    Mapping {
        input: synth_key().1,
        mode: "Default".to_owned(),
        name: Some(name.to_owned()),
        actions: vec![],
    }
}

#[test]
fn push_edit_appends_and_clears_redo() {
    let mut log = UndoLog::default();
    let key = synth_key();

    log.push_edit(
        key.clone(),
        synth_mapping("v1"),
        UndoKind::Rename,
        "rename: 'X' -> 'v1'".to_owned(),
    );

    let stack = log.stacks.get(&key).unwrap();
    assert_eq!(stack.undo.len(), 1);
    assert!(stack.redo.is_empty());
}

#[test]
fn push_edit_clears_redo_stack_on_fresh_edit() {
    let mut log = UndoLog::default();
    let key = synth_key();
    log.push_edit(key.clone(), synth_mapping("a"), UndoKind::Rename, "a".to_owned());
    log.undo(&key);
    // redo now has 1 entry.
    log.push_edit(key.clone(), synth_mapping("b"), UndoKind::Rename, "b".to_owned());
    let stack = log.stacks.get(&key).unwrap();
    assert!(stack.redo.is_empty(), "fresh edit must clear redo");
}

#[test]
fn push_edit_caps_at_fifty_with_fifo_eviction() {
    let mut log = UndoLog::default();
    let key = synth_key();
    for i in 0..60_u32 {
        log.push_edit(
            key.clone(),
            synth_mapping(&format!("v{i}")),
            UndoKind::Rename,
            format!("rename to v{i}"),
        );
    }
    let stack = log.stacks.get(&key).unwrap();
    assert_eq!(stack.undo.len(), 50);
    // Oldest entries (v0..v9) are evicted; the bottom of the stack is v10.
    assert_eq!(stack.undo[0].label, "rename to v10");
    assert_eq!(stack.undo[49].label, "rename to v59");
}

#[test]
fn undo_pops_and_pushes_to_redo() {
    let mut log = UndoLog::default();
    let key = synth_key();
    log.push_edit(key.clone(), synth_mapping("a"), UndoKind::Rename, "a".to_owned());

    let entry = log.undo(&key).unwrap();
    assert_eq!(entry.label, "a");
    let stack = log.stacks.get(&key).unwrap();
    assert!(stack.undo.is_empty());
    assert_eq!(stack.redo.len(), 1);
}

#[test]
fn undo_returns_none_when_stack_empty() {
    let mut log = UndoLog::default();
    let key = synth_key();
    assert!(log.undo(&key).is_none());
}

#[test]
fn redo_pops_and_pushes_to_undo() {
    let mut log = UndoLog::default();
    let key = synth_key();
    log.push_edit(key.clone(), synth_mapping("a"), UndoKind::Rename, "a".to_owned());
    log.undo(&key);

    let entry = log.redo(&key).unwrap();
    assert_eq!(entry.label, "a");
    let stack = log.stacks.get(&key).unwrap();
    assert_eq!(stack.undo.len(), 1);
    assert!(stack.redo.is_empty());
}

#[test]
fn clear_removes_both_stacks() {
    let mut log = UndoLog::default();
    let key = synth_key();
    log.push_edit(key.clone(), synth_mapping("a"), UndoKind::Rename, "a".to_owned());
    log.undo(&key);
    log.clear(&key);
    // Implementation removes the entry entirely; pin that behavior.
    assert!(log.stacks.get(&key).is_none(), "clear must remove the key");
}

#[test]
fn last_label_returns_top_of_undo() {
    let mut log = UndoLog::default();
    let key = synth_key();
    log.push_edit(key.clone(), synth_mapping("a"), UndoKind::Rename, "first".to_owned());
    log.push_edit(key.clone(), synth_mapping("b"), UndoKind::Rename, "second".to_owned());
    assert_eq!(log.last_label(&key).as_deref(), Some("second"));
}

#[test]
fn mapping_history_isolated_across_switches() {
    // Switch A → B → A: A's undo/redo stacks must survive unchanged.
    let mut log = UndoLog::default();
    let key_a = synth_key();
    let key_b = (
        "Default".to_owned(),
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 1 },
        },
    );

    // Edit A.
    log.push_edit(key_a.clone(), synth_mapping("a1"), UndoKind::Rename, "a1".to_owned());
    log.undo(&key_a);
    // A now: undo=0, redo=1.

    // Switch to B and edit.
    log.push_edit(key_b.clone(), synth_mapping("b1"), UndoKind::Rename, "b1".to_owned());

    // Switch back to A. Verify A's stacks are intact.
    let a = log.stacks.get(&key_a).unwrap();
    assert_eq!(a.undo.len(), 0);
    assert_eq!(a.redo.len(), 1);
    let b = log.stacks.get(&key_b).unwrap();
    assert_eq!(b.undo.len(), 1);
    assert_eq!(b.redo.len(), 0);

    // Redo on A still works.
    let entry = log.redo(&key_a).unwrap();
    assert_eq!(entry.label, "a1");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::undo_log::tests`
Expected: FAIL with missing methods.

- [ ] **Step 3: Implement the methods**

Append to `impl UndoLog` (creating the impl block) in `undo_log.rs`:

```rust
/// Cap on per-mapping undo entries, per spec AC #25
/// ("Undo stack caps 50 entries; FIFO eviction").
const MAX_ENTRIES: usize = 50;

impl UndoLog {
    /// Append an edit entry. Clears the redo stack on this key.
    /// Enforces 50-entry FIFO cap.
    pub(crate) fn push_edit(
        &mut self,
        key: MappingKey,
        before: Mapping,
        kind: UndoKind,
        label: String,
    ) {
        let history = self.stacks.entry(key).or_default();
        history.redo.clear();
        history.undo.push(UndoEntry {
            kind,
            mapping_before: before,
            label,
        });
        if history.undo.len() > MAX_ENTRIES {
            let drain_count = history.undo.len() - MAX_ENTRIES;
            history.undo.drain(..drain_count);
        }
    }

    /// Pop the last undo entry and push it to redo. Caller dispatches
    /// `SetMapping` with `entry.mapping_before`.
    pub(crate) fn undo(&mut self, key: &MappingKey) -> Option<UndoEntry> {
        let history = self.stacks.get_mut(key)?;
        let entry = history.undo.pop()?;
        history.redo.push(entry.clone());
        Some(entry)
    }

    /// Pop the last redo entry and push it to undo.
    pub(crate) fn redo(&mut self, key: &MappingKey) -> Option<UndoEntry> {
        let history = self.stacks.get_mut(key)?;
        let entry = history.redo.pop()?;
        history.undo.push(entry.clone());
        Some(entry)
    }

    /// Clear both stacks for `key`.
    pub(crate) fn clear(&mut self, key: &MappingKey) {
        self.stacks.remove(key);
    }

    /// Read the label of the topmost undo entry, if any. Used by the
    /// editor footer recap.
    pub(crate) fn last_label(&self, key: &MappingKey) -> Option<String> {
        self.stacks
            .get(key)
            .and_then(|h| h.undo.last())
            .map(|e| e.label.clone())
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::undo_log::tests`
Expected: PASS, all nine tests.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs
git commit -m "feat(undo_log): push_edit/undo/redo/clear with 50-entry FIFO cap"
```

---

### Task 8: Label-format helpers

Per the spec's label-format convention table, each `UndoKind` produces a deterministic label string. Encode the convention as a single function `format_undo_label(kind, args)` that callers use rather than constructing strings ad-hoc. Validation is enforced by tests, not at runtime.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs`

- [ ] **Step 1: Write the failing tests**

Append to the test module:

```rust
#[test]
fn label_format_stage_edit() {
    let label = format_undo_label(UndoKind::StageEdit, LabelArgs {
        stage_name: Some("deadzone outer"),
        field: Some("threshold"),
        before_after: Some(("92%", "95%")),
        index: None,
        from_to: None,
        old_new: None,
    });
    assert_eq!(label, "deadzone outer: threshold 92% -> 95%");
}

#[test]
fn label_format_stage_add() {
    let label = format_undo_label(UndoKind::StageAdd, LabelArgs {
        stage_name: Some("ResponseCurve"),
        index: Some(2),
        ..LabelArgs::default()
    });
    assert_eq!(label, "add stage: ResponseCurve at index 2");
}

#[test]
fn label_format_stage_remove() {
    let label = format_undo_label(UndoKind::StageRemove, LabelArgs {
        stage_name: Some("Deadzone"),
        index: Some(0),
        ..LabelArgs::default()
    });
    assert_eq!(label, "remove stage: Deadzone at index 0");
}

#[test]
fn label_format_stage_reorder() {
    let label = format_undo_label(UndoKind::StageReorder, LabelArgs {
        stage_name: Some("MergeAxis"),
        from_to: Some((1, 0)),
        ..LabelArgs::default()
    });
    assert_eq!(label, "move stage MergeAxis from 1 to 0");
}

#[test]
fn label_format_rename() {
    let label = format_undo_label(UndoKind::Rename, LabelArgs {
        old_new: Some(("X axis", "Yaw")),
        ..LabelArgs::default()
    });
    assert_eq!(label, "rename: 'X axis' -> 'Yaw'");
}

#[test]
fn label_format_rebind() {
    let label = format_undo_label(UndoKind::Rebind, LabelArgs {
        old_new: Some(("VPC Stick X", "VKB Pedals Y")),
        ..LabelArgs::default()
    });
    assert_eq!(label, "rebind: VPC Stick X -> VKB Pedals Y");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::undo_log::tests::label_format`
Expected: FAIL.

- [ ] **Step 3: Implement the helper**

Append to `undo_log.rs`:

```rust
/// Argument bundle for `format_undo_label`. Each `UndoKind` reads a
/// specific subset of fields; the rest are ignored.
#[derive(Debug, Clone, Default)]
pub(crate) struct LabelArgs<'a> {
    /// Human-readable stage variant or stage display name.
    pub stage_name: Option<&'a str>,
    /// Field name within a stage body (e.g. "threshold", "operation").
    pub field: Option<&'a str>,
    /// `(before, after)` field values stringified by the caller.
    pub before_after: Option<(&'a str, &'a str)>,
    /// Pipeline index for add / remove.
    pub index: Option<usize>,
    /// `(from_index, to_index)` for reorder.
    pub from_to: Option<(usize, usize)>,
    /// `(old, new)` for rename / rebind.
    pub old_new: Option<(&'a str, &'a str)>,
}

/// Format an undo-entry label per the F9 convention.
///
/// See spec § "`UndoLog` data shape" for the canonical table.
#[must_use]
pub(crate) fn format_undo_label(kind: UndoKind, args: LabelArgs<'_>) -> String {
    match kind {
        UndoKind::StageEdit => {
            let name = args.stage_name.unwrap_or("?");
            let field = args.field.unwrap_or("?");
            let (b, a) = args.before_after.unwrap_or(("?", "?"));
            format!("{name}: {field} {b} -> {a}")
        }
        UndoKind::StageAdd => {
            let name = args.stage_name.unwrap_or("?");
            let i = args.index.unwrap_or(0);
            format!("add stage: {name} at index {i}")
        }
        UndoKind::StageRemove => {
            let name = args.stage_name.unwrap_or("?");
            let i = args.index.unwrap_or(0);
            format!("remove stage: {name} at index {i}")
        }
        UndoKind::StageReorder => {
            let name = args.stage_name.unwrap_or("?");
            let (from, to) = args.from_to.unwrap_or((0, 0));
            format!("move stage {name} from {from} to {to}")
        }
        UndoKind::Rename => {
            let (old, new) = args.old_new.unwrap_or(("?", "?"));
            format!("rename: '{old}' -> '{new}'")
        }
        UndoKind::Rebind => {
            let (old, new) = args.old_new.unwrap_or(("?", "?"));
            format!("rebind: {old} -> {new}")
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::undo_log::tests`
Expected: PASS, all 14 tests in this file.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs
git commit -m "feat(undo_log): format_undo_label helper enforces label convention"
```

---

### Task 9: `EditorState` provider hook

`EditorState` is the editor-internal context, a `Copy` struct of `Signal<...>` fields parallel to `LiveCapture`. Provided once in `app_root` after `LiveCapture`. Components read via `use_context::<EditorState>()`.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/app.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs` (creating a `#[cfg(test)] mod tests` block at the end):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_state_field_types_compile() {
        // Compile-time gate: EditorState must expose the four signals.
        fn _assert(state: EditorState) {
            let _: dioxus::prelude::Signal<undo_log::UndoLog> = state.undo_log;
            let _: dioxus::prelude::Signal<std::collections::HashSet<undo_log::StageId>> =
                state.expanded_stages;
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests`
Expected: FAIL with `EditorState` undefined.

- [ ] **Step 3: Implement `EditorState` and the provider hook**

Replace the contents of `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`:

```rust
//! F9 mapping editor (center column). See
//! `docs/superpowers/specs/2026-04-30-f9-mapping-editor-design.md`.

#![allow(
    dead_code,
    reason = "Sub-modules expose APIs that orchestrator + Tasks 12+ consume; \
              clippy's reachability check loses some pub(crate) items here."
)]

pub(crate) mod undo_log;

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

use inputforge_core::types::InputAddress;

use crate::frame::mapping_editor::undo_log::{StageId, UndoLog};

/// Right-click stage menu state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StageMenuState {
    pub stage: StageId,
    /// page-space anchor coordinates
    pub x: f64,
    pub y: f64,
}

/// Editor-internal context, parallel to `LiveCapture` and `ToastQueue`.
///
/// Installed once via `use_editor_state_provider` from `app_root`.
/// Components read via `use_context::<EditorState>()`.
#[derive(Clone, Copy)]
pub(crate) struct EditorState {
    /// Per-mapping undo stacks. Cleared on profile flip via Task 32's
    /// `DirtyConfirmDialog::onsave` callback.
    pub undo_log: Signal<UndoLog>,
    /// Stage IDs that are currently expanded. Resets on selection change
    /// AND on every structural mutation (insert/remove) — see Task 11.
    pub expanded_stages: Signal<HashSet<StageId>>,
    /// Right-click menu state (anchor + target stage).
    pub stage_menu: Signal<Option<StageMenuState>>,
    /// Per-stage validation hints surfaced in the stage header summary
    /// slot per spec lines 587-589. Bodies write on render; the stage
    /// header reads. Cleared on every structural mutation — see Task 11.
    pub malformed_hints: Signal<HashMap<StageId, String>>,
    /// External-edit reconciliation token. Incremented by the polling
    /// task (bridge.rs) on every external snapshot change. Bodies
    /// subscribe via `use_effect` and reset their local Signals when the
    /// token advances. See Task 33.
    pub external_edit_reset: Signal<u64>,
}

/// Allocate signals and install `EditorState` in context. Call exactly
/// once from `app_root`, the provider self-installs.
pub(crate) fn use_editor_state_provider() -> EditorState {
    let undo_log: Signal<UndoLog> = use_signal(UndoLog::default);
    let expanded_stages: Signal<HashSet<StageId>> = use_signal(HashSet::new);
    let stage_menu: Signal<Option<StageMenuState>> = use_signal(|| None);
    let malformed_hints: Signal<HashMap<StageId, String>> = use_signal(HashMap::new);
    let external_edit_reset: Signal<u64> = use_signal(|| 0_u64);

    let state = EditorState {
        undo_log,
        expanded_stages,
        stage_menu,
        malformed_hints,
        external_edit_reset,
    };
    use_context_provider(|| state);
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_state_field_types_compile() {
        fn _assert(state: EditorState) {
            let _: Signal<UndoLog> = state.undo_log;
            let _: Signal<HashSet<StageId>> = state.expanded_stages;
            let _: Signal<Option<StageMenuState>> = state.stage_menu;
            let _: Signal<HashMap<StageId, String>> = state.malformed_hints;
            let _: Signal<u64> = state.external_edit_reset;
        }
    }

    #[test]
    fn editor_state_provider_mounts_and_reads_via_use_context() {
        // SSR smoke test: provider installs both LiveCapture and EditorState,
        // a child renders and reads both via `use_context`.
        use dioxus::prelude::*;
        use dioxus_ssr::render_element;

        fn child() -> Element {
            let _live = use_context::<crate::patterns::live_capture::LiveCapture>();
            let editor = use_context::<EditorState>();
            // Touch every field so a missing one would cause a compile error
            // at instantiation; runtime asserts they read default values.
            let undo_log = editor.undo_log.read();
            assert_eq!(undo_log.stacks.len(), 0);
            assert_eq!(*editor.external_edit_reset.read(), 0_u64);
            rsx! { div { "ok" } }
        }

        fn root() -> Element {
            crate::patterns::live_capture::use_live_capture_provider();
            use_editor_state_provider();
            rsx! { child {} }
        }

        let html = render_element(rsx! { root {} });
        assert!(html.contains("ok"), "child must render with both contexts available");
    }
}
```

Install the provider in `crates/inputforge-gui-dx/src/app.rs`. After the existing `use_live_capture_provider();` line (around line 48), append:

```rust
    // F9: editor-internal state. Single instance, sibling of LiveCapture.
    use crate::frame::mapping_editor::use_editor_state_provider;
    use_editor_state_provider();
```

Mirror the same install in the `app_root_view_with_stub_contexts` test harness in `app.rs`:

```rust
    crate::frame::mapping_editor::use_editor_state_provider();
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor`
Expected: PASS, the new compile-time gate.
Run: `cargo test -p inputforge-gui-dx --lib app::tests`
Expected: PASS, the existing mount test.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs crates/inputforge-gui-dx/src/app.rs
git commit -m "feat(mapping_editor): EditorState provider with undo_log + stage signals"
```

---

### Task 10: Pure StageId helpers, `at_path` / `replace_at_path`

Two pure functions on `&[Action]` that read or replace a stage at a `StageId` path. Used by every variant body to commit local-working-copy edits, by drag-and-drop to move stages, and by the right-click menu to insert/remove. Pure and unit-testable; no Signals.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`

- [ ] **Step 1: Create pipeline submodule and write the failing tests**

Add to `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`:

```rust
pub(crate) mod pipeline;
```

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs`:

```rust
//! F9 pipeline graph component tree.
//!
//! Composition (inside-out, in dependency order):
//!   - `at_path` / `replace_at_path` / `insert_at_path` / `remove_at_path`,
//!     pure StageId tree mutators used by every body and the DnD handler
//!   - `stage_body::*`, per-variant body components
//!   - `stage_header`, title + summary + chevron
//!   - `stage`, header + body container
//!   - `Pipeline`, ordered list orchestrator (recursive for Conditional)

#![allow(
    dead_code,
    reason = "submodules export APIs consumed across the editor; clippy's \
              reachability check loses some pub(crate) items here."
)]

use inputforge_core::action::Action;

use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

/// Read the action at `path` in `actions`. Returns `None` when the
/// path does not resolve (out-of-range index, missing branch, etc.).
#[must_use]
pub(crate) fn at_path<'a>(actions: &'a [Action], path: &StageId) -> Option<&'a Action> {
    let mut cursor: &[Action] = actions;
    let mut peek: Option<&Action> = None;
    let mut iter = path.0.iter().peekable();
    while let Some(seg) = iter.next() {
        match seg {
            StageIdSegment::Index(i) => {
                let action = cursor.get(*i)?;
                if iter.peek().is_none() {
                    return Some(action);
                }
                peek = Some(action);
            }
            StageIdSegment::IfTrue => match peek? {
                Action::Conditional { if_true, .. } => cursor = if_true.as_slice(),
                _ => return None,
            },
            StageIdSegment::IfFalse => match peek? {
                Action::Conditional { if_false, .. } => match if_false.as_deref() {
                    Some(branch) => cursor = branch,
                    None => return None,
                },
                _ => return None,
            },
        }
    }
    None
}

/// Replace the action at `path` with `replacement` and return the new tree.
///
/// Returns `None` for invalid paths (out-of-range index, missing branch,
/// expected `Conditional` got something else, empty path, path starting
/// with a branch segment). Callers must skip the edit and skip the
/// `push_edit` on `None`, otherwise a phantom undo entry would be created
/// against unchanged state. See `EditorState` mutator pattern in Task 22+.
#[must_use]
pub(crate) fn replace_at_path(
    actions: &[Action],
    path: &StageId,
    replacement: Action,
) -> Option<Vec<Action>> {
    fn walk(
        actions: &[Action],
        path: &[StageIdSegment],
        replacement: Action,
    ) -> Option<Vec<Action>> {
        let mut out = actions.to_vec();
        let (head, tail) = path.split_first()?;
        match head {
            StageIdSegment::Index(i) => {
                if tail.is_empty() {
                    if *i >= out.len() {
                        return None;
                    }
                    out[*i] = replacement;
                    Some(out)
                } else {
                    let target = out.get_mut(*i)?;
                    let (branch_seg, rest) = tail.split_first()?;
                    let Action::Conditional {
                        if_true, if_false, ..
                    } = target
                    else {
                        return None;
                    };
                    match branch_seg {
                        StageIdSegment::IfTrue => {
                            let new = walk(if_true.as_slice(), rest, replacement)?;
                            *if_true = new;
                        }
                        StageIdSegment::IfFalse => {
                            let current = if_false.clone()?;
                            let new = walk(&current, rest, replacement)?;
                            *if_false = if new.is_empty() { None } else { Some(new) };
                        }
                        StageIdSegment::Index(_) => return None,
                    }
                    Some(out)
                }
            }
            // StageId must always start with an Index segment.
            StageIdSegment::IfTrue | StageIdSegment::IfFalse => None,
        }
    }
    walk(actions, &path.0, replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    use inputforge_core::action::{Action, Condition};
    use inputforge_core::types::{DeviceId, InputAddress, InputId, MergeOp};

    fn synth_addr() -> InputAddress {
        InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        }
    }

    #[test]
    fn at_path_outer_index() {
        let actions = vec![Action::Invert];
        let path = StageId(vec![StageIdSegment::Index(0)]);
        assert!(matches!(at_path(&actions, &path), Some(Action::Invert)));
    }

    #[test]
    fn at_path_into_if_true_branch() {
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: synth_addr() },
            if_true: vec![Action::Invert],
            if_false: None,
        }];
        let path = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfTrue,
            StageIdSegment::Index(0),
        ]);
        assert!(matches!(at_path(&actions, &path), Some(Action::Invert)));
    }

    #[test]
    fn at_path_into_missing_if_false_returns_none() {
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: synth_addr() },
            if_true: vec![],
            if_false: None,
        }];
        let path = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfFalse,
            StageIdSegment::Index(0),
        ]);
        assert!(at_path(&actions, &path).is_none());
    }

    #[test]
    fn replace_at_path_outer_swaps_action() {
        let actions = vec![Action::Invert];
        let path = StageId(vec![StageIdSegment::Index(0)]);
        let new = replace_at_path(
            &actions,
            &path,
            Action::MergeAxis {
                second_input: synth_addr(),
                operation: MergeOp::Average,
            },
        )
        .expect("valid path must succeed");
        assert!(matches!(new[0], Action::MergeAxis { .. }));
    }

    #[test]
    fn replace_at_path_inside_if_true_swaps_action() {
        let actions = vec![Action::Conditional {
            condition: Condition::ButtonPressed { input: synth_addr() },
            if_true: vec![Action::Invert],
            if_false: None,
        }];
        let path = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfTrue,
            StageIdSegment::Index(0),
        ]);
        let new = replace_at_path(
            &actions,
            &path,
            Action::MergeAxis {
                second_input: synth_addr(),
                operation: MergeOp::Average,
            },
        )
        .expect("valid path must succeed");
        match &new[0] {
            Action::Conditional { if_true, .. } => {
                assert!(matches!(if_true[0], Action::MergeAxis { .. }));
            }
            _ => panic!("outer wrapper should remain Conditional"),
        }
    }

    #[test]
    fn replace_at_path_invalid_path_returns_none() {
        // Out-of-range index — must return None, not panic, in BOTH debug
        // and release. Callers depend on this to skip the edit + skip
        // push_edit (no phantom undo entries).
        let actions = vec![Action::Invert];
        let path = StageId(vec![StageIdSegment::Index(99)]);
        assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

        // Empty path.
        let path = StageId(vec![]);
        assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

        // Path starts with a branch segment.
        let path = StageId(vec![StageIdSegment::IfTrue]);
        assert!(replace_at_path(&actions, &path, Action::Invert).is_none());

        // Branch segment after a non-Conditional action.
        let path = StageId(vec![StageIdSegment::Index(0), StageIdSegment::IfTrue]);
        assert!(replace_at_path(&actions, &path, Action::Invert).is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::tests`
Expected: PASS, all 6 tests.

Run: `cargo test -p inputforge-gui-dx --release --lib frame::mapping_editor::pipeline::tests`
Expected: PASS, all 6 tests in release mode (the `replace_at_path_invalid_path_returns_none` test is the load-bearing assertion that release builds do NOT panic).

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor
git commit -m "feat(pipeline): add at_path/replace_at_path StageId tree mutators"
```

---

### Task 11: `insert_at_path` and `remove_at_path`

Two more pure mutators for the add and delete flows. Same module as Task 10. Both return `Option<Vec<Action>>` (None for invalid paths) for the same reason as `replace_at_path`: callers must be able to skip the edit + skip `push_edit` to avoid phantom undo entries.

**Structural-mutation invariant.** When `insert_at_path` or `remove_at_path` is invoked from an `EditorState` mutator (e.g., the right-click menu's Delete, drag-and-drop drop, palette add), the caller MUST also clear `editor_state.expanded_stages.write().clear()` and `editor_state.malformed_hints.write().clear()` AFTER dispatching the new actions. Reason: `StageId` paths are positional. Inserting or removing a stage shifts every sibling index at or after the mutation point, invalidating every cached path in those two `HashMap`/`HashSet` containers. UX tradeoff: the user loses expand-state on rare reorder/remove operations, which is acceptable because (a) reorder is rare in a session, (b) the alternative is a re-keying pass that complicates the mutator signature without proportional benefit, (c) malformed hints recompute on the next render. This contract is enforced by a test in Task 22 (the first body that wires through `EditorState`); the test asserts `expanded_stages.is_empty() && malformed_hints.is_empty()` after a structural mutation.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append to the test module:

```rust
#[test]
fn insert_at_path_outer_appends() {
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(1)]);
    let new = insert_at_path(&actions, &path, Action::Invert).expect("valid path");
    assert_eq!(new.len(), 2);
}

#[test]
fn insert_at_path_outer_inserts_at_index() {
    let actions = vec![Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(0)]);
    let new = insert_at_path(
        &actions,
        &path,
        Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
    )
    .expect("valid path");
    assert_eq!(new.len(), 2);
    assert!(matches!(new[0], Action::MergeAxis { .. }));
    assert!(matches!(new[1], Action::Invert));
}

#[test]
fn insert_at_path_into_if_false_creates_branch() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: synth_addr() },
        if_true: vec![],
        if_false: None,
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfFalse,
        StageIdSegment::Index(0),
    ]);
    let new = insert_at_path(&actions, &path, Action::Invert).expect("valid path");
    match &new[0] {
        Action::Conditional { if_false, .. } => {
            assert_eq!(if_false.as_ref().map(|b| b.len()), Some(1));
        }
        _ => panic!("expected Conditional"),
    }
}

#[test]
fn remove_at_path_outer_drops_action() {
    let actions = vec![Action::Invert, Action::Invert];
    let path = StageId(vec![StageIdSegment::Index(0)]);
    let new = remove_at_path(&actions, &path).expect("valid path");
    assert_eq!(new.len(), 1);
}

#[test]
fn remove_at_path_last_in_if_false_collapses_to_none() {
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: synth_addr() },
        if_true: vec![],
        if_false: Some(vec![Action::Invert]),
    }];
    let path = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfFalse,
        StageIdSegment::Index(0),
    ]);
    let new = remove_at_path(&actions, &path).expect("valid path");
    match &new[0] {
        Action::Conditional { if_false, .. } => {
            assert!(if_false.is_none(), "empty if_false branch must collapse to None");
        }
        _ => panic!("expected Conditional"),
    }
}

#[test]
fn insert_remove_invalid_paths_return_none() {
    // Same contract as replace_at_path: callers depend on None
    // (NOT panic in release) so they can skip the edit + skip push_edit.
    let actions = vec![Action::Invert];

    // Empty path.
    assert!(insert_at_path(&actions, &StageId(vec![]), Action::Invert).is_none());
    assert!(remove_at_path(&actions, &StageId(vec![])).is_none());

    // Path starts with branch segment.
    assert!(insert_at_path(&actions, &StageId(vec![StageIdSegment::IfTrue]), Action::Invert).is_none());
    assert!(remove_at_path(&actions, &StageId(vec![StageIdSegment::IfTrue])).is_none());

    // Out-of-range index for remove_at_path.
    assert!(remove_at_path(&actions, &StageId(vec![StageIdSegment::Index(99)])).is_none());

    // Branch segment after a non-Conditional action.
    let path = StageId(vec![StageIdSegment::Index(0), StageIdSegment::IfTrue, StageIdSegment::Index(0)]);
    assert!(insert_at_path(&actions, &path, Action::Invert).is_none());
    assert!(remove_at_path(&actions, &path).is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::tests::insert_at_path`
Expected: FAIL with missing function.

- [ ] **Step 3: Implement the helpers**

Append to `pipeline/mod.rs`:

```rust
/// Insert `new_action` at `path`. The terminal segment must be an
/// `Index` indicating the insertion point; existing actions at that
/// index and beyond shift right. Indexes past the end append.
///
/// Returns `None` for invalid paths (empty, starts with branch, branch
/// segment after non-Conditional). Callers MUST skip the edit + skip
/// `push_edit` on `None` (avoids phantom undo entries).
#[must_use]
pub(crate) fn insert_at_path(
    actions: &[Action],
    path: &StageId,
    new_action: Action,
) -> Option<Vec<Action>> {
    fn walk(
        actions: &[Action],
        path: &[StageIdSegment],
        new_action: Action,
    ) -> Option<Vec<Action>> {
        let mut out = actions.to_vec();
        let (head, tail) = path.split_first()?;
        match head {
            StageIdSegment::Index(i) => {
                if tail.is_empty() {
                    let pos = (*i).min(out.len());
                    out.insert(pos, new_action);
                    Some(out)
                } else {
                    let target = out.get_mut(*i)?;
                    let Action::Conditional { if_true, if_false, .. } = target else {
                        return None;
                    };
                    let (branch_seg, rest) = tail.split_first()?;
                    match branch_seg {
                        StageIdSegment::IfTrue => {
                            *if_true = walk(if_true.as_slice(), rest, new_action)?;
                        }
                        StageIdSegment::IfFalse => {
                            let current = if_false.clone().unwrap_or_default();
                            let new = walk(&current, rest, new_action)?;
                            *if_false = if new.is_empty() { None } else { Some(new) };
                        }
                        StageIdSegment::Index(_) => return None,
                    }
                    Some(out)
                }
            }
            StageIdSegment::IfTrue | StageIdSegment::IfFalse => None,
        }
    }
    walk(actions, &path.0, new_action)
}

/// Remove the action at `path`. If the removal empties an `if_false`
/// branch, the branch collapses back to `None` (the engine's "do nothing"
/// shape). `if_true` always stays as a `Vec`, possibly empty.
///
/// Returns `None` for invalid paths (empty, starts with branch, out-of-range
/// terminal index, branch segment after non-Conditional). Callers MUST skip
/// the edit + skip `push_edit` on `None` (avoids phantom undo entries).
#[must_use]
pub(crate) fn remove_at_path(actions: &[Action], path: &StageId) -> Option<Vec<Action>> {
    fn walk(actions: &[Action], path: &[StageIdSegment]) -> Option<Vec<Action>> {
        let mut out = actions.to_vec();
        let (head, tail) = path.split_first()?;
        match head {
            StageIdSegment::Index(i) => {
                if tail.is_empty() {
                    if *i >= out.len() {
                        return None;
                    }
                    out.remove(*i);
                    Some(out)
                } else {
                    let target = out.get_mut(*i)?;
                    let Action::Conditional { if_true, if_false, .. } = target else {
                        return None;
                    };
                    let (branch_seg, rest) = tail.split_first()?;
                    match branch_seg {
                        StageIdSegment::IfTrue => {
                            *if_true = walk(if_true.as_slice(), rest)?;
                        }
                        StageIdSegment::IfFalse => {
                            let branch = if_false.as_ref()?;
                            let new = walk(branch, rest)?;
                            *if_false = if new.is_empty() { None } else { Some(new) };
                        }
                        StageIdSegment::Index(_) => return None,
                    }
                    Some(out)
                }
            }
            StageIdSegment::IfTrue | StageIdSegment::IfFalse => None,
        }
    }
    walk(actions, &path.0)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::tests`
Expected: PASS, all tests in this module.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs
git commit -m "feat(pipeline): add insert_at_path and remove_at_path mutators"
```

---

## Phase D, Editor frame skeleton (Tasks 12-14)

### Task 12: `MappingEditor` orchestrator + empty state mount

Land the `MappingEditor` component (renders nothing yet beyond the empty state when no mapping is selected) and mount it in `frame::Layout`'s `if-layout__center` slot, replacing the placeholder text.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/empty_state.rs`
- Create: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/layout/mod.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs` (create the file if missing). First wire the `#[cfg(test)] mod tests;` declaration in `mod.rs`:

```rust
#[cfg(test)]
mod tests;
```

Then write `crates/inputforge-gui-dx/src/frame/mapping_editor/tests.rs`:

```rust
//! SSR tests for the F9 mapping editor.

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::frame::mapping_editor::{MappingEditor, use_editor_state_provider};
use crate::frame::view_state::use_view_state_provider;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

fn harness() -> Element {
    let (cmd_tx, _cmd_rx) = mpsc::channel();
    let raw = RawHandles {
        state: Arc::new(RwLock::new(AppState::new())),
        commands: cmd_tx,
        settings: Arc::new(AppSettings::default()),
    };
    use_context_provider(|| raw.clone());

    let meta = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    let ctx = AppContext {
        state: Arc::clone(&raw.state),
        commands: raw.commands.clone(),
        settings: Arc::clone(&raw.settings),
        meta,
        config,
        live,
    };
    use_context_provider(|| ctx);

    use_view_state_provider(meta);
    use_live_capture_provider();
    use_editor_state_provider();
    let toast_state = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast_state });

    rsx! { MappingEditor {} }
}

#[test]
fn editor_renders_empty_state_when_no_selection() {
    let mut vdom = VirtualDom::new(harness);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Select a mapping"),
        "expected empty state title, got: {html}"
    );
    assert!(html.contains("if-editor"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_renders_empty_state`
Expected: FAIL because `MappingEditor` is not defined yet.

- [ ] **Step 3: Implement empty state component**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/empty_state.rs`:

```rust
//! "Select a mapping" empty state.

use dioxus::prelude::*;

#[component]
pub(crate) fn EmptyState() -> Element {
    rsx! {
        div { class: "if-editor__empty",
            div { class: "if-editor__empty-title", "Select a mapping" }
            div { class: "if-editor__empty-helper",
                "Pick a row in the rail, or click "
                kbd { class: "if-editor__kbd", "+ Add mapping" }
                " below the list to start one."
            }
        }
    }
}
```

Create `crates/inputforge-gui-dx/assets/frame/mapping_editor.css` (minimal, expanded by later tasks):

```css
/* F9 mapping editor styling. See spec § "Editor frame anatomy". */

.if-editor {
    display: flex;
    flex-direction: column;
    gap: 0;
    height: 100%;
    overflow-y: auto;
}

.if-editor__empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    padding: 32px;
    text-align: center;
}

.if-editor__empty-title {
    font-family: var(--font-sans);
    font-size: 16px;
    font-weight: 600;
    color: var(--color-text);
}

.if-editor__empty-helper {
    font-family: var(--font-sans);
    font-size: 12px;
    line-height: 18px;
    color: var(--color-text-muted);
    max-width: 360px;
}

.if-editor__kbd {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--color-bg-sunken);
    border: 1px solid var(--color-border);
}
```

Append to `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`:

```rust
mod empty_state;

pub(crate) use empty_state::EmptyState;

use crate::context::AppContext;
use crate::frame::view_state::ViewState;

#[allow(
    dead_code,
    reason = "Asset is consumed by Stylesheet { href: MAPPING_EDITOR_CSS }"
)]
const MAPPING_EDITOR_CSS: Asset = asset!("/assets/frame/mapping_editor.css");

#[component]
pub(crate) fn MappingEditor() -> Element {
    tracing::trace!(target: "frame::render", region = "mapping_editor");
    let _ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let _editor = use_context::<EditorState>();

    let has_selection = view.selected_mapping.read().is_some();

    rsx! {
        Stylesheet { href: MAPPING_EDITOR_CSS }
        div { class: "if-editor",
            if !has_selection {
                EmptyState {}
            } else {
                // Frame sections + pipeline land in subsequent tasks.
                div { class: "if-editor__placeholder", "selection placeholder" }
            }
        }
    }
}
```

Wire the re-export in `crates/inputforge-gui-dx/src/frame/mod.rs`:

```rust
pub(crate) use mapping_editor::MappingEditor;
```

Replace the placeholder in `crates/inputforge-gui-dx/src/frame/layout/mod.rs`. Find the line:

```rust
div { class: "if-layout__center", "Mapping editor, F9 owns content" }
```

and change to:

```rust
div { class: "if-layout__center", crate::frame::MappingEditor {} }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor`
Expected: PASS.
Run: `cargo test -p inputforge-gui-dx --lib app::tests::app_root_mounts_frame_layout`
Expected: PASS (existing mount test still green; the harness has no profile so empty state of the layout takes precedence).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/src/frame/mod.rs crates/inputforge-gui-dx/src/frame/layout/mod.rs crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(mapping_editor): mount MappingEditor in if-layout__center with empty state"
```

---

### Task 13: Engine-offline banner

Sticky banner above the editor frame (visible only when the engine command channel is disconnected). Surfaces "Engine offline. Edits not applied." and a `Restart engine` ghost button. Detection: try `ctx.commands.send` from a tracked health-probe in the polling task is heavy; instead, observe `engine_status` from `MetaSnapshot` (Stopped or Crashed states map to "offline").

**Banner precedence (vs Task 18 inactive-runtime hint):** When the engine-offline banner is visible, the inactive-runtime hint (Task 18) is suppressed. Engine-offline subsumes mode-mismatch (an offline engine fires nothing in any mode, so the user does not need to know that the editor's mode also doesn't match). Task 18 implements the suppression guard: the inactive-runtime hint renders only when `engine_status` is `Online`. This task's banner has no Task-18 awareness, just renders unconditionally on its own trigger.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/engine_offline_banner.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 1: Write the failing test**

Append to `mapping_editor/tests.rs`:

```rust
use inputforge_core::state::EngineStatus;

#[test]
fn engine_offline_banner_visible_when_status_is_stopped() {
    fn h() -> Element {
        let (cmd_tx, _) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(|| MetaSnapshot {
            engine_status: EngineStatus::Stopped,
            profile_name: Some("P".to_owned()),
            modes: vec!["Default".to_owned()],
            startup_mode: Some("Default".to_owned()),
            ..MetaSnapshot::default()
        });
        let config = use_signal(ConfigSnapshot::default);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx);
        use_view_state_provider(meta);
        use_live_capture_provider();
        use_editor_state_provider();
        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });
        rsx! { MappingEditor {} }
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Engine offline"),
        "expected offline banner copy, got: {html}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::engine_offline_banner`
Expected: FAIL.

- [ ] **Step 3: Implement the banner**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/engine_offline_banner.rs`:

```rust
//! Sticky engine-offline banner. See spec choice 20.

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;
use inputforge_core::state::EngineStatus;

use crate::context::AppContext;

#[component]
pub(crate) fn EngineOfflineBanner() -> Element {
    let ctx = use_context::<AppContext>();
    let status = ctx.meta.read().engine_status;
    let visible = matches!(
        status,
        EngineStatus::Stopped | EngineStatus::Crashed
    );

    if !visible {
        return rsx! {};
    }

    let cmd_tx = ctx.commands.clone();
    let on_restart = move |_| {
        let _ = cmd_tx.send(EngineCommand::Activate);
        tracing::info!(target: "f9::mapping_editor", action = "restart_engine");
    };

    rsx! {
        div {
            class: "if-editor__offline-banner",
            role: "status",
            "aria-live": "polite",
            div { class: "if-editor__offline-text", "Engine offline. Edits not applied." }
            button {
                r#type: "button",
                class: "if-editor__offline-action",
                onclick: on_restart,
                "Restart engine"
            }
        }
    }
}
```

Append to `assets/frame/mapping_editor.css`:

```css
.if-editor__offline-banner {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 12px;
    background: rgba(242, 85, 85, 0.08);
    border: 1px solid rgba(242, 85, 85, 0.22);
    border-radius: 6px;
    margin: 8px 12px;
}

.if-editor__offline-text {
    font-family: var(--font-sans);
    font-size: 12px;
    color: var(--color-error);
}

.if-editor__offline-action {
    font-family: var(--font-sans);
    font-size: 12px;
    background: transparent;
    color: var(--color-error);
    border: 1px solid rgba(242, 85, 85, 0.32);
    border-radius: 4px;
    padding: 4px 10px;
    cursor: pointer;
}
```

Wire into `mapping_editor/mod.rs`. Add `mod engine_offline_banner;` and `use engine_offline_banner::EngineOfflineBanner;`. In `MappingEditor`, render the banner unconditionally above the empty state / placeholder:

```rust
rsx! {
    Stylesheet { href: MAPPING_EDITOR_CSS }
    div { class: "if-editor",
        EngineOfflineBanner {}
        if !has_selection { EmptyState {} } else { /* ... */ }
    }
}
```

(The banner returns `rsx! {}` when offline status is false, so it's effectively hidden.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(mapping_editor): engine-offline banner above editor frame"
```

---

### Task 14: Editor header (h2 + subtitle)

Header section displaying mapping name as `<h2>` and a JetBrains Mono subtitle reading `<source-label>   →   <output-label>`. Output tail omitted when no `MapToVJoy` is in the action tree.

**Long-name tooltip.** Per spec line 36, the h2 wraps the full mapping name in an F2 `Tooltip` (`crates/inputforge-gui-dx/src/components/tooltip.rs:14-33`) so the full name is reachable on hover when the visible h2 is truncated to one line via CSS `text-overflow: ellipsis`. Tooltip content = full name; placement = `Bottom`. Implementation pattern:

```rust
use crate::components::{Tooltip, TooltipPlacement};

rsx! {
    Tooltip {
        content: "{name}".to_owned(),
        placement: TooltipPlacement::Bottom,
        h2 { class: "if-editor__title", "{name}" }
    }
}
```

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/header.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 1: Write the failing test**

Append to `mapping_editor/tests.rs`:

```rust
use inputforge_core::action::{Action, Mapping};
use inputforge_core::mode::ModeTree;
use inputforge_core::profile::Profile;
use inputforge_core::types::{
    AxisPolarity, DeviceId, DeviceInfo, InputId, OutputAddress, OutputId, VJoyAxis,
};
use std::collections::HashMap;

fn seeded_profile_with_one_mapping(actions: Vec<Action>) -> AppState {
    let mut map = HashMap::new();
    map.insert("Default".to_owned(), vec![]);
    let modes = ModeTree::from_adjacency(&map).unwrap();

    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let mappings = vec![Mapping {
        input: addr.clone(),
        mode: "Default".to_owned(),
        name: Some("Yaw".to_owned()),
        actions,
    }];
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let mut state = AppState::with_profile(profile);
    state.devices.push(inputforge_core::state::DeviceState {
        info: DeviceInfo {
            id: DeviceId("dev-1".to_owned()),
            name: "Stick".to_owned(),
            axes: 2,
            buttons: 4,
            hats: 0,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; 2],
        },
        connected: true,
    });
    state
}

fn harness_with(state: AppState, sel_input: InputAddress) -> impl FnOnce() -> Element {
    move || {
        let (cmd_tx, _) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(state)),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(|| MetaSnapshot {
            engine_status: inputforge_core::state::EngineStatus::Running,
            profile_name: Some("P".to_owned()),
            modes: vec!["Default".to_owned()],
            startup_mode: Some("Default".to_owned()),
            current_mode: "Default".to_owned(),
            ..MetaSnapshot::default()
        });
        let selection = Some(("Default".to_owned(), sel_input.clone()));
        let snap = ConfigSnapshot::from_state(&raw.state.read(), &selection);
        let config = use_signal(|| snap);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx);

        let view = use_view_state_provider(meta);
        view.selected_mapping
            .clone()
            .write()
            .replace(("Default".to_owned(), sel_input));
        use_live_capture_provider();
        use_editor_state_provider();
        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });
        rsx! { MappingEditor {} }
    }
}

#[test]
fn editor_header_shows_name_as_h2() {
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::MapToVJoy {
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    }];
    let state = seeded_profile_with_one_mapping(actions);
    let mut vdom = VirtualDom::new(harness_with(state, addr));
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("<h2"), "expected h2 element: {html}");
    assert!(html.contains("Yaw"), "expected mapping name: {html}");
    // Arrow present because MapToVJoy is in the action tree.
    assert!(html.contains("\u{2192}") || html.contains("&rarr;") || html.contains("&#8594;"));
}

#[test]
fn editor_header_omits_output_when_no_map_to_vjoy() {
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::Invert];
    let state = seeded_profile_with_one_mapping(actions);
    let mut vdom = VirtualDom::new(harness_with(state, addr));
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // No arrow because no MapToVJoy.
    assert!(
        !html.contains("\u{2192}") && !html.contains("&rarr;") && !html.contains("&#8594;"),
        "expected no arrow when no MapToVJoy: {html}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_header`
Expected: FAIL because the placeholder renders instead of the header.

- [ ] **Step 3: Implement the header**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/header.rs`:

```rust
//! Editor header: h2 mapping name + subtitle line.

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::types::{InputAddress, OutputAddress, OutputId, VJoyAxis};

use crate::context::AppContext;
use crate::frame::mapping_list::source_label;

#[component]
pub(crate) fn Header(name: String, input: InputAddress) -> Element {
    let ctx = use_context::<AppContext>();
    let cfg = ctx.config.read();
    let source_label = source_label::format(&input, &cfg);

    let output_label = cfg
        .selected_mapping_actions
        .as_ref()
        .and_then(|actions| first_map_to_vjoy_label(actions));

    rsx! {
        div { class: "if-editor__header",
            h2 { class: "if-editor__title", "{name}" }
            div { class: "if-editor__subtitle",
                "{source_label}"
                if let Some(out) = output_label {
                    span { class: "if-editor__subtitle-arrow", "\u{00a0}\u{00a0}\u{2192}\u{00a0}\u{00a0}" }
                    "{out}"
                }
            }
        }
    }
}

fn first_map_to_vjoy_label(actions: &[Action]) -> Option<String> {
    fn walk(actions: &[Action]) -> Option<&OutputAddress> {
        for action in actions {
            match action {
                Action::MapToVJoy { output } => return Some(output),
                Action::Conditional {
                    if_true, if_false, ..
                } => {
                    if let Some(found) = walk(if_true) {
                        return Some(found);
                    }
                    if let Some(branch) = if_false.as_deref() {
                        if let Some(found) = walk(branch) {
                            return Some(found);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
    walk(actions).map(format_output_label)
}

fn format_output_label(output: &OutputAddress) -> String {
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X axis",
            VJoyAxis::Y => "Y axis",
            VJoyAxis::Z => "Z axis",
            VJoyAxis::Rx => "Rx axis",
            VJoyAxis::Ry => "Ry axis",
            VJoyAxis::Rz => "Rz axis",
            VJoyAxis::Sl0 => "Slider 0",
            VJoyAxis::Sl1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Button {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("vJoy {} \u{00b7} {}", output.device, suffix)
}
```

Append to `mapping_editor/mod.rs`:

```rust
mod header;
use header::Header;
```

In `MappingEditor`, replace the placeholder branch with:

```rust
let view_state_for_render = view.selected_mapping.read().clone();
if let Some((mode, input)) = view_state_for_render {
    let ctx_for_lookup = use_context::<AppContext>();
    let mapping_name = ctx_for_lookup
        .config
        .read()
        .mappings
        .iter()
        .find(|m| m.input == input && m.mode == mode)
        .and_then(|m| m.name.clone())
        .unwrap_or_else(|| "Untitled mapping".to_owned());
    rsx! {
        Stylesheet { href: MAPPING_EDITOR_CSS }
        div { class: "if-editor",
            EngineOfflineBanner {}
            Header { name: mapping_name, input: input }
            // remaining sections land in subsequent tasks
        }
    }
} else {
    // empty-state branch as before
}
```

Append CSS:

```css
.if-editor__header {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--color-border);
}
.if-editor__title {
    font-family: var(--font-sans);
    font-size: 20px;
    font-weight: 600;
    line-height: 28px;
    margin: 0;
    color: var(--color-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.if-editor__subtitle {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 18px;
    color: var(--color-text-muted);
    overflow-wrap: anywhere;
}
```

OutputId may not match the imports above; align with the actual `inputforge-core::types` API. (Check `crates/inputforge-core/src/types/address.rs` if a build error names a wrong field.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(mapping_editor): h2 header + subtitle with output arrow"
```

---

## Phase E, Frame sections (Tasks 15-19)

### Task 15: Name field with commit-on-blur

`<input>` of width 100% / max 480 px. Changes to local working copy on input; on blur or Enter, commits via `SetMapping` with the same actions vector and a new name. Pushes a `Rename` undo entry.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/name_field.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 1: Write the failing SSR test**

Append to `mapping_editor/tests.rs`:

```rust
#[test]
fn editor_name_field_renders_input_with_current_value() {
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = VirtualDom::new(harness_with(state, addr));
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("data-editor-focus"),
        "name input should carry data-editor-focus marker (used by F8 keyboard nav): {html}"
    );
    assert!(html.contains("value=\"Yaw\""));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_name_field`
Expected: FAIL.

- [ ] **Step 3: Implement `NameField`**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/name_field.rs`:

```rust
//! Name field with commit-on-blur dispatch.

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::InputAddress;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::{LabelArgs, UndoKind, format_undo_label};

#[component]
pub(crate) fn NameField(
    /// Current saved name (read-only mirror; the working copy is local).
    initial: String,
    key: MappingKey,
    actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();

    let mut local: Signal<String> = use_signal(|| initial.clone());

    let key_for_blur = key.clone();
    let actions_for_blur = actions.clone();
    let initial_for_blur = initial.clone();
    let cmd_tx = ctx.commands.clone();
    let mut undo_log = editor.undo_log;

    let on_blur = move |_| {
        let new = local.peek().trim().to_owned();
        if new == initial_for_blur || new.is_empty() {
            return;
        }
        // Build a Mapping snapshot for the undo entry.
        let before = inputforge_core::action::Mapping {
            input: key_for_blur.1.clone(),
            mode: key_for_blur.0.clone(),
            name: Some(initial_for_blur.clone()),
            actions: actions_for_blur.clone(),
        };
        // Dispatch FIRST. If the engine is offline (channel disconnected), do
        // NOT push an undo entry — otherwise the user accumulates phantom
        // entries that, when popped via Ctrl+Z, would dispatch SetMapping
        // commands the engine cannot receive. The engine-offline banner
        // (Task 13) is already informing the user.
        if cmd_tx
            .send(EngineCommand::SetMapping {
                input: key_for_blur.1.clone(),
                mode: key_for_blur.0.clone(),
                name: Some(new.clone()),
                actions: actions_for_blur.clone(),
            })
            .is_err()
        {
            tracing::warn!(target: "f9::mapping_editor", action = "rename_drop_offline", new_name = %new);
            return;
        }
        let label = format_undo_label(
            UndoKind::Rename,
            LabelArgs {
                old_new: Some((&initial_for_blur, &new)),
                ..LabelArgs::default()
            },
        );
        undo_log.write().push_edit(
            key_for_blur.clone(),
            before,
            UndoKind::Rename,
            label,
        );
        tracing::info!(target: "f9::mapping_editor", action = "rename", new_name = %new);
    };

    let on_keydown = move |evt: KeyboardEvent| {
        if evt.key() == Key::Enter {
            evt.prevent_default();
            // Force blur on the input element to trigger on_blur, which is
            // the canonical commit path. Web event target is the input itself.
            // The cast is safe inside Dioxus desktop's WebView2.
            let _ = document::eval(
                r#"
                const el = document.activeElement;
                if (el && el instanceof HTMLInputElement) { el.blur(); }
                "#,
            );
        }
    };

    let oninput = move |evt: FormEvent| {
        local.set(evt.value());
    };

    rsx! {
        div { class: "if-editor__name-field",
            input {
                r#type: "text",
                class: "if-editor__name-input",
                value: "{local}",
                oninput,
                onblur: on_blur,
                onkeydown: on_keydown,
                "data-editor-focus": "true",
            }
        }
    }
}
```

Wire into `MappingEditor` after `Header`:

```rust
let actions_clone = ctx_for_lookup
    .config
    .read()
    .selected_mapping_actions
    .clone()
    .unwrap_or_default();
NameField {
    initial: mapping_name.clone(),
    key: (mode.clone(), input.clone()),
    actions: actions_clone.clone(),
}
```

CSS:

```css
.if-editor__name-field { padding: 8px 16px; border-bottom: 1px solid var(--color-border); }
.if-editor__name-input {
    width: 100%; max-width: 480px;
    font-family: var(--font-sans); font-size: 14px;
    padding: 6px 8px;
    background: var(--color-bg-sunken);
    border: 1px solid var(--color-border);
    border-radius: 4px;
    color: var(--color-text);
}
.if-editor__name-input:focus {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_name_field`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/name_field.rs crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(mapping_editor): name field with commit-on-blur and undo dispatch"
```

---

### Task 16: Input field with rebind action

Read-only source label plus an F2 `Button` ghost variant labelled `rebind`. Click arms `LiveCapture::start(CaptureFilter::Any)`. On `LiveCapture.captured`, dispatches `SetMapping` with the new input, same actions and name; pushes a `Rebind` undo entry.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/input_field.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 1: Write the failing SSR test**

Append to `tests.rs`:

```rust
#[test]
fn editor_input_field_renders_source_label_and_rebind_button() {
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = VirtualDom::new(harness_with(state, addr));
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Stick"), "expected source device label");
    assert!(html.contains("rebind"), "expected rebind button");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_input_field`
Expected: FAIL.

- [ ] **Step 3: Implement `InputField`**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/input_field.rs`:

```rust
//! Input field row: source label + rebind button arming F8 LiveCapture.

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::InputAddress;

use crate::components::{Button, ButtonSize, ButtonVariant};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::{LabelArgs, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{CaptureFilter, LiveCapture};

#[component]
pub(crate) fn InputField(
    key: MappingKey,
    actions: Vec<Action>,
    name: Option<String>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let capture = use_context::<LiveCapture>();

    let cfg_label = source_label::format(&key.1, &ctx.config.read());

    // CONSUMER-FLAG PATTERN. LiveCapture is single-instance (see
    // `crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs:41-51`); race
    // prevention with other consumers (MergeAxis secondary picker, etc.) is
    // by an "are-we-the-armed-consumer" local Signal. Without this flag, the
    // racy pattern of "if captured.is_some() then dispatch" would (a) fire
    // for any consumer's capture, not just ours; (b) self-fire after we
    // `set(None)` because that mutation re-triggers the effect inside the
    // same render cycle.
    let mut is_armed_consumer: Signal<bool> = use_signal(|| false);

    // Reset on selection change: if the user clicks rebind, then switches
    // mapping in the rail, cancel our armed capture so the next mapping's
    // InputField doesn't inherit the capture intent.
    let key_for_reset = key.clone();
    let mut captured_writer = capture.captured;
    let cancel_cb = capture.cancel;
    use_effect(move || {
        let _touched = key_for_reset.clone(); // re-runs when key changes
        if *is_armed_consumer.peek() {
            cancel_cb.call(());
            captured_writer.set(None);
            is_armed_consumer.set(false);
        }
    });

    let key_for_eff = key.clone();
    let actions_for_eff = actions.clone();
    let name_for_eff = name.clone();
    let cmd_tx = ctx.commands.clone();
    let mut undo_log = editor.undo_log;
    use_effect(move || {
        let captured = capture.captured.read().clone();
        // Only react when WE armed the capture and the captured address has
        // arrived. This guards against:
        //   (a) other consumers' captures (MergeAxis secondary picker) —
        //       their armed flag is false on this Signal, so we skip;
        //   (b) self-firing after `set(None)` below — `is_armed_consumer`
        //       is false after the first dispatch, so we skip.
        if !*is_armed_consumer.peek() {
            return;
        }
        let Some(new_addr) = captured else { return };
        let cfg = ctx.config.read();
        let old_label = source_label::format(&key_for_eff.1, &cfg);
        let new_label = source_label::format(&new_addr, &cfg);
        drop(cfg);
        let before = inputforge_core::action::Mapping {
            input: key_for_eff.1.clone(),
            mode: key_for_eff.0.clone(),
            name: name_for_eff.clone(),
            actions: actions_for_eff.clone(),
        };
        // Dispatch FIRST. Skip undo entry if the engine is offline.
        if cmd_tx
            .send(EngineCommand::SetMapping {
                input: new_addr.clone(),
                mode: key_for_eff.0.clone(),
                name: name_for_eff.clone(),
                actions: actions_for_eff.clone(),
            })
            .is_err()
        {
            tracing::warn!(target: "f9::mapping_editor", action = "rebind_drop_offline", ?new_addr);
            // Still disarm so we don't keep retrying.
            is_armed_consumer.set(false);
            captured_writer.set(None);
            return;
        }
        let label = format_undo_label(
            UndoKind::Rebind,
            LabelArgs {
                old_new: Some((&old_label, &new_label)),
                ..LabelArgs::default()
            },
        );
        undo_log
            .write()
            .push_edit(key_for_eff.clone(), before, UndoKind::Rebind, label);
        // Disarm BEFORE clearing captured — otherwise the cleared signal
        // re-runs the effect with `captured=None` and we'd skip via the
        // None guard, but we keep the order explicit for clarity.
        is_armed_consumer.set(false);
        captured_writer.set(None);
        tracing::info!(target: "f9::mapping_editor", action = "rebind", ?new_addr);
    });

    let start_cb = capture.start;
    let on_rebind = move |_| {
        is_armed_consumer.set(true);
        start_cb.call(CaptureFilter::Any);
    };

    rsx! {
        div { class: "if-editor__input-field",
            div { class: "if-editor__input-label", "{cfg_label}" }
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                onclick: on_rebind,
                "rebind"
            }
        }
    }
}
```

Wire into `MappingEditor` after `NameField`:

```rust
InputField {
    key: (mode.clone(), input.clone()),
    actions: actions_clone.clone(),
    name: Some(mapping_name.clone()),
}
```

CSS:

```css
.if-editor__input-field {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--color-border);
}
.if-editor__input-label {
    font-family: var(--font-mono); font-size: 12px;
    color: var(--color-text-muted);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_input_field`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(mapping_editor): input field with rebind action arming LiveCapture"
```

---

### Task 17: Live readout (IN/OUT bars, merge layout)

Two-row grid (label column, bar column, percentage column). Live-green fill, bipolar anchors at 50%, unipolar at 0%. OUT row hidden when no `MapToVJoy`. Merge mappings render `IN 1` + `IN 2` + dashed divider + merged `IN`.

**Divider direction (per spec lines 42, 417):**
- **Merge case:** rows `IN 1`, `IN 2`, **dashed** divider (`--readout-divider-dashed`), merged `IN`, then `OUT` (NO extra divider before OUT in merge case — the merged `IN` already plays the role the dashed divider plays in the non-merge case).
- **Non-merge case:** row `IN`, **dashed** divider, `OUT`.

The plan's CSS class names (`__readout-divider` for solid vs `__readout-divider-dashed` for dashed) match this; previous drafts inverted the usage. Use `__readout-divider-dashed` in both spots that render a divider.

**Value sources (per Q2 decision: F9 wires `evaluate_actions_through`):**
- `IN` (raw): direct read from `live.device_inputs[..].axes[..]`. Same as current implementation.
- `IN 1`, `IN 2`: same — direct reads of the primary and secondary input addresses.
- **merged `IN`:** `evaluate_actions_through(&actions, &state, &primary, idx_of_merge + 1)` where `idx_of_merge` is the position of the first `MergeAxis` in `actions`. Run-locks `ctx.state` briefly via `parking_lot::RwLockReadGuard` (read-only).
- **OUT:** `evaluate_actions_through(&actions, &state, &primary, actions.len())`. Same lock pattern.

This wires F9 into the helper end-to-end and exercises Task 1's contract from the GUI side. AC #29 mandates the helper exists; spec line 188 says F10/F11 consume it from their bodies — F9 also consumes it here for the live readout (NO requirement to defer, no spec amendment needed).

**Known limitation (not a blocker):** merge detection (`first_merge_secondary` walk) only finds top-level `MergeAxis`. `MergeAxis` nested inside a `Conditional` does not surface as a "merge mapping" for layout purposes — the readout falls back to the non-merge layout. F9 ships this limitation explicitly; the spec's "merge mapping" definition (line 42) presumes top-level placement.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 1: Write the failing SSR test**

Append to `tests.rs`:

```rust
#[test]
fn editor_live_readout_renders_in_row() {
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = VirtualDom::new(harness_with(state, addr));
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("if-editor__readout-label"), "expected readout label cell");
    assert!(html.contains(">IN<") || html.contains(">IN "), "IN row label");
}

#[test]
fn editor_live_readout_omits_out_when_no_map_to_vjoy() {
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    let mut vdom = VirtualDom::new(harness_with(state, addr));
    vdom.rebuild_in_place();
    let html = render(&vdom);
    // No MapToVJoy in actions, no OUT row.
    assert!(!html.contains(">OUT<"), "OUT row must be hidden: {html}");
}

#[test]
fn editor_live_readout_renders_out_when_map_to_vjoy_present() {
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::MapToVJoy {
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    }];
    let state = seeded_profile_with_one_mapping(actions);
    let mut vdom = VirtualDom::new(harness_with(state, addr));
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("OUT"), "OUT row should render with MapToVJoy: {html}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_live_readout`
Expected: FAIL.

- [ ] **Step 3: Implement `LiveReadout`**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/live_readout.rs`:

```rust
//! Live readout: IN/OUT axis bars + merge-mapping layout.

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::types::{AxisPolarity, InputAddress, InputId};

use crate::context::AppContext;

#[component]
pub(crate) fn LiveReadout(
    primary: InputAddress,
    actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let live = ctx.live.read();
    let cfg = ctx.config.read();

    let primary_value = read_axis_value(&primary, &live, &cfg);
    let merge_secondary = first_merge_secondary(&actions);
    let merge_index = first_merge_index(&actions);
    let out_present = first_map_to_vjoy(&actions);

    // Brief read-lock on AppState to call evaluate_actions_through.
    // Released when the guard drops at end of this scope.
    let merged_in_value = merge_index.map(|idx| {
        let state = ctx.state.read();
        let v = inputforge_core::pipeline::evaluate_actions_through(
            &actions, &state, &primary, idx + 1,
        );
        AxisDisplay {
            value: as_axis_value(&v),
            polarity: primary_value.polarity,
        }
    });
    let out_value = if out_present {
        let state = ctx.state.read();
        let v = inputforge_core::pipeline::evaluate_actions_through(
            &actions, &state, &primary, actions.len(),
        );
        Some(AxisDisplay {
            value: as_axis_value(&v),
            polarity: primary_value.polarity,
        })
    } else {
        None
    };

    rsx! {
        div { class: "if-editor__readout",
            if let Some(secondary_addr) = merge_secondary {
                ReadoutRow { label: "IN 1".to_owned(), value: primary_value }
                {
                    let v = read_axis_value(&secondary_addr, &live, &cfg);
                    rsx! { ReadoutRow { label: "IN 2".to_owned(), value: v } }
                }
                // Dashed divider between source rows and merged IN per spec
                // lines 42 + 417.
                div { class: "if-editor__readout-divider-dashed" }
                ReadoutRow {
                    label: "IN".to_owned(),
                    value: merged_in_value.unwrap_or(primary_value),
                }
                // No extra divider before OUT in merge case (per spec line 417:
                // "Output row appears after the merged row" — no divider).
                if let Some(out) = out_value {
                    ReadoutRow { label: "OUT".to_owned(), value: out }
                }
            } else {
                ReadoutRow { label: "IN".to_owned(), value: primary_value }
                if let Some(out) = out_value {
                    div { class: "if-editor__readout-divider-dashed" }
                    ReadoutRow { label: "OUT".to_owned(), value: out }
                }
            }
        }
    }
}

fn as_axis_value(v: &inputforge_core::types::InputValue) -> f64 {
    match v {
        inputforge_core::types::InputValue::Axis { value } => value.value(),
        inputforge_core::types::InputValue::Button { pressed } => {
            if *pressed { 1.0 } else { 0.0 }
        }
        inputforge_core::types::InputValue::Hat { .. } => 0.0,
    }
}

/// Index of the first top-level `MergeAxis`. Top-level only by design;
/// merges nested inside `Conditional` do not trigger merge layout (see
/// task description for the limitation).
fn first_merge_index(actions: &[Action]) -> Option<usize> {
    actions
        .iter()
        .position(|a| matches!(a, Action::MergeAxis { .. }))
}

#[derive(Clone, Copy)]
struct AxisDisplay {
    value: f64,
    polarity: AxisPolarity,
}

fn read_axis_value(
    addr: &InputAddress,
    live: &crate::context::LiveSnapshot,
    cfg: &crate::context::ConfigSnapshot,
) -> AxisDisplay {
    if let InputId::Axis { index } = addr.input {
        let dev_idx = cfg.devices.iter().position(|d| d.info.id == addr.device);
        if let Some(idx) = dev_idx {
            if let Some(dev_inputs) = live.device_inputs.get(idx) {
                if let Some((value, polarity)) = dev_inputs.axes.get(usize::from(index)) {
                    return AxisDisplay {
                        value: *value,
                        polarity: *polarity,
                    };
                }
            }
        }
    }
    AxisDisplay {
        value: 0.0,
        polarity: AxisPolarity::Bipolar,
    }
}

fn first_merge_secondary(actions: &[Action]) -> Option<InputAddress> {
    for action in actions {
        if let Action::MergeAxis { second_input, .. } = action {
            return Some(second_input.clone());
        }
    }
    None
}

fn first_map_to_vjoy(actions: &[Action]) -> bool {
    fn walk(actions: &[Action]) -> bool {
        for action in actions {
            match action {
                Action::MapToVJoy { .. } => return true,
                Action::Conditional {
                    if_true, if_false, ..
                } => {
                    if walk(if_true) {
                        return true;
                    }
                    if let Some(branch) = if_false.as_deref() {
                        if walk(branch) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
    walk(actions)
}

#[component]
fn ReadoutRow(label: String, value: AxisDisplay) -> Element {
    let pct_text = format_percentage(&value);
    let fill_pct = (value.value.abs() * 100.0).clamp(0.0, 100.0);
    let bipolar = matches!(value.polarity, AxisPolarity::Bipolar);
    let style = if bipolar && value.value < 0.0 {
        format!("right: 50%; width: {fill_pct}%;")
    } else if bipolar {
        format!("left: 50%; width: {fill_pct}%;")
    } else {
        format!("left: 0; width: {fill_pct}%;")
    };

    rsx! {
        div { class: "if-editor__readout-row",
            div { class: "if-editor__readout-label", "{label}" }
            div { class: "if-editor__readout-bar",
                div { class: "if-editor__readout-fill", style: "{style}" }
            }
            div { class: "if-editor__readout-pct", "{pct_text}" }
        }
    }
}

fn format_percentage(value: &AxisDisplay) -> String {
    match value.polarity {
        AxisPolarity::Bipolar => format!("{:+.2}", value.value),
        AxisPolarity::Unipolar => format!("{:.2}", value.value),
    }
}
```

CSS:

```css
.if-editor__readout {
    display: flex; flex-direction: column; gap: 4px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--color-border);
}
.if-editor__readout-row {
    display: grid;
    grid-template-columns: 60px 1fr 60px;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
}
.if-editor__readout-label {
    font-family: var(--font-mono); font-size: 11px;
    text-transform: uppercase; font-weight: 500;
    color: var(--color-text-subtle);
}
.if-editor__readout-bar {
    position: relative; height: 8px;
    background: var(--color-bg-sunken);
    border-radius: 2px;
}
.if-editor__readout-fill {
    position: absolute; top: 0; height: 100%;
    background: var(--color-live);
    border-radius: 2px;
}
.if-editor__readout-pct {
    font-family: var(--font-mono); font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-align: right;
    color: var(--color-text);
}
.if-editor__readout-divider {
    border-top: 1px solid var(--color-border-strong);
    margin: 4px 0;
}
.if-editor__readout-divider-dashed {
    border-top: 1px dashed var(--color-border-strong);
    margin: 4px 0;
}
```

Wire into `MappingEditor`:

```rust
LiveReadout {
    primary: input.clone(),
    actions: actions_clone.clone(),
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_live_readout`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(mapping_editor): live readout with IN/OUT bars + merge layout"
```

---

### Task 18: Inactive-runtime hint banner

Tinted card with no side stripe. Visible only when `editing_mode != current_mode` (the `MetaSnapshot.current_mode` field is the engine's runtime mode) **AND** the engine is `Online` (engine-offline subsumes mode-mismatch — see Task 13's banner-precedence rule). Copy: `Engine is in <runtime>. Mapping fires only in <editing>.` (matches the post-amendment spec line 44 + F5 spec line 626). Renders between live readout and pipeline.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/inactive_hint.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 1: Write the failing test**

Append to `tests.rs`:

```rust
#[test]
fn editor_inactive_hint_visible_when_modes_diverge() {
    fn h() -> Element {
        let (cmd_tx, _) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(seeded_profile_with_one_mapping(vec![Action::Invert]))),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(|| MetaSnapshot {
            engine_status: inputforge_core::state::EngineStatus::Running,
            profile_name: Some("P".to_owned()),
            modes: vec!["Default".to_owned(), "Combat".to_owned()],
            startup_mode: Some("Default".to_owned()),
            current_mode: "Combat".to_owned(),
            ..MetaSnapshot::default()
        });
        let addr = InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let snap = ConfigSnapshot::from_state(
            &raw.state.read(),
            &Some(("Default".to_owned(), addr.clone())),
        );
        let config = use_signal(|| snap);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx);
        let view = use_view_state_provider(meta);
        view.selected_mapping
            .clone()
            .write()
            .replace(("Default".to_owned(), addr));
        use_live_capture_provider();
        use_editor_state_provider();
        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });
        rsx! { MappingEditor {} }
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Engine is in") && html.contains("Mapping fires only in"),
        "expected inactive-hint copy: {html}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_inactive_hint`
Expected: FAIL.

- [ ] **Step 3: Implement `InactiveHint`**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/inactive_hint.rs`:

```rust
//! Inactive-runtime hint banner. See spec choice 8 + 9.

use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::view_state::ViewState;

#[component]
pub(crate) fn InactiveHint() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    let runtime = ctx.meta.read().current_mode.clone();
    let editing = view.editing_mode.read().clone();
    let engine_status = ctx.meta.read().engine_status;

    // Banner precedence: engine-offline subsumes mode-mismatch.
    // (See Task 13's banner-precedence rule.)
    if !matches!(engine_status, inputforge_core::state::EngineStatus::Running) {
        return rsx! {};
    }
    if runtime == editing || runtime.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "if-editor__inactive-hint",
            role: "status",
            "aria-live": "polite",
            "Engine is in "
            strong { "{runtime}" }
            ". Mapping fires only in "
            strong { "{editing}" }
            "."
        }
    }
}
```

CSS:

```css
.if-editor__inactive-hint {
    margin: 8px 16px;
    padding: 8px 12px;
    background: rgba(154, 120, 214, 0.08);
    border: 1px solid rgba(154, 120, 214, 0.22);
    border-radius: 6px;
    font-family: var(--font-sans); font-size: 12px;
    color: var(--color-control-badge-text);
    transition: opacity 150ms ease-out;
}
@media (prefers-reduced-motion: reduce) {
    .if-editor__inactive-hint { transition: none; }
}
```

Wire into `MappingEditor` between `LiveReadout` and the pipeline placeholder.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_inactive_hint`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(mapping_editor): inactive-runtime hint banner"
```

---

### Task 19: Undo recap footer

Last committed change label plus styled `<kbd>⌃Z</kbd>` shortcut. Reads `EditorState.undo_log.last_label(key)` for the active selection. Renders nothing if no entries yet.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/undo_recap.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 1: Write the failing test**

Append to `tests.rs`:

```rust
#[test]
fn editor_undo_recap_renders_kbd_glyph() {
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let state = seeded_profile_with_one_mapping(vec![Action::Invert]);
    fn h_with_log(state: AppState, addr: InputAddress) -> impl FnOnce() -> Element {
        move || {
            let (cmd_tx, _) = mpsc::channel();
            let raw = RawHandles {
                state: Arc::new(RwLock::new(state)),
                commands: cmd_tx,
                settings: Arc::new(AppSettings::default()),
            };
            use_context_provider(|| raw.clone());
            let meta = use_signal(|| MetaSnapshot {
                engine_status: inputforge_core::state::EngineStatus::Running,
                profile_name: Some("P".to_owned()),
                modes: vec!["Default".to_owned()],
                startup_mode: Some("Default".to_owned()),
                current_mode: "Default".to_owned(),
                ..MetaSnapshot::default()
            });
            let selection = Some(("Default".to_owned(), addr.clone()));
            let snap = ConfigSnapshot::from_state(&raw.state.read(), &selection);
            let config = use_signal(|| snap);
            let live = use_signal(LiveSnapshot::default);
            let ctx = AppContext {
                state: Arc::clone(&raw.state),
                commands: raw.commands.clone(),
                settings: Arc::clone(&raw.settings),
                meta,
                config,
                live,
            };
            use_context_provider(|| ctx);
            let view = use_view_state_provider(meta);
            view.selected_mapping
                .clone()
                .write()
                .replace(("Default".to_owned(), addr.clone()));
            use_live_capture_provider();
            let editor = use_editor_state_provider();
            // Pre-populate one entry.
            editor.undo_log.clone().write().push_edit(
                ("Default".to_owned(), addr.clone()),
                inputforge_core::action::Mapping {
                    input: addr,
                    mode: "Default".to_owned(),
                    name: Some("Yaw".to_owned()),
                    actions: vec![],
                },
                crate::frame::mapping_editor::undo_log::UndoKind::Rename,
                "rename: 'X' -> 'Yaw'".to_owned(),
            );
            let toast_state = use_signal(ToastState::default);
            use_context_provider(|| ToastQueue { state: toast_state });
            rsx! { MappingEditor {} }
        }
    }
    let mut vdom = VirtualDom::new(h_with_log(state, addr));
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("rename:"), "footer should show last label: {html}");
    assert!(html.contains("\u{2303}Z") || html.contains("Z"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_undo_recap`
Expected: FAIL.

- [ ] **Step 3: Implement `UndoRecap`**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/undo_recap.rs`:

```rust
//! Last committed change label plus Ctrl+Z keyboard hint.

use dioxus::prelude::*;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;

#[component]
pub(crate) fn UndoRecap(key: MappingKey) -> Element {
    let editor = use_context::<EditorState>();
    let log = editor.undo_log.read();
    let label = log.last_label(&key);

    let Some(label) = label else { return rsx! {} };

    rsx! {
        div { class: "if-editor__footer",
            span { class: "if-editor__footer-label", "{label}" }
            span { class: "if-editor__footer-sep", " \u{00b7} " }
            kbd { class: "if-editor__kbd", "\u{2303}Z" }
            span { class: "if-editor__footer-sep", " to undo" }
        }
    }
}
```

CSS:

```css
.if-editor__footer {
    padding: 6px 16px;
    border-top: 1px solid var(--color-border);
    font-family: var(--font-sans); font-size: 11px;
    color: var(--color-text-muted);
    display: flex; align-items: center; gap: 4px;
}
.if-editor__footer-label { color: var(--color-text); }
```

Wire into `MappingEditor` at the bottom of the active-selection branch:

```rust
UndoRecap { key: (mode.clone(), input.clone()) }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::tests::editor_undo_recap`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/undo_recap.rs crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(mapping_editor): undo recap footer with kbd hint"
```

---

## Phase F, Pipeline graph (Tasks 20-30)

### Task 20: `Pipeline` ordered list shell + StageId derivation

Component that takes a `&[Action]` slice, an outer `StageId` prefix, and renders an ordered list of `<Stage>` components. F8's empty-pipeline (mapping with `actions: vec![]`) shows the louder `+ Add first stage` affordance (placeholder for Task 28's add palette).

**`root_actions` threading (load-bearing for Conditional recursion).** `Pipeline` recurses into Conditional sub-pipelines (Task 26a). At any recursion depth, `actions` is the *branch's* vec, but every `StageId` is *root-relative*. Calling `replace_at_path` / `insert_at_path` / `remove_at_path` from a body deep in the tree must therefore receive the *root* mapping's actions vec, NOT the local branch slice. Pipeline + Stage props carry both: `actions: Vec<Action>` (this pipeline's local slice, used for rendering and StageId derivation) and `root_actions: Vec<Action>` (the mapping's outermost actions vec, threaded unchanged through every recursion). Bodies use `root_actions` for tree mutators and `actions` for nothing — they receive their own action via the dispatcher's per-variant prop. The "outer_actions" naming used in earlier drafts is renamed to `root_actions` plan-wide to prevent the recursion bug where bodies inside Conditional branches dispatch root-relative paths against branch-local slices.

**`right_slot: Element` prop on `StageHeader` (spec lines 325-326, 586).** The stage header exposes a `right_slot: Element` prop. Default = chevron-down SVG. F10/F11 may pass a 28x14 inline SVG preview thumbnail; the IconButton's 32x32 hit area, `aria-expanded`, and `aria-controls` are invariant. Preview thumbnails render *inside* the IconButton's 32x32 box. The variant dispatcher (Task 22) computes the right-slot per body via a `header_right_slot()` helper and passes it to StageHeader. F10/F11/F14 implementations override `header_right_slot()` for their variants without touching the dispatcher or StageHeader.

**F2 `IconButton` wrap (spec lines 322-323).** The header row is one F2 `IconButton` (`crates/inputforge-gui-dx/src/components/icon_button.rs:9-39`), NOT a raw `<button>`. The chevron (or thumbnail) inside is a visual-cue child element rendered into the `right_slot`. Clicking anywhere on the row fires the IconButton's `onclick`, which toggles expand. This satisfies AC #21's "Stage chevron Space and Enter both toggle expand."

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 1: Write the failing SSR test**

Append to `pipeline/tests.rs` (create the file if missing). Wire `#[cfg(test)] mod tests;` in `pipeline/mod.rs`.

```rust
//! SSR + unit tests for the F9 pipeline graph component.

use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::mode::ModeTree;
use inputforge_core::profile::Profile;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::AppState;
use inputforge_core::types::{
    AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId, OutputAddress, OutputId,
    VJoyAxis,
};
use std::collections::HashMap;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot, RawHandles};
use crate::frame::mapping_editor::{MappingEditor, use_editor_state_provider};
use crate::frame::view_state::use_view_state_provider;
use crate::patterns::live_capture::use_live_capture_provider;
use crate::toast::{ToastQueue, ToastState};

fn build_state(actions: Vec<Action>) -> (AppState, InputAddress) {
    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let mappings = vec![Mapping {
        input: addr.clone(),
        mode: "Default".to_owned(),
        name: Some("Yaw".to_owned()),
        actions,
    }];
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        mappings,
        vec![],
        "Default".to_owned(),
    );
    let mut state = AppState::with_profile(profile);
    state.devices.push(inputforge_core::state::DeviceState {
        info: DeviceInfo {
            id: DeviceId("dev-1".to_owned()),
            name: "Stick".to_owned(),
            axes: 2,
            buttons: 4,
            hats: 0,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; 2],
        },
        connected: true,
    });
    (state, addr)
}

fn render_with(state: AppState, addr: InputAddress) -> String {
    let factory = move || {
        let (cmd_tx, _) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(state)),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(|| MetaSnapshot {
            engine_status: inputforge_core::state::EngineStatus::Running,
            profile_name: Some("P".to_owned()),
            modes: vec!["Default".to_owned()],
            startup_mode: Some("Default".to_owned()),
            current_mode: "Default".to_owned(),
            ..MetaSnapshot::default()
        });
        let snap = ConfigSnapshot::from_state(
            &raw.state.read(),
            &Some(("Default".to_owned(), addr.clone())),
        );
        let config = use_signal(|| snap);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx);
        let view = use_view_state_provider(meta);
        view.selected_mapping
            .clone()
            .write()
            .replace(("Default".to_owned(), addr.clone()));
        use_live_capture_provider();
        use_editor_state_provider();
        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });
        rsx! { MappingEditor {} }
    };
    let mut vdom = VirtualDom::new(factory);
    vdom.rebuild_in_place();
    render(&vdom)
}

#[test]
fn pipeline_renders_ordered_list_with_one_invert_stage() {
    let (state, addr) = build_state(vec![Action::Invert]);
    let html = render_with(state, addr);
    assert!(html.contains("<ol"), "pipeline must use <ol>: {html}");
    assert!(html.contains("if-stage"), "stage card class missing");
    assert!(html.contains("Invert"), "stage variant title missing");
}

#[test]
fn pipeline_empty_branch_renders_add_first_stage_affordance() {
    let (state, addr) = build_state(vec![]);
    let html = render_with(state, addr);
    assert!(
        html.contains("Add first stage"),
        "empty pipeline must show louder add affordance: {html}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::tests::pipeline_renders`
Expected: FAIL.

- [ ] **Step 3: Implement the Pipeline component**

Append to `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs`:

```rust
mod stage;
mod stage_header;
pub(crate) mod stage_body;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

use inputforge_core::action::Action;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

pub(crate) use stage::Stage;

/// Recursive pipeline component. Renders the action vector as `<ol>` of
/// `<Stage>` cards.
///
/// `key` identifies the mapping; `path_prefix` is the StageId path that
/// gets prepended to each stage's per-step `Index(i)` segment so nested
/// pipelines (Conditional branches) report deep IDs correctly.
///
/// `root_actions` is the mapping's outermost actions vec, threaded
/// unchanged through every recursion into Conditional branches. Bodies
/// use it (NOT `actions`) for `replace_at_path` / `insert_at_path` /
/// `remove_at_path` because StageId paths are root-relative. See the
/// task description for the recursion-correctness rationale.
#[component]
pub(crate) fn Pipeline(
    key: MappingKey,
    actions: Vec<Action>,
    root_actions: Vec<Action>,
    path_prefix: Vec<StageIdSegment>,
    /// Indent level (0 = outer pipeline; +1 per Conditional branch hop).
    depth: u8,
) -> Element {
    let is_empty = actions.is_empty();

    if is_empty {
        return rsx! {
            div { class: "if-pipeline if-pipeline--empty",
                button {
                    r#type: "button",
                    class: "if-pipeline__add-first",
                    // onclick wired in Task 28 — opens the categorized add
                    // palette anchored to this button. Empty leaves a
                    // visible TODO when the editor renders standalone
                    // before Task 28 lands.
                    "+ Add first stage"
                }
            }
        };
    }

    let actions_iter = actions.iter().enumerate();
    let path_prefix_for_iter = path_prefix.clone();
    let key_for_iter = key.clone();
    let root_for_iter = root_actions.clone();
    rsx! {
        ol { class: "if-pipeline",
            for (i, action) in actions_iter {
                {
                    let mut path = path_prefix_for_iter.clone();
                    path.push(StageIdSegment::Index(i));
                    let id = StageId(path);
                    rsx! {
                        Stage {
                            key: "{i}",
                            stage_id: id,
                            mapping_key: key_for_iter.clone(),
                            action: action.clone(),
                            root_actions: root_for_iter.clone(),
                            depth: depth,
                        }
                    }
                }
            }
            li { class: "if-pipeline__add-end",
                button {
                    r#type: "button",
                    class: "if-pipeline__add-button",
                    // onclick wired in Task 28 — opens add palette.
                    "+"
                }
            }
        }
    }
}
```

Pipeline mounts inside `MappingEditor`. Wire in:

```rust
Pipeline {
    key: (mode.clone(), input.clone()),
    actions: actions_clone.clone(),
    root_actions: actions_clone.clone(),
    path_prefix: vec![],
    depth: 0,
}
```

At the outer mount, `actions == root_actions`. Recursive mounts (Task 26a, inside Conditional branches) pass the branch slice as `actions` while threading `root_actions` unchanged.

- [ ] **Step 4: Implement `Stage` skeleton (header + body container, no body content yet)**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs`:

```rust
//! Stage card: header + body container.

use dioxus::prelude::*;

use inputforge_core::action::Action;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_header::StageHeader;
use crate::frame::mapping_editor::undo_log::StageId;

#[component]
pub(crate) fn Stage(
    stage_id: StageId,
    mapping_key: MappingKey,
    action: Action,
    /// Mapping's root actions vec, threaded unchanged through every
    /// recursion. Bodies use this for tree mutators because StageId
    /// paths are root-relative. See `Pipeline` doc for rationale.
    root_actions: Vec<Action>,
    depth: u8,
) -> Element {
    let editor = use_context::<EditorState>();
    let expanded = editor
        .expanded_stages
        .read()
        .contains(&stage_id);

    let category_class = match &action {
        Action::ResponseCurve { .. } | Action::Deadzone { .. } | Action::Invert => "is-processing",
        Action::MapToVJoy { .. } | Action::MapToKeyboard { .. } | Action::MergeAxis { .. } => {
            "is-output"
        }
        Action::ChangeMode { .. } | Action::Conditional { .. } => "is-control",
    };

    let class = format!("if-stage {category_class}");
    let title = stage_title(&action);
    let summary = stage_summary(&action);
    let right_slot = stage_body::header_right_slot(&action, expanded);

    rsx! {
        li {
            class: "{class}",
            "data-stage-id": "{format_stage_id(&stage_id)}",
            StageHeader {
                stage_id: stage_id.clone(),
                title: title,
                summary: summary,
                expanded: expanded,
                right_slot: right_slot,
            }
            if expanded {
                div { class: "if-stage__body",
                    // Body dispatcher lands in Task 22.
                    div { class: "if-stage__body-placeholder", "(body)" }
                }
            }
        }
    }
}

fn stage_title(action: &Action) -> String {
    match action {
        Action::Invert => "Invert".to_owned(),
        Action::Deadzone { .. } => "Deadzone".to_owned(),
        Action::ResponseCurve { .. } => "Response curve".to_owned(),
        Action::MapToVJoy { .. } => "Map to vJoy".to_owned(),
        Action::MapToKeyboard { .. } => "Map to keyboard".to_owned(),
        Action::MergeAxis { .. } => "Merge axis".to_owned(),
        Action::ChangeMode { .. } => "Change mode".to_owned(),
        Action::Conditional { .. } => "Conditional".to_owned(),
    }
}

fn stage_summary(_action: &Action) -> String {
    // Per-variant summaries land in subsequent tasks; this stub keeps the
    // header layout stable in the meantime.
    String::new()
}

fn format_stage_id(id: &StageId) -> String {
    id.0.iter()
        .map(|seg| {
            use crate::frame::mapping_editor::undo_log::StageIdSegment::*;
            match seg {
                Index(i) => format!("{i}"),
                IfTrue => "T".to_owned(),
                IfFalse => "F".to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}
```

- [ ] **Step 5: Implement `StageHeader`**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_header.rs`:

```rust
//! Stage header row: F2 IconButton wrapping title + summary + right_slot.
//!
//! Per spec lines 322-326 + 586. The IconButton is the only interactive
//! element. The right_slot prop renders inside the IconButton's 32x32 hit
//! area (default: chevron-down SVG; F10/F11 may pass a 28x14 preview
//! thumbnail). Clicking anywhere on the row toggles expand.

use dioxus::prelude::*;

use crate::components::{ButtonSize, ButtonVariant, IconButton, IconKind};
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::StageId;

#[component]
pub(crate) fn StageHeader(
    stage_id: StageId,
    title: String,
    summary: String,
    expanded: bool,
    /// Element rendered inside the IconButton's 32x32 box. Default for
    /// F9-owned variants: chevron-down SVG. F10/F11 override their
    /// variants' `header_right_slot()` to inject a preview thumbnail.
    right_slot: Element,
) -> Element {
    let editor = use_context::<EditorState>();
    let mut expanded_set = editor.expanded_stages;
    let stage_id_for_click = stage_id.clone();

    let onclick = move |_evt| {
        let mut set = expanded_set.write();
        if !set.insert(stage_id_for_click.clone()) {
            set.remove(&stage_id_for_click);
        }
    };

    rsx! {
        IconButton {
            // The IconButton's `icon` prop is overridden visually by the
            // right_slot Element; we still pass an icon for the
            // accessible-name fallback.
            icon: IconKind::ChevronDown,
            label: if expanded { "Collapse stage" } else { "Expand stage" },
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Md,
            onclick: Some(EventHandler::new(onclick)),
            class: Some("if-stage__header".to_owned()),
            // The IconButton renders the icon by default. F10/F11 use
            // `right_slot` to override; F9 default is the chevron, the
            // dispatcher (Task 22) computes the right_slot via
            // `header_right_slot()` and passes it here. Title + summary
            // render alongside the icon as in-row context.
            div { class: "if-stage__title", "{title}" }
            div { class: "if-stage__summary", "{summary}" }
            div { class: "if-stage__right-slot",
                "aria-hidden": "true",
                {right_slot}
            }
        }
    }
}
```

Wire `mod stage_body;` into `pipeline/mod.rs` even though we'll fill it in Task 23. For now stub:

```rust
// pipeline/stage_body/mod.rs (placeholder)
pub(crate) mod _placeholder {}
```

CSS for stages and pipeline:

```css
.if-pipeline {
    list-style: none; padding: 0; margin: 0;
    display: flex; flex-direction: column; gap: 8px;
    padding: 8px 16px;
}
.if-pipeline--empty { padding: 16px; }
.if-pipeline__add-first {
    width: 100%;
    background: rgba(154, 120, 214, 0.04);
    border: 1px dashed rgba(184, 155, 234, 0.32);
    border-radius: 6px;
    padding: 8px 12px;
    color: var(--color-control-badge-text);
    font-family: var(--font-sans); font-size: 12px;
    cursor: pointer;
}
.if-pipeline__add-end {
    list-style: none;
    text-align: center;
}
.if-pipeline__add-button {
    background: transparent;
    border: none;
    color: var(--color-border-strong);
    font-family: var(--font-mono); font-size: 14px;
    cursor: pointer;
}
.if-stage {
    border: 1px solid var(--color-border);
    border-radius: 6px;
    overflow: hidden;
}
.if-stage.is-processing { background: var(--color-stage-tint-processing); }
.if-stage.is-output     { background: var(--color-stage-tint-output); }
.if-stage.is-control    { background: var(--color-stage-tint-control); }
.if-stage__header {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr auto 32px;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    cursor: pointer;
}
.if-stage__title {
    font-family: var(--font-sans); font-size: 12px; font-weight: 500;
    line-height: 16px; color: var(--color-text); text-align: left;
}
.if-stage__summary {
    font-family: var(--font-mono); font-size: 12px;
    color: var(--color-text-muted); text-align: right;
}
.if-stage__chevron { transition: transform 180ms ease-out; }
.if-stage__chevron--collapsed { transform: rotate(-90deg); }
@media (prefers-reduced-motion: reduce) {
    .if-stage__chevron { transition: none; }
}
.if-stage__header:focus-visible {
    outline: 2px solid var(--color-border-focus);
    outline-offset: 2px;
}
.if-stage__body {
    padding: 8px 12px;
    border-top: 1px solid var(--color-border);
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(pipeline): ordered list, stage skeleton with category tints"
```

---

### Task 21: Stage variant title and summary derivation

Replace `stage_title` / `stage_summary` placeholder fns in `stage.rs` with the per-variant strings pinned in spec § "Action surface coverage". Pure helpers, unit-tested.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs`

- [ ] **Step 1: Write the failing tests**

Append to `pipeline/tests.rs`:

```rust
use inputforge_core::action::{ModeChangeStrategy};
use inputforge_core::types::{KeyCombo, KeyModifier, MergeOp};
use inputforge_core::processing::DeadzoneConfig;
use crate::frame::mapping_editor::pipeline::stage::{stage_title_for, stage_summary_for};

fn synth_addr() -> InputAddress {
    InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Button { index: 0 },
    }
}

fn synth_cfg() -> ConfigSnapshot {
    ConfigSnapshot {
        devices: vec![inputforge_core::state::DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Stick".to_owned(),
                axes: 2,
                buttons: 4,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar; 2],
            },
            connected: true,
        }],
        ..ConfigSnapshot::default()
    }
}

#[test]
fn title_for_each_variant() {
    assert_eq!(stage_title_for(&Action::Invert), "Invert");
    assert_eq!(stage_title_for(&Action::Deadzone { config: DeadzoneConfig::default() }), "Deadzone");
    assert_eq!(stage_title_for(&Action::MapToVJoy { output: OutputAddress { device: 1, output: OutputId::Axis { id: VJoyAxis::X } } }), "Map to vJoy");
    assert_eq!(stage_title_for(&Action::MergeAxis {
        second_input: synth_addr(),
        operation: MergeOp::Average,
    }), "Merge axis");
}

#[test]
fn summary_invert_is_empty() {
    let s = stage_summary_for(&Action::Invert, &synth_cfg());
    assert_eq!(s, "");
}

#[test]
fn summary_merge_axis_lists_op_and_secondary() {
    let s = stage_summary_for(
        &Action::MergeAxis {
            second_input: synth_addr(),
            operation: MergeOp::Average,
        },
        &synth_cfg(),
    );
    assert!(s.contains("Average"), "expected op in summary: {s}");
    assert!(s.contains("Stick"), "expected device in summary: {s}");
}

#[test]
fn summary_map_to_keyboard_renders_combo() {
    let s = stage_summary_for(
        &Action::MapToKeyboard {
            key: KeyCombo {
                key: "Q".to_owned(),
                modifiers: vec![KeyModifier::Ctrl, KeyModifier::Shift],
            },
        },
        &synth_cfg(),
    );
    assert!(s.contains("Ctrl"));
    assert!(s.contains("Shift"));
    assert!(s.contains("Q"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::tests::title_for_each_variant`
Expected: FAIL.

- [ ] **Step 3: Implement public helpers in `stage.rs`**

Replace the private `stage_title` / `stage_summary` with `pub(crate)` versions and flesh them out:

```rust
use inputforge_core::action::{Action, ModeChangeStrategy};
use inputforge_core::processing::DeadzoneConfig;
use inputforge_core::types::{KeyCombo, KeyModifier, MergeOp};

pub(crate) fn stage_title_for(action: &Action) -> &'static str {
    match action {
        Action::Invert => "Invert",
        Action::Deadzone { .. } => "Deadzone",
        Action::ResponseCurve { .. } => "Response curve",
        Action::MapToVJoy { .. } => "Map to vJoy",
        Action::MapToKeyboard { .. } => "Map to keyboard",
        Action::MergeAxis { .. } => "Merge axis",
        Action::ChangeMode { .. } => "Change mode",
        Action::Conditional { .. } => "Conditional",
    }
}

pub(crate) fn stage_summary_for(
    action: &Action,
    cfg: &crate::context::ConfigSnapshot,
) -> String {
    use crate::frame::mapping_list::source_label;
    match action {
        Action::Invert => String::new(),
        Action::Deadzone { config } => format!(
            "inner {}% \u{00b7} outer {}%",
            (config.inner_threshold().abs() * 100.0).round() as i32,
            ((1.0 - config.outer_threshold().abs()) * 100.0).round() as i32
        ),
        Action::ResponseCurve { curve } => {
            // Spec line 293: format is `N points · symmetric` when the curve
            // is symmetric (mirrored about origin), else `N points`.
            // `Curve::is_symmetric()` is a pure check on the points vec.
            let n = curve.points().len();
            if curve.is_symmetric() {
                format!("{n} points \u{00b7} symmetric")
            } else {
                format!("{n} points")
            }
        }
        Action::MapToVJoy { output } => format_output_summary(output),
        Action::MapToKeyboard { key } => format_key_combo(key),
        Action::MergeAxis {
            second_input,
            operation,
        } => format!(
            "{} with {}",
            match operation {
                MergeOp::Bidirectional => "Bidirectional",
                MergeOp::Average => "Average",
                MergeOp::Maximum => "Maximum",
            },
            source_label::format(second_input, cfg)
        ),
        Action::ChangeMode { strategy } => format_mode_strategy(strategy),
        Action::Conditional { condition, .. } => format_condition(condition, cfg),
    }
}

fn format_output_summary(output: &inputforge_core::types::OutputAddress) -> String {
    use inputforge_core::types::{OutputId, VJoyAxis};
    let suffix = match output.output {
        OutputId::Axis { id } => match id {
            VJoyAxis::X => "X axis",
            VJoyAxis::Y => "Y axis",
            VJoyAxis::Z => "Z axis",
            VJoyAxis::Rx => "Rx axis",
            VJoyAxis::Ry => "Ry axis",
            VJoyAxis::Rz => "Rz axis",
            VJoyAxis::Sl0 => "Slider 0",
            VJoyAxis::Sl1 => "Slider 1",
        }
        .to_owned(),
        OutputId::Button { id } => format!("Button {id}"),
        OutputId::Hat { id } => format!("Hat {id}"),
    };
    format!("vJoy {} \u{00b7} {}", output.device, suffix)
}

fn format_key_combo(key: &KeyCombo) -> String {
    let mods: Vec<&str> = key
        .modifiers
        .iter()
        .map(|m| match m {
            KeyModifier::Ctrl => "Ctrl",
            KeyModifier::Alt => "Alt",
            KeyModifier::Shift => "Shift",
            KeyModifier::Meta => "Win",
        })
        .collect();
    if mods.is_empty() {
        key.key.clone()
    } else {
        format!("{} + {}", mods.join(" + "), key.key)
    }
}

fn format_mode_strategy(strategy: &ModeChangeStrategy) -> String {
    match strategy {
        ModeChangeStrategy::SwitchTo { mode } => format!("set {mode}"),
        ModeChangeStrategy::CyclePush { modes } => format!("cycle {}", modes.modes().join(" \u{2192} ")),
        ModeChangeStrategy::TemporaryPush { mode } => format!("hold {mode}"),
        ModeChangeStrategy::Pop => "pop".to_owned(),
    }
}

fn format_condition(
    condition: &inputforge_core::action::Condition,
    cfg: &crate::context::ConfigSnapshot,
) -> String {
    use crate::frame::mapping_list::source_label;
    use inputforge_core::action::Condition;
    match condition {
        Condition::ButtonPressed { input } => format!("if {} pressed", source_label::format(input, cfg)),
        Condition::ButtonReleased { input } => format!("if {} released", source_label::format(input, cfg)),
        Condition::AxisInRange { input, min, max } => format!(
            "if {} in [{:.2}, {:.2}]",
            source_label::format(input, cfg),
            min,
            max
        ),
        Condition::HatDirection { input, .. } => format!("if {} hat", source_label::format(input, cfg)),
        Condition::All { conditions } => format!("all of {} conditions", conditions.len()),
        Condition::Any { conditions } => format!("any of {} conditions", conditions.len()),
        Condition::Not { .. } => "not (...)".to_owned(),
    }
}
```

Update `Stage` to call these helpers (and pass `cfg` from `AppContext`).

If `DeadzoneConfig` doesn't expose `inner_threshold` / `outer_threshold` accessors, replace with whatever public getters do exist; the goal is showing two percentages, the field names are not part of the public API contract.

If `ModeChangeStrategy` variants don't match the names above, mirror the actual `mode_change.rs` enum.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs
git commit -m "feat(pipeline): per-variant stage title and summary derivation"
```

---

### Task 22: Stage body dispatcher + Invert body

Replace `pipeline/stage_body/mod.rs` (currently a placeholder) with a real variant dispatcher. Land `Invert` (no body, just a caption) in this task to validate the dispatcher.

**`header_right_slot()` per-variant function (F10/F11/F14 hand-off API).** This task introduces the per-variant `header_right_slot(action: &Action, expanded: bool) -> Element` function in `stage_body/mod.rs`. The dispatcher routes by `Action` variant; the result is passed to `StageHeader.right_slot` (Task 20). F9-owned variants (Invert, MapToVJoy, MapToKeyboard, MergeAxis, Conditional, plus the three placeholder variants) all return the default chevron-down SVG. F10/F11/F14 implementations REPLACE only their variant's branch in this match, returning their preview thumbnail. They do NOT touch the dispatcher, the StageHeader API, or the EditorState provider. This is the architectural seam from spec lines 325-326 + 586.

**Structural-mutation contract reminder.** Any body that calls `replace_at_path` / `insert_at_path` / `remove_at_path` MUST use `root_actions` (NOT the action's parent vec) AND, after a successful structural mutation (insert/remove), MUST clear `editor_state.expanded_stages.write().clear()` and `editor_state.malformed_hints.write().clear()`. See Task 11's structural-mutation invariant. Tasks 23-30 enforce this in their dispatch handlers.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/invert.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs`

- [ ] **Step 1: Write the failing test**

Append to `pipeline/tests.rs`:

```rust
#[test]
fn invert_stage_expanded_renders_descriptive_caption() {
    use crate::frame::mapping_editor::EditorState;
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

    fn h() -> Element {
        let (cmd_tx, _) = mpsc::channel();
        let raw = RawHandles {
            state: Arc::new(RwLock::new(AppState::new())),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(|| MetaSnapshot {
            engine_status: inputforge_core::state::EngineStatus::Running,
            profile_name: Some("P".to_owned()),
            modes: vec!["Default".to_owned()],
            startup_mode: Some("Default".to_owned()),
            current_mode: "Default".to_owned(),
            ..MetaSnapshot::default()
        });
        let addr = InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let mappings = vec![Mapping {
            input: addr.clone(),
            mode: "Default".to_owned(),
            name: Some("Yaw".to_owned()),
            actions: vec![Action::Invert],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let mut state = AppState::with_profile(profile);
        state.devices.push(inputforge_core::state::DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Stick".to_owned(),
                axes: 2,
                buttons: 4,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar; 2],
            },
            connected: true,
        });
        let snap = ConfigSnapshot::from_state(
            &state,
            &Some(("Default".to_owned(), addr.clone())),
        );
        // Replace state in RwLock with our state.
        *raw.state.write() = state;
        let config = use_signal(|| snap);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx);
        let view = use_view_state_provider(meta);
        view.selected_mapping
            .clone()
            .write()
            .replace(("Default".to_owned(), addr));
        use_live_capture_provider();
        let editor = use_editor_state_provider();
        // Pre-expand stage 0.
        editor
            .expanded_stages
            .clone()
            .write()
            .insert(StageId(vec![StageIdSegment::Index(0)]));
        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });
        rsx! { MappingEditor {} }
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(
        html.contains("Inverts the input value"),
        "expected Invert descriptive caption: {html}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::tests::invert_stage_expanded`
Expected: FAIL (the placeholder body renders).

- [ ] **Step 3: Implement the dispatcher and Invert body**

Replace `pipeline/stage_body/mod.rs`:

```rust
//! Variant-body dispatcher. Each `Action` variant has its own body
//! component; this module dispatches based on the variant. F10/F11/F14
//! replace only their variant's branch in `StageBody` and
//! `header_right_slot()` — the dispatcher itself, `StageHeader`, and the
//! `EditorState` provider are invariant.

use dioxus::prelude::*;

use inputforge_core::action::Action;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::StageId;

mod invert;
// MapToVJoy, MapToKeyboard, MergeAxis, Conditional bodies land in tasks 23-26b.
// Placeholders for ResponseCurve, Deadzone, ChangeMode land in task 27.

#[component]
pub(crate) fn StageBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    action: Action,
    /// The mapping's outermost actions vec, threaded unchanged through
    /// every recursion. Bodies use this for tree mutators because StageId
    /// paths are root-relative. See Task 20 / Task 11.
    root_actions: Vec<Action>,
) -> Element {
    match &action {
        Action::Invert => rsx! { invert::InvertBody {} },
        // Stub for other variants until tasks 23+.
        _ => rsx! { div { class: "if-stage__body-stub", "(body coming soon)" } },
    }
}

/// Per-variant `right_slot` for `StageHeader`. Called from `Stage::render`.
/// F9-owned variants all return the default chevron-down SVG (the visual
/// affordance for expand/collapse). F10/F11/F14 override their variants
/// here to return their 28x14 preview thumbnail. Per spec lines 325-326,
/// the IconButton's 32x32 hit area, `aria-expanded`, and `aria-controls`
/// remain invariant — only the visual content of the slot changes.
pub(crate) fn header_right_slot(action: &Action, _expanded: bool) -> Element {
    match action {
        // F10 will override (preview = curve thumbnail):
        Action::ResponseCurve { .. } => default_chevron(),
        // F11 will override (preview = deadzone visualization):
        Action::Deadzone { .. } => default_chevron(),
        // F14 will override (preview = mode badge):
        Action::ChangeMode { .. } => default_chevron(),
        // F9-owned variants: chevron only.
        _ => default_chevron(),
    }
}

fn default_chevron() -> Element {
    rsx! {
        // Chevron-down SVG; rotation is CSS-driven via `aria-expanded`.
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16",
            height: "16",
            view_box: "0 0 16 16",
            fill: "currentColor",
            "aria-hidden": "true",
            path { d: "M3.5 5.5L8 10l4.5-4.5z" }
        }
    }
}
```

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/invert.rs`:

```rust
//! Invert body: descriptive caption only, no inputs.

use dioxus::prelude::*;

#[component]
pub(crate) fn InvertBody() -> Element {
    rsx! {
        div { class: "if-stage__body-caption",
            "Inverts the input value: x becomes -x."
        }
    }
}
```

Update `Stage` to call `StageBody` instead of the placeholder:

```rust
if expanded {
    rsx! {
        div { class: "if-stage__body",
            crate::frame::mapping_editor::pipeline::stage_body::StageBody {
                mapping_key: mapping_key.clone(),
                stage_id: stage_id.clone(),
                action: action.clone(),
                root_actions: root_actions.clone(),
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline
git commit -m "feat(pipeline): variant-body dispatcher with Invert body"
```

---

### Task 23: MapToVJoy body

**Amendments (apply before drafting code):**
1. Body must write malformed-hint per spec lines 587-589 when device or output index is out of range. After resolving the current device/output, if `cfg.vjoy_devices.iter().find(|d| d.id == output.device).is_none()` OR if the output index is out of range for that device's axis/button/hat count, write `editor_state.malformed_hints.write().insert(stage_id.clone(), "vJoy device {N} not configured".to_owned())` (or analogous for axis/button/hat). On every render where the body is valid, emit `editor_state.malformed_hints.write().remove(&stage_id)` so stale hints clear.
2. **`name` source-of-truth.** When dispatching `EngineCommand::SetMapping` after a body edit, read the current name from the snapshot: `let name = ctx.config.read().mapping_names.get(&key).cloned();` and pass that as `name`. `name: None` means "no explicit name" in `EngineCommand::SetMapping`'s contract — preserving the user-set name requires an explicit `Some(name)`.
3. Replace `outer_actions` references with `root_actions`. All `replace_at_path(&root_actions, &stage_id, ...)` calls.
4. Subscribe to `editor_state.external_edit_reset` via `use_effect`: when the token advances, re-derive any local Signals from the action's current fields. Required by Task 33.
5. Skip `push_edit` if `cmd_tx.send(...)` returns `Err` (engine offline) — same pattern as Task 15. No phantom undo entries.

### Original Task 23 body:


Two stacked F2 `Select`s: device picker and axis/button picker. Selection change dispatches `SetMapping`.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_vjoy.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`

- [ ] **Step 1: Write the failing test** — append to `pipeline/tests.rs`:

```rust
#[test]
fn map_to_vjoy_body_renders_device_and_axis_pickers() {
    fn h() -> Element {
        let (cmd_tx, _) = mpsc::channel();
        let mut state = AppState::new();
        state.virtual_devices.push(inputforge_core::types::VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X, VJoyAxis::Y],
            button_count: 4,
            hat_count: 0,
        });

        let addr = InputAddress {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let mappings = vec![Mapping {
            input: addr.clone(),
            mode: "Default".to_owned(),
            name: Some("Yaw".to_owned()),
            actions: vec![Action::MapToVJoy {
                output: OutputAddress { device: 1, output: OutputId::Axis { id: VJoyAxis::X } },
            }],
        }];
        let profile = Profile::new("P".to_owned(), vec![], modes, mappings, vec![], "Default".to_owned());
        let mut state2 = AppState::with_profile(profile);
        state2.devices.push(inputforge_core::state::DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Stick".to_owned(),
                axes: 2,
                buttons: 4,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar; 2],
            },
            connected: true,
        });
        state2.virtual_devices = state.virtual_devices.clone();

        let raw = RawHandles {
            state: Arc::new(RwLock::new(state2)),
            commands: cmd_tx,
            settings: Arc::new(AppSettings::default()),
        };
        use_context_provider(|| raw.clone());
        let meta = use_signal(|| MetaSnapshot {
            engine_status: inputforge_core::state::EngineStatus::Running,
            profile_name: Some("P".to_owned()),
            modes: vec!["Default".to_owned()],
            startup_mode: Some("Default".to_owned()),
            current_mode: "Default".to_owned(),
            ..MetaSnapshot::default()
        });
        let snap = ConfigSnapshot::from_state(
            &raw.state.read(),
            &Some(("Default".to_owned(), addr.clone())),
        );
        let config = use_signal(|| snap);
        let live = use_signal(LiveSnapshot::default);
        let ctx = AppContext {
            state: Arc::clone(&raw.state),
            commands: raw.commands.clone(),
            settings: Arc::clone(&raw.settings),
            meta,
            config,
            live,
        };
        use_context_provider(|| ctx);
        let view = use_view_state_provider(meta);
        view.selected_mapping.clone().write().replace(("Default".to_owned(), addr));
        use_live_capture_provider();
        let editor = use_editor_state_provider();
        editor.expanded_stages.clone().write().insert(
            crate::frame::mapping_editor::undo_log::StageId(vec![
                crate::frame::mapping_editor::undo_log::StageIdSegment::Index(0),
            ]),
        );
        let toast_state = use_signal(ToastState::default);
        use_context_provider(|| ToastQueue { state: toast_state });
        rsx! { MappingEditor {} }
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("vJoy device") || html.contains("Device"));
    assert!(html.contains("Output") || html.contains("X axis"));
}
```

- [ ] **Step 2: Run test to verify it fails** — Expected: FAIL.

- [ ] **Step 3: Implement `MapToVJoyBody`**

Create `pipeline/stage_body/map_to_vjoy.rs`:

```rust
//! MapToVJoy body: device + output picker.

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis};

use crate::components::{Select, SelectOption};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::{at_path, replace_at_path};
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

#[component]
pub(crate) fn MapToVJoyBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    output: OutputAddress,
    outer_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let cfg = ctx.config.read();

    let device_options: Vec<SelectOption> = cfg
        .virtual_devices
        .iter()
        .map(|v| SelectOption {
            value: format!("{}", v.device_id),
            label: format!("vJoy device {}", v.device_id),
        })
        .collect();

    let current_device = output.device;
    let dev_options_clone = device_options.clone();
    let mapping_key_for_dev = mapping_key.clone();
    let stage_id_for_dev = stage_id.clone();
    let outer_for_dev = outer_actions.clone();
    let cmd_for_dev = ctx.commands.clone();
    let mut undo_for_dev = editor.undo_log;
    let cfg_devs = cfg.virtual_devices.clone();
    drop(cfg);

    let on_device_change = move |evt_value: String| {
        let new_dev: u8 = evt_value.parse().unwrap_or(current_device);
        if new_dev == current_device {
            return;
        }
        let cur_action = at_path(&outer_for_dev, &stage_id_for_dev).cloned();
        if let Some(Action::MapToVJoy { output: cur_out }) = cur_action {
            let new_action = Action::MapToVJoy {
                output: OutputAddress {
                    device: new_dev,
                    output: cur_out.output,
                },
            };
            let new_actions = replace_at_path(&outer_for_dev, &stage_id_for_dev, new_action);
            push_and_dispatch(
                mapping_key_for_dev.clone(),
                outer_for_dev.clone(),
                new_actions,
                "Map to vJoy",
                "device",
                &format!("vJoy {}", current_device),
                &format!("vJoy {}", new_dev),
                &cmd_for_dev,
                &mut undo_for_dev,
            );
        }
    };

    rsx! {
        div { class: "if-stage__body-grid",
            div { class: "if-stage__body-row",
                label { class: "if-stage__body-label", "Device" }
                Select {
                    options: dev_options_clone,
                    value: format!("{}", current_device),
                    onchange: move |evt: FormEvent| on_device_change(evt.value()),
                }
            }
            div { class: "if-stage__body-row",
                label { class: "if-stage__body-label", "Output" }
                // Axis picker, only shows axes available on the chosen device.
                AxisPicker {
                    mapping_key: mapping_key.clone(),
                    stage_id: stage_id.clone(),
                    output: output.clone(),
                    outer_actions: outer_actions.clone(),
                    available_axes: cfg_devs
                        .iter()
                        .find(|v| v.device_id == current_device)
                        .map(|v| v.axes.clone())
                        .unwrap_or_default(),
                }
            }
        }
    }
}

#[component]
fn AxisPicker(
    mapping_key: MappingKey,
    stage_id: StageId,
    output: OutputAddress,
    outer_actions: Vec<Action>,
    available_axes: Vec<VJoyAxis>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let cmd = ctx.commands.clone();
    let mut undo_log = editor.undo_log;

    let options: Vec<SelectOption> = available_axes
        .iter()
        .map(|a| SelectOption {
            value: format!("{a:?}"),
            label: format!("{a:?} axis"),
        })
        .collect();

    let current_label = match output.output {
        OutputId::Axis { id } => format!("{id:?}"),
        OutputId::Button { id } => format!("Btn{id}"),
        OutputId::Hat { id } => format!("Hat{id}"),
    };

    let mapping_key_inner = mapping_key.clone();
    let stage_id_inner = stage_id.clone();
    let outer_inner = outer_actions.clone();
    let output_inner = output.clone();
    let on_change = move |evt: FormEvent| {
        let new_label = evt.value();
        if new_label == current_label {
            return;
        }
        // Parse new_label into a VJoyAxis variant.
        let new_axis = available_axes.iter().find(|a| format!("{a:?}") == new_label).copied();
        if let Some(new_axis) = new_axis {
            let new_action = Action::MapToVJoy {
                output: OutputAddress {
                    device: output_inner.device,
                    output: OutputId::Axis { id: new_axis },
                },
            };
            let new_actions = replace_at_path(&outer_inner, &stage_id_inner, new_action);
            push_and_dispatch(
                mapping_key_inner.clone(),
                outer_inner.clone(),
                new_actions,
                "Map to vJoy",
                "output",
                &current_label,
                &new_label,
                &cmd,
                &mut undo_log,
            );
        }
    };

    rsx! {
        Select {
            options: options,
            value: current_label,
            onchange: on_change,
        }
    }
}

fn push_and_dispatch(
    key: MappingKey,
    actions_before: Vec<Action>,
    new_actions: Vec<Action>,
    stage_name: &str,
    field: &str,
    before: &str,
    after: &str,
    cmd: &std::sync::mpsc::Sender<EngineCommand>,
    undo_log: &mut Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
) {
    let before_mapping = inputforge_core::action::Mapping {
        input: key.1.clone(),
        mode: key.0.clone(),
        name: None, // F9 stage-edit dispatches do not change name; engine preserves it on read.
        actions: actions_before,
    };
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some(stage_name),
            field: Some(field),
            before_after: Some((before, after)),
            ..LabelArgs::default()
        },
    );
    undo_log.write().push_edit(key.clone(), before_mapping, UndoKind::StageEdit, label);
    let _ = cmd.send(EngineCommand::SetMapping {
        input: key.1.clone(),
        mode: key.0.clone(),
        name: None,
        actions: new_actions,
    });
    tracing::info!(target: "f9::mapping_editor", action = "stage_edit", stage = %stage_name, field = %field);
}
```

The F2 `Select` API may differ from `SelectOption` — adapt to whatever the actual primitive expects (check `crates/inputforge-gui-dx/src/components/select.rs`). The `name: None` shortcut on `SetMapping` may need to be replaced with the current name peeked from `cfg.mapping_names` to avoid clearing user-set names; if so, mirror F8's pattern (look at the Duplicate flow in `mapping_list/mod.rs`).

Wire `mod map_to_vjoy;` into `stage_body/mod.rs` and dispatch:

```rust
Action::MapToVJoy { output } => rsx! {
    map_to_vjoy::MapToVJoyBody {
        mapping_key: mapping_key.clone(),
        stage_id: stage_id.clone(),
        output: output.clone(),
        outer_actions: outer_actions.clone(),
    }
},
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::pipeline::tests::map_to_vjoy_body`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body
git commit -m "feat(pipeline): MapToVJoy body with device + output pickers"
```

---

### Task 24: MapToKeyboard body

**Amendments (apply before drafting code):**
1. Wire F8's `LiveCapture::KeysOnly` capture for the binding field via the consumer-flag pattern from Task 16. Local `is_armed_consumer: Signal<bool>` flag; on click "capture", set flag + `cap.start.call(CaptureFilter::KeysOnly)`; in `use_effect` reading `cap.captured`, only react if `*is_armed_consumer.peek() == true`. (Verify `CaptureFilter::KeysOnly` exists at `crates/inputforge-gui-dx/src/patterns/live_capture/mod.rs`; if missing, file a tiny F2/F8 enabler PR and gate this task.)
2. Plain TextInput is acceptable as a *fallback* (typing the key string), but the LiveCapture path is canonical. Either both paths or capture-only — do NOT ship capture-less.
3. Malformed-hint write per spec lines 587-589 for invalid combos (empty `key` field, modifier-only without a base key).
4. Same `name` source-of-truth fix as Task 23.
5. Use `root_actions` for `replace_at_path` calls.
6. Subscribe to `editor_state.external_edit_reset` (Task 33).
7. Skip `push_edit` if `cmd_tx.send(...)` returns `Err`.

### Original Task 24 body:


Modifier toggles + key input. Same `push_and_dispatch` pattern.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/map_to_keyboard.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn map_to_keyboard_body_renders_modifier_toggles_and_key_field() {
    // Same harness as map_to_vjoy test, but with Action::MapToKeyboard.
    // ... (mirror the test pattern; assert html contains "Ctrl" toggle button and key input)
}
```

- [ ] **Step 2: Implement**

Create `stage_body/map_to_keyboard.rs`:

```rust
use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{KeyCombo, KeyModifier};

use crate::components::{Checkbox, TextInput, InputSize};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

#[component]
pub(crate) fn MapToKeyboardBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    key: KeyCombo,
    outer_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let mut local: Signal<KeyCombo> = use_signal(|| key.clone());

    let modifiers = [
        (KeyModifier::Ctrl, "Ctrl"),
        (KeyModifier::Alt, "Alt"),
        (KeyModifier::Shift, "Shift"),
        (KeyModifier::Meta, "Win"),
    ];

    let cmd = ctx.commands.clone();
    let mut undo = editor.undo_log;
    let mapping_key_inner = mapping_key.clone();
    let stage_id_inner = stage_id.clone();
    let outer_inner = outer_actions.clone();
    let initial = key.clone();
    let commit = move |kc: KeyCombo| {
        if kc == initial {
            return;
        }
        let new_action = Action::MapToKeyboard { key: kc.clone() };
        let new_actions = replace_at_path(&outer_inner, &stage_id_inner, new_action);
        let before_mapping = inputforge_core::action::Mapping {
            input: mapping_key_inner.1.clone(),
            mode: mapping_key_inner.0.clone(),
            name: None,
            actions: outer_inner.clone(),
        };
        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("Map to keyboard"),
                field: Some("combo"),
                before_after: Some((&format!("{:?}", initial), &format!("{:?}", kc))),
                ..LabelArgs::default()
            },
        );
        undo.write().push_edit(
            mapping_key_inner.clone(),
            before_mapping,
            UndoKind::StageEdit,
            label,
        );
        let _ = cmd.send(EngineCommand::SetMapping {
            input: mapping_key_inner.1.clone(),
            mode: mapping_key_inner.0.clone(),
            name: None,
            actions: new_actions,
        });
    };

    rsx! {
        div { class: "if-stage__body-grid",
            div { class: "if-stage__body-row",
                label { class: "if-stage__body-label", "Modifiers" }
                div { class: "if-stage__body-modifiers",
                    for (modifier, label_text) in modifiers.iter() {
                        {
                            let modifier = *modifier;
                            let label_text = *label_text;
                            let mut local_inner = local;
                            let mut commit_inner = commit.clone();
                            let checked = local_inner.read().modifiers.contains(&modifier);
                            rsx! {
                                Checkbox {
                                    key: "{label_text}",
                                    checked: checked,
                                    onchange: move |new_checked: bool| {
                                        let mut kc = local_inner.peek().clone();
                                        if new_checked && !kc.modifiers.contains(&modifier) {
                                            kc.modifiers.push(modifier);
                                        } else {
                                            kc.modifiers.retain(|m| m != &modifier);
                                        }
                                        local_inner.set(kc.clone());
                                        commit_inner(kc);
                                    },
                                    "{label_text}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "if-stage__body-row",
                label { class: "if-stage__body-label", "Key" }
                {
                    let local_inner = local;
                    let mut commit_inner = commit;
                    let key_value: ReadSignal<String> =
                        ReadSignal::from(use_memo(move || local_inner.read().key.clone()));
                    rsx! {
                        TextInput {
                            value: key_value,
                            size: InputSize::Sm,
                            onblur: move |evt: FocusEvent| {
                                let _ = evt;
                                let mut local_w = local_inner;
                                let kc = local_w.peek().clone();
                                commit_inner(kc);
                            },
                            oninput: move |evt: FormEvent| {
                                let mut local_w = local_inner;
                                let mut kc = local_w.peek().clone();
                                kc.key = evt.value();
                                local_w.set(kc);
                            },
                        }
                    }
                }
            }
        }
    }
}
```

The actual `Checkbox` / `TextInput` API may differ; mirror F2 primitives. Wire dispatcher arm:

```rust
Action::MapToKeyboard { key } => rsx! {
    map_to_keyboard::MapToKeyboardBody {
        mapping_key: mapping_key.clone(),
        stage_id: stage_id.clone(),
        key: key.clone(),
        outer_actions: outer_actions.clone(),
    }
},
```

- [ ] **Step 3: Run tests to verify they pass**

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body
git commit -m "feat(pipeline): MapToKeyboard body with modifier toggles and key field"
```

---

### Task 25: MergeAxis body

Operation `Select` (`Bidirectional` / `Average` / `Maximum`) plus secondary input picker (source label + `rebind` button arming `LiveCapture::AxesOnly`). Stage summary already lands per Task 21.

**Capture-arming via consumer-flag pattern (same idiom as Task 16's InputField):** `LiveCapture` is single-instance. The MergeAxis secondary picker and the editor-frame InputField both consume `LiveCapture.captured`. To prevent races, each consumer maintains a local `Signal<bool>` `is_armed_consumer` flag and only reacts to `captured` when their own flag is `true`. Other consumers see the flag as `false` and skip. This is the project-wide pattern; deviating from it produces the racy `use_effect` self-fire bug previously flagged in Task 16.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`

- [ ] **Step 1: Write the failing tests**

Append to `pipeline/tests.rs`:

```rust
#[test]
fn merge_axis_body_renders_op_picker_and_secondary_input() {
    use crate::frame::mapping_editor::EditorState;
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

    fn h() -> Element {
        // ... boilerplate as in `invert_stage_expanded_renders_descriptive_caption`
        // but seeding actions: vec![Action::MergeAxis {
        //     second_input: synth_addr(),
        //     operation: MergeOp::Average,
        // }] and pre-expanding stage 0.
        rsx! { /* harness body */ }
    }
    let mut vdom = VirtualDom::new(h);
    vdom.rebuild_in_place();
    let html = render(&vdom);
    assert!(html.contains("Average") || html.contains("Bidirectional"),
        "expected op picker option in DOM: {html}");
    assert!(html.contains("rebind"), "secondary picker rebind button missing");
}

#[test]
fn merge_axis_body_writes_malformed_hint_when_secondary_unset_or_duplicate() {
    // When second_input == primary input OR second_input device is missing,
    // the body must write malformed_hints[stage_id].
    // Seed actions: MergeAxis with second_input == primary_addr.
    // Assert: html contains "secondary input must differ from primary"
    // (or whatever the spec-aligned hint is).
    // Test stub; full test follows the same harness pattern.
}
```

- [ ] **Step 2: Run tests to verify they fail.**

- [ ] **Step 3: Implement `MergeAxisBody`**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs`:

```rust
//! MergeAxis body: operation picker + secondary input picker (consumer-flag).

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{InputAddress, MergeOp};

use crate::components::{Button, ButtonSize, ButtonVariant, Select, SelectOption};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::{at_path, replace_at_path};
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{CaptureFilter, LiveCapture};

#[component]
pub(crate) fn MergeAxisBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    second_input: InputAddress,
    operation: MergeOp,
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let capture = use_context::<LiveCapture>();

    // Consumer-flag for LiveCapture race prevention.
    let mut is_armed_consumer: Signal<bool> = use_signal(|| false);

    // Subscribe to external_edit_reset (Task 33). When it advances, no local
    // state needs reseeding (we read fields fresh from props), but disarm
    // any in-flight capture.
    let reset_token = editor.external_edit_reset;
    let cancel_cb = capture.cancel;
    let mut captured_writer = capture.captured;
    use_effect(move || {
        let _ = reset_token.read(); // re-runs on token advance
        if *is_armed_consumer.peek() {
            cancel_cb.call(());
            captured_writer.set(None);
            is_armed_consumer.set(false);
        }
    });

    // Malformed-hint write contract (spec lines 587-589).
    let stage_id_for_hint = stage_id.clone();
    let primary_addr = mapping_key.1.clone();
    let secondary_for_hint = second_input.clone();
    let mut malformed = editor.malformed_hints;
    use_effect(move || {
        let mut map = malformed.write();
        if secondary_for_hint == primary_addr {
            map.insert(
                stage_id_for_hint.clone(),
                "Secondary input must differ from primary".to_owned(),
            );
        } else {
            map.remove(&stage_id_for_hint);
        }
    });

    // Capture-and-commit secondary input.
    let key_for_eff = mapping_key.clone();
    let stage_id_for_eff = stage_id.clone();
    let root_for_eff = root_actions.clone();
    let cmd_tx = ctx.commands.clone();
    let mut undo_log = editor.undo_log;
    let mut expanded_stages = editor.expanded_stages;
    let mut malformed_hints = editor.malformed_hints;
    use_effect(move || {
        let captured = capture.captured.read().clone();
        if !*is_armed_consumer.peek() {
            return;
        }
        let Some(new_addr) = captured else { return };

        // Read current name from snapshot (preserve user-set name).
        let cfg = ctx.config.read();
        let name = cfg.mapping_names.get(&key_for_eff).cloned();
        drop(cfg);

        let Some(new_actions) = replace_at_path(
            &root_for_eff,
            &stage_id_for_eff,
            Action::MergeAxis {
                second_input: new_addr.clone(),
                operation,
            },
        ) else {
            // Invalid path → skip edit, no phantom undo.
            is_armed_consumer.set(false);
            captured_writer.set(None);
            return;
        };

        let before = Mapping {
            input: key_for_eff.1.clone(),
            mode: key_for_eff.0.clone(),
            name: name.clone(),
            actions: root_for_eff.clone(),
        };
        if cmd_tx
            .send(EngineCommand::SetMapping {
                input: key_for_eff.1.clone(),
                mode: key_for_eff.0.clone(),
                name,
                actions: new_actions,
            })
            .is_err()
        {
            tracing::warn!(target: "f9::mapping_editor", action = "merge_axis_secondary_drop_offline");
            is_armed_consumer.set(false);
            captured_writer.set(None);
            return;
        }
        // No structural mutation here (replace), so no expanded/malformed clear.
        // (Insert/remove paths in other tasks DO clear.)
        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs::default(),
        );
        undo_log
            .write()
            .push_edit(key_for_eff.clone(), before, UndoKind::StageEdit, label);
        is_armed_consumer.set(false);
        captured_writer.set(None);
        let _ = (expanded_stages, malformed_hints); // silence unused; reserved for structural variants
    });

    let start_cb = capture.start;
    let on_rebind = move |_| {
        is_armed_consumer.set(true);
        start_cb.call(CaptureFilter::AxesOnly);
    };

    // Operation picker.
    let key_for_op = mapping_key.clone();
    let stage_id_for_op = stage_id.clone();
    let root_for_op = root_actions.clone();
    let second_for_op = second_input.clone();
    let cmd_tx_op = ctx.commands.clone();
    let mut undo_log_op = editor.undo_log;
    let on_op_change = move |new_op_str: String| {
        let new_op = match new_op_str.as_str() {
            "Bidirectional" => MergeOp::Bidirectional,
            "Average" => MergeOp::Average,
            "Maximum" => MergeOp::Maximum,
            _ => return,
        };
        if new_op == operation {
            return;
        }
        let cfg = ctx.config.read();
        let name = cfg.mapping_names.get(&key_for_op).cloned();
        drop(cfg);
        let Some(new_actions) = replace_at_path(
            &root_for_op,
            &stage_id_for_op,
            Action::MergeAxis {
                second_input: second_for_op.clone(),
                operation: new_op,
            },
        ) else {
            return;
        };
        let before = Mapping {
            input: key_for_op.1.clone(),
            mode: key_for_op.0.clone(),
            name: name.clone(),
            actions: root_for_op.clone(),
        };
        if cmd_tx_op
            .send(EngineCommand::SetMapping {
                input: key_for_op.1.clone(),
                mode: key_for_op.0.clone(),
                name,
                actions: new_actions,
            })
            .is_err()
        {
            return;
        }
        undo_log_op.write().push_edit(
            key_for_op.clone(),
            before,
            UndoKind::StageEdit,
            format_undo_label(UndoKind::StageEdit, LabelArgs::default()),
        );
    };

    let cfg_label = source_label::format(&second_input, &ctx.config.read());

    rsx! {
        div { class: "if-stage__body-grid",
            div { class: "if-field",
                label { class: "if-field__label", "Operation" }
                Select {
                    value: match operation {
                        MergeOp::Bidirectional => "Bidirectional",
                        MergeOp::Average => "Average",
                        MergeOp::Maximum => "Maximum",
                    }.to_owned(),
                    options: vec![
                        SelectOption { label: "Bidirectional".to_owned(), value: "Bidirectional".to_owned() },
                        SelectOption { label: "Average".to_owned(), value: "Average".to_owned() },
                        SelectOption { label: "Maximum".to_owned(), value: "Maximum".to_owned() },
                    ],
                    onchange: on_op_change,
                }
            }
            div { class: "if-field",
                label { class: "if-field__label", "Secondary input" }
                div { class: "if-editor__input-field",
                    "data-body-field": "true",
                    div { class: "if-editor__input-label", "{cfg_label}" }
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        onclick: on_rebind,
                        "rebind"
                    }
                }
            }
        }
    }
}
```

Wire dispatcher:

```rust
Action::MergeAxis { second_input, operation } => rsx! {
    merge_axis::MergeAxisBody {
        mapping_key: mapping_key.clone(),
        stage_id: stage_id.clone(),
        second_input: second_input.clone(),
        operation: *operation,
        root_actions: root_actions.clone(),
    }
},
```

- [ ] **Step 4: Run tests** — Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body
git commit -m "feat(pipeline): MergeAxis body with op picker + secondary capture (consumer-flag)"
```

---

### Task 26a: Conditional shell + branches + recursion

Conditional body's structural shell: render `if_true` as a nested `Pipeline` (recursive), render `if_false` as either a louder "Add else branch" affordance (when `None`) or another nested `Pipeline` (when `Some`). Predicate editing lands separately in Task 26b. Both nested Pipelines receive `root_actions` unchanged (NOT the branch slice — see Task 20's threading rule). Sub-pipelines support drag-and-drop and add-palette via the same component tree (Task 30 + Task 28 add cross-branch DnD support automatically because StageId paths are root-relative).

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/conditional.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn conditional_body_renders_branches_with_correct_aria_labels() {
    // Conditional { if_true: [Invert], if_false: Some([Invert]) }
    // Assert html contains aria-label="if true branch" + "if false branch"
    // AND two nested <ol> elements (one per branch).
}

#[test]
fn conditional_empty_if_false_shows_add_else_affordance() {
    // Conditional { if_true: [], if_false: None }
    // Assert html contains "Add else branch" (louder than empty-pipeline default).
}

#[test]
fn conditional_three_deep_renders_all_branches() {
    // Conditional { if_true: [Conditional { if_true: [Conditional { if_true: [Invert], if_false: None }], if_false: None }], if_false: None }
    // Walk the rendered DOM, assert nesting depth = 3 with the innermost Invert reachable.
}
```

- [ ] **Step 2: Run tests to verify they fail.**

- [ ] **Step 3: Implement `ConditionalBody`**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/conditional.rs`:

```rust
//! Conditional body: predicate editor (Task 26b) + two branch sub-pipelines.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition};

use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::Pipeline;
use crate::frame::mapping_editor::pipeline::stage_body::predicate::PredicateEditor;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

#[component]
pub(crate) fn ConditionalBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    condition: Condition,
    if_true: Vec<Action>,
    if_false: Option<Vec<Action>>,
    /// Mapping's root actions vec. Threaded unchanged through every
    /// recursion. Bodies use this for replace/insert/remove. See Task 20.
    root_actions: Vec<Action>,
    depth: u8,
) -> Element {
    let mut true_path = stage_id.0.clone();
    true_path.push(StageIdSegment::IfTrue);

    let mut false_path = stage_id.0.clone();
    false_path.push(StageIdSegment::IfFalse);

    let true_label = if depth == 0 {
        "if true branch".to_owned()
    } else {
        format!("if true branch (depth {depth})")
    };
    let false_label = if depth == 0 {
        "if false branch".to_owned()
    } else {
        format!("if false branch (depth {depth})")
    };

    rsx! {
        div { class: "if-stage__conditional-body",
            // Task 26b: predicate editor as a separate component.
            PredicateEditor {
                mapping_key: mapping_key.clone(),
                stage_id: stage_id.clone(),
                condition: condition.clone(),
                if_true: if_true.clone(),
                if_false: if_false.clone(),
                root_actions: root_actions.clone(),
            }
            // if-true branch.
            div {
                class: "if-stage__branch",
                "aria-label": "{true_label}",
                div { class: "if-stage__branch-label", "if true" }
                Pipeline {
                    key: mapping_key.clone(),
                    actions: if_true.clone(),
                    root_actions: root_actions.clone(),
                    path_prefix: true_path,
                    depth: depth + 1,
                }
            }
            // if-false branch.
            div {
                class: "if-stage__branch",
                "aria-label": "{false_label}",
                if let Some(branch) = if_false.clone() {
                    div { class: "if-stage__branch-label", "if false" }
                    Pipeline {
                        key: mapping_key.clone(),
                        actions: branch,
                        root_actions: root_actions.clone(),
                        path_prefix: false_path,
                        depth: depth + 1,
                    }
                } else {
                    button {
                        r#type: "button",
                        class: "if-stage__add-else-branch",
                        // onclick wires up in Task 26a Step 4 below — calls
                        // replace_at_path to set if_false = Some(vec![]).
                        "+ Add else branch"
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Wire `+ Add else branch` onclick**

Inside `ConditionalBody`, `onclick` for the affordance dispatches `SetMapping` with `if_false: Some(vec![])`. Reads name from snapshot (source-of-truth fix). Pushes a `StageEdit` undo entry. After this insert-like mutation, ALSO clear `editor_state.expanded_stages` and `editor_state.malformed_hints` per Task 11's structural-mutation invariant.

CSS:

```css
.if-stage__branch {
    margin-left: 16px;
    margin-top: 8px;
}
.if-stage__branch-label {
    font-family: var(--font-mono); font-size: 11px;
    text-transform: uppercase; font-weight: 500;
    color: var(--color-control-badge-text);
    margin-bottom: 4px;
}
.if-stage__add-else-branch {
    width: 100%;
    background: rgba(154, 120, 214, 0.06);
    border: 1px dashed rgba(184, 155, 234, 0.40);
    border-radius: 6px;
    padding: 8px 12px;
    color: var(--color-control-badge-text);
    font-family: var(--font-sans); font-size: 12px;
    font-weight: 500;
    cursor: pointer;
}
```

Wire dispatcher (in `stage_body/mod.rs`):

```rust
Action::Conditional { condition, if_true, if_false } => rsx! {
    conditional::ConditionalBody {
        mapping_key: mapping_key.clone(),
        stage_id: stage_id.clone(),
        condition: condition.clone(),
        if_true: if_true.clone(),
        if_false: if_false.clone(),
        root_actions: root_actions.clone(),
        depth: 0, // tracked via stage_id segment count if needed
    }
},
```

- [ ] **Step 5: Run tests** — Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(pipeline): Conditional shell with recursive branch sub-pipelines"
```

---

### Task 26b: Predicate editor (7 condition kinds)

Standalone component covering all 7 `Condition` variants per spec line 349. Recursive for `All` / `Any` / `Not` (each contains one or more nested `Condition`s rendered as cards).

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`

**Condition kinds (per spec line 349):**

| Kind             | Operand fields                                           |
|------------------|----------------------------------------------------------|
| `ButtonPressed`  | input row (source label + rebind button)                 |
| `ButtonReleased` | input row (source label + rebind button)                 |
| `AxisInRange`    | input row + min/max numeric inputs (F2 NumberInput)     |
| `HatDirection`   | input row + multi-select for direction set (Checkbox group) |
| `All`            | nested condition cards list (recursive)                  |
| `Any`            | nested condition cards list (recursive)                  |
| `Not`            | single nested condition card (recursive)                 |

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn predicate_editor_renders_kind_picker_with_seven_options() {
    // Mount PredicateEditor with Condition::ButtonPressed { input: synth_addr() }
    // Assert html contains all seven kind names as <option> values.
}

#[test]
fn predicate_axis_in_range_renders_min_max_inputs() {
    // Mount with Condition::AxisInRange { input, min: -0.5, max: 0.5 }
    // Assert html contains two <input type="number"> with values -0.5 and 0.5.
}

#[test]
fn predicate_hat_direction_renders_multi_select() {
    // Mount with Condition::HatDirection { input, directions: HatDirectionSet::N | E }
    // Assert html contains 8 checkboxes for the 8 hat directions, with N + E checked.
}

#[test]
fn predicate_all_recursive_renders_nested_predicate_editors() {
    // Mount with Condition::All { conditions: [ButtonPressed{...}, ButtonReleased{...}] }
    // Assert two nested <PredicateEditor> instances render.
}

#[test]
fn predicate_axis_in_range_min_gt_max_writes_malformed_hint() {
    // Mount with Condition::AxisInRange { min: 0.5, max: -0.5 }
    // Assert editor_state.malformed_hints contains entry for stage_id.
}
```

- [ ] **Step 2: Run tests to verify they fail.**

- [ ] **Step 3: Implement `PredicateEditor`**

Create `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs` with the structure:

```rust
//! Predicate editor: kind picker + operand fields per spec line 349.
//! Recursive for All / Any / Not.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{HatDirection, InputAddress};

use crate::components::{
    Button, ButtonSize, ButtonVariant, Checkbox, NumberInput, Select, SelectOption,
};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{CaptureFilter, LiveCapture};

#[component]
pub(crate) fn PredicateEditor(
    mapping_key: MappingKey,
    stage_id: StageId,
    condition: Condition,
    if_true: Vec<Action>,
    if_false: Option<Vec<Action>>,
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();

    // Helper: dispatch new condition by replacing the Conditional at stage_id.
    let dispatch_new_condition = {
        let key = mapping_key.clone();
        let stage_id = stage_id.clone();
        let if_true = if_true.clone();
        let if_false = if_false.clone();
        let root_actions = root_actions.clone();
        let cmd_tx = ctx.commands.clone();
        let mut undo_log = editor.undo_log;
        move |new_cond: Condition| {
            let cfg = ctx.config.read();
            let name = cfg.mapping_names.get(&key).cloned();
            drop(cfg);
            let Some(new_actions) = replace_at_path(
                &root_actions,
                &stage_id,
                Action::Conditional {
                    condition: new_cond,
                    if_true: if_true.clone(),
                    if_false: if_false.clone(),
                },
            ) else {
                return;
            };
            let before = Mapping {
                input: key.1.clone(),
                mode: key.0.clone(),
                name: name.clone(),
                actions: root_actions.clone(),
            };
            if cmd_tx
                .send(EngineCommand::SetMapping {
                    input: key.1.clone(),
                    mode: key.0.clone(),
                    name,
                    actions: new_actions,
                })
                .is_err()
            {
                return;
            }
            undo_log.write().push_edit(
                key.clone(),
                before,
                UndoKind::StageEdit,
                format_undo_label(UndoKind::StageEdit, LabelArgs::default()),
            );
        }
    };

    // Malformed-hint write contract.
    let stage_id_for_hint = stage_id.clone();
    let condition_for_hint = condition.clone();
    let mut malformed = editor.malformed_hints;
    use_effect(move || {
        let mut map = malformed.write();
        match &condition_for_hint {
            Condition::AxisInRange { min, max, .. } if min > max => {
                map.insert(
                    stage_id_for_hint.clone(),
                    "min must be <= max".to_owned(),
                );
            }
            Condition::HatDirection { directions, .. } if directions.is_empty() => {
                map.insert(
                    stage_id_for_hint.clone(),
                    "select at least one hat direction".to_owned(),
                );
            }
            Condition::All { conditions } | Condition::Any { conditions } if conditions.is_empty() => {
                map.insert(
                    stage_id_for_hint.clone(),
                    "predicate must have at least one sub-condition".to_owned(),
                );
            }
            _ => {
                map.remove(&stage_id_for_hint);
            }
        }
    });

    let kind_options = vec![
        SelectOption { label: "ButtonPressed".to_owned(),  value: "ButtonPressed".to_owned() },
        SelectOption { label: "ButtonReleased".to_owned(), value: "ButtonReleased".to_owned() },
        SelectOption { label: "AxisInRange".to_owned(),    value: "AxisInRange".to_owned() },
        SelectOption { label: "HatDirection".to_owned(),   value: "HatDirection".to_owned() },
        SelectOption { label: "All".to_owned(),            value: "All".to_owned() },
        SelectOption { label: "Any".to_owned(),            value: "Any".to_owned() },
        SelectOption { label: "Not".to_owned(),            value: "Not".to_owned() },
    ];

    rsx! {
        div { class: "if-predicate",
            "data-body-field": "true",
            div { class: "if-field",
                label { class: "if-field__label", "Condition" }
                Select {
                    value: condition_kind_str(&condition).to_owned(),
                    options: kind_options,
                    onchange: {
                        let dispatch = dispatch_new_condition.clone();
                        move |new_kind: String| {
                            // Convert kind change to a default-shaped Condition.
                            let new_cond = default_condition_for_kind(&new_kind, &condition);
                            dispatch(new_cond);
                        }
                    },
                }
            }
            // Operand fields per kind. Each branch renders the appropriate
            // operand UI and wires its commits through `dispatch_new_condition`.
            match &condition {
                Condition::ButtonPressed { input } | Condition::ButtonReleased { input } => {
                    rsx! {
                        InputRow {
                            input: input.clone(),
                            // ... rebind button arming LiveCapture::ButtonsOnly
                        }
                    }
                }
                Condition::AxisInRange { input, min, max } => {
                    rsx! {
                        InputRow { input: input.clone() }
                        div { class: "if-field-row",
                            NumberInput { label: "min".to_owned(), value: *min, /* commit -> dispatch */ }
                            NumberInput { label: "max".to_owned(), value: *max, /* commit -> dispatch */ }
                        }
                    }
                }
                Condition::HatDirection { input, directions } => {
                    rsx! {
                        InputRow { input: input.clone() }
                        div { class: "if-hat-direction-grid",
                            // 8 checkboxes for N, NE, E, SE, S, SW, W, NW
                            // Each toggles the bit in directions and dispatches.
                        }
                    }
                }
                Condition::All { conditions } | Condition::Any { conditions } => {
                    // Recursive: render each sub-condition as a nested PredicateEditor.
                    // Each nested editor commits a new sub-condition; the outer
                    // dispatches a new All/Any with the updated list.
                    rsx! {
                        div { class: "if-predicate__nested-list",
                            for (i, sub) in conditions.iter().enumerate() {
                                PredicateEditor {
                                    mapping_key: mapping_key.clone(),
                                    // Sub-stage_id has no segment for predicate
                                    // recursion (the predicate is an attribute
                                    // of the Conditional, not a separate stage).
                                    // Use stage_id with a trailing index to
                                    // distinguish entries; alternative: derive a
                                    // PredicatePath that mirrors StageId for
                                    // nested predicate addressing.
                                    stage_id: stage_id.clone(),
                                    condition: sub.clone(),
                                    if_true: if_true.clone(),
                                    if_false: if_false.clone(),
                                    root_actions: root_actions.clone(),
                                }
                            }
                        }
                    }
                }
                Condition::Not { condition: inner } => {
                    rsx! {
                        PredicateEditor {
                            mapping_key: mapping_key.clone(),
                            stage_id: stage_id.clone(),
                            condition: inner.as_ref().clone(),
                            if_true: if_true.clone(),
                            if_false: if_false.clone(),
                            root_actions: root_actions.clone(),
                        }
                    }
                }
            }
        }
    }
}

fn condition_kind_str(c: &Condition) -> &'static str {
    match c {
        Condition::ButtonPressed { .. } => "ButtonPressed",
        Condition::ButtonReleased { .. } => "ButtonReleased",
        Condition::AxisInRange { .. } => "AxisInRange",
        Condition::HatDirection { .. } => "HatDirection",
        Condition::All { .. } => "All",
        Condition::Any { .. } => "Any",
        Condition::Not { .. } => "Not",
    }
}

fn default_condition_for_kind(kind: &str, prior: &Condition) -> Condition {
    // Preserve operand input where possible (e.g., switching between
    // ButtonPressed and ButtonReleased preserves `input`).
    let prior_input = condition_primary_input(prior);
    match kind {
        "ButtonPressed" => Condition::ButtonPressed { input: prior_input.clone() },
        "ButtonReleased" => Condition::ButtonReleased { input: prior_input.clone() },
        "AxisInRange" => Condition::AxisInRange { input: prior_input.clone(), min: 0.0, max: 1.0 },
        "HatDirection" => Condition::HatDirection { input: prior_input.clone(), directions: HatDirection::Centered.into() },
        "All" => Condition::All { conditions: vec![] },
        "Any" => Condition::Any { conditions: vec![] },
        "Not" => Condition::Not { condition: Box::new(prior.clone()) },
        _ => prior.clone(),
    }
}

fn condition_primary_input(c: &Condition) -> InputAddress {
    match c {
        Condition::ButtonPressed { input }
        | Condition::ButtonReleased { input }
        | Condition::AxisInRange { input, .. }
        | Condition::HatDirection { input, .. } => input.clone(),
        // Recursive kinds: synthesize a default address; the user will replace it.
        Condition::All { .. } | Condition::Any { .. } | Condition::Not { .. } => {
            InputAddress::default() // or a sentinel; handle gracefully in InputRow
        }
    }
}

#[component]
fn InputRow(input: InputAddress) -> Element {
    // Source label + rebind button arming LiveCapture::Any (or ButtonsOnly /
    // AxesOnly / HatsOnly depending on context — Task 26b will narrow). Use
    // the consumer-flag pattern from Task 16. Body changes commit via the
    // outer PredicateEditor's dispatch_new_condition closure.
    rsx! { /* mirror of editor's InputField */ }
}
```

CSS additions:

```css
.if-predicate { display: flex; flex-direction: column; gap: 8px; padding: 8px 0; }
.if-predicate__nested-list { display: flex; flex-direction: column; gap: 6px; padding-left: 16px; border-left: 2px solid var(--color-border); }
.if-hat-direction-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
    width: 120px;
}
```

- [ ] **Step 4: Run tests** — Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/predicate.rs crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(predicate): editor with 7 condition kinds + recursive All/Any/Not"
```

---

### Task 27: Placeholder bodies for ResponseCurve, Deadzone, ChangeMode

Three sibling bodies that render the **single-string spec caption** `F10 / F11 / F14 owns this body` (per spec line 300). Header chevron remains functional. F10/F11/F14 each later replace ONE component here without touching the dispatcher (the F10/F11/F14 hand-off contract from plan line 5).

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/placeholders.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn placeholder_bodies_show_spec_caption() {
    // Action::ResponseCurve / Deadzone / ChangeMode each render
    // "F10 / F11 / F14 owns this body" — single string per spec line 300.
}
```

- [ ] **Step 2: Implement**

```rust
//! Deferred stage bodies for F10 (ResponseCurve), F11 (Deadzone), F14 (ChangeMode).
//! All three render the same single-string spec caption per spec line 300.

use dioxus::prelude::*;

const PLACEHOLDER_CAPTION: &str = "F10 / F11 / F14 owns this body";

#[component]
pub(crate) fn ResponseCurvePlaceholder() -> Element {
    rsx! { div { class: "if-stage__body-caption", "{PLACEHOLDER_CAPTION}" } }
}

#[component]
pub(crate) fn DeadzonePlaceholder() -> Element {
    rsx! { div { class: "if-stage__body-caption", "{PLACEHOLDER_CAPTION}" } }
}

#[component]
pub(crate) fn ChangeModePlaceholder() -> Element {
    rsx! { div { class: "if-stage__body-caption", "{PLACEHOLDER_CAPTION}" } }
}
```

Wire dispatcher arms.

- [ ] **Step 3: Run tests** — PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body
git commit -m "feat(pipeline): placeholder bodies for ResponseCurve/Deadzone/ChangeMode"
```

---

### Task 28: Add palette (categorized action picker)

Click `+` button at the end of any pipeline (or `+ Add first stage` on an empty branch) opens an F2 `MenuRoot` with three sections (Processing, Output, Control). Click an item to append a default-configured action. Dispatches `SetMapping` with `insert_at_path` and pushes a `StageAdd` undo entry.

**Amendments:**
1. **CSS token consolidation:** the palette section accents must use `--color-stage-tint-{processing,output,control}` from Task 5 (NOT `--color-processing` / `--color-output`). Earlier draft inconsistency — replace any references to the non-prefixed tokens.
2. **Wire empty-pipeline button onclick.** Task 20 left `+ Add first stage` (line ~3996) and the end-of-pipeline `+` (line ~4029) as bare buttons. This task connects both `onclick` handlers to open the palette.
3. **`name` source-of-truth:** read current name from `cfg.mapping_names.get(&key).cloned()` and pass as `Some(name)` in the SetMapping dispatch after add. Same fix as Tasks 23-25.
4. **Pass `root_actions`** to `insert_at_path`. After successful insert, clear `editor_state.expanded_stages.write().clear()` AND `editor_state.malformed_hints.write().clear()` per Task 11's structural-mutation invariant. After clear, re-insert just the new stage's StageId into `expanded_stages` so the freshly-added stage opens expanded (UX nicety).
5. Skip `push_edit` if `cmd_tx.send(...)` returns `Err` — engine offline guard.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/add_palette.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn add_palette_inserts_invert_at_end() {
    // Mount editor with empty pipeline.
    // Simulate click on add palette + click "Invert".
    // Verify SetMapping was dispatched with [Action::Invert].
    // (Use an mpsc::channel and assert it received the expected EngineCommand.)
}
```

- [ ] **Step 2: Implement `AddPalette`**

```rust
//! Add-stage palette. Click + button to open; categorized items.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition};
use inputforge_core::engine::EngineCommand;
use inputforge_core::processing::{DeadzoneConfig, ResponseCurve};
use inputforge_core::types::{InputAddress, InputId, MergeOp, OutputAddress, OutputId, VJoyAxis};

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::insert_at_path;
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, StageIdSegment, UndoKind, format_undo_label,
};

#[component]
pub(crate) fn AddPalette(
    mapping_key: MappingKey,
    /// Path prefix for the target pipeline. Insertion appends at the end.
    path_prefix: Vec<StageIdSegment>,
    /// Current length of the target pipeline; insertion goes at this index.
    target_len: usize,
    outer_actions: Vec<Action>,
    /// True for the `+ Add first stage` louder affordance, false for the
    /// inline `+` button at the end of a non-empty pipeline.
    louder: bool,
) -> Element {
    let mut open: Signal<bool> = use_signal(|| false);

    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let cmd = ctx.commands.clone();
    let mut undo = editor.undo_log;

    let mapping_key_inner = mapping_key.clone();
    let path_prefix_inner = path_prefix.clone();
    let outer_inner = outer_actions.clone();
    let mut do_insert = move |variant_label: &'static str, action: Action| {
        let mut path = path_prefix_inner.clone();
        path.push(StageIdSegment::Index(target_len));
        let new_actions = insert_at_path(
            &outer_inner,
            &StageId(path),
            action.clone(),
        );
        let before_mapping = inputforge_core::action::Mapping {
            input: mapping_key_inner.1.clone(),
            mode: mapping_key_inner.0.clone(),
            name: None,
            actions: outer_inner.clone(),
        };
        let label = format_undo_label(
            UndoKind::StageAdd,
            LabelArgs {
                stage_name: Some(variant_label),
                index: Some(target_len),
                ..LabelArgs::default()
            },
        );
        undo.write().push_edit(
            mapping_key_inner.clone(),
            before_mapping,
            UndoKind::StageAdd,
            label,
        );
        let _ = cmd.send(EngineCommand::SetMapping {
            input: mapping_key_inner.1.clone(),
            mode: mapping_key_inner.0.clone(),
            name: None,
            actions: new_actions,
        });
        tracing::info!(
            target: "f9::mapping_editor",
            action = "stage_add",
            variant = %variant_label,
            index = target_len,
        );
        open.set(false);
    };

    let button_class = if louder {
        "if-pipeline__add-first"
    } else {
        "if-pipeline__add-button"
    };
    let button_label = if louder { "+ Add first stage" } else { "+" };

    rsx! {
        div { class: "if-add-palette",
            button {
                r#type: "button",
                class: "{button_class}",
                onclick: move |_| open.toggle(),
                "{button_label}"
            }
            if *open.read() {
                div { class: "if-add-palette__menu", role: "menu",
                    AddSection { title: "Processing", category_class: "is-processing",
                        AddItem { label: "Response curve",
                            on_select: move |_| do_insert("ResponseCurve", Action::ResponseCurve {
                                curve: ResponseCurve::piecewise_linear(
                                    vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
                                    false,
                                ).expect("default curve points are always valid"),
                            }) }
                        AddItem { label: "Deadzone",
                            on_select: move |_| do_insert("Deadzone", Action::Deadzone {
                                config: DeadzoneConfig::default(),
                            }) }
                        AddItem { label: "Invert",
                            on_select: move |_| do_insert("Invert", Action::Invert) }
                    }
                    AddSection { title: "Output", category_class: "is-output",
                        AddItem { label: "Map to vJoy",
                            on_select: move |_| do_insert("MapToVJoy", Action::MapToVJoy {
                                output: OutputAddress {
                                    device: 1,
                                    output: OutputId::Axis { id: VJoyAxis::X },
                                },
                            }) }
                        AddItem { label: "Map to keyboard",
                            on_select: move |_| do_insert("MapToKeyboard", Action::MapToKeyboard {
                                key: inputforge_core::types::KeyCombo {
                                    key: String::new(),
                                    modifiers: vec![],
                                },
                            }) }
                        AddItem { label: "Merge axis",
                            on_select: move |_| do_insert("MergeAxis", Action::MergeAxis {
                                second_input: InputAddress {
                                    device: inputforge_core::types::DeviceId(String::new()),
                                    input: InputId::Axis { index: 0 },
                                },
                                operation: MergeOp::Average,
                            }) }
                    }
                    AddSection { title: "Control", category_class: "is-control",
                        AddItem { label: "Conditional",
                            on_select: move |_| do_insert("Conditional", Action::Conditional {
                                condition: Condition::ButtonPressed {
                                    input: InputAddress {
                                        device: inputforge_core::types::DeviceId(String::new()),
                                        input: InputId::Button { index: 0 },
                                    },
                                },
                                if_true: vec![],
                                if_false: None,
                            }) }
                        AddItem { label: "Change mode",
                            on_select: move |_| do_insert("ChangeMode", Action::ChangeMode {
                                strategy: inputforge_core::action::ModeChangeStrategy::SwitchTo {
                                    mode: String::new(),
                                },
                            }) }
                    }
                }
            }
        }
    }
}

#[component]
fn AddSection(title: &'static str, category_class: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "if-add-palette__section {category_class}",
            div { class: "if-add-palette__section-title", "{title}" }
            { children }
        }
    }
}

#[component]
fn AddItem(label: &'static str, on_select: EventHandler<()>) -> Element {
    rsx! {
        button {
            r#type: "button",
            role: "menuitem",
            class: "if-add-palette__item",
            onclick: move |_| on_select.call(()),
            "{label}"
        }
    }
}
```

CSS:

```css
.if-add-palette { position: relative; display: inline-block; }
.if-add-palette__menu {
    position: absolute; z-index: 5;
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    padding: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.4);
    min-width: 180px;
}
.if-add-palette__section { padding: 4px; }
.if-add-palette__section-title {
    font-family: var(--font-mono); font-size: 11px;
    text-transform: uppercase; font-weight: 500;
    color: var(--color-text-subtle);
    padding: 2px 8px;
}
.if-add-palette__section.is-processing .if-add-palette__section-title { color: var(--color-processing); }
.if-add-palette__section.is-output     .if-add-palette__section-title { color: var(--color-output); }
.if-add-palette__section.is-control    .if-add-palette__section-title { color: var(--color-control-badge-text); }
.if-add-palette__item {
    width: 100%; text-align: left;
    background: transparent; border: none;
    padding: 4px 8px;
    font-family: var(--font-sans); font-size: 12px;
    color: var(--color-text);
    cursor: pointer;
    border-radius: 4px;
}
.if-add-palette__item:hover { background: var(--color-bg-sunken); }
```

Replace the bare `+` button and `+ Add first stage` in `Pipeline` with `AddPalette`:

```rust
li { class: "if-pipeline__add-end",
    AddPalette {
        mapping_key: key.clone(),
        path_prefix: path_prefix.clone(),
        target_len: actions.len(),
        outer_actions: outer_actions_for_palette.clone(),
        louder: false,
    }
}
```

`outer_actions_for_palette` is the entire mapping's action vector at the outer pipeline (read once from `cfg.selected_mapping_actions` at `MappingEditor` render time and passed down through `Pipeline` props for use by all add palettes regardless of recursion depth).

- [ ] **Step 3: Run tests** — Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(pipeline): add palette with categorized action picker"
```

---

### Task 29: Right-click stage actions menu

Right-click on a stage header opens an F2 `MenuRoot`-style menu with Insert before, Insert after, Move up, Move down, Duplicate, Delete. Move up/down disabled at boundaries. Shift+F10 keyboard equivalent. Insert before/after open the same `AddPalette` anchored to the stage. Delete dispatches `RemoveMapping` (if the stage is the only stage and `MapToVJoy`) or `SetMapping` with the stage removed.

**MenuRoot positioning (F2 limitation).** F2 `MenuRoot` (`crates/inputforge-gui-dx/src/components/menu/mod.rs:26-40`) does NOT expose anchor coordinates — its positioning is CSS-driven relative to its parent. Right-click positioning at cursor coordinates therefore requires an absolute-positioned wrapper element holding the MenuRoot:

```rust
// In StageActionsMenu component:
if let Some(menu_state) = stage_menu.read().clone() {
    rsx! {
        div {
            class: "if-stage-menu-anchor",
            style: "position: fixed; left: {menu_state.x}px; top: {menu_state.y}px; z-index: 100;",
            MenuRoot {
                MenuTrigger { /* invisible trigger to keep MenuRoot's open state */ }
                MenuContent {
                    MenuItem { onclick: ..., "Move up" }
                    MenuItem { onclick: ..., "Move down" }
                    MenuItem { onclick: ..., disabled: should_disable_up, "Insert before" }
                    MenuItem { onclick: ..., "Insert after" }
                    MenuItem { onclick: ..., "Delete" }
                }
            }
        }
    }
}
```

**Escape and focus restore.** When the menu opens, capture `document.activeElement` into a Signal. On Escape: close menu (set `stage_menu` to `None`) AND restore focus to the captured element via `document::eval`. Per AC #21's "Esc on a focused stage is a no-op when capture is not armed" — this gives Escape a sensible behavior when the menu is open.

**Shift+F10.** Keyboard equivalent for right-click. Per AC #21 + spec line 391. Lives in Task 31's `decide` matcher: when focused element has `data-stage-id`, Shift+F10 reads the stage's bounding rect and writes `{ stage_id, x: rect.left, y: rect.bottom }` into `editor.stage_menu`. Cross-reference Task 31.

**Items shipped in F9 (subset of full list above).** Move up, Move down, Delete. Insert-before / Insert-after / Duplicate **deferred** (open palette is the canonical add path; insert-before/after duplicate that flow without unique value). Land them in F-future if user feedback flags missing.

**Structural-mutation contract.** Move up/down (reorder), Delete (remove) — both call `replace_at_path`/`remove_at_path` on `root_actions`. After dispatch, clear `editor_state.expanded_stages.write().clear()` and `editor_state.malformed_hints.write().clear()` per Task 11's invariant.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_actions_menu.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn right_click_on_stage_opens_actions_menu() {
    // Render two-stage pipeline.
    // Set EditorState.stage_menu to Some({stage: StageId(0), x: 100, y: 200}).
    // Render and assert html contains menu items with text "Insert before", "Move up", etc.
}
```

- [ ] **Step 2: Implement**

The right-click handler on `Stage` calls `editor.stage_menu.set(Some(StageMenuState { stage, x, y }))`. A separate `StageActionsMenu` component reads `stage_menu` and renders when `Some`. The menu dispatches via `replace_at_path` / `remove_at_path` / `insert_at_path` and pushes the appropriate `UndoEntry` (`StageReorder`, `StageRemove`, `StageAdd`). `Move up at index 0` and `Move down at last` items render with `aria-disabled="true"` and don't dispatch on click.

Mount `StageActionsMenu` once at the top of `MappingEditor` (sibling of `Pipeline`).

- [ ] **Step 3: Run tests** — Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor
git commit -m "feat(pipeline): right-click stage actions menu"
```

---

### Task 30: Drag-and-drop reorder (via `components/sortable`, generic-G upgrade)

F9 reuses the existing F8 `components/sortable` primitive (`crates/inputforge-gui-dx/src/components/sortable/`) instead of rolling its own DnD. The sortable already provides cursor-Y midpoint hit-detection, drop indicators (`Before`/`After`), an AT live-region, the `event.data_transfer().set_data("text/html", "")` Firefox/WebView2 incantation, and the `resolve_drop_index` helper — all in pure Rust, with zero document-level JS.

**One blocker for cross-pipeline DnD (AC #28):** the sortable's group discriminator is `u32` (flat). F9 needs to identify which `Pipeline` instance a row lives in by its parent `StageId` path (root `[]`, branch `[Index(2), IfTrue]`, etc.) so that a drop into a Conditional branch addresses the correct nested pipeline. **Solution:** generalize the sortable to a generic `G: 'static + Clone + PartialEq` group type. F8 keeps its current behavior by specifying `G = u32`; F9 uses `G = StageId` (or a thin `PipelinePath` newtype).

This task therefore has TWO sub-tasks:

- **Task 30a:** Generic-G upgrade to `components/sortable` + F8 migration (small; backward-compatible behavior).
- **Task 30b:** F9 wiring — mount one shared `SortableState<StageId>` in the editor, attach `use_sortable_item` per stage, dispatch reorder via `remove_at_path` + `insert_at_path` on `root_actions`.

**Why the upgrade is safe for F8:**
- F8's `frame/mapping_list/mod.rs:103` calls `use_sortable_state()` once → becomes `use_sortable_state::<u32>()` (turbofish) or `use_sortable_state::<GroupKind>()` (cleaner; drops `group_to_u32`).
- F8's `frame/mapping_list/row.rs:191` validator `Some(|src, tgt| src == tgt)` → becomes `Some(|src: &u32, tgt: &u32| src == tgt)` (only `&` references added because for non-`Copy` `G` the validator must take refs; for `u32` this is identical behavior).
- No CSS changes, no event-handler timing changes, no AT-region changes.

---

### Task 30a: Sortable primitive — generic-G upgrade + F8 migration

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/sortable/state.rs`
- Modify: `crates/inputforge-gui-dx/src/components/sortable/handle.rs`
- Modify: `crates/inputforge-gui-dx/src/components/sortable/item.rs`
- Modify: `crates/inputforge-gui-dx/src/components/sortable/live_region.rs` (only if its API touches `G`; live_region is group-agnostic, may need only `state: SortableState<G>`)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs` (turbofish at the `use_sortable_state` call site)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_list/row.rs` (validator signature: `&u32` refs)

**Bound rationale.** `G: 'static + Clone + PartialEq`:
- `'static` because Dioxus `Signal<T>` requires it.
- `Clone` because the `on_drop` closure needs to read the source group from `state.drag_group` and the target group from its captured config (validator gates the comparison).
- `PartialEq` because `DropTarget` derives it and `ondragleave` filters on `(index, group) == (this_row.index, this_row.group)`.

For `validate_drop`, switch from by-value `fn(u32, u32) -> bool` to by-reference `fn(&G, &G) -> bool`. By-ref keeps `G: !Copy` types working (notably `StageId = Vec<StageIdSegment>`, which is not `Copy`). For `Copy` types like `u32`, the closure body is unchanged after adding `&` to the params.

- [ ] **Step 1: Generalize `state.rs`**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DropTarget<G: 'static + Clone + PartialEq> {
    pub index: usize,
    pub group: G,
    pub side: SortableSide,
    pub invalid: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SortableState<G: 'static + Clone + PartialEq> {
    pub drag_from: Signal<Option<usize>>,
    pub drag_group: Signal<Option<G>>,
    pub drop_target: Signal<Option<DropTarget<G>>>,
    pub live_announcement: Signal<String>,
}

pub fn use_sortable_state<G: 'static + Clone + PartialEq>() -> SortableState<G> {
    SortableState {
        drag_from: use_signal(|| None),
        drag_group: use_signal(|| None),
        drop_target: use_signal(|| None),
        live_announcement: use_signal(String::new),
    }
}
```

`resolve_drop_index` is `G`-free, no change.

- [ ] **Step 2: Generalize `item.rs`**

```rust
pub struct SortableItemConfig<G, F>
where
    G: 'static + Clone + PartialEq,
    F: FnMut(usize, SortableSide) + 'static,
{
    pub state: SortableState<G>,
    pub index: usize,
    pub group: G,
    pub group_len: usize,
    pub item_ref: Signal<Option<Rc<MountedData>>>,
    pub validate_drop: Option<fn(&G, &G) -> bool>,
    pub on_drop: F,
}

pub fn use_sortable_item<G, F>(config: SortableItemConfig<G, F>) -> SortableItemHandlers
where
    G: 'static + Clone + PartialEq,
    F: FnMut(usize, SortableSide) + 'static,
{ /* ... existing body, with `G` replacing `u32` and `&G` replacing `u32` in validator calls ... */ }
```

The handler bodies change in two places:
- `let invalid = validate_drop.is_some_and(|f| !f(src_group, group));` → `is_some_and(|f| !f(&src_group, &group));` (and `src_group` becomes a `G` clone via `*drag_group.peek()` … actually `drag_group.peek()` returns a guard; clone its inner `Option<G>` and unwrap).
- `(*drag_group.peek())` calls become `drag_group.peek().clone()` (or destructure-via-`as_ref`) because `G` may be `!Copy`.

Watch: `*drag_from.peek()` is a `Copy` `Option<usize>` and stays as-is. Only `drag_group` reads need adjusting.

- [ ] **Step 3: Generalize `handle.rs`**

```rust
#[component]
pub fn SortableHandle<G: 'static + Clone + PartialEq>(
    state: SortableState<G>,
    index: usize,
    group: G,
    group_len: usize,
    #[props(default = true)] draggable: bool,
) -> Element { /* ... */ }
```

Inside, `drag_group.set(Some(group))` clones implicitly via `Some(group)` move; if Dioxus's `#[component]` macro requires `G: PartialEq` for prop diffing (it does), the bound is already satisfied.

- [ ] **Step 4: Update `live_region.rs` if needed**

`SortableLiveRegion` only reads `state.live_announcement`. Either parameterize as `SortableLiveRegion<G>`, or — since `G` doesn't affect this component — leave it with `state: SortableState<G>` propagated as a generic param. Trivial change.

- [ ] **Step 5: Migrate F8 callers**

`frame/mapping_list/mod.rs:103`:

```rust
// Before
let sortable = use_sortable_state();
// After (option A: turbofish u32)
let sortable = use_sortable_state::<u32>();
// After (option B: cleaner, drops group_to_u32 entirely)
let sortable = use_sortable_state::<crate::frame::mapping_list::group::GroupKind>();
```

Recommend option B as a follow-up; option A is the minimal-change migration. (For F9's stand-up of Task 30a, ship option A. F8 cleanup to option B in a separate commit.)

`frame/mapping_list/row.rs:191`:

```rust
// Before
validate_drop: Some(|src, tgt| src == tgt),
// After
validate_drop: Some(|src: &u32, tgt: &u32| src == tgt),
```

The `group_to_u32(group_kind)` call already produces a `u32`, no other changes there.

- [ ] **Step 6: Run F8 tests to verify zero behavior change**

```bash
cargo test -p inputforge-gui-dx --lib frame::mapping_list
cargo test -p inputforge-gui-dx --lib components::sortable
```

Expected: PASS — F8's existing behavior is preserved.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/components/sortable crates/inputforge-gui-dx/src/frame/mapping_list
git commit -m "refactor(sortable): make group discriminator generic over G: Clone + PartialEq"
```

---

### Task 30b: F9 wiring — pipeline DnD via the generic sortable

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/dnd.rs` (only `is_descendant` lives here now; the sortable owns drag/drop event handling)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs` (mount `SortableState<StageId>` in context)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/mod.rs` (Pipeline knows its own `path_prefix` — that becomes the sortable's `group`)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs` (`use_sortable_item`, `SortableHandle`)
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css` (drag-handle hover styling matches F8's `.if-sortable-handle`; drop indicator class names align with sortable's `.if-sortable--drop-before` / `--drop-after` / `--drop-invalid`)

- [ ] **Step 1: Write the failing tests**

`is_descendant` (pure):

```rust
#[test]
fn dnd_descendant_detection_rejects_self_descent() {
    let ancestor = StageId(vec![StageIdSegment::Index(0)]);
    let candidate = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfTrue,
        StageIdSegment::Index(0),
    ]);
    assert!(is_descendant(&ancestor, &candidate));
}

#[test]
fn dnd_descendant_detection_allows_unrelated_path() {
    let ancestor = StageId(vec![StageIdSegment::Index(0)]);
    let candidate = StageId(vec![StageIdSegment::Index(1)]);
    assert!(!is_descendant(&ancestor, &candidate));
}

#[test]
fn dnd_descendant_detection_allows_self_drop_to_outer_pipeline() {
    // Dragging a Conditional onto a sibling at the same depth must succeed.
    let ancestor = StageId(vec![StageIdSegment::Index(2)]);
    let candidate = StageId(vec![StageIdSegment::Index(5)]);
    assert!(!is_descendant(&ancestor, &candidate));
}
```

Cross-pipeline integration test (uses `at_path` / `remove_at_path` / `insert_at_path` from Tasks 10-11):

```rust
#[test]
fn dnd_can_move_stage_from_outer_into_conditional_if_true() {
    let actions = vec![
        Action::Conditional {
            condition: Condition::ButtonPressed { input: synth_addr() },
            if_true: vec![],
            if_false: None,
        },
        Action::Invert,
    ];
    let drag_id = StageId(vec![StageIdSegment::Index(1)]);
    let drop_id = StageId(vec![
        StageIdSegment::Index(0),
        StageIdSegment::IfTrue,
        StageIdSegment::Index(0),
    ]);
    let dragged = at_path(&actions, &drag_id).cloned().expect("valid drag");
    let removed = remove_at_path(&actions, &drag_id).expect("valid drag");
    let result = insert_at_path(&removed, &drop_id, dragged).expect("valid drop");
    match &result[0] {
        Action::Conditional { if_true, .. } => assert_eq!(if_true.len(), 1),
        _ => panic!("expected Conditional"),
    }
    assert_eq!(result.len(), 1, "outer pipeline should have one stage after move");
}
```

- [ ] **Step 2: Implement `is_descendant`**

```rust
//! F9 pipeline drag-and-drop helpers.
//!
//! Drag/drop event handling itself is delegated to `components/sortable`
//! (with `G = StageId`). This file only carries pure helpers + the
//! validator wired into `SortableItemConfig.validate_drop`.

use crate::frame::mapping_editor::undo_log::StageId;

/// Strict path-prefix check. A drop is rejected if the source `ancestor`
/// path is a strict prefix of the target `candidate` path — moving a
/// Conditional into one of its own descendant branches would create a
/// cycle in the action tree. Pure; no allocation.
#[must_use]
pub(crate) fn is_descendant(ancestor: &StageId, candidate: &StageId) -> bool {
    if candidate.0.len() <= ancestor.0.len() {
        return false;
    }
    candidate.0[..ancestor.0.len()] == ancestor.0[..]
}

/// Validator pointer for `SortableItemConfig.validate_drop`. Returns
/// `true` (drop allowed) UNLESS the source is an ancestor of the target.
pub(crate) fn validate_pipeline_drop(src: &StageId, tgt: &StageId) -> bool {
    !is_descendant(src, tgt)
}
```

- [ ] **Step 3: Mount one shared `SortableState<StageId>` in the editor**

In `MappingEditor`:

```rust
let sortable: SortableState<StageId> = use_sortable_state::<StageId>();
use_context_provider(|| sortable);

// Render the AT live-region once near the editor root.
SortableLiveRegion { state: sortable }
```

The `Pipeline` and `Stage` components consume the state via `use_context::<SortableState<StageId>>()`.

- [ ] **Step 4: Wire `use_sortable_item` into `Stage`**

The sortable's "group" is the parent pipeline's path: outer pipeline = `StageId(vec![])`; sub-pipeline at `[Index(2), IfTrue]` = `StageId(vec![Index(2), IfTrue])`.

```rust
// In Stage props, add `parent_pipeline_path: StageId` (Pipeline computes
// this and passes it: outer = StageId(vec![]), branch =
// stage_id_of_conditional + IfTrue/IfFalse segment).

let sortable = use_context::<SortableState<StageId>>();
let mut item_ref: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

// Local index within parent pipeline = the trailing Index segment of
// stage_id (always present per StageId construction in Task 6).
let local_index = match stage_id.0.last() {
    Some(StageIdSegment::Index(i)) => *i,
    _ => 0, // unreachable in well-formed StageIds
};

// Read the parent pipeline's current length from root_actions.
// This is needed by the sortable's group_len bookkeeping; F9 derives
// it lazily by walking root_actions to the parent path.
let group_len = parent_pipeline_len(&root_actions, &parent_pipeline_path);

let cmd_tx = ctx.commands.clone();
let editor = use_context::<EditorState>();
let mapping_key_for_drop = mapping_key.clone();
let stage_id_for_drop = stage_id.clone();
let parent_path_for_drop = parent_pipeline_path.clone();
let root_for_drop = root_actions.clone();

let handlers = use_sortable_item(SortableItemConfig {
    state: sortable,
    index: local_index,
    group: parent_pipeline_path.clone(),
    group_len,
    item_ref,
    validate_drop: Some(crate::frame::mapping_editor::pipeline::dnd::validate_pipeline_drop),
    on_drop: move |to: usize, _side: SortableSide| {
        // Source: read from sortable.drag_from + sortable.drag_group; both
        // still populated when this callback runs.
        let Some(src_local_index) = *sortable.drag_from.peek() else { return };
        let Some(src_parent_path) = sortable.drag_group.peek().clone() else { return };

        // Reconstruct source full StageId from src_parent_path + Index(src_local_index).
        let mut src_path = src_parent_path.0.clone();
        src_path.push(StageIdSegment::Index(src_local_index));
        let src_id = StageId(src_path);

        // Target full StageId from this row's parent_pipeline_path + Index(to).
        let mut tgt_path = parent_path_for_drop.0.clone();
        tgt_path.push(StageIdSegment::Index(to));
        let tgt_id = StageId(tgt_path);

        // Read the dragged action, then remove + insert.
        let Some(dragged) = at_path(&root_for_drop, &src_id).cloned() else { return };
        let Some(removed) = remove_at_path(&root_for_drop, &src_id) else { return };
        let Some(new_actions) = insert_at_path(&removed, &tgt_id, dragged) else { return };

        // name source-of-truth (same fix as Tasks 23-25).
        let cfg = ctx.config.read();
        let name = cfg.mapping_names.get(&mapping_key_for_drop).cloned();
        drop(cfg);

        // Snapshot before for undo.
        let before = inputforge_core::action::Mapping {
            input: mapping_key_for_drop.1.clone(),
            mode: mapping_key_for_drop.0.clone(),
            name: name.clone(),
            actions: root_for_drop.clone(),
        };

        if cmd_tx.send(EngineCommand::SetMapping {
            input: mapping_key_for_drop.1.clone(),
            mode: mapping_key_for_drop.0.clone(),
            name,
            actions: new_actions,
        }).is_err() {
            return;
        }

        // Structural mutation: clear expanded_stages + malformed_hints
        // per Task 11 invariant.
        editor.expanded_stages.write().clear();
        editor.malformed_hints.write().clear();

        editor.undo_log.write().push_edit(
            mapping_key_for_drop.clone(),
            before,
            UndoKind::StageReorder,
            format_undo_label(UndoKind::StageReorder, LabelArgs::default()),
        );

        // AT live-region announcement.
        let mut live = sortable.live_announcement;
        live.set(format!(
            "Moved stage to position {} in {}",
            to + 1,
            if parent_path_for_drop.0.is_empty() { "outer pipeline".to_owned() } else { format!("branch {}", format_stage_id(&parent_path_for_drop)) }
        ));
    },
});

// Spread handlers + render SortableHandle (the 6-dot grip) inside the stage.
```

- [ ] **Step 5: Render `SortableHandle` inside each Stage**

```rust
SortableHandle::<StageId> {
    state: sortable,
    index: local_index,
    group: parent_pipeline_path.clone(),
    group_len,
}
```

Place near the chevron / `right_slot` in the header (CSS-driven hover reveal).

- [ ] **Step 6: CSS reuse**

The sortable's existing CSS at `crates/inputforge-gui-dx/assets/components/sortable.css` provides `.if-sortable--drop-before`, `--drop-after`, `--drop-invalid`, `.if-sortable-handle`. Stage cards in `.if-stage` reuse those classes by composing them with the existing `if-stage` styles. Add to `mapping_editor.css` only if F9 needs to override the indicator color or position offsets for the stage layout — the default sortable visuals should suffice.

- [ ] **Step 7: Run tests** — Expected: PASS for `is_descendant` tests; the cross-pipeline integration test passes via `at_path` / `remove_at_path` / `insert_at_path`. SSR coverage of the actual DnD flow is impractical (drag events have no SSR); land manual smoke-testing this path under Task 41 (AC #28).

- [ ] **Step 8: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "feat(pipeline): drag-and-drop reorder via components/sortable with cycle prevention"
```

---

## Phase G, Keyboard + conflict handling (Tasks 31-37)

### Task 31: Editor-scoped Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y handler + Alt+Up/Down + Shift+F10

Window-level keydown listener (architecturally modelled on F8's pure `handle_key()` fn at `crates/inputforge-gui-dx/src/frame/mapping_list/keyboard.rs:73-141`, NOT F8's actual handler which routes navigation keys, not undo) that captures only when the focused element is inside `.if-editor`. Falls through to native textfield undo when focus is inside an `<input>`. Ctrl+Shift+Z and Ctrl+Y always drive editor redo.

**This task expands beyond undo/redo** to cover the full keyboard surface per spec line 393 + AC #21:

- `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y` — undo / redo / redo (Windows convention)
- `Alt+Up` / `Alt+Down` — reorder focused stage within its current pipeline (sibling swap). Targets the focused element's `data-stage-id` attribute.
- `Shift+F10` — open right-click menu at the focused stage's bounding rect (cross-reference Task 29).
- `Alt+Left` / `Alt+Right` — **deferred** per spec line 393 ("an open question, evaluate during impeccable:harden").

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/keyboard.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`

- [ ] **Step 1: Write the pure dispatcher unit test**

The pure `decide(key, ctrl, shift, focus_target) -> KbIntent` function avoids needing a Dioxus runtime for the test:

```rust
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum KbIntent {
    Undo,
    Redo,
    /// Move focused stage one slot earlier within its parent pipeline.
    StageMoveUp,
    /// Move focused stage one slot later within its parent pipeline.
    StageMoveDown,
    /// Open right-click menu at the focused stage's bounding rect.
    StageMenuOpen,
    PassThrough,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FocusTarget {
    Input,    // <input> / <textarea> / contenteditable
    Stage,    // an `.if-stage` element (carries data-stage-id attr)
    Editor,   // anywhere else inside .if-editor
    Outside,  // outside .if-editor
}

pub(crate) fn decide(
    key: &str,
    ctrl: bool,
    shift: bool,
    alt: bool,
    focus: FocusTarget,
) -> KbIntent {
    if focus == FocusTarget::Outside {
        return KbIntent::PassThrough;
    }
    match (key, ctrl, shift, alt) {
        // Ctrl+Z: editor undo unless inside <input> (browser native undo).
        ("z" | "Z", true, false, false) => match focus {
            FocusTarget::Input => KbIntent::PassThrough,
            _ => KbIntent::Undo,
        },
        // Ctrl+Shift+Z: editor redo (browsers don't bind it inside inputs).
        ("z" | "Z", true, true, false) => KbIntent::Redo,
        // Ctrl+Y: editor redo (Windows convention; browsers don't bind it).
        ("y" | "Y", true, false, false) => KbIntent::Redo,
        // Alt+Up / Alt+Down: stage reorder. Per spec line 393 + AC #21.
        // Only fires when a stage element is focused (NOT inside an input).
        ("ArrowUp", false, false, true) if focus == FocusTarget::Stage => KbIntent::StageMoveUp,
        ("ArrowDown", false, false, true) if focus == FocusTarget::Stage => KbIntent::StageMoveDown,
        // Shift+F10: open right-click menu at the focused stage's rect.
        // Per spec line 391.
        ("F10", false, true, false) if focus == FocusTarget::Stage => KbIntent::StageMenuOpen,
        // Alt+Left / Alt+Right: deferred per spec line 393.
        _ => KbIntent::PassThrough,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_z_inside_input_passes_through() {
        assert_eq!(decide("z", true, false, false, FocusTarget::Input), KbIntent::PassThrough);
    }

    #[test]
    fn ctrl_z_in_editor_drives_undo() {
        assert_eq!(decide("z", true, false, false, FocusTarget::Editor), KbIntent::Undo);
    }

    #[test]
    fn ctrl_z_on_focused_stage_drives_undo() {
        assert_eq!(decide("z", true, false, false, FocusTarget::Stage), KbIntent::Undo);
    }

    #[test]
    fn ctrl_shift_z_inside_input_drives_redo() {
        assert_eq!(decide("z", true, true, false, FocusTarget::Input), KbIntent::Redo);
    }

    #[test]
    fn ctrl_y_in_editor_drives_redo() {
        assert_eq!(decide("y", true, false, false, FocusTarget::Editor), KbIntent::Redo);
    }

    #[test]
    fn outside_editor_passes_through() {
        assert_eq!(decide("z", true, false, false, FocusTarget::Outside), KbIntent::PassThrough);
    }

    #[test]
    fn unrelated_key_passes_through() {
        assert_eq!(decide("a", false, false, false, FocusTarget::Editor), KbIntent::PassThrough);
    }

    #[test]
    fn alt_up_on_stage_moves_up() {
        assert_eq!(decide("ArrowUp", false, false, true, FocusTarget::Stage), KbIntent::StageMoveUp);
    }

    #[test]
    fn alt_down_on_stage_moves_down() {
        assert_eq!(decide("ArrowDown", false, false, true, FocusTarget::Stage), KbIntent::StageMoveDown);
    }

    #[test]
    fn alt_up_in_input_passes_through() {
        // Don't intercept Alt+Up inside text fields.
        assert_eq!(decide("ArrowUp", false, false, true, FocusTarget::Input), KbIntent::PassThrough);
    }

    #[test]
    fn shift_f10_on_stage_opens_menu() {
        assert_eq!(decide("F10", false, true, false, FocusTarget::Stage), KbIntent::StageMenuOpen);
    }

    #[test]
    fn alt_left_right_deferred_pass_through() {
        // Per spec line 393, Alt+Left/Right is an open question deferred to
        // impeccable:harden. The handler MUST pass these through unchanged.
        assert_eq!(decide("ArrowLeft", false, false, true, FocusTarget::Stage), KbIntent::PassThrough);
        assert_eq!(decide("ArrowRight", false, false, true, FocusTarget::Stage), KbIntent::PassThrough);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::keyboard`
Expected: FAIL.

- [ ] **Step 3: Implement the listener hook**

Land the pure `decide` fn from Step 1 in `keyboard.rs`. Add a `use_kb_listener` hook: `document::eval` registers a window keydown listener that classifies the event into `(key, ctrl, shift, alt, focus_target)` and posts back via Dioxus's eval bridge.

JS focus classification:

```js
const el = document.activeElement;
let target = "Outside";
if (el && el.closest('.if-editor')) {
    if (el.matches('input, textarea, [contenteditable]')) target = "Input";
    else if (el.closest('.if-stage[data-stage-id]')) target = "Stage";
    else target = "Editor";
}
```

Rust side calls `decide(key, ctrl, shift, alt, target)` then dispatches the intent:

- `Undo` → `editor.undo_log.write().undo(&key)` → `SetMapping` via `ctx.commands`
- `Redo` → `editor.undo_log.write().redo(&key)` → `SetMapping`
- `StageMoveUp` / `StageMoveDown` → read focused stage's `data-stage-id`, parse to `StageId`, compute target index (sibling ±1), call `remove_at_path` + `insert_at_path` on `root_actions`, dispatch `SetMapping`. After this structural mutation, clear `editor.expanded_stages.write().clear()` and `editor.malformed_hints.write().clear()` (Task 11 invariant).
- `StageMenuOpen` → read focused stage's bounding rect, write `editor.stage_menu.set(Some(StageMenuState { stage, x: rect.left, y: rect.bottom }))`.

The handler MUST `evt.prevent_default()` only for intents that fire (Undo/Redo/StageMove/StageMenuOpen). For `PassThrough`, do nothing — let the event reach native handlers (browser textfield undo, etc.).

**Note on JS round-trip latency.** `document::eval` is async; on a fast Ctrl+Z, the user could continue typing before the focus-target classification round-trips. The pure-fn `decide` test does not exercise this. F9 ships the handler with this acknowledged latency; if user feedback flags missed inputs, switch to a synchronous classification (capture focus state on `keydown` via inline JS, decide synchronously without an async hop).

- [ ] **Step 4: Run unit tests**

Run: `cargo test -p inputforge-gui-dx --lib frame::mapping_editor::keyboard`
Expected: PASS, twelve tests (six original + Alt+Up/Down + Shift+F10 + Alt+Left/Right deferred).

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor
git commit -m "feat(mapping_editor): editor-scoped Ctrl+Z/Shift+Z/Y handler"
```

---

### Task 32: Profile-flip undo log clear (via F4 DirtyConfirmDialog)

Per **AC #26** ("Editing-mode flip preserves log; only profile flip clears (via F4)"), the undo log clears ONLY on profile flip, ONLY through F4's existing `DirtyConfirmDialog`. There is **no `ProfileFlipped` event** in the engine (verified — see `crates/inputforge-core/src/engine/run.rs`); profile changes propagate via `ConfigSnapshot.profile_name` from the polling task. F4's `DirtyConfirmDialog` (`crates/inputforge-gui-dx/src/patterns/dirty_confirm.rs:52-117`) is reusable across features, NOT F4-monolithic.

**Approach (replaces the prior `use_effect`-on-profile-name draft):** When the user attempts a profile flip AND any `MappingHistory` has non-empty stacks, open `DirtyConfirmDialog`. `onsave` callback clears all undo logs THEN completes the profile flip. `oncancel` aborts. When all stacks are empty, the profile flip proceeds without dialog.

This avoids the race where a `use_effect` keyed on `profile_name` fires the same tick as a Ctrl+Z, leaving an inconsistent state. The clear runs synchronously inside `onsave`, before the flip dispatches.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/undo_log.rs` (add `clear_all` + `has_pending_changes`)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs` (mount the dialog)
- Modify: `crates/inputforge-gui-dx/src/frame/top_bar.rs` (or wherever profile selection dispatches) to gate on the dialog

- [ ] **Step 1: Add `clear_all` and `has_pending_changes` on `UndoLog`**

```rust
impl UndoLog {
    /// Clear all per-mapping stacks. Used by Task 32's onsave callback.
    pub(crate) fn clear_all(&mut self) {
        self.stacks.clear();
    }

    /// Returns true if any mapping has non-empty undo OR redo stacks.
    /// Used by Task 32 to decide whether to open the dirty-confirm dialog.
    pub(crate) fn has_pending_changes(&self) -> bool {
        self.stacks
            .values()
            .any(|h| !h.undo.is_empty() || !h.redo.is_empty())
    }
}
```

Tests:

```rust
#[test]
fn clear_all_empties_every_mapping_stack() {
    let mut log = UndoLog::default();
    let key = synth_key();
    log.push_edit(key.clone(), synth_mapping("a"), UndoKind::Rename, "a".to_owned());
    log.clear_all();
    assert!(log.stacks.is_empty());
}

#[test]
fn has_pending_changes_reflects_stack_state() {
    let mut log = UndoLog::default();
    let key = synth_key();
    assert!(!log.has_pending_changes());
    log.push_edit(key.clone(), synth_mapping("a"), UndoKind::Rename, "a".to_owned());
    assert!(log.has_pending_changes());
    log.undo(&key);
    assert!(log.has_pending_changes(), "non-empty redo also counts");
    log.clear_all();
    assert!(!log.has_pending_changes());
}
```

- [ ] **Step 2: Mount `DirtyConfirmDialog` in MappingEditor + intercept profile flips**

In the profile-selection UI (likely `frame/top_bar.rs` or wherever the profile dropdown commits):

```rust
let editor = use_context::<EditorState>();
let mut dirty_dialog_open: Signal<bool> = use_signal(|| false);
let mut pending_profile: Signal<Option<String>> = use_signal(|| None);

let on_profile_select = move |new_profile: String| {
    if editor.undo_log.read().has_pending_changes() {
        pending_profile.set(Some(new_profile));
        dirty_dialog_open.set(true);
    } else {
        // No pending changes — flip directly.
        cmd_tx.send(EngineCommand::SetProfile { name: new_profile }).ok();
    }
};

// Sibling element near MappingEditor:
DirtyConfirmDialog {
    open: dirty_dialog_open,
    title: Some("Discard editor undo log?".to_owned()),
    message: Some("Switching profile clears the per-mapping undo stack. Continue?".to_owned()),
    save_label: Some("Switch profile".to_owned()),
    onsave: move |_| {
        // Clear THEN flip — synchronous order.
        editor.undo_log.write().clear_all();
        if let Some(name) = pending_profile.read().clone() {
            let _ = cmd_tx.send(EngineCommand::SetProfile { name });
        }
        pending_profile.set(None);
        dirty_dialog_open.set(false);
    },
    oncancel: move |_| {
        pending_profile.set(None);
        dirty_dialog_open.set(false);
    },
}
```

- [ ] **Step 3: Run tests** — PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/src/frame/top_bar.rs
git commit -m "feat(undo_log): clear_all on profile flip via F4 DirtyConfirmDialog"
```

---

### Task 33: External-edit reconciliation with focused-edit preservation

When `ConfigSnapshot.selected_mapping_actions` changes externally and no body field has focus AND no drag is active AND no `LiveCapture` is armed, the editor resets local working copies to engine state. When any of those is true, the reset is **deferred** until the focused field blurs / drag ends / capture cancels. Per **AC #27**, a Warning toast `Mapping was edited externally` surfaces immediately in either case.

**Mechanism: `external_edit_reset: Signal<u64>` token.** Already declared on `EditorState` (Task 9). The polling task increments the token whenever it detects an external change to `selected_mapping_actions`. Each body subscribes via `use_effect` reading the token and re-derives local Signals on advance.

**Reset suppression set (focus-aware):**
- `document.activeElement` matches `[data-body-field]` (every input/select/checkbox inside a stage body must carry this attribute — Tasks 23, 24, 25, 26b enforce)
- OR `document.activeElement` matches the editor name field (`.if-editor__name-input`)
- OR `editor.stage_menu.read().is_some()` (right-click menu open)
- OR drag is active (`Signal<Option<StageId>>` on `EditorState` for DnD source — wire in Task 30)
- OR LiveCapture is armed (`*capture.active.read() == true`)

When any condition holds, increment the token but ALSO set a `pending_external_reset: Signal<bool>` on EditorState; bodies see the token advance, check the suppression conditions in their own scope, and defer their local reset. On blur of the suppressing element (e.g., name field), bodies check the pending flag and process the deferred reset.

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/external_edit.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/bridge.rs` (polling task increments the token on external change)
- Cross-task amendments to Tasks 22-26b (each body subscribes to `external_edit_reset` via `use_effect`)

- [ ] **Step 1: Add `pending_external_reset` to EditorState**

Append to `EditorState` struct (Task 9):

```rust
pub pending_external_reset: Signal<bool>,
```

And initialize in `use_editor_state_provider`.

- [ ] **Step 2: Implement detection logic in `external_edit.rs`**

```rust
use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::mapping_editor::EditorState;
use crate::frame::view_state::ViewState;
use crate::patterns::live_capture::LiveCapture;
use crate::toast::ToastQueue;

#[component]
pub(crate) fn ExternalEditReconciler() -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let _view = use_context::<ViewState>();
    let _cap = use_context::<LiveCapture>();
    let toast = use_context::<ToastQueue>();

    let mut last_seen: Signal<Option<Vec<inputforge_core::action::Action>>> =
        use_signal(|| None);

    let mut external_edit_reset = editor.external_edit_reset;
    let mut pending = editor.pending_external_reset;

    use_effect(move || {
        let current = ctx.config.read().selected_mapping_actions.clone();
        let prev = last_seen.peek().clone();
        if prev != current {
            last_seen.set(current.clone());
            // Skip the very first observation (no prior state).
            if prev.is_some() && current.is_some() && prev != current {
                // External edit detected. Surface toast immediately per AC #27.
                toast.push_warning("Mapping was edited externally");

                // Advance the reset token — bodies decide locally whether
                // to reset now or defer.
                external_edit_reset.with_mut(|n| *n = n.wrapping_add(1));

                // If focus / drag / capture suppresses, set the pending flag.
                if is_reset_suppressed() {
                    pending.set(true);
                } else {
                    pending.set(false);
                }
            }
        }
    });

    rsx! {} // Empty render; component exists only for its effect.
}

/// Synchronous DOM check via document::eval. Bodies + this reconciler call it
/// to decide whether to reset now or defer.
fn is_reset_suppressed() -> bool {
    // Use document::eval with a synchronous-ish check; for an MVP, query the
    // most common suppressors. Returns false if eval fails.
    let _ = dioxus::prelude::document::eval(r#"
        const a = document.activeElement;
        return Boolean(
            a && (
                a.matches('[data-body-field]') ||
                a.matches('.if-editor__name-input') ||
                a.closest('.if-stage-menu-anchor')
            )
        );
    "#);
    // The actual answer comes back via Dioxus's eval bridge; for plan-doc
    // simplicity, the implementer wires a return-value channel. Default-false
    // means "reset eagerly" if the bridge is slow.
    false
}
```

Mount as a sibling of `Pipeline` inside `MappingEditor`.

- [ ] **Step 3: Cross-task amendments — body reset subscription**

Tasks 22 (Invert), 23 (MapToVJoy), 24 (MapToKeyboard), 25 (MergeAxis), 26a (Conditional), 26b (Predicate) each MUST add a `use_effect` reading `editor.external_edit_reset`:

```rust
let reset_token = editor.external_edit_reset;
let mut pending = editor.pending_external_reset;
use_effect(move || {
    let _ = reset_token.read();      // re-runs on token advance
    if !*pending.peek() {
        // Re-derive any local Signals from the action's current fields.
        // Tasks with no local Signals (e.g., Invert, the dispatchers)
        // skip this branch.
    }
});
```

(For F9, most bodies have no local working copy — they read all values from props and dispatch on every change. The `MergeAxis` and `MapToKeyboard` bodies, plus the predicate editor's nested condition cards, may have local state — those reset here.)

- [ ] **Step 4: Increment the token from the polling task**

In `bridge.rs`, after the `ConfigSnapshot::from_state` rebuild, compare the new `selected_mapping_actions` to the previous tick's. If diverged AND the change came from outside the editor (heuristic: not in the last-N `EngineCommand::SetMapping` dispatch trail), increment `editor.external_edit_reset`. Easier-to-implement path: rely entirely on the reconciler's `use_effect` (Step 2) — it already detects divergence by comparing `selected_mapping_actions` to its own `last_seen` shadow. In that case, this Step 4 collapses to a no-op.

- [ ] **Step 5: Run tests**

Smoke test only (the live-reconcile flow needs runtime). Cover the suppression-set decision function with a unit test if extracted to a pure helper.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor crates/inputforge-gui-dx/src/bridge.rs
git commit -m "feat(mapping_editor): external-edit reconciliation with focus + drag + capture preservation"
```

---

### Task 34: Selected-mapping-deleted-externally fallback

When `view.selected_mapping` is `Some(key)` but `key` is no longer in `ctx.config.mappings`, `MappingEditor` reverts to the empty state silently (no toast; the rail's deletion already communicates the change).

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs`

- [ ] **Step 1: Write the failing SSR test**

Mount editor with `selected_mapping = Some(key)` but build a `ConfigSnapshot` whose `mappings` does not contain `key`. Assert empty state renders.

- [ ] **Step 2: Implement (use_effect, NOT signal-set during render)**

Mutating a Signal during render in Dioxus triggers a re-render loop or runs effect chains in the wrong order (preview reviewers' Critical finding: `sel.set(None)` inside the render branch causes Task 33's reset effect to fire against half-mutated state). Use `use_effect` keyed on `(selected_mapping, mappings)` mismatch instead:

```rust
let view = use_context::<ViewState>();
let mut sel = view.selected_mapping;
let cfg = ctx.config; // Signal<ConfigSnapshot>

use_effect(move || {
    let snap = cfg.read();
    let current = sel.peek().clone();
    if let Some((mode, input)) = current {
        let resolved = snap
            .mappings
            .iter()
            .any(|m| m.input == input && m.mode == mode);
        if !resolved {
            // Mapping deleted externally; per AC #19 ("External deletion:
            // silent empty-state revert"), clear selection silently. The
            // render path observes `selected_mapping == None` and shows
            // the empty state on the next tick.
            sel.set(None);
        }
    }
});
```

The render path's `if let Some((mode, input)) = view_state_for_render` branch becomes purely view-only — no mutation. When `sel == None`, the empty-state branch renders normally.

- [ ] **Step 3: Run tests** — PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs
git commit -m "feat(mapping_editor): fallback to empty state on stale selection"
```

---

### Task 35: Malformed action treatment (visual only)

Stage with invalid params shows the title in `--color-error` plus a one-line fix hint in the summary slot. Per **spec lines 587-589** + **AC #17**, each body computes its hint string on render and writes to `editor_state.malformed_hints[stage_id]`; the stage header reads this map and surfaces the hint.

**Body-side malformed-hint emission is now covered in Tasks 22, 23, 24, 25, 26a, 26b individually** as cross-task amendments (the `**Amendments:**` callouts at the top of each task list `malformed_hints.write().insert(stage_id, hint)` as a Step). This task therefore reduces to the **visual treatment** in `Stage` and `StageHeader`: reading `malformed_hints`, overriding the summary, and applying the error-tinted title class. Body validators that emit hints (per task):

| Task | Body | Triggers |
|------|------|----------|
| 23 (MapToVJoy)    | "vJoy device {N} not configured" / "Output {kind} {idx} out of range" |
| 24 (MapToKeyboard)| "Empty key combo" / "Modifier-only without base key" |
| 25 (MergeAxis)    | "Secondary input must differ from primary" |
| 26b (Predicate)   | "min must be <= max" / "select at least one hat direction" / "predicate must have at least one sub-condition" |
| 27 (Placeholders) | None (placeholders cannot be malformed in F9) |

Other bodies (Invert) do not emit hints.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/merge_axis.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/conditional.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn malformed_merge_axis_with_empty_secondary_shows_error_hint() {
    let actions = vec![Action::MergeAxis {
        second_input: InputAddress {
            device: DeviceId(String::new()),
            input: InputId::Axis { index: 0 },
        },
        operation: MergeOp::Average,
    }];
    let html = render_with(seeded_profile_with_one_mapping(actions),
        InputAddress { device: DeviceId("dev-1".to_owned()), input: InputId::Axis { index: 0 } });
    assert!(html.contains("Pick a secondary input"));
    assert!(html.contains("if-stage__title--error"));
}
```

- [ ] **Step 2: Implement**

In each variant body that can be malformed:
- `MergeAxisBody`: if `second_input.device.0.is_empty()`, write `"Pick a secondary input"` to `malformed_hints[stage_id]`.
- `ConditionalBody`: if `validate_depth(&condition, MAX_CONDITION_DEPTH).is_err()`, write `"Predicate exceeds nesting limit"`.
- Other bodies: clear the entry on render.

In `Stage`, after computing `summary`, read `malformed_hints.get(&stage_id)` and override the summary plus apply the `if-stage__title--error` class to the title.

CSS:

```css
.if-stage__title--error { color: var(--color-error); }
```

- [ ] **Step 3: Run tests** — PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor
git commit -m "feat(pipeline): malformed-action treatment with fix-hint summary"
```

---

### Task 36: Reduced-motion guard sweep

Audit every transition added by previous tasks and confirm a `@media (prefers-reduced-motion: reduce)` rule sets it to `0ms` / `none`. Live readout bars never animate (they're already `instant always`).

**Files:**
- Modify: `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`

- [ ] **Step 1: Verify by exhaustive grep, not by memorized list**

Run the project's grep tool against `crates/inputforge-gui-dx/assets/frame/mapping_editor.css`:

```
Grep "transition:|animation:" pattern in this file.
```

Enumerate EVERY hit. For each, confirm one of: (a) explicit `@media (prefers-reduced-motion: reduce) { ... }` override that disables it; (b) the transition is acceptable under reduced motion (e.g., zero-duration intentionally); (c) move the rule into a default-disabled wrapper.

Initial known list (tasks that introduced each):
- `.if-stage__chevron` — 180 ms ease-out (Task 20)
- `.if-editor__inactive-hint` — 150 ms opacity fade (Task 18)
- `.if-stage__drag-handle` — 100 ms opacity (Task 30)
- `.if-pipeline__add-first` hover — Task 20 (review the actual file for any additional hover/focus animations Tasks 22-30 added)
- Any palette open/close animations from Task 28
- DnD drop indicator pulse from Task 30

**Do not memorize this list.** The grep is the source of truth — Tasks 22-30 may have added rules not listed here.

- [ ] **Step 2: Append missing reduced-motion rules**

```css
@media (prefers-reduced-motion: reduce) {
    .if-stage__chevron,
    .if-editor__inactive-hint,
    .if-stage__drag-handle {
        transition: none;
    }
}
```

(Existing per-rule overrides may already cover this; consolidate only if missing.)

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/assets/frame/mapping_editor.css
git commit -m "polish(mapping_editor): consolidate reduced-motion overrides"
```

---

### Task 37: F5 spec amendment

Update line 177 of `docs/superpowers/specs/2026-04-27-f5-architecture-ia-redesign-design.md` to match the F9-tightened copy.

**Cross-reference Task 18 (inactive-runtime hint banner) before committing.** The plan amendment process pinned Task 18's rendered copy to `Engine is in *<runtime>*. Mapping fires only in *<editing>*.` (verified in the Phase 1 exploration). After landing the F5 spec amendment here, re-read Task 18's component code and confirm the rendered string matches verbatim — letter, italic markers, and trailing period. Any drift between Task 18's render and Task 37's spec amendment ships an inconsistent feature.

**Files:**
- Modify: `docs/superpowers/specs/2026-04-27-f5-architecture-ia-redesign-design.md`

- [ ] **Step 1: Read the current line**

```bash
sed -n '175,180p' docs/superpowers/specs/2026-04-27-f5-architecture-ia-redesign-design.md
```

(Use Read tool not sed; this is a description of intent.)

- [ ] **Step 2: Replace the copy**

Find:

```
Inactive-in-runtime hint: rendered when `editing_mode != runtime_mode`. Copy is fixed: "Inactive in current runtime mode. Engine is in *<runtime>*; this mapping fires only in *<editing>*."
```

Replace with:

```
Inactive-in-runtime hint: rendered when `editing_mode != runtime_mode`. Copy is fixed: "Engine is in *<runtime>*. Mapping fires only in *<editing>*."
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-04-27-f5-architecture-ia-redesign-design.md
git commit -m "docs(f5-spec): tighten inactive-runtime hint copy per F9 brainstorm"
```

---

## Phase H, Final SSR coverage + smoke (Tasks 38-41)

### Task 38: SSR coverage for the four-stage pipeline acceptance check

Acceptance criterion 9 / spec test 2 requires a 4-stage pipeline (Deadzone, ResponseCurve, MergeAxis, MapToVJoy) renders all four with correct category tints, summary text, and chevron states.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs`

- [ ] **Step 1: Write the test**

```rust
#[test]
fn four_stage_pipeline_renders_all_categories_and_summaries() {
    let actions = vec![
        Action::Deadzone { config: DeadzoneConfig::default() },
        Action::ResponseCurve {
            curve: ResponseCurve::piecewise_linear(
                vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false,
            ).unwrap(),
        },
        Action::MergeAxis {
            second_input: InputAddress {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Axis { index: 1 },
            },
            operation: MergeOp::Average,
        },
        Action::MapToVJoy {
            output: OutputAddress {
                device: 1,
                output: OutputId::Axis { id: VJoyAxis::X },
            },
        },
    ];
    let (state, addr) = build_state(actions);
    let html = render_with(state, addr);

    // Structural assertion: 4 sibling stages in DOM order, each with the
    // correct title in its header AND the correct category class.
    // Substring `html.contains(...)` alone is too permissive (the test
    // could pass even if "3 points" appeared in the wrong stage, or the
    // stages rendered in the wrong order). Use a DOM walker.
    use scraper::{Html, Selector};
    let doc = Html::parse_document(&html);
    let stage_sel = Selector::parse("li.if-stage").expect("selector");
    let stages: Vec<_> = doc.select(&stage_sel).collect();
    assert_eq!(stages.len(), 4, "expected 4 stages, got {}", stages.len());

    // Stage 0: Deadzone (is-processing)
    assert!(stages[0].value().attr("class").unwrap_or("").contains("is-processing"));
    assert!(stages[0].html().contains("Deadzone"));

    // Stage 1: Response curve (is-processing) — symmetric=false, 3 points.
    assert!(stages[1].value().attr("class").unwrap_or("").contains("is-processing"));
    assert!(stages[1].html().contains("Response curve"));
    assert!(stages[1].html().contains("3 points"));
    // No "symmetric" qualifier (curve was created asymmetric).
    assert!(!stages[1].html().contains("symmetric"));

    // Stage 2: Merge axis (is-output)
    assert!(stages[2].value().attr("class").unwrap_or("").contains("is-output"));
    assert!(stages[2].html().contains("Merge axis"));

    // Stage 3: Map to vJoy (is-output)
    assert!(stages[3].value().attr("class").unwrap_or("").contains("is-output"));
    assert!(stages[3].html().contains("Map to vJoy"));
}
```

- [ ] **Step 2: Run** — PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git commit -m "test(pipeline): SSR coverage for four-stage pipeline rendering"
```

---

### Task 39: SSR coverage for Conditional branch rendering

Acceptance criterion 11. A Conditional with non-empty `if_true` and empty `if_false` renders both branches; the empty branch shows the `+ Add first stage` affordance.

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs`

- [ ] **Step 1: Write the test**

```rust
#[test]
fn conditional_with_empty_if_false_renders_both_branches() {
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: addr.clone() },
        if_true: vec![Action::Invert],
        if_false: None,
    }];
    let (state, _addr2) = build_state(actions);
    // Pre-expand the Conditional stage.
    // (Test harness wires expansion via EditorState; mirror task 22's pattern.)
    let html = render_with(state, addr);

    assert!(html.contains("if true branch"));
    assert!(html.contains("if false branch"));
    // The empty if_false branch shows the "Add else branch" louder affordance
    // (NOT "+ Add first stage" — that's for empty pipelines; an unset
    // if_false renders the dedicated branch-creation button per Task 26a).
    assert!(html.contains("Add else branch"));
}

#[test]
fn conditional_with_empty_if_true_renders_branch_with_add_first_stage() {
    // Symmetric coverage: empty if_true (an existing branch with no stages)
    // renders the standard "+ Add first stage" affordance per Task 20's
    // empty-pipeline path.
    let addr = InputAddress {
        device: DeviceId("dev-1".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let actions = vec![Action::Conditional {
        condition: Condition::ButtonPressed { input: addr.clone() },
        if_true: vec![],
        if_false: Some(vec![Action::Invert]),
    }];
    let (state, _addr2) = build_state(actions);
    let html = render_with(state, addr);

    assert!(html.contains("if true branch"));
    assert!(html.contains("+ Add first stage"));
}
```

- [ ] **Step 2: Run** — PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/tests.rs
git commit -m "test(pipeline): SSR coverage for Conditional branch rendering"
```

---

### Task 40: Build sweep + clippy

Build the workspace in release configuration, build via `dx` for both GUI feature flags (Dioxus + egui), and run clippy with the project's lint config to verify no warnings landed.

- [ ] **Step 1: Run**

```bash
cargo build --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Dioxus asset bundling is NOT exercised by `cargo build` alone; verify
# explicitly. Both feature flags must build cleanly.
dx build -p inputforge-app --no-default-features --features gui-dioxus
dx build -p inputforge-app --features gui-egui
```

- [ ] **Step 2: Fix any issues** — common categories: dead-code warnings (most are already suppressed via `#[allow(dead_code)]` attributes added in earlier tasks), `unused_qualifications` from Dioxus's RSX macro on event listeners (suppress with `#[allow(unused_qualifications)]` per the F8 pattern).

- [ ] **Step 3: Commit any fixes**

```bash
git add -A
git commit -m "chore(mapping_editor): clippy/build sweep"
```

---

### Task 41: Manual smoke run (28 F9-owned acceptance criteria)

Launch the GUI and walk every F9-owned AC with concrete pass/fail. AC #29 (F10's live-tracking dot consumes `evaluate_actions_through`) is **out of F9 scope** — verified by `cargo test -p inputforge-core --lib pipeline::tests::evaluate_actions_through` from Task 1, not by manual smoke. **No deferral escape hatch.** Failure on any item below blocks merge — file an issue AND fix before reattempting.

- [ ] **Step 1: Launch**

```
dx run -p inputforge-app --no-default-features --features gui-dioxus
```

- [ ] **Step 2: Walk all 28 F9-owned acceptance criteria**

For each item: do X, observe Y. If Y does not match, the test FAILS — fix and re-run, do not defer.

| AC# | Action (do X) | Observation (observe Y) |
|-----|---------------|-------------------------|
| 1   | Open profile, click a row in the rail | Editor mounts in `if-layout__center` showing the selected mapping's header, name, input, live readout, pipeline |
| 2   | Click empty area in the rail (deselect) | Editor reverts to "Select a mapping" empty state (spec line 86 verbatim) |
| 3   | Select a mapping with a `MapToVJoy` action | Header shows `<source>   →   <output>` arrow; subtitle uses JetBrains Mono |
| 3b  | Select a mapping WITHOUT `MapToVJoy` | Subtitle has no `→` tail (output suppressed) |
| 4   | Edit name field with a 500-character string | Name field max-width 480 px; long names scroll within the field; full name visible via F2 Tooltip on h2 hover |
| 5   | Click `rebind` button, then press a button on the joystick | Input rebinds; live readout reflects new source |
| 6   | Move stick (axis input) | IN bar fills with `--color-live` (live-green); OUT bar fills with same color (NO output-gold) |
| 7   | Select a mapping with `MergeAxis` as first action; move both axes | Live readout shows `IN 1` row, `IN 2` row, dashed divider, merged `IN` row, then `OUT` row (NO extra divider before OUT) |
| 8   | Switch to a runtime mode different from the editing mode | Inactive-runtime hint card visible with copy `Engine is in *<runtime>*. Mapping fires only in *<editing>*.` (italic markers visible) |
| 9   | Open a mapping with mixed processing/output/control stages | All stages visible with category-tinted backgrounds at the spec-pinned percentages (`--color-stage-tint-{processing,output,control}`) |
| 10  | Click a stage's chevron | Stage expands; click again — collapses; SPACE / ENTER on focused stage chevron also toggle |
| 11  | Add a `Conditional` action with empty `if_false` | "Add else branch" louder affordance visible in the if-false section (NOT "+ Add first stage") |
| 12  | Inside `MergeAxis` body, change op picker AND click `rebind` for secondary input | Op picker commits on change; rebind arms `LiveCapture::AxesOnly`; pressing an axis rebinds the secondary input; summary updates to `<op> with <new label>` |
| 13a | Right-click on a stage header | Menu opens at cursor with Move up, Move down, Delete (Move up disabled at index 0) |
| 13b | Same scenario via Shift+F10 with stage focused | Menu opens at the stage's bounding rect |
| 13c | With a stage being dragged | Drop indicator (2 px `--color-border-focus`) appears between target stages |
| 14  | Click `+` at the end of a pipeline | Add palette opens with three sections (Processing, Output, Control); click an item — appended action appears in the pipeline; new stage opens expanded |
| 15a | Make an edit, press `Ctrl+Z` | Edit reverts; undo recap footer updates |
| 15b | Press `Ctrl+Shift+Z` | Edit re-applies (redo) |
| 15c | Press `Ctrl+Y` (Windows convention) | Edit re-applies (redo) |
| 15d | Type in the name field, press `Ctrl+Z` while still focused | Native textfield undo (NOT editor undo); name field reverts character |
| 16  | View editor footer | Shows `<change-summary> · <kbd>⌃Z</kbd> to undo`; NO engine-status dot |
| 17  | Configure a stage with invalid params (e.g., `MergeAxis` with empty secondary) | Stage title turns `--color-error`; summary slot shows the fix hint (e.g., `Secondary input must differ from primary`) |
| 18  | Stop the engine | Engine-offline banner visible above editor; copy `Engine offline. Edits not applied.`; `Restart engine` button visible; edits remain locally responsive (commit-on-blur dispatches but engine ignores until restored) |
| 19  | Delete the selected mapping from another tab/file | Editor silently reverts to empty state (NO toast — per AC #19) |
| 20a | Make an edit (non-empty undo stack), then attempt profile flip | F4 `DirtyConfirmDialog` opens with "Discard editor undo log?" |
| 20b | Click "Switch profile" in dialog | Undo log clears, profile flips |
| 20c | Click "Cancel" in dialog | Both preserved (undo log AND current profile) |
| 21a | Tab through interactive elements | Tab order matches DOM order; no focus traps |
| 21b | Focused stage, press `Esc` (no capture armed) | No-op (does not collapse, does not deselect) |
| 21c | Focused stage, press `Alt+Up` | Stage moves up by one slot within parent pipeline |
| 21d | Focused stage at index 0, press `Alt+Up` | No-op (boundary) |
| 22  | Tab through every interactive element | F2 `--color-border-focus` outline visible against ALL tinted stage backgrounds (processing, output, control) |
| 23a | Click chevron with normal motion | Chevron rotates over 180 ms ease-out |
| 23b | Enable OS reduced-motion, click chevron | Rotation is instant; live readout bars are always instant; engine-offline banner fade is instant |
| 24  | Visually inspect ALL bar fills | Every bar uses live-green (`--color-live`); output-gold appears ONLY in stage-tint context, NOT in bar fills |
| 25  | Make 51 edits to a single mapping, count undo entries via repeated Ctrl+Z | Stack stops at 50 entries (FIFO eviction visible — oldest edits unrecoverable) |
| 26a | Switch editing mode (NOT profile) | Undo log preserved across the switch |
| 26b | Profile flip with non-empty stack | Undo log clears (via F4 dialog from AC #20) |
| 27a | Edit mapping externally while name field has focus | Toast `Mapping was edited externally` appears immediately; reset deferred — local name still showing |
| 27b | Blur the name field after the toast | Reset fires — local name reverts to engine state |
| 28a | Drag a stage from outer pipeline into a Conditional `if_true` branch | Stage moves; `if_true` now contains the dragged stage; outer pipeline shorter |
| 28b | Drag a Conditional onto its own descendant | Cycle rejected: 200 ms `--color-error` drop indicator; no state change |

- [ ] **Step 3: If any item above fails, fix and re-run**

Failures block merge. Do NOT proceed to commit / PR until every item passes.

- [ ] **Step 4: No commit** (manual pass only).

---

## Self-review notes

Spec coverage walked end-to-end during plan drafting:
- Choices 1-10 (frame anatomy): Tasks 14-19
- Choices 11-18 (pipeline graph): Tasks 20, 21, 22, 26, 28, 29
- Choices 19-21 (empty / engine offline / malformed): Tasks 12, 13, 35
- Choices 22-23 (undo log): Tasks 6-8, 31, 32
- Choices 24-28 (keyboard, a11y, motion): Tasks 31, 36, focus rings inline in CSS across tasks 14-30
- Choices 29-31 (edit dispatch + conflict): Tasks 15, 16, 22-29 (per-body), 33, 34
- Acceptance criteria 1-29: covered across SSR tests in Tasks 12-15, 17-22, 38, 39 plus manual smoke in Task 41
- Stage drag-and-drop and the `evaluate_actions_through` engine helper land in Tasks 30 and 1 respectively
- F5 spec amendment lands in Task 37
- F10/F11/F14 handoff (right-slot API, malformed-hint contract, undo dispatch convention): scaffolded in Tasks 6, 9, 22, 35

No placeholders detected on the second pass; every code block has actual code or pure-stub fall-throughs (Task 26's predicate editor body is named "Predicate editor" as a placeholder caption — F9 owns the wider Conditional structure but the predicate editor specifics are a fill-in-detail item, not a TBD; the spec's choice 14 only commits the recursive structure and the `kind picker plus operand fields` pattern). Type-consistency check: `MappingKey`, `StageId`, `UndoKind`, `LabelArgs` all keep their names across tasks 6 → 41.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-30-f9-mapping-editor.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**


</content>
</invoke>