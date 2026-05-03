# F11 Deadzone Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the F11 deadzone editor body (square XY plot with zone-banded background, 4 draggable handles, numeric toolbar, mini zone-bar thumbnail) plugged into F9's StageBody dispatcher, after first extracting a shared `instruments/` module from F10.

**Architecture:** Tasks 1-5 lift cross-instrument concerns out of F10's `response_curve/` into a sibling `instruments/` Rust module + a new `assets/tokens/instruments.css` token sheet (refactor only, behaviour unchanged). Task 6 extends the F2 `NumberInput` primitive with an `oncommit` event. Tasks 7-15 build F11 file-by-file (state, mutation, interaction, keyboard, rendering, thumbnail, toolbar, body wire-up, dispatcher swap). Task 15.5 extracts F10's inline SSR-mount block into a shared `mount_stage_body_test` helper that both F10 and F11 consume. Task 16 ships F11's SSR mount tests. Task 17 backports F11's phantom-undo guard to F10 so the two bodies stay symmetric. Each task is TDD: failing test, minimal implementation, passing test, commit.

**Tech Stack:** Rust 2024 edition · Dioxus 0.7 (desktop / WebView2) · `inputforge-core` engine API (`DeadzoneConfig::new`, `apply`) · CSS custom properties · JS bridge via `document::eval`

**Spec:** [`docs/superpowers/specs/2026-05-02-f11-deadzone-editor-design.md`](../specs/2026-05-02-f11-deadzone-editor-design.md)

**Coding rules:** never use em-dash, en-dash, or `--` substitutes in any text artefact (code, comments, docs). Use comma, colon, semicolon, period, parentheses. No `Co-Authored-By` footer on commits. After each multi-file edit, run `cargo check -p inputforge-gui-dx` before committing; after each module-level edit, also run `cargo test -p inputforge-gui-dx <module>::tests` to scope the run.

---

## File Structure

### New files

| Path | Purpose |
|---|---|
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/mod.rs` | `pub(crate)` re-exports + `INSTR_GLOW_STDDEV: f64 = 0.012` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/nudge_coalesce.rs` | `NudgeCoalesce` struct + `should_merge` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/live_axis.rs` | `compute_live_axis_value` (renamed from F10's `compute_live_value`) |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/bridge.rs` | `BridgeEvent`, `BRIDGE_JS_TEMPLATE`, `mount_mouse_bridge`, `stage_id_dom_id` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs` | `dispatch_stage_edit`, `dispatch_stage_edit_no_undo` (generic over `Action`) |
| `crates/inputforge-gui-dx/assets/tokens/instruments.css` | Shared instrument tokens (`--instr-grid-major`, `--instr-axis-cross`, `--instr-identity-stroke`) |
| `crates/inputforge-gui-dx/assets/frame/deadzone.css` | F11 component-scoped CSS |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/mod.rs` | `DeadzoneBody` Dioxus component |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/state.rs` | `BodyState`, `DragInProgress`, `HandleId` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/mutation.rs` | `handle_positions`, `adjacent_bounds`, `with_handle`, `default_config` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/interaction.rs` | `handle_pointer_down/move/up` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/keyboard.rs` | `KeyInput`, `KeyKind`, `KeyOutcome`, `handle_key` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/rendering.rs` | `render_plot` + private layer fns |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/thumbnail.rs` | `header_thumbnail(config) -> Element` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/toolbar.rs` | `Toolbar` component (4 `NumberInput` + `Reset`) |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/tests.rs` | SSR mount tests (cfg(test)) |

### Modified files (F10 / F2 / dispatcher / theme)

| Path | Change |
|---|---|
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs` | New `mod deadzone;` and `mod instruments;`; replace two `Action::Deadzone` arms |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs` | Remove inlined helpers, import from `instruments::*`, rename one fn |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/state.rs` | Replace `last_nudge_at_ms` + `last_nudge_key` with embedded `NudgeCoalesce` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/keyboard.rs` | Update merge-vs-push logic to call `NudgeCoalesce::should_merge` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/toolbar.rs` | `dispatch_curve_edit*` becomes a thin wrapper around `instruments::stage_dispatch::dispatch_stage_edit*` |
| `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/rendering.rs` | Glow filter id `if-curve-glow` becomes `if-instr-glow`; consume `INSTR_GLOW_STDDEV` |
| `crates/inputforge-gui-dx/assets/frame/response_curve.css` | Reference `--instr-grid-major`/`--instr-axis-cross`; `filter: url(#if-instr-glow)` |
| `crates/inputforge-gui-dx/src/components/number_input.rs` | New `oncommit: Option<EventHandler<f64>>` prop with Enter/blur wiring |
| `crates/inputforge-gui-dx/src/theme/mod.rs` | Register `INSTRUMENTS_TOKENS_CSS` (in tokens block) and `DEADZONE_CSS` (in frame block) |

---

## Task 1: Extract `instruments::nudge_coalesce`

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/mod.rs`
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/nudge_coalesce.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs` (add `mod instruments;`)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/state.rs` (replace two fields with embedded struct)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/keyboard.rs` (call `should_merge`)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs` (`on_focus_out` resets the struct)

- [ ] **Step 1: Write the failing test**

Add to `instruments/nudge_coalesce.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::mapping_editor::pipeline::stage_body::response_curve::keyboard::KeyKind;

    #[test]
    fn no_prior_nudge_returns_false() {
        let coalesce = NudgeCoalesce::default();
        assert!(!coalesce.should_merge(0, KeyKind::ArrowLeft));
    }

    #[test]
    fn same_key_within_window_returns_true() {
        let mut coalesce = NudgeCoalesce::default();
        coalesce.record(100, KeyKind::ArrowRight);
        assert!(coalesce.should_merge(200, KeyKind::ArrowRight));
    }

    #[test]
    fn same_key_past_window_returns_false() {
        let mut coalesce = NudgeCoalesce::default();
        coalesce.record(100, KeyKind::ArrowRight);
        assert!(!coalesce.should_merge(100 + 251, KeyKind::ArrowRight));
    }

    #[test]
    fn different_key_within_window_returns_false() {
        let mut coalesce = NudgeCoalesce::default();
        coalesce.record(100, KeyKind::ArrowRight);
        assert!(!coalesce.should_merge(150, KeyKind::ArrowLeft));
    }

    #[test]
    fn reset_clears_prior_state() {
        let mut coalesce = NudgeCoalesce::default();
        coalesce.record(100, KeyKind::ArrowRight);
        coalesce.reset();
        assert!(!coalesce.should_merge(150, KeyKind::ArrowRight));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx instruments::nudge_coalesce::tests --no-run`
Expected: compile error: `unresolved import` or `module not found`.

- [ ] **Step 3: Write the module skeleton + struct**

Create `instruments/mod.rs`:

```rust
// Rust guideline compliant 2026-05-03

//! Cross-instrument shared infrastructure for `StageBody` editors (F10, F11,
//! and future signature instruments). Each helper here has at least two
//! consumers; helpers with only one consumer stay inside their owning editor.

pub(crate) mod bridge;
pub(crate) mod live_axis;
pub(crate) mod nudge_coalesce;
pub(crate) mod stage_dispatch;

/// SVG `feGaussianBlur` standard deviation used by every instrument's curve
/// glow filter. Pinned in Rust (rather than CSS) because SVG attributes do
/// not resolve CSS custom properties.
pub(crate) const INSTR_GLOW_STDDEV: f64 = 0.012;
```

Create `instruments/nudge_coalesce.rs`:

```rust
// Rust guideline compliant 2026-05-03

//! Shared keyboard-nudge undo coalesce: same-`(stage_id, key)` repeats
//! arriving within `COALESCE_WINDOW_MS` merge into the prior undo entry.
//! Embedded as a field on each editor's `BodyState`. Generic over the
//! editor-specific `KeyKind` so `instruments/` carries no back-import to F10
//! or F11; each editor instantiates `NudgeCoalesce<KeyKind>` with its own
//! local enum.

const COALESCE_WINDOW_MS: u64 = 250;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NudgeCoalesce<K: Copy + Eq> {
    last_at_ms: Option<u64>,
    last_key: Option<K>,
}

impl<K: Copy + Eq> NudgeCoalesce<K> {
    /// Decide whether a nudge at `now_ms` for `key` should merge into the
    /// previously-recorded entry (true) or push a new undo entry (false).
    /// The caller invokes `record` after dispatching to persist the new
    /// timestamp/key for the next merge decision.
    pub(crate) fn should_merge(&self, now_ms: u64, key: K) -> bool {
        match (self.last_at_ms, self.last_key) {
            (Some(prev), Some(prev_key)) => {
                prev_key == key && now_ms.saturating_sub(prev) <= COALESCE_WINDOW_MS
            }
            _ => false,
        }
    }

    pub(crate) fn record(&mut self, now_ms: u64, key: K) {
        self.last_at_ms = Some(now_ms);
        self.last_key = Some(key);
    }

    pub(crate) fn reset(&mut self) {
        self.last_at_ms = None;
        self.last_key = None;
    }
}
```

The existing failing tests (Step 1 above) instantiate `NudgeCoalesce::<KeyKind>` implicitly through the `record` / `should_merge` calls; they remain valid as written and document the F10 specialisation.

Add `mod instruments;` (NOT `pub mod`, crate-private) to `stage_body/mod.rs` near the other module declarations:

```rust
mod conditional;
mod instruments;
mod invert;
mod map_to_keyboard;
// ...
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p inputforge-gui-dx instruments::nudge_coalesce::tests`
Expected: 5 passed.

- [ ] **Step 5: Migrate F10 `BodyState` to embed `NudgeCoalesce`**

In `response_curve/state.rs`, replace the two fields:

```rust
// BEFORE:
pub last_nudge_at_ms: Option<u64>,
pub last_nudge_key: Option<KeyKind>,

// AFTER:
pub nudge_coalesce: crate::frame::mapping_editor::pipeline::stage_body::instruments::nudge_coalesce::NudgeCoalesce<KeyKind>,
```

Update `BodyState::default()` to drop the two old field initializers and rely on `NudgeCoalesce::default()` (which is `Default`-derived).

Add `use` shim to `state.rs`:

```rust
use crate::frame::mapping_editor::pipeline::stage_body::instruments::nudge_coalesce::NudgeCoalesce;
use crate::frame::mapping_editor::pipeline::stage_body::response_curve::keyboard::KeyKind;
```

Then replace the field type:

```rust
pub nudge_coalesce: NudgeCoalesce<KeyKind>,
```

- [ ] **Step 6: Update F10 `keyboard.rs` to call `should_merge` / `record`**

Search `response_curve/keyboard.rs` for the existing same-key-within-window branch (around the nudge handlers; the file's existing `same_key_within_window_merges_undo` test is the canonical reference for the touched code path). Replace direct field reads of `last_nudge_at_ms` / `last_nudge_key` with:

```rust
let merge = state.nudge_coalesce.should_merge(now_ms, kind);
state.nudge_coalesce.record(now_ms, kind);
let outcome = if merge {
    KeyOutcome::MergeUndo
} else {
    KeyOutcome::PushUndo { label: format!("curve: nudge {label_suffix}") }
};
```

(The existing code computes the same boolean; this just routes it through the new struct's API.)

- [ ] **Step 7: Update F10 `response_curve/mod.rs` `on_focus_out`**

```rust
let on_focus_out = move |_| {
    body_for_focusout.with_mut(|s| {
        s.nudge_coalesce.reset();
    });
};
```

- [ ] **Step 8: Delete the now-orphaned `KEY_COALESCE_WINDOW_MS`**

The constant `KEY_COALESCE_WINDOW_MS = 250` lives at `response_curve/keyboard.rs:27` and was the sole window-value source pre-migration. After Step 6 routes the merge decision through `NudgeCoalesce::should_merge`, the constant has no remaining consumers (`NudgeCoalesce` owns the equivalent `COALESCE_WINDOW_MS` internally). Project clippy is strict on `dead_code`; leaving the const would fail Step 9's `cargo test`.

Delete the line in `response_curve/keyboard.rs`. If a doc-comment references the constant, rephrase to point at `instruments::nudge_coalesce::COALESCE_WINDOW_MS` instead.

- [ ] **Step 9: Run F10 tests to verify behaviour unchanged**

Run: `cargo test -p inputforge-gui-dx response_curve::keyboard::tests::same_key_within_window_merges_undo response_curve::keyboard::tests::same_key_after_window_pushes_new_undo`
Expected: both passed.

Run: `cargo test -p inputforge-gui-dx response_curve::state::tests`
Expected: all passed (existing tests reference the renamed field; update them inline if they assert the old field names).

Run: `cargo test -p inputforge-gui-dx`
Expected: full crate green.

- [ ] **Step 10: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/nudge_coalesce.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/state.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/keyboard.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs
git commit -m "refactor(stage_body): extract NudgeCoalesce into shared instruments module"
```

---

## Task 2: Extract `instruments::live_axis`

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/live_axis.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs` (delete `compute_live_value`, call `instruments::live_axis::compute_live_axis_value` instead)

- [ ] **Step 1: Write the failing test**

Add to `instruments/live_axis.rs` (build the test fixture from the same shape F10's tests use):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};
    use inputforge_core::types::InputAddress;

    fn nested_stage_id() -> StageId {
        StageId(vec![StageIdSegment::Index(0), StageIdSegment::IfTrue])
    }

    fn unbound_addr() -> InputAddress {
        InputAddress::Unbound
    }

    #[test]
    fn nested_stage_returns_none() {
        let id = nested_stage_id();
        // ctx and actions intentionally not constructed: the gate trips before they are read.
        let result = gate_top_level(&id);
        assert!(result.is_none());
    }

    #[test]
    fn unbound_addr_returns_none() {
        let device = unbound_addr().device();
        assert!(device.is_none());
    }
}
```

(`gate_top_level` is a thin extracted helper; see step 3. Note: these tests exercise only the gate predicate and the `InputAddress::device()` branch; the full `compute_live_axis_value` body is NOT exercised here. The existing F10 file has zero tests of `compute_live_value` either, so this is parity, not a regression. End-to-end coverage of the live-tracking dot lands via Task 16's SSR mount tests.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx instruments::live_axis::tests --no-run`
Expected: compile error: `unresolved import` or `cannot find function gate_top_level`.

- [ ] **Step 3: Move `compute_live_value` into the new module, renaming**

Create `instruments/live_axis.rs`. Copy the body of F10's `compute_live_value` from `response_curve/mod.rs` (currently around line 119), rename to `compute_live_axis_value`, and adjust the visibility / imports so it lives at the new path:

```rust
// Rust guideline compliant 2026-05-03

//! Project the engine's live-axis reading through the F10/F11 pipeline so
//! each instrument can render the live tracking dot at the right viewBox
//! coordinate. Gates: top-level stage, bound input, connected device, axis
//! input. Any failed gate returns `None` (no dot, no guides).

use inputforge_core::action::Action;
use inputforge_core::pipeline::evaluate_actions_through;
use inputforge_core::types::{InputAddress, InputValue};

use crate::context::AppContext;
use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

/// Internal gate exposed for unit testing. Returns the top-level stage index
/// when `stage_id` is exactly `[Index(n)]`, else `None`.
pub(crate) fn gate_top_level(stage_id: &StageId) -> Option<usize> {
    match stage_id.0.as_slice() {
        [StageIdSegment::Index(n)] => Some(*n),
        _ => None,
    }
}

pub(crate) fn compute_live_axis_value(
    stage_id: &StageId,
    addr: &InputAddress,
    ctx: &AppContext,
    actions: &[Action],
) -> Option<f64> {
    let stop_at = gate_top_level(stage_id)?;
    let device_id = addr.device()?;
    let _ = ctx.live.read();
    let state_guard = ctx.state.try_read()?;
    let device_present = state_guard
        .devices
        .iter()
        .any(|d| &d.info.id == device_id && d.connected);
    if !device_present {
        return None;
    }
    let value = evaluate_actions_through(actions, &state_guard, addr, stop_at);
    drop(state_guard);
    match value {
        InputValue::Axis { value, .. } => Some(value.value()),
        _ => None,
    }
}
```

- [ ] **Step 4: Delete `compute_live_value` from `response_curve/mod.rs`**

Remove the function definition and update its single caller (also in `response_curve/mod.rs`):

```rust
let live_value: Option<f64> = crate::frame::mapping_editor::pipeline::stage_body::instruments::live_axis::compute_live_axis_value(
    &stage_id, &mapping_key.1, &ctx, &live_actions,
);
```

(Or add a `use` shorthand at the top of the file.)

- [ ] **Step 5: Run tests to verify behaviour preserved**

Run: `cargo test -p inputforge-gui-dx instruments::live_axis::tests`
Expected: 2 passed.

Run: `cargo test -p inputforge-gui-dx response_curve::`
Expected: full F10 module green; `live_value`-dependent tests still pass.

Run: `cargo check -p inputforge-gui-dx`
Expected: no warnings, no errors.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/live_axis.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs
git commit -m "refactor(stage_body): move compute_live_value to instruments::live_axis"
```

---

## Task 3: Extract `instruments::bridge`

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/bridge.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs` (delete `BridgeEvent`, `BRIDGE_JS_TEMPLATE`, `stage_id_dom_id`; rewrite `on_mounted` to call `mount_mouse_bridge`)

The bridge factory takes the full per-event dispatch closure as `dispatch_fn`; this is the principled boundary between "shared infrastructure" (JS install, listener cleanup, event parse, rect projection) and "per-editor state" (per-arm match on `kind`, signal updates, dispatch).

- [ ] **Step 1: Write the failing test**

Add to `instruments/bridge.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

    #[test]
    fn dom_id_for_top_level_stage() {
        let id = StageId(vec![StageIdSegment::Index(2)]);
        assert_eq!(stage_id_dom_id("if-curve-plot", &id), "if-curve-plot-i2");
    }

    #[test]
    fn dom_id_for_nested_stage() {
        let id = StageId(vec![
            StageIdSegment::Index(0),
            StageIdSegment::IfTrue,
            StageIdSegment::Index(1),
        ]);
        assert_eq!(
            stage_id_dom_id("if-deadzone-plot", &id),
            "if-deadzone-plot-i0-t-i1"
        );
    }

    #[test]
    fn template_has_placeholder() {
        assert!(BRIDGE_JS_TEMPLATE.contains("__PLOT_ID__"));
    }

    #[test]
    fn parses_event_payload() {
        let raw = r#"{"kind":"down","x":120,"y":80,"rl":10,"rt":20,"rs":300}"#;
        let evt: BridgeEvent = serde_json::from_str(raw).unwrap();
        assert_eq!(evt.kind, "down");
        assert_eq!(evt.x, 120.0);
        assert_eq!(evt.rs, 300.0);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx instruments::bridge::tests --no-run`
Expected: compile error: `cannot find function stage_id_dom_id`.

- [ ] **Step 3: Move bridge code into the new module**

Create `instruments/bridge.rs`. Copy `BRIDGE_JS_TEMPLATE` and `BridgeEvent` from `response_curve/mod.rs` verbatim. Generalise `stage_id_dom_id`: F10's version hardcodes the prefix `"if-curve-plot"`; the new shared version takes the prefix as a parameter:

```rust
// Rust guideline compliant 2026-05-03

//! Document-level mouse-event bridge for instrument plots. Dioxus 0.7 desktop
//! does not deliver mousedown/move/up/dblclick/contextmenu to non-button
//! divs, so we install raw addEventListener handlers via `document::eval`,
//! parse the JSON payload back through the eval channel, and route each event
//! to the per-instrument dispatch closure provided by the caller.

use std::fmt::Write as _;

use dioxus::prelude::*;
use serde::Deserialize;

use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

pub(crate) const BRIDGE_JS_TEMPLATE: &str = r"
    var plotEl = document.getElementById('__PLOT_ID__');
    if (!plotEl) return;
    var plotId = '__PLOT_ID__';
    var dragging = false;

    var sendEvt = function(kind, e) {
        var r = plotEl.getBoundingClientRect();
        dioxus.send({
            kind: kind,
            x: e.clientX | 0,
            y: e.clientY | 0,
            rl: r.left,
            rt: r.top,
            rs: Math.min(r.width, r.height),
        });
    };

    var inPlot = function(e) {
        var r = plotEl.getBoundingClientRect();
        return e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    };

    plotEl.addEventListener('mousedown', function(e) {
        if (e.button !== 0) return;
        dragging = true;
        sendEvt('down', e);
    });

    document.addEventListener('mousemove', function(e) {
        if (!document.getElementById(plotId)) return;
        if (!dragging && !inPlot(e)) return;
        sendEvt('move', e);
    });

    document.addEventListener('mouseup', function(e) {
        if (!document.getElementById(plotId)) return;
        if (e.button !== 0) return;
        if (!dragging) return;
        dragging = false;
        sendEvt('up', e);
    });

    plotEl.addEventListener('dblclick', function(e) {
        sendEvt('dbl', e);
    });

    plotEl.addEventListener('contextmenu', function(e) {
        e.preventDefault();
        if (dragging) {
            dragging = false;
            sendEvt('up', e);
        }
        sendEvt('ctx', e);
    });
";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BridgeEvent {
    pub kind: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub rl: f64,
    #[serde(default)]
    pub rt: f64,
    #[serde(default)]
    pub rs: f64,
}

pub(crate) fn stage_id_dom_id(prefix: &str, stage_id: &StageId) -> String {
    let mut s = String::from(prefix);
    for seg in &stage_id.0 {
        match seg {
            StageIdSegment::Index(n) => {
                let _ = write!(s, "-i{n}");
            }
            StageIdSegment::IfTrue => s.push_str("-t"),
            StageIdSegment::IfFalse => s.push_str("-f"),
        }
    }
    s
}

/// Mount the JS bridge for a plot identified by `plot_dom_id`. Returns an
/// `EventHandler<MountedEvent>` to attach via `onmounted: ...`. Each parsed
/// `BridgeEvent` is forwarded to `dispatch_fn`; the spawned task self-exits
/// when the eval channel closes (component unmount).
pub(crate) fn mount_mouse_bridge(
    plot_dom_id: String,
    dispatch_fn: impl Fn(BridgeEvent) + Clone + 'static,
) -> EventHandler<MountedEvent> {
    EventHandler::new(move |_evt: MountedEvent| {
        let id = plot_dom_id.clone();
        let dispatch_fn = dispatch_fn.clone();
        spawn(async move {
            let js = BRIDGE_JS_TEMPLATE.replace("__PLOT_ID__", &id);
            let mut handle = document::eval(&js);
            loop {
                let Ok(payload) = handle.recv::<BridgeEvent>().await else {
                    break;
                };
                dispatch_fn(payload);
            }
        });
    })
}
```

- [ ] **Step 4: Update F10 `response_curve/mod.rs` to consume the shared bridge**

Delete F10's local `BRIDGE_JS_TEMPLATE`, `BridgeEvent`, and `stage_id_dom_id`. Replace the bridge wiring inside `ResponseCurveBody` so `on_mounted` becomes:

```rust
use crate::frame::mapping_editor::pipeline::stage_body::instruments::bridge::{
    mount_mouse_bridge, BridgeEvent,
};

let plot_dom_id =
    crate::frame::mapping_editor::pipeline::stage_body::instruments::bridge::stage_id_dom_id(
        "if-curve-plot",
        &stage_id,
    );
let curve_for_bridge = curve.clone();
let mapping_key_for_bridge = mapping_key_for_evt.clone();
let stage_id_for_bridge = stage_id_for_evt.clone();
let cmd_tx_for_bridge = cmd_tx.clone();
let dispatch = move |payload: BridgeEvent| {
    dispatch_bridge_event(
        &payload,
        body,
        working_curve,
        config_signal,
        undo_log,
        malformed_hints,
        &mapping_key_for_bridge,
        &stage_id_for_bridge,
        &curve_for_bridge,
        &cmd_tx_for_bridge,
    );
};
let on_mounted = mount_mouse_bridge(plot_dom_id.clone(), dispatch);
```

`dispatch_bridge_event` keeps its existing signature (per-editor state, F10-shaped); this task only changes the JS install / channel boundary.

- [ ] **Step 5: Run tests**

Run: `cargo test -p inputforge-gui-dx instruments::bridge::tests`
Expected: 4 passed.

Run: `cargo check -p inputforge-gui-dx`
Expected: clean.

- [ ] **Step 6: Manual smoke (human only, not for agent execution)**

Run: `dx run -p inputforge-app`. Drag a curve anchor; behaviour identical to before. Quit when verified.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/bridge.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs
git commit -m "refactor(stage_body): extract JS bridge into instruments::bridge"
```

---

## Task 4: Extract `instruments::stage_dispatch`

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/toolbar.rs` (delete `dispatch_curve_edit*`; have callers call the new shared functions directly)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs` (callers in `dispatch_bridge_event` and `on_key`)

The shared functions accept a full `Action` payload (not a `ResponseCurve`); F10 callers wrap their value as `Action::ResponseCurve { curve }` at the call site, F11 callers will wrap as `Action::Deadzone { config }`.

The module is structured as a pure helper (`dispatch_stage_edit_into`) over `&mut UndoLog`, plus a 3-line `Signal<UndoLog>`-wrapping public form. Tests target the helper, so they need neither a `VirtualDom` nor a `Signal::new` outside the runtime (which would leak per `app.rs:41`).

- [ ] **Step 1: Write the failing tests**

Add to `instruments/stage_dispatch.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::action::Action;
    use inputforge_core::engine::EngineCommand;
    use inputforge_core::processing::curves::ResponseCurve;
    use inputforge_core::types::InputAddress;
    use std::sync::mpsc;

    use crate::frame::mapping_editor::undo_log::StageIdSegment;

    fn linear_curve() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false)
            .expect("linear curve is valid")
    }

    #[test]
    fn dispatch_with_undo_sends_set_mapping_and_pushes_undo() {
        let (tx, rx) = mpsc::channel::<EngineCommand>();
        let key: MappingKey = ("default".into(), InputAddress::Unbound);
        let actions_before = vec![Action::ResponseCurve { curve: linear_curve() }];
        let mut undo_log = UndoLog::default();
        let stage_id = StageId(vec![StageIdSegment::Index(0)]);

        dispatch_stage_edit_into(
            &mut undo_log,
            &actions_before,
            &stage_id,
            Action::ResponseCurve { curve: linear_curve() },
            &key,
            None,
            &tx,
            "test: dispatch".to_owned(),
        );

        let cmd = rx.try_recv().expect("SetMapping should be sent");
        match cmd {
            EngineCommand::SetMapping { actions, .. } => {
                assert_eq!(actions.len(), 1);
            }
            _ => panic!("expected SetMapping"),
        }
        let entries = undo_log.stacks.get(&key).map(|h| h.undo.len()).unwrap_or(0);
        assert_eq!(entries, 1);
    }

    #[test]
    fn dispatch_no_undo_sends_command_but_skips_undo() {
        let (tx, rx) = mpsc::channel::<EngineCommand>();
        let key: MappingKey = ("default".into(), InputAddress::Unbound);
        let actions_before = vec![Action::ResponseCurve { curve: linear_curve() }];
        let stage_id = StageId(vec![StageIdSegment::Index(0)]);

        dispatch_stage_edit_no_undo(
            &actions_before,
            &stage_id,
            Action::ResponseCurve { curve: linear_curve() },
            &key,
            None,
            &tx,
        );

        assert!(matches!(rx.try_recv(), Ok(EngineCommand::SetMapping { .. })));
    }

    #[test]
    fn dispatch_with_invalid_path_drops_silently() {
        let (tx, rx) = mpsc::channel::<EngineCommand>();
        let key: MappingKey = ("default".into(), InputAddress::Unbound);
        let actions_before: Vec<Action> = vec![];
        let mut undo_log = UndoLog::default();
        let stage_id = StageId(vec![StageIdSegment::Index(99)]);

        dispatch_stage_edit_into(
            &mut undo_log,
            &actions_before,
            &stage_id,
            Action::Invert,
            &key,
            None,
            &tx,
            "test: bad path".to_owned(),
        );

        assert!(rx.try_recv().is_err(), "no SetMapping for invalid path");
        let entries = undo_log.stacks.get(&key).map(|h| h.undo.len()).unwrap_or(0);
        assert_eq!(entries, 0);
    }
}
```

Imports for the test module: `MappingKey` is a type alias `(String, InputAddress)` (`frame/view_state.rs:20`), so the `let key: MappingKey = (...)` form is required (NOT a tuple-struct constructor). `ResponseCurve::piecewise_linear` takes `(points, symmetric)` per `crates/inputforge-core/src/processing/curves.rs`; the in-repo call site at `response_curve/state.rs:124` is the canonical pattern. `UndoLog::stacks` is the public field per `undo_log.rs:73-86`; entries land in `MappingHistory.undo: Vec<UndoEntry>`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx instruments::stage_dispatch::tests --no-run`
Expected: compile error: `cannot find function dispatch_stage_edit_into`.

- [ ] **Step 3: Write the shared dispatch functions**

Create `instruments/stage_dispatch.rs`:

```rust
// Rust guideline compliant 2026-05-03

//! Shared `SetMapping` dispatch + undo bookkeeping for instrument bodies.
//! Generic over the new `Action` payload: F10 passes
//! `Action::ResponseCurve { curve }`, F11 passes `Action::Deadzone { config }`.
//!
//! Two-layer design: `dispatch_stage_edit_into` is the pure helper
//! (`&mut UndoLog`); `dispatch_stage_edit` is a Signal-wrapping wrapper for
//! Dioxus call sites. Tests target the helper so they do not need a
//! `VirtualDom` or a runtime-context `Signal`.

use std::sync::mpsc::Sender;

use dioxus::prelude::Signal;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::Mapping;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{StageId, UndoKind, UndoLog};

/// Pure helper: takes `&mut UndoLog` directly. Test-friendly.
#[expect(
    clippy::too_many_arguments,
    reason = "F9 convention; matches dispatch_input_field_edit signature"
)]
pub(crate) fn dispatch_stage_edit_into(
    undo_log: &mut UndoLog,
    actions_before: &[Action],
    stage_id: &StageId,
    new_action: Action,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
    label: String,
) {
    let Some(new_actions) = replace_at_path(actions_before, stage_id, new_action) else {
        return;
    };
    let before = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: name.clone(),
        actions: actions_before.to_vec(),
    };
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(
            target: "instruments::stage_dispatch",
            action = "set_mapping_drop_offline",
            "dropped SetMapping command: receiver disconnected"
        );
        return;
    }
    undo_log.push_edit(mapping_key.clone(), before, UndoKind::StageEdit, label);
}

/// Signal-wrapping public form. Body call sites pass their `Signal<UndoLog>`
/// here; the wrapper takes the `write()` borrow once and threads it into the
/// helper.
#[expect(
    clippy::too_many_arguments,
    reason = "matches dispatch_stage_edit_into signature plus the Signal handle"
)]
pub(crate) fn dispatch_stage_edit(
    actions_before: &[Action],
    stage_id: &StageId,
    new_action: Action,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<UndoLog>,
    label: String,
) {
    let mut guard = undo_log.write();
    dispatch_stage_edit_into(
        &mut guard,
        actions_before,
        stage_id,
        new_action,
        mapping_key,
        name,
        cmd_tx,
        label,
    );
}

pub(crate) fn dispatch_stage_edit_no_undo(
    actions_before: &[Action],
    stage_id: &StageId,
    new_action: Action,
    mapping_key: &MappingKey,
    name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
) {
    let Some(new_actions) = replace_at_path(actions_before, stage_id, new_action) else {
        return;
    };
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(
            target: "instruments::stage_dispatch",
            action = "set_mapping_no_undo_drop_offline",
            "dropped no-undo SetMapping command: receiver disconnected"
        );
    }
}
```

`dispatch_stage_edit_no_undo` does not touch the undo log; no helper split needed for it.

- [ ] **Step 4: Migrate F10 callers**

In `response_curve/toolbar.rs`, delete `dispatch_curve_edit` and `dispatch_curve_edit_no_undo`. Add re-exports for backward compat OR update the call sites. Recommended: update the call sites so the API is one canonical name.

Find the call sites in `response_curve/mod.rs`:

- `dispatch_bridge_event` "up" arm: replace each `toolbar::dispatch_curve_edit(actions_snap, stage_id, valid, mapping_key, name, cmd_tx, undo_log, label)` with:

```rust
crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::dispatch_stage_edit(
    &actions_snap,
    stage_id,
    Action::ResponseCurve { curve: valid },
    mapping_key,
    name,
    cmd_tx,
    &mut undo_log,
    label,
);
```

- `dispatch_bridge_event` "dbl" and "ctx" arms: same migration.
- `on_key` `KeyOutcome::PushUndo` arm: same.
- `on_key` `KeyOutcome::MergeUndo` arm: replace `toolbar::dispatch_curve_edit_no_undo(...)` with `instruments::stage_dispatch::dispatch_stage_edit_no_undo(..., Action::ResponseCurve { curve: new }, ...)`.

After migration, `toolbar.rs` no longer exports the dispatch helpers.

- [ ] **Step 5: Run tests**

Run: `cargo test -p inputforge-gui-dx instruments::stage_dispatch::tests`
Expected: 3 passed.

Run: `cargo test -p inputforge-gui-dx response_curve::`
Expected: full F10 module green.

Run: `cargo check -p inputforge-gui-dx`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/instruments/stage_dispatch.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/toolbar.rs
git commit -m "refactor(stage_body): hoist dispatch helpers into instruments::stage_dispatch"
```

---

## Task 5: Lift CSS tokens + rename glow filter id

**Files:**
- Create: `crates/inputforge-gui-dx/assets/tokens/instruments.css`
- Modify: `crates/inputforge-gui-dx/assets/frame/response_curve.css` (consume new tokens; rename `if-curve-glow` to `if-instr-glow`)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/rendering.rs` (filter id rename; consume `instruments::INSTR_GLOW_STDDEV`)
- Modify: `crates/inputforge-gui-dx/src/theme/mod.rs` (register new token sheet in tokens block)

- [ ] **Step 1: Create the new shared token sheet**

Create `crates/inputforge-gui-dx/assets/tokens/instruments.css`:

```css
/* Shared instrument visual tokens (F10 + F11 + future signature instruments). */
:root {
  --instr-grid-major: rgba(150, 165, 220, 0.18);
  --instr-axis-cross: rgba(150, 165, 220, 0.32);
  --instr-identity-stroke: var(--color-text-subtle);
}
```

- [ ] **Step 2: Amend `response_curve.css` to consume the shared tokens**

In `assets/frame/response_curve.css`, change the `.if-curve` block so the curve-grid / axis-cross tokens reference the shared `--instr-*` tokens, and rebind `--color-curve-identity` (the variable name does not change; only its right-hand side rebinds to `var(--instr-identity-stroke)`):

```css
.if-curve {
  --color-curve-plot-bg: var(--color-bg-sunken);
  --color-curve-grid-major: var(--instr-grid-major);
  --color-curve-axis-cross: var(--instr-axis-cross);
  --color-curve-identity: var(--instr-identity-stroke);
  --color-curve-stroke: var(--color-primary);
  --color-curve-handle: rgb(from var(--color-primary) r g b / 0.40);
  --color-curve-anchor-fill: var(--color-text);
  --color-curve-anchor-stroke: var(--color-curve-plot-bg);

  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3);
  width: 100%;
}
```

Then rename the two `filter: url(#if-curve-glow);` references at lines 84 and 122 to `filter: url(#if-instr-glow);`. The selectors `.if-curve__path` and `.if-curve__live-dot` keep their names; only the filter id changes.

- [ ] **Step 3: Update F10 rendering to emit the renamed filter and consume the shared constant**

In `response_curve/rendering.rs::render_plot`:

```rust
use crate::frame::mapping_editor::pipeline::stage_body::instruments::INSTR_GLOW_STDDEV;
// ...
defs {
    filter {
        id: "if-instr-glow",
        x: "-50%", y: "-50%", width: "200%", height: "200%",
        feGaussianBlur { std_deviation: "{INSTR_GLOW_STDDEV}" }
    }
}
```

Delete the old `const GLOW_STDDEV` if it lives in `rendering.rs`. (Search for `GLOW_STDDEV` in the file: there is one definition that becomes redundant after this change.)

- [ ] **Step 4: Register the new token sheet in `ThemeProvider`**

In `crates/inputforge-gui-dx/src/theme/mod.rs`, add the asset declaration alongside the other token sheets and the `Stylesheet` mount inside the tokens block:

```rust
const INSTRUMENTS_TOKENS_CSS: Asset = asset!("/assets/tokens/instruments.css");
```

Inside the `rsx!` body of `ThemeProvider`, after `MOTION_CSS` and before `GLOBAL_CSS`:

```rust
Stylesheet { href: MOTION_CSS }
Stylesheet { href: INSTRUMENTS_TOKENS_CSS }
Stylesheet { href: GLOBAL_CSS }
```

(Token sheets sit lowest in the cascade; instruments tokens build on the colour primitives so they slot in after the colour/typography/spacing/radii/elevation/motion stack.)

- [ ] **Step 5: Run tests**

Run: `cargo check -p inputforge-gui-dx`
Expected: clean.

Run: `cargo test -p inputforge-gui-dx response_curve::`
Expected: full F10 module green. F10's tests reference the glow filter id only inside `rendering.rs` snapshots; check those snapshot strings and update from `if-curve-glow` to `if-instr-glow` if any test asserts the literal string.

- [ ] **Step 6: Manual smoke (human only, not for agent execution)**

Run: `dx run -p inputforge-app`. Open a curve editor; the curve glow renders identically. Quit when verified.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/assets/tokens/instruments.css \
        crates/inputforge-gui-dx/assets/frame/response_curve.css \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/rendering.rs \
        crates/inputforge-gui-dx/src/theme/mod.rs
git commit -m "refactor(theme): lift instrument tokens to shared sheet, rename glow filter"
```

---

## Task 6: Add `oncommit` event to `NumberInput`

**Files:**
- Modify: `crates/inputforge-gui-dx/src/components/number_input.rs`

The new prop fires when the user finishes editing (Enter pressed or input blurs), with the post-clamp value. Free-typing continues to fire `oninput` only. F11's toolbar is the only initial consumer; existing call sites are unaffected because the prop is `Option`-typed.

NOTE on test scope: the unit tests below exercise only the `parse_and_clamp` helper. They do NOT exercise the Dioxus event wiring (FocusEvent value reading, document::eval blur trigger). Behavioural confirmation lands in F11's toolbar SSR mount tests (Task 16) which assert the rendered field structure. F2 `NumberInput` already ships with the same test posture (parse-only unit tests).

- [ ] **Step 1: Write the failing test**

Add to `number_input.rs` (or a sibling `tests.rs` if the file already has one):

```rust
#[cfg(test)]
mod oncommit_tests {
    use super::*;

    /// Helper: parse a string the same way the component will, then clamp
    /// to `[min, max]`. Mirrors the production logic so the test guarantees
    /// the post-clamp contract without spinning a virtual DOM.
    fn parse_and_clamp(raw: &str, min: f64, max: f64) -> Option<f64> {
        let v: f64 = raw.parse().ok()?;
        Some(v.min(max).max(min))
    }

    #[test]
    fn enter_clamps_above_max() {
        assert_eq!(parse_and_clamp("1.5", -1.0, 1.0), Some(1.0));
    }

    #[test]
    fn blur_clamps_below_min() {
        assert_eq!(parse_and_clamp("-2.5", -1.0, 1.0), Some(-1.0));
    }

    #[test]
    fn invalid_text_returns_none() {
        assert_eq!(parse_and_clamp("abc", -1.0, 1.0), None);
    }

    #[test]
    fn comma_decimal_returns_none() {
        // Locale-aware parsing is out of F11 scope; comma decimals fail to parse.
        assert_eq!(parse_and_clamp("0,5", -1.0, 1.0), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx number_input::oncommit_tests --no-run`
Expected: compile error (the test references a function not yet defined) or test failure.

- [ ] **Step 3: Add the prop and wiring to `NumberInput`**

In `components/number_input.rs::NumberInput`, add the new prop after `onstep`:

```rust
/// Emits the post-parse, post-clamp value when the user finishes editing
/// (Enter pressed or input loses focus). Free-typing fires `oninput` only.
oncommit: Option<EventHandler<f64>>,
```

Add a small private helper used both by the production handlers and by the new tests:

```rust
fn parse_and_clamp(raw: &str, min: f64, max: f64) -> Option<f64> {
    let v: f64 = raw.parse().ok()?;
    Some(v.min(max).max(min))
}
```

Build the Enter / blur handlers using the canonical blur-via-JS pattern from `crates/inputforge-gui-dx/src/frame/mapping_editor/header.rs:288-311`. In Dioxus 0.7 in this codebase, `KeyboardEvent` does NOT expose the live `<input>` text; the canonical commit path is "Enter triggers `blur()` via `document::eval`, then `onblur` reads the value via `FormEvent::value()`". Mirror exactly:

```rust
let on_keydown = move |evt: KeyboardEvent| {
    if evt.key() == Key::Enter {
        evt.prevent_default();
        // Trigger blur on the active input; the onblur handler below
        // does the parse + clamp + dispatch via `oncommit`. Mirrors
        // header.rs:288-299 (rename-inline pattern).
        let _ = document::eval(
            r"
            const el = document.activeElement;
            if (el && el instanceof HTMLInputElement) { el.blur(); }
            ",
        );
    }
};

let oncommit_for_blur = oncommit;
let on_blur = move |evt: FocusEvent| {
    let Some(handler) = oncommit_for_blur.as_ref() else { return };
    let raw = evt.value();  // FocusEvent here is `Event<FocusData>`; `.value()` returns the input's current text via Dioxus 0.7's FormEvent-shaped API. See header.rs:309-311 for `FormEvent::value()` precedent.
    if let Some(v) = parse_and_clamp(&raw, min, max) {
        handler.call(v);
    }
};
```

Attach both to the `<input>` element inside the existing `rsx!`:

```rust
input {
    r#type: "number",
    class: "if-number-input__field",
    id: "{id_val}",
    value: "{display_value}",
    min: "{min}",
    max: "{max}",
    step: "{step}",
    disabled,
    oninput: input_handler,
    onkeydown: on_keydown,
    onblur: on_blur,
}
```

(Repeat the two new handlers in the `else` branch where `id` is `None`. `EventHandler<f64>` is `Copy`; the `oncommit` prop can be reused without `clone()` per branch.)

If `FocusEvent::value()` does not compile in Dioxus 0.7 (the API may instead be `evt.data().value()` for a different event type), fall back to reading the input via a second `document::eval` round-trip that posts the value back through the eval channel. The `header.rs` rename-inline editor uses `oninput` to mirror the value into a Signal in real-time as a workaround for this exact limitation; if the value-on-blur path proves intractable, mirror that approach instead and have `oncommit` fire from a `Signal<String>` watched in `oninput`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p inputforge-gui-dx number_input::oncommit_tests`
Expected: 4 passed.

Run: `cargo check -p inputforge-gui-dx`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/components/number_input.rs
git commit -m "feat(number-input): add oncommit prop firing on Enter or blur"
```

---

## Task 7: F11 module skeleton + state types

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/mod.rs`
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/state.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs` (add `mod deadzone;`)

This task only adds the directory skeleton, the state struct, the `HandleId` enum, and a stub `DeadzoneBody` so subsequent tasks can compile against the new types. No SVG yet.

- [ ] **Step 1: Write the failing test**

Create `deadzone/state.rs`:

```rust
// Rust guideline compliant 2026-05-03

//! F11 deadzone body local state. No Signals here; pure types so the body's
//! interaction / keyboard handlers stay unit-testable without Dioxus.

use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::mapping_editor::pipeline::stage_body::deadzone::keyboard::KeyKind;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::nudge_coalesce::NudgeCoalesce;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum HandleId {
    Low,
    CenterLow,
    CenterHigh,
    High,
}

impl HandleId {
    pub(crate) const ALL: [HandleId; 4] = [
        HandleId::Low,
        HandleId::CenterLow,
        HandleId::CenterHigh,
        HandleId::High,
    ];

    pub(crate) const fn next(self) -> Option<HandleId> {
        match self {
            HandleId::Low => Some(HandleId::CenterLow),
            HandleId::CenterLow => Some(HandleId::CenterHigh),
            HandleId::CenterHigh => Some(HandleId::High),
            HandleId::High => None,
        }
    }

    pub(crate) const fn prev(self) -> Option<HandleId> {
        match self {
            HandleId::Low => None,
            HandleId::CenterLow => Some(HandleId::Low),
            HandleId::CenterHigh => Some(HandleId::CenterLow),
            HandleId::High => Some(HandleId::CenterHigh),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DragInProgress {
    pub handle: HandleId,
    /// Inclusive viewBox-x bounds derived once at drag start from the
    /// neighbour thresholds; the candidate config is built only after
    /// clamping the cursor X to this interval.
    pub bounds: (f64, f64),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BodyState {
    pub dragging: Option<DragInProgress>,
    pub hovered_handle: Option<HandleId>,
    pub focused_handle: Option<HandleId>,
    pub pre_drag_config: Option<DeadzoneConfig>,
    pub nudge_coalesce: NudgeCoalesce<KeyKind>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_next_chain_hits_each_id_then_none() {
        assert_eq!(HandleId::Low.next(), Some(HandleId::CenterLow));
        assert_eq!(HandleId::CenterLow.next(), Some(HandleId::CenterHigh));
        assert_eq!(HandleId::CenterHigh.next(), Some(HandleId::High));
        assert_eq!(HandleId::High.next(), None);
    }

    #[test]
    fn handle_prev_chain_hits_each_id_then_none() {
        assert_eq!(HandleId::High.prev(), Some(HandleId::CenterHigh));
        assert_eq!(HandleId::CenterHigh.prev(), Some(HandleId::CenterLow));
        assert_eq!(HandleId::CenterLow.prev(), Some(HandleId::Low));
        assert_eq!(HandleId::Low.prev(), None);
    }

    #[test]
    fn body_state_default_is_idle() {
        let s = BodyState::default();
        assert!(s.dragging.is_none());
        assert!(s.hovered_handle.is_none());
        assert!(s.focused_handle.is_none());
        assert!(s.pre_drag_config.is_none());
    }
}
```

- [ ] **Step 2: Write the module skeleton**

Create `deadzone/mod.rs`:

```rust
// Rust guideline compliant 2026-05-03

//! F11 deadzone body. See spec
//! `docs/superpowers/specs/2026-05-02-f11-deadzone-editor-design.md`.

pub(crate) mod interaction;
pub(crate) mod keyboard;
pub(crate) mod mutation;
pub(crate) mod rendering;
pub(crate) mod state;
pub(crate) mod thumbnail;
pub(crate) mod toolbar;

#[cfg(test)]
mod tests;

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::StageId;

/// Body component for an `Action::Deadzone` pipeline stage. Stub for now;
/// fully wired up in Task 14 once interaction / keyboard / rendering land.
#[component]
pub(crate) fn DeadzoneBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    config: DeadzoneConfig,
    root_actions: Vec<Action>,
) -> Element {
    let _ = (mapping_key, stage_id, config, root_actions);
    rsx! { div { class: "if-deadzone", "deadzone body (under construction)" } }
}
```

Stub each sibling module so the crate compiles even before subsequent tasks land. Add temporary "module exists" placeholders for `interaction.rs`, `keyboard.rs`, `mutation.rs`, `rendering.rs`, `thumbnail.rs`, `toolbar.rs`, `tests.rs`. Each stub file is one line:

```rust
// Rust guideline compliant 2026-05-03
```

- [ ] **Step 3: Wire `mod deadzone;` into the dispatcher**

In `stage_body/mod.rs`, add `mod deadzone;` next to the existing `mod conditional;` line (alphabetical). Do NOT swap the `Action::Deadzone` arms yet; that happens in Task 15.

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-gui-dx deadzone::state::tests`
Expected: 3 passed.

Run: `cargo check -p inputforge-gui-dx`
Expected: clean. (Stubs are valid Rust; the module compiles.)

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/ \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs
git commit -m "feat(deadzone): scaffold module skeleton and BodyState types"
```

---

## Task 8: F11 mutation primitives

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/mutation.rs` (replace stub)

Pure functions consumed by interaction.rs, keyboard.rs, and rendering.rs. No engine code added.

- [ ] **Step 1: Write the failing tests**

Replace the stub `mutation.rs` with the test block (and a single placeholder fn) so the test target exists:

```rust
// Rust guideline compliant 2026-05-03

//! Pure handle-mutation helpers for F11. Each function takes a current
//! `DeadzoneConfig` and returns either a candidate `DeadzoneConfig` or the
//! geometry needed to render / hit-test handles.

use inputforge_core::error::Result;
use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::HandleId;

/// Inclusive (min, max) viewBox-x bounds the given handle is allowed to
/// occupy without violating the engine's `low < center_low <= center_high
/// < high` invariant. The 0.001 epsilon mirrors what the egui editor used
/// (chosen because `DeadzoneConfig::new` rejects equality between `low`
/// and `center_low`, and between `center_high` and `high`).
pub(crate) fn adjacent_bounds(handle: HandleId, config: &DeadzoneConfig) -> (f64, f64) {
    todo!()
}

/// Build a candidate `DeadzoneConfig` with the named handle's X coordinate
/// replaced by `new_x`, clamped to the handle's adjacent bounds. Validation
/// runs through `DeadzoneConfig::new`.
pub(crate) fn with_handle(
    config: &DeadzoneConfig,
    handle: HandleId,
    new_x: f64,
) -> Result<DeadzoneConfig> {
    todo!()
}

/// Return the four `(x, y)` viewBox coordinates of the four handles in
/// `HandleId::ALL` order. Y is fixed per handle: Low/High at +/- 1.0,
/// CenterLow/CenterHigh at 0.0.
pub(crate) fn handle_positions(config: &DeadzoneConfig) -> [(f64, f64); 4] {
    todo!()
}

/// Convenience alias for `DeadzoneConfig::default()`.
pub(crate) fn default_config() -> DeadzoneConfig {
    DeadzoneConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(low: f64, cl: f64, ch: f64, high: f64) -> DeadzoneConfig {
        DeadzoneConfig::new(low, cl, ch, high).expect("valid")
    }

    #[test]
    fn handle_positions_y_locks() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let p = handle_positions(&c);
        assert_eq!(p[0], (-0.85, -1.0));
        assert_eq!(p[1], (-0.15, 0.0));
        assert_eq!(p[2], (0.15, 0.0));
        assert_eq!(p[3], (0.85, 1.0));
    }

    #[test]
    fn adjacent_bounds_low_runs_to_center_low_minus_epsilon() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let (lo, hi) = adjacent_bounds(HandleId::Low, &c);
        assert!((lo - -1.0).abs() < 1e-9);
        assert!((hi - -0.151).abs() < 1e-9);
    }

    #[test]
    fn adjacent_bounds_center_low_runs_from_low_to_center_high() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let (lo, hi) = adjacent_bounds(HandleId::CenterLow, &c);
        assert!((lo - -0.849).abs() < 1e-9);
        assert!((hi - 0.15).abs() < 1e-9);
    }

    #[test]
    fn adjacent_bounds_center_high_runs_from_center_low_to_high() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let (lo, hi) = adjacent_bounds(HandleId::CenterHigh, &c);
        assert!((lo - -0.15).abs() < 1e-9);
        assert!((hi - 0.849).abs() < 1e-9);
    }

    #[test]
    fn adjacent_bounds_high_runs_from_center_high_plus_epsilon_to_one() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let (lo, hi) = adjacent_bounds(HandleId::High, &c);
        assert!((lo - 0.151).abs() < 1e-9);
        assert!((hi - 1.0).abs() < 1e-9);
    }

    #[test]
    fn with_handle_clamps_to_adjacent_bounds() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let result = with_handle(&c, HandleId::CenterLow, 0.5).expect("valid");
        assert!(result.center_low() <= result.center_high());
    }

    #[test]
    fn with_handle_low_replaces_only_low() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let result = with_handle(&c, HandleId::Low, -0.6).expect("valid");
        assert!((result.low() - -0.6).abs() < 1e-9);
        assert!((result.center_low() - -0.15).abs() < 1e-9);
    }

    #[test]
    fn with_handle_rejects_nan_input() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let result = with_handle(&c, HandleId::CenterLow, f64::NAN);
        assert!(result.is_err(), "NaN must not slip through to DeadzoneConfig::new");
        let result = with_handle(&c, HandleId::CenterLow, f64::INFINITY);
        assert!(result.is_err(), "Inf must not slip through either");
    }

    #[test]
    fn default_config_round_trips_engine_default() {
        let c = default_config();
        assert!((c.low() - -1.0).abs() < 1e-9);
        assert!((c.center_low()).abs() < 1e-9);
        assert!((c.center_high()).abs() < 1e-9);
        assert!((c.high() - 1.0).abs() < 1e-9);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p inputforge-gui-dx deadzone::mutation::tests`
Expected: every test that calls a `todo!()` panics; `default_config_round_trips_engine_default` passes.

- [ ] **Step 3: Implement the helpers**

```rust
const EPSILON: f64 = 0.001;

pub(crate) fn adjacent_bounds(handle: HandleId, config: &DeadzoneConfig) -> (f64, f64) {
    match handle {
        HandleId::Low => (-1.0, config.center_low() - EPSILON),
        HandleId::CenterLow => (config.low() + EPSILON, config.center_high()),
        HandleId::CenterHigh => (config.center_low(), config.high() - EPSILON),
        HandleId::High => (config.center_high() + EPSILON, 1.0),
    }
}

pub(crate) fn with_handle(
    config: &DeadzoneConfig,
    handle: HandleId,
    new_x: f64,
) -> Result<DeadzoneConfig> {
    // NaN/Inf would propagate through `min`/`max` and slip past
    // `DeadzoneConfig::new`, whose ordering checks (`low < center_low`, etc.)
    // are all false for non-finite values, allowing an invalid config through.
    // Reject at the API boundary instead.
    if !new_x.is_finite() {
        return Err(inputforge_core::error::InputForgeError::Validation(
            "deadzone handle position must be finite".into(),
        ));
    }
    let (lo, hi) = adjacent_bounds(handle, config);
    let clamped = new_x.min(hi).max(lo);
    let (low, cl, ch, high) = match handle {
        HandleId::Low => (clamped, config.center_low(), config.center_high(), config.high()),
        HandleId::CenterLow => (config.low(), clamped, config.center_high(), config.high()),
        HandleId::CenterHigh => (config.low(), config.center_low(), clamped, config.high()),
        HandleId::High => (config.low(), config.center_low(), config.center_high(), clamped),
    };
    DeadzoneConfig::new(low, cl, ch, high)
}
```

(Verify the exact error variant against `crates/inputforge-core/src/error.rs`. If `InputForgeError` does not have a `Validation` variant, use the engine's existing variant for "invalid arithmetic" / "invalid input" instead. The point is to fail closed at the API boundary.)

```rust

pub(crate) fn handle_positions(config: &DeadzoneConfig) -> [(f64, f64); 4] {
    [
        (config.low(), -1.0),
        (config.center_low(), 0.0),
        (config.center_high(), 0.0),
        (config.high(), 1.0),
    ]
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx deadzone::mutation::tests`
Expected: 9 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/mutation.rs
git commit -m "feat(deadzone): pure mutation helpers for handles"
```

---

## Task 9: F11 pointer-event handlers

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/interaction.rs`

Mirrors F10's `interaction.rs` shape: pure functions over `(BodyState, DeadzoneConfig, cursor, PlotRect)` returning `(BodyState, Option<DeadzoneConfig>, ChangedFlag)`. Reuses the screen-to-viewBox helpers F10 already exposes (or copies them locally; small enough not to lift yet).

- [ ] **Step 1: Replace the stub with the failing tests**

Replace `interaction.rs`:

```rust
// Rust guideline compliant 2026-05-03

//! Pure pointer-event handlers for F11. Each takes a snapshot of `BodyState`
//! and the current `DeadzoneConfig` and returns the next state plus an
//! optional candidate config (for the host to dispatch on commit).

use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::mapping_editor::pipeline::stage_body::deadzone::mutation::{
    adjacent_bounds, handle_positions, with_handle,
};
use crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::{
    BodyState, DragInProgress, HandleId,
};

/// Hit-test radius in screen pixels (matches F10's `HIT_RADIUS_PX`).
pub(crate) const HIT_RADIUS_PX: f64 = 10.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlotRect {
    pub x: f64,
    pub y: f64,
    pub size: f64,
}

pub(crate) type HandlerOut = (BodyState, Option<DeadzoneConfig>, bool);

#[must_use]
pub(crate) fn screen_to_viewbox(cursor: (f64, f64), r: &PlotRect) -> Option<(f64, f64)> {
    if r.size <= 0.0 {
        return None;
    }
    let nx = (cursor.0 - r.x) / r.size;
    let ny = (cursor.1 - r.y) / r.size;
    let input = -1.05 + nx * 2.1;
    let output = 1.05 - ny * 2.1;
    Some((input, output))
}

fn viewbox_to_screen(p: (f64, f64), r: &PlotRect) -> (f64, f64) {
    let nx = (p.0 + 1.05) / 2.1;
    let ny = (1.05 - p.1) / 2.1;
    (r.x + nx * r.size, r.y + ny * r.size)
}

#[must_use]
pub(crate) fn nearest_handle(
    cursor: (f64, f64),
    config: &DeadzoneConfig,
    r: &PlotRect,
    radius_px: f64,
) -> Option<HandleId> {
    let radius_sq = radius_px * radius_px;
    let positions = handle_positions(config);
    let mut best: Option<(HandleId, f64)> = None;
    for (handle, pos) in HandleId::ALL.iter().zip(positions.iter()) {
        let s = viewbox_to_screen(*pos, r);
        let dx = s.0 - cursor.0;
        let dy = s.1 - cursor.1;
        let d2 = dx * dx + dy * dy;
        if d2 <= radius_sq {
            match best {
                Some((_, bd)) if bd <= d2 => {}
                _ => best = Some((*handle, d2)),
            }
        }
    }
    best.map(|(h, _)| h)
}

pub(crate) fn handle_pointer_down(
    mut state: BodyState,
    config: &DeadzoneConfig,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    let Some(handle) = nearest_handle(cursor, config, r, HIT_RADIUS_PX) else {
        return (state, None, false);
    };
    state.dragging = Some(DragInProgress {
        handle,
        bounds: adjacent_bounds(handle, config),
    });
    state.pre_drag_config = Some(config.clone());
    (state, None, false)
}

pub(crate) fn handle_pointer_move(
    mut state: BodyState,
    config: &DeadzoneConfig,
    cursor: (f64, f64),
    r: &PlotRect,
) -> HandlerOut {
    if let Some(drag) = state.dragging {
        let Some((vx, _vy)) = screen_to_viewbox(cursor, r) else {
            return (state, None, false);
        };
        let clamped = vx.min(drag.bounds.1).max(drag.bounds.0);
        return match with_handle(config, drag.handle, clamped) {
            Ok(new_config) => (state, Some(new_config), true),
            Err(_) => (state, None, false),
        };
    }
    state.hovered_handle = nearest_handle(cursor, config, r, HIT_RADIUS_PX);
    (state, None, false)
}

pub(crate) fn handle_pointer_up(
    mut state: BodyState,
    working_config: &DeadzoneConfig,
) -> (BodyState, Result<DeadzoneConfig, String>, bool) {
    if state.dragging.is_none() {
        return (state, Err(String::new()), false);
    }
    state.dragging = None;
    // Working copy was built via `with_handle` which already runs through
    // `DeadzoneConfig::new`, so this re-validate is defensive only.
    match DeadzoneConfig::new(
        working_config.low(),
        working_config.center_low(),
        working_config.center_high(),
        working_config.high(),
    ) {
        Ok(valid) => {
            state.pre_drag_config = None;
            (state, Ok(valid), true)
        }
        Err(err) => (state, Err(err.to_string()), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> DeadzoneConfig {
        DeadzoneConfig::new(-0.85, -0.15, 0.15, 0.85).expect("valid")
    }

    fn rect() -> PlotRect {
        PlotRect { x: 0.0, y: 0.0, size: 200.0 }
    }

    fn screen_for(handle: HandleId, c: &DeadzoneConfig, r: &PlotRect) -> (f64, f64) {
        let positions = handle_positions(c);
        let idx = HandleId::ALL.iter().position(|h| *h == handle).unwrap();
        viewbox_to_screen(positions[idx], r)
    }

    #[test]
    fn pointer_down_on_low_handle_starts_drag_and_snapshots() {
        let c = cfg();
        let r = rect();
        let cursor = screen_for(HandleId::Low, &c, &r);
        let (s, _, _) = handle_pointer_down(BodyState::default(), &c, cursor, &r);
        assert_eq!(s.dragging.unwrap().handle, HandleId::Low);
        assert_eq!(s.pre_drag_config, Some(c));
    }

    #[test]
    fn pointer_down_off_handle_no_drag() {
        let c = cfg();
        let r = rect();
        let (s, _, _) = handle_pointer_down(BodyState::default(), &c, (0.0, 0.0), &r);
        assert!(s.dragging.is_none());
    }

    #[test]
    fn pointer_move_during_drag_produces_candidate() {
        let c = cfg();
        let r = rect();
        let mut s = BodyState::default();
        s.dragging = Some(DragInProgress {
            handle: HandleId::CenterLow,
            bounds: adjacent_bounds(HandleId::CenterLow, &c),
        });
        let cursor = screen_for(HandleId::CenterHigh, &c, &r);
        let (_, new_cfg, changed) = handle_pointer_move(s, &c, cursor, &r);
        assert!(changed);
        assert!(new_cfg.is_some());
    }

    #[test]
    fn pointer_move_idle_updates_hover_only() {
        let c = cfg();
        let r = rect();
        let cursor = screen_for(HandleId::High, &c, &r);
        let (s, new_cfg, _) = handle_pointer_move(BodyState::default(), &c, cursor, &r);
        assert!(new_cfg.is_none());
        assert_eq!(s.hovered_handle, Some(HandleId::High));
    }

    #[test]
    fn pointer_up_after_drag_validates() {
        let c = cfg();
        let mut s = BodyState::default();
        s.dragging = Some(DragInProgress {
            handle: HandleId::CenterLow,
            bounds: adjacent_bounds(HandleId::CenterLow, &c),
        });
        s.pre_drag_config = Some(c.clone());
        let working = with_handle(&c, HandleId::CenterLow, -0.05).unwrap();
        let (next, result, changed) = handle_pointer_up(s, &working);
        assert!(changed);
        assert!(result.is_ok());
        assert!(next.dragging.is_none());
        assert!(next.pre_drag_config.is_none());
    }

    #[test]
    fn pointer_up_without_drag_is_noop() {
        let c = cfg();
        let (_, result, changed) = handle_pointer_up(BodyState::default(), &c);
        assert!(matches!(result, Err(ref e) if e.is_empty()));
        assert!(!changed);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p inputforge-gui-dx deadzone::interaction::tests`
Expected: 6 passed.

Run: `cargo check -p inputforge-gui-dx`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/interaction.rs
git commit -m "feat(deadzone): pure pointer-event handlers and hit-test"
```

---

## Task 10: F11 keyboard handler

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/keyboard.rs`

Pure keyboard handler. Mirrors F10's `KeyOutcome::PushUndo / MergeUndo` shape; uses `instruments::nudge_coalesce::NudgeCoalesce` for the merge decision.

- [ ] **Step 1: Replace the stub with the failing tests**

Replace `keyboard.rs`:

```rust
// Rust guideline compliant 2026-05-03

//! Pure keyboard handler for F11. Driven by `KeyInput` (a normalised event
//! shape) so the host body can route Dioxus `KeyboardEvent` through this
//! function without coupling tests to Dioxus types.

use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::mapping_editor::pipeline::stage_body::deadzone::mutation::{
    adjacent_bounds, default_config, with_handle,
};
use crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::{BodyState, HandleId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyInput {
    Tab,
    ShiftTab,
    ArrowLeft { shift: bool },
    ArrowRight { shift: bool },
    ArrowUp { shift: bool },
    ArrowDown { shift: bool },
    Home,
    End,
    Enter,
    Delete,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyKind {
    ArrowLeft,
    ArrowRight,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KeyOutcome {
    PushUndo { label: String },
    MergeUndo,
}

pub(crate) type KeyHandlerOut = (BodyState, Option<DeadzoneConfig>, Option<KeyOutcome>, bool);

const SMALL_STEP: f64 = 0.01;
const LARGE_STEP: f64 = 0.10;

pub(crate) fn handle_key(
    mut state: BodyState,
    config: &DeadzoneConfig,
    key: KeyInput,
    now_ms: u64,
) -> KeyHandlerOut {
    match key {
        KeyInput::Tab => {
            state.focused_handle = match state.focused_handle {
                None => Some(HandleId::Low),
                Some(h) => h.next(),
            };
            (state, None, None, false)
        }
        KeyInput::ShiftTab => {
            state.focused_handle = match state.focused_handle {
                None => Some(HandleId::Low),
                Some(h) => h.prev(),
            };
            (state, None, None, false)
        }
        KeyInput::Home => {
            state.focused_handle = Some(HandleId::Low);
            (state, None, None, false)
        }
        KeyInput::End => {
            state.focused_handle = Some(HandleId::High);
            (state, None, None, false)
        }
        KeyInput::Escape => {
            if let Some(prev) = state.pre_drag_config.take() {
                state.dragging = None;
                return (state, Some(prev), None, true);
            }
            (state, None, None, false)
        }
        KeyInput::ArrowLeft { shift } => nudge(state, config, -step(shift), KeyKind::ArrowLeft, now_ms),
        KeyInput::ArrowRight { shift } => nudge(state, config, step(shift), KeyKind::ArrowRight, now_ms),
        KeyInput::ArrowUp { .. } | KeyInput::ArrowDown { .. } => (state, None, None, false),
        KeyInput::Enter | KeyInput::Delete => (state, None, None, false),
    }
}

const fn step(shift: bool) -> f64 {
    if shift { LARGE_STEP } else { SMALL_STEP }
}

fn nudge(
    mut state: BodyState,
    config: &DeadzoneConfig,
    delta: f64,
    kind: KeyKind,
    now_ms: u64,
) -> KeyHandlerOut {
    let Some(handle) = state.focused_handle else {
        return (state, None, None, false);
    };
    let current_x = match handle {
        HandleId::Low => config.low(),
        HandleId::CenterLow => config.center_low(),
        HandleId::CenterHigh => config.center_high(),
        HandleId::High => config.high(),
    };
    let (lo, hi) = adjacent_bounds(handle, config);
    let target = (current_x + delta).min(hi).max(lo);
    if (target - current_x).abs() < f64::EPSILON {
        return (state, None, None, false);
    }
    let Ok(new_config) = with_handle(config, handle, target) else {
        return (state, None, None, false);
    };
    let merge = state.nudge_coalesce.should_merge(now_ms, kind);
    state.nudge_coalesce.record(now_ms, kind);
    let outcome = if merge {
        KeyOutcome::MergeUndo
    } else {
        let label_handle = match handle {
            HandleId::Low => "low",
            HandleId::CenterLow => "center_low",
            HandleId::CenterHigh => "center_high",
            HandleId::High => "high",
        };
        KeyOutcome::PushUndo {
            label: format!("deadzone: {label_handle} {current_x:+.2} -> {target:+.2}"),
        }
    };
    (state, Some(new_config), Some(outcome), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> DeadzoneConfig {
        DeadzoneConfig::new(-0.85, -0.15, 0.15, 0.85).expect("valid")
    }

    fn seed() -> (DeadzoneConfig, BodyState) {
        let c = cfg();
        let mut s = BodyState::default();
        s.focused_handle = Some(HandleId::Low);
        (c, s)
    }

    #[test]
    fn tab_advances_focus() {
        let (c, mut s) = seed();
        s.focused_handle = Some(HandleId::CenterLow);
        let (next, _, _, _) = handle_key(s, &c, KeyInput::Tab, 0);
        assert_eq!(next.focused_handle, Some(HandleId::CenterHigh));
    }

    #[test]
    fn tab_at_high_yields_none_so_browser_advances_focus() {
        let (c, mut s) = seed();
        s.focused_handle = Some(HandleId::High);
        let (next, _, _, _) = handle_key(s, &c, KeyInput::Tab, 0);
        assert_eq!(next.focused_handle, None);
    }

    #[test]
    fn shift_tab_at_low_yields_none() {
        let (c, mut s) = seed();
        s.focused_handle = Some(HandleId::Low);
        let (next, _, _, _) = handle_key(s, &c, KeyInput::ShiftTab, 0);
        assert_eq!(next.focused_handle, None);
    }

    #[test]
    fn home_jumps_to_low() {
        let (c, mut s) = seed();
        s.focused_handle = Some(HandleId::High);
        let (next, _, _, _) = handle_key(s, &c, KeyInput::Home, 0);
        assert_eq!(next.focused_handle, Some(HandleId::Low));
    }

    #[test]
    fn arrow_left_nudges_low_by_step_and_pushes_undo() {
        let (c, s) = seed();
        let (_, new_cfg, outcome, changed) = handle_key(s, &c, KeyInput::ArrowLeft { shift: false }, 0);
        assert!(changed);
        let new = new_cfg.unwrap();
        assert!((new.low() - -0.86).abs() < 1e-9);
        assert!(matches!(outcome, Some(KeyOutcome::PushUndo { .. })));
    }

    #[test]
    fn shift_arrow_uses_large_step() {
        let (c, s) = seed();
        let (_, new_cfg, _, _) = handle_key(s, &c, KeyInput::ArrowRight { shift: true }, 0);
        let new = new_cfg.unwrap();
        assert!((new.low() - -0.75).abs() < 1e-9);
    }

    #[test]
    fn second_same_key_within_window_merges_undo() {
        let (c, s) = seed();
        let (s1, new1, _, _) = handle_key(s, &c, KeyInput::ArrowRight { shift: false }, 100);
        let new1 = new1.unwrap();
        let (_, _, outcome2, _) = handle_key(s1, &new1, KeyInput::ArrowRight { shift: false }, 200);
        assert_eq!(outcome2, Some(KeyOutcome::MergeUndo));
    }

    #[test]
    fn second_same_key_past_window_pushes_new_undo() {
        let (c, s) = seed();
        let (s1, new1, _, _) = handle_key(s, &c, KeyInput::ArrowRight { shift: false }, 100);
        let new1 = new1.unwrap();
        let (_, _, outcome2, _) = handle_key(s1, &new1, KeyInput::ArrowRight { shift: false }, 100 + 251);
        assert!(matches!(outcome2, Some(KeyOutcome::PushUndo { .. })));
    }

    #[test]
    fn arrow_at_clamp_boundary_is_noop() {
        let (c, mut s) = seed();
        // Drive the Low handle to the clamp bound first.
        let (s2, _, _, _) = handle_key(s, &c, KeyInput::ArrowLeft { shift: false }, 0);
        // Now nudge again from -1.0; nothing further is allowed.
        s = s2;
        s.focused_handle = Some(HandleId::Low);
        let stuck = DeadzoneConfig::new(-1.0, -0.15, 0.15, 0.85).expect("valid");
        let (_, new_cfg, outcome, changed) = handle_key(s, &stuck, KeyInput::ArrowLeft { shift: false }, 1000);
        assert!(!changed);
        assert!(new_cfg.is_none());
        assert!(outcome.is_none());
    }

    #[test]
    fn arrow_up_and_down_silent_no_op() {
        let (c, s) = seed();
        let (_, new_cfg, outcome, changed) = handle_key(s, &c, KeyInput::ArrowUp { shift: false }, 0);
        assert!(!changed);
        assert!(new_cfg.is_none());
        assert!(outcome.is_none());
    }

    #[test]
    fn enter_and_delete_silent_no_op() {
        let (c, s) = seed();
        let (_, new_cfg, _, changed) = handle_key(s, &c, KeyInput::Enter, 0);
        assert!(!changed);
        assert!(new_cfg.is_none());
        let (c, s) = seed();
        let (_, new_cfg, _, changed) = handle_key(s, &c, KeyInput::Delete, 0);
        assert!(!changed);
        assert!(new_cfg.is_none());
    }

    #[test]
    fn escape_during_drag_reverts_to_pre_drag_config() {
        let (c, mut s) = seed();
        s.dragging = Some(crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::DragInProgress {
            handle: HandleId::Low,
            bounds: adjacent_bounds(HandleId::Low, &c),
        });
        s.pre_drag_config = Some(c.clone());
        let working = with_handle(&c, HandleId::Low, -0.5).unwrap();
        let (next, new_cfg, outcome, changed) = handle_key(s, &working, KeyInput::Escape, 0);
        assert!(changed);
        assert_eq!(new_cfg.unwrap(), c);
        assert!(next.dragging.is_none());
        assert!(next.pre_drag_config.is_none());
        assert!(outcome.is_none());
    }

    // `default_config` referenced by the body's Reset path.
    #[test]
    fn default_config_alias_exists() {
        let _ = default_config();
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p inputforge-gui-dx deadzone::keyboard::tests`
Expected: 12 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/keyboard.rs
git commit -m "feat(deadzone): pure keyboard handler with same-key undo coalesce"
```

---

## Task 11: F11 SVG rendering and CSS

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/rendering.rs`
- Create: `crates/inputforge-gui-dx/assets/frame/deadzone.css`
- Modify: `crates/inputforge-gui-dx/src/theme/mod.rs` (register `DEADZONE_CSS`)

15 layers per the spec table. Layer 1 (background) and layer 15 (tick labels) sit at the SVG root; layers 2 through 14 sit inside the y-flip group.

- [ ] **Step 1: Replace the stub `rendering.rs`**

```rust
// Rust guideline compliant 2026-05-03

//! F11 SVG layer stack. Renders zone-banded background, grid, axis cross,
//! identity diagonal, deadzone curve polyline, four handles, hover/drag/focus
//! decorations, optional live tracking dot, and tick labels.

use dioxus::prelude::*;

use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::{BodyState, HandleId};
use crate::frame::mapping_editor::pipeline::stage_body::deadzone::mutation::handle_positions;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::INSTR_GLOW_STDDEV;

pub(crate) fn render_plot(
    config: &DeadzoneConfig,
    state: &BodyState,
    live_value: Option<f64>,
) -> Element {
    let live_pair = live_value.map(|v| (v, config.apply(v)));
    rsx! {
        svg {
            class: "if-deadzone__plot",
            view_box: "-1.05 -1.05 2.1 2.1",
            preserve_aspect_ratio: "xMidYMid meet",
            title { "deadzone curve" }
            defs {
                filter {
                    id: "if-instr-glow",
                    x: "-50%", y: "-50%", width: "200%", height: "200%",
                    feGaussianBlur { std_deviation: "{INSTR_GLOW_STDDEV}" }
                }
            }
            // Layer 1: background (root, not flipped).
            rect {
                class: "if-deadzone__bg",
                x: "-1.05", y: "-1.05", width: "2.1", height: "2.1",
            }
            g {
                transform: "scale(1, -1)",
                {render_zone_bands(config)}
                {render_grid(config)}
                {render_axis_cross()}
                {render_identity()}
                {render_curve(config)}
                {render_handles(config, state)}
                {render_hover_ring(config, state)}
                {render_drag_halo(config, state)}
                {render_focus_ring(config, state)}
                if let Some((input, output)) = live_pair {
                    {render_live(input, output)}
                }
            }
            {render_tick_labels()}
        }
    }
}

fn render_zone_bands(config: &DeadzoneConfig) -> Element {
    let l = config.low();
    let cl = config.center_low();
    let ch = config.center_high();
    let h = config.high();
    // Bands are anchored at the live `-1..1` region (NOT the `-1.05..1.05`
    // viewBox padding). At default config (low=-1, high=1) this collapses
    // the saturated bands to width 0 instead of leaking a visible 0.05-unit
    // tint outside the live region. `(l - -1.0).max(0.0)` guards against
    // any future config where low briefly exceeds -1 mid-validation.
    rsx! {
        // Outer saturated bands.
        rect { class: "if-deadzone__zone if-deadzone__zone--sat", x: "-1.0", y: "-1.0", width: "{(l - -1.0).max(0.0)}", height: "2.0" }
        rect { class: "if-deadzone__zone if-deadzone__zone--sat", x: "{h}", y: "-1.0", width: "{(1.0 - h).max(0.0)}", height: "2.0" }
        // Ramp bands.
        rect { class: "if-deadzone__zone if-deadzone__zone--ramp", x: "{l}", y: "-1.0", width: "{(cl - l).max(0.0)}", height: "2.0" }
        rect { class: "if-deadzone__zone if-deadzone__zone--ramp", x: "{ch}", y: "-1.0", width: "{(h - ch).max(0.0)}", height: "2.0" }
        // Dead band.
        rect { class: "if-deadzone__zone if-deadzone__zone--dead", x: "{cl}", y: "-1.0", width: "{(ch - cl).max(0.0)}", height: "2.0" }
    }
}

fn render_grid(_config: &DeadzoneConfig) -> Element {
    let lines = [-0.75, -0.5, -0.25, 0.25, 0.5, 0.75];
    rsx! {
        g {
            class: "if-deadzone__grid",
            for x in lines.iter() {
                line { class: "if-deadzone__grid-major", x1: "{x}", y1: "-1.0", x2: "{x}", y2: "1.0" }
                line { class: "if-deadzone__grid-major", x1: "-1.0", y1: "{x}", x2: "1.0", y2: "{x}" }
            }
        }
    }
}

fn render_axis_cross() -> Element {
    rsx! {
        g {
            line { class: "if-deadzone__axis-cross", x1: "0", y1: "-1.0", x2: "0", y2: "1.0" }
            line { class: "if-deadzone__axis-cross", x1: "-1.0", y1: "0", x2: "1.0", y2: "0" }
        }
    }
}

fn render_identity() -> Element {
    rsx! {
        line {
            class: "if-deadzone__identity",
            x1: "-1.0", y1: "-1.0", x2: "1.0", y2: "1.0",
        }
    }
}

fn render_curve(config: &DeadzoneConfig) -> Element {
    let l = config.low();
    let cl = config.center_low();
    let ch = config.center_high();
    let h = config.high();
    let pts = format!("-1,-1 {l},-1 {cl},0 {ch},0 {h},1 1,1");
    rsx! {
        polyline {
            class: "if-deadzone__path",
            points: "{pts}",
        }
    }
}

fn render_handles(config: &DeadzoneConfig, _state: &BodyState) -> Element {
    let p = handle_positions(config);
    rsx! {
        g {
            for (px, py) in p.iter() {
                circle { class: "if-deadzone__handle", cx: "{px}", cy: "{py}", r: "0.022" }
            }
        }
    }
}

fn render_hover_ring(config: &DeadzoneConfig, state: &BodyState) -> Element {
    let Some(handle) = state.hovered_handle else {
        return rsx! {};
    };
    let p = handle_positions(config);
    let idx = HandleId::ALL.iter().position(|h| *h == handle).unwrap();
    let (cx, cy) = p[idx];
    rsx! {
        circle { class: "if-deadzone__hover-ring", cx: "{cx}", cy: "{cy}", r: "0.085", fill: "none" }
    }
}

fn render_drag_halo(config: &DeadzoneConfig, state: &BodyState) -> Element {
    let Some(drag) = state.dragging else {
        return rsx! {};
    };
    let p = handle_positions(config);
    let idx = HandleId::ALL.iter().position(|h| *h == drag.handle).unwrap();
    let (cx, cy) = p[idx];
    rsx! {
        circle { class: "if-deadzone__drag-halo", cx: "{cx}", cy: "{cy}", r: "0.07" }
    }
}

fn render_focus_ring(config: &DeadzoneConfig, state: &BodyState) -> Element {
    let Some(handle) = state.focused_handle else {
        return rsx! {};
    };
    let p = handle_positions(config);
    let idx = HandleId::ALL.iter().position(|h| *h == handle).unwrap();
    let (cx, cy) = p[idx];
    rsx! {
        circle { class: "if-deadzone__focus-ring", cx: "{cx}", cy: "{cy}", r: "0.105", fill: "none" }
    }
}

fn render_live(input: f64, output: f64) -> Element {
    rsx! {
        // Horizontal guide.
        line { class: "if-deadzone__live-guide", x1: "-1.0", y1: "{output}", x2: "{input}", y2: "{output}" }
        // Vertical guide anchored at the axis cross (deadzone bipolarity).
        line { class: "if-deadzone__live-guide", x1: "{input}", y1: "0", x2: "{input}", y2: "{output}" }
        circle { class: "if-deadzone__live-dot-halo", cx: "{input}", cy: "{output}", r: "0.07" }
        circle { class: "if-deadzone__live-dot", cx: "{input}", cy: "{output}", r: "0.04" }
    }
}

fn render_tick_labels() -> Element {
    // Hardcoded label table mirrors F10 (`response_curve/rendering.rs:284-292`).
    // The previous algorithmic version produced asymmetric output for negatives
    // (e.g. -0.5 -> "-0.5", but 0.5 -> ".5") because trim_start_matches('0')
    // stripped nothing past the leading '-'. Lookup table avoids the trap.
    let xs = [
        (-1.0_f64, "-1"),
        (-0.5, "-.5"),
        (0.0, "0"),
        (0.5, ".5"),
        (1.0, "1"),
    ];
    let ys = [(-1.0_f64, "-1"), (0.0, "0"), (1.0, "1")];
    rsx! {
        g {
            class: "if-deadzone__ticks",
            for (x, lbl) in xs.iter().copied() {
                text {
                    key: "tx-{x}",
                    class: "if-deadzone__tick-label",
                    x: "{x}", y: "1.04",
                    text_anchor: "middle",
                    "{lbl}"
                }
            }
            for (y, lbl) in ys.iter().copied() {
                text {
                    key: "ty-{y}",
                    class: "if-deadzone__tick-label",
                    x: "-1.0", dx: "0.015", y: "{-y}",
                    text_anchor: "start",
                    dominant_baseline: "central",
                    "{lbl}"
                }
            }
        }
    }
}
```

- [ ] **Step 2: Create `assets/frame/deadzone.css`**

```css
.if-deadzone {
  --color-deadzone-zone-sat: var(--color-error);
  --color-deadzone-zone-ramp: var(--color-primary);
  --color-deadzone-zone-dead: var(--color-text-subtle);
  --color-deadzone-curve: var(--color-primary);
  --color-deadzone-handle-fill: var(--color-text);
  --color-deadzone-handle-stroke: var(--color-bg-sunken);

  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3);
  width: 100%;
}

.if-deadzone__toolbar {
  display: flex;
  flex-direction: row;
  align-items: end;
  gap: var(--space-3);
}

.if-deadzone__toolbar-spacer { flex: 1; }

.if-deadzone__plot-frame {
  display: flex;
  position: relative;
  width: clamp(240px, 100%, 480px);
  aspect-ratio: 1 / 1;
  cursor: default;
  outline: none;
  user-select: none;
  -webkit-user-select: none;
}
.if-deadzone__plot-frame:focus-visible {
  outline: 2px solid var(--color-border-focus);
  outline-offset: 2px;
}
.if-deadzone__plot-frame[data-hovered="true"] { cursor: pointer; }
.if-deadzone__plot-frame[data-dragging="true"] { cursor: grabbing; }

.if-deadzone__plot {
  display: flex;
  width: 100%;
  height: 100%;
}

.if-deadzone__bg { fill: var(--color-bg-sunken); }

.if-deadzone__zone--sat  { fill: var(--color-deadzone-zone-sat);  opacity: 0.12; }
.if-deadzone__zone--ramp { fill: var(--color-deadzone-zone-ramp); opacity: 0.06; }
.if-deadzone__zone--dead { fill: var(--color-deadzone-zone-dead); opacity: 0.10; }

.if-deadzone__grid-major { stroke: var(--instr-grid-major); stroke-width: 1px; vector-effect: non-scaling-stroke; }
.if-deadzone__axis-cross { stroke: var(--instr-axis-cross); stroke-width: 1px; vector-effect: non-scaling-stroke; }
.if-deadzone__identity {
  stroke: var(--instr-identity-stroke);
  stroke-width: 1px;
  stroke-dasharray: 4 6;
  vector-effect: non-scaling-stroke;
  fill: none;
  opacity: 0.55;
}

.if-deadzone__path {
  stroke: var(--color-deadzone-curve);
  stroke-width: 1.75px;
  stroke-linejoin: round;
  filter: url(#if-instr-glow);
  vector-effect: non-scaling-stroke;
  fill: none;
}

.if-deadzone__handle {
  fill: var(--color-deadzone-handle-fill);
  stroke: var(--color-deadzone-handle-stroke);
  stroke-width: 1px;
  vector-effect: non-scaling-stroke;
}

.if-deadzone__hover-ring {
  stroke: var(--color-border-focus);
  stroke-width: 1.5px;
  vector-effect: non-scaling-stroke;
  opacity: 0.55;
}
.if-deadzone__drag-halo {
  fill: var(--color-border-focus);
  opacity: 0.30;
}
.if-deadzone__focus-ring {
  stroke: var(--color-border-focus);
  stroke-width: 1.5px;
  stroke-dasharray: 2 2;
  vector-effect: non-scaling-stroke;
  fill: none;
}
.if-deadzone__live-guide {
  stroke: var(--color-live);
  stroke-width: 0.5px;
  stroke-dasharray: 2 3;
  vector-effect: non-scaling-stroke;
  opacity: 0.5;
}
.if-deadzone__live-dot-halo { fill: var(--color-live); opacity: 0.18; }
.if-deadzone__live-dot {
  fill: var(--color-live);
  filter: url(#if-instr-glow);
}
.if-deadzone__tick-label {
  font-family: var(--font-mono);
  /* Unit-less: SVG resolves font-size in user units inside the viewBox.
     A `px` suffix here regresses to clipping (the px is interpreted as
     CSS pixels, far larger than 0.05 user units). Mirrors the F10 lesson
     at response_curve.css:132. */
  font-size: 0.05;
  fill: var(--color-text-subtle);
}

@media (prefers-reduced-motion: reduce) {
  .if-deadzone * {
    transition-duration: 0ms !important;
    animation-duration: 0ms !important;
  }
}
```

- [ ] **Step 3: Register `DEADZONE_CSS` in `ThemeProvider`**

In `theme/mod.rs`:

```rust
const DEADZONE_CSS: Asset = asset!("/assets/frame/deadzone.css");
```

Inside `rsx!`, add the `Stylesheet` mount immediately after `RESPONSE_CURVE_CSS`:

```rust
Stylesheet { href: RESPONSE_CURVE_CSS }
Stylesheet { href: DEADZONE_CSS }
```

- [ ] **Step 4: Add a smoke render test**

Append to `deadzone/rendering.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::VirtualDom;
    use dioxus_ssr::render;

    #[test]
    fn render_plot_emits_expected_class_count() {
        let cfg = DeadzoneConfig::default();
        let state = BodyState::default();
        let html = {
            let mut dom = VirtualDom::new_with_props(
                |props: RenderProps| render_plot(&props.cfg, &props.state, None),
                RenderProps { cfg: cfg.clone(), state: state.clone() },
            );
            let _ = dom.rebuild_in_place();
            render(&dom)
        };
        assert!(html.contains("if-deadzone__bg"));
        assert!(html.contains("if-deadzone__zone--sat"));
        assert!(html.contains("if-deadzone__zone--ramp"));
        assert!(html.contains("if-deadzone__zone--dead"));
        assert!(html.contains("if-deadzone__path"));
        assert!(html.contains("if-deadzone__handle"));
        assert!(html.contains("if-instr-glow"));
    }

    #[derive(Props, Clone, PartialEq)]
    struct RenderProps {
        cfg: DeadzoneConfig,
        state: BodyState,
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p inputforge-gui-dx deadzone::rendering::tests`
Expected: 1 passed.

Run: `cargo check -p inputforge-gui-dx`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/rendering.rs \
        crates/inputforge-gui-dx/assets/frame/deadzone.css \
        crates/inputforge-gui-dx/src/theme/mod.rs
git commit -m "feat(deadzone): SVG render layers and component CSS"
```

---

## Task 12: F11 header thumbnail (28x14 mini zone bar)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/thumbnail.rs`

Renders the F1 thumbnail per Q3: a horizontal mini zone bar at native 28x14 aspect, no grid, no live marker, no interactivity.

- [ ] **Step 1: Replace the stub with the failing tests**

```rust
// Rust guideline compliant 2026-05-03

//! F11 stage-header right-slot thumbnail. A 28x14 SVG mini zone bar that
//! replaces the default chevron when the stage is collapsed (and scales as
//! the right-slot content for the F2 IconButton's invariant 32x32 hit area).

use dioxus::prelude::*;

use inputforge_core::processing::deadzone::DeadzoneConfig;

/// Map a [-1, 1] viewBox-x value to the thumbnail's [0, 28] x range.
fn x_for(v: f64) -> f64 {
    (v + 1.0) * 14.0
}

pub(crate) fn header_thumbnail(config: &DeadzoneConfig) -> Element {
    let l = x_for(config.low());
    let cl = x_for(config.center_low());
    let ch = x_for(config.center_high());
    let h = x_for(config.high());
    rsx! {
        svg {
            class: "if-deadzone-thumb",
            view_box: "0 0 28 14",
            width: "28", height: "14",
            // Saturated outer bands (red, 55%).
            rect { fill: "var(--color-error)", fill_opacity: "0.55", x: "0", y: "0", width: "{l}", height: "14" }
            rect { fill: "var(--color-error)", fill_opacity: "0.55", x: "{h}", y: "0", width: "{28.0 - h}", height: "14" }
            // Ramp bands (blue, 30%).
            rect { fill: "var(--color-primary)", fill_opacity: "0.30", x: "{l}", y: "0", width: "{cl - l}", height: "14" }
            rect { fill: "var(--color-primary)", fill_opacity: "0.30", x: "{ch}", y: "0", width: "{h - ch}", height: "14" }
            // Dead band (sunken).
            rect { fill: "var(--color-bg-sunken)", x: "{cl}", y: "0", width: "{ch - cl}", height: "14" }
            // Threshold marks: 0.4px white-with-50% lines at the four positions.
            line { stroke: "white", stroke_opacity: "0.5", stroke_width: "0.4", x1: "{l}",  y1: "0", x2: "{l}",  y2: "14" }
            line { stroke: "white", stroke_opacity: "0.5", stroke_width: "0.4", x1: "{cl}", y1: "0", x2: "{cl}", y2: "14" }
            line { stroke: "white", stroke_opacity: "0.5", stroke_width: "0.4", x1: "{ch}", y1: "0", x2: "{ch}", y2: "14" }
            line { stroke: "white", stroke_opacity: "0.5", stroke_width: "0.4", x1: "{h}",  y1: "0", x2: "{h}",  y2: "14" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::VirtualDom;
    use dioxus_ssr::render;

    fn html_for(cfg: DeadzoneConfig) -> String {
        let mut dom = VirtualDom::new_with_props(
            |props: HtmlProps| header_thumbnail(&props.cfg),
            HtmlProps { cfg },
        );
        let _ = dom.rebuild_in_place();
        render(&dom)
    }

    #[derive(Props, Clone, PartialEq)]
    struct HtmlProps {
        cfg: DeadzoneConfig,
    }

    #[test]
    fn default_config_renders_full_outer_dead() {
        let html = html_for(DeadzoneConfig::default());
        // Default: low=-1, cl=0, ch=0, high=1.
        // Saturated rects collapse to width 0 at the edges, ramp covers
        // the full halves, dead band is zero-width. Exactly THREE rects
        // should have width=0 (left sat, right sat, dead band).
        assert_eq!(html.matches(r#"width="0""#).count(), 3);
    }

    #[test]
    fn aggressive_config_has_visible_outer_sat() {
        let cfg = DeadzoneConfig::new(-0.5, -0.1, 0.1, 0.5).expect("valid");
        let html = html_for(cfg);
        // x_for(-0.5) = 7.0, so left sat rect starts at width 7.
        assert!(html.contains(r#"width="7""#));
    }

    #[test]
    fn wide_dead_band_is_visible() {
        let cfg = DeadzoneConfig::new(-0.85, -0.3, 0.3, 0.85).expect("valid");
        let html = html_for(cfg);
        // Dead band: cl=-0.3 -> 9.8, ch=0.3 -> 18.2, width 8.4.
        // Approximate: the rendered width attr is 8.4 or 8.4000... depending on Rust f64 fmt.
        assert!(html.contains("8.4"));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p inputforge-gui-dx deadzone::thumbnail::tests`
Expected: 3 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/thumbnail.rs
git commit -m "feat(deadzone): header thumbnail mini zone bar"
```

---

## Task 13: F11 toolbar (4 NumberInput rows + Reset)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/toolbar.rs`

Q2 = E1: numeric row above plot. Each commit goes through `DeadzoneConfig::new`. Reset on default config is a no-op.

- [ ] **Step 1: Replace the stub `toolbar.rs`**

```rust
// Rust guideline compliant 2026-05-03

//! Numeric toolbar above the plot. Four `NumberInput` rows wrapped in
//! `Field` form-rows (for label and inline error), plus a Reset button.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::components::{Button, ButtonSize, ButtonVariant, Field, NumberInput};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_body::deadzone::mutation::default_config;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::dispatch_stage_edit;
use crate::frame::mapping_editor::undo_log::StageId;

#[component]
pub(crate) fn Toolbar(
    config: DeadzoneConfig,
    stage_id: StageId,
    root_actions: Vec<Action>,
    mapping_key: MappingKey,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let mut undo_log = editor.undo_log;
    let mut malformed_hints = editor.malformed_hints;
    let cmd_tx = ctx.commands.clone();
    let config_signal = ctx.config;

    let low_signal = use_signal(|| config.low());
    let cl_signal = use_signal(|| config.center_low());
    let ch_signal = use_signal(|| config.center_high());
    let high_signal = use_signal(|| config.high());

    let stage_id_low = stage_id.clone();
    let stage_id_cl = stage_id.clone();
    let stage_id_ch = stage_id.clone();
    let stage_id_high = stage_id.clone();
    let stage_id_reset = stage_id.clone();
    let mapping_key_low = mapping_key.clone();
    let mapping_key_cl = mapping_key.clone();
    let mapping_key_ch = mapping_key.clone();
    let mapping_key_high = mapping_key.clone();
    let mapping_key_reset = mapping_key.clone();
    let cmd_tx_low = cmd_tx.clone();
    let cmd_tx_cl = cmd_tx.clone();
    let cmd_tx_ch = cmd_tx.clone();
    let cmd_tx_high = cmd_tx.clone();
    let cmd_tx_reset = cmd_tx.clone();
    let cfg_low = config.clone();
    let cfg_cl = config.clone();
    let cfg_ch = config.clone();
    let cfg_high = config.clone();
    let cfg_current = config.clone();
    let actions_low = root_actions.clone();
    let actions_cl = root_actions.clone();
    let actions_ch = root_actions.clone();
    let actions_high = root_actions.clone();
    let actions_reset = root_actions.clone();

    let on_low_commit = move |v: f64| commit_field(
        &cfg_low, FieldId::Low, v,
        &actions_low, &stage_id_low, &mapping_key_low,
        &cmd_tx_low, &mut undo_log, &mut malformed_hints,
        config_signal,
    );
    let on_cl_commit = move |v: f64| commit_field(
        &cfg_cl, FieldId::CL, v,
        &actions_cl, &stage_id_cl, &mapping_key_cl,
        &cmd_tx_cl, &mut undo_log, &mut malformed_hints,
        config_signal,
    );
    let on_ch_commit = move |v: f64| commit_field(
        &cfg_ch, FieldId::CH, v,
        &actions_ch, &stage_id_ch, &mapping_key_ch,
        &cmd_tx_ch, &mut undo_log, &mut malformed_hints,
        config_signal,
    );
    let on_high_commit = move |v: f64| commit_field(
        &cfg_high, FieldId::High, v,
        &actions_high, &stage_id_high, &mapping_key_high,
        &cmd_tx_high, &mut undo_log, &mut malformed_hints,
        config_signal,
    );
    let on_reset = move |_| {
        let default = default_config();
        // Exact float equality is intentional: a config that's been moved and
        // returned to literally the same f64 values is still a no-op reset.
        if cfg_current == default { return; }
        let cfg_snap = config_signal.read();
        let name = cfg_snap.mapping_names.get(&mapping_key_reset.1).cloned();
        drop(cfg_snap);
        dispatch_stage_edit(
            &actions_reset,
            &stage_id_reset,
            Action::Deadzone { config: default },
            &mapping_key_reset,
            name,
            &cmd_tx_reset,
            &mut undo_log,
            "deadzone: reset".to_owned(),
        );
        malformed_hints.write().remove(&stage_id_reset);
    };

    let err = malformed_hints.read().get(&stage_id).cloned();

    rsx! {
        div { class: "if-deadzone__toolbar",
            Field { label: "Low".into(), for_id: Some("dz-low".into()), error: err.clone(),
                NumberInput {
                    id: Some("dz-low".into()),
                    value: low_signal.into(),
                    min: -1.0,
                    // The 0.001 epsilon is a UX nudge: it gives the
                    // spinner a buffer above the validator's strict `<`
                    // boundary so a single up-arrow press does not
                    // immediately collide with center_low. NOT a
                    // correctness gate (DeadzoneConfig::new uses strict
                    // `<` with no epsilon).
                    max: config.center_low() - 0.001,
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_low_commit,
                }
            }
            Field { label: "CL".into(), for_id: Some("dz-cl".into()), error: err.clone(),
                NumberInput {
                    id: Some("dz-cl".into()),
                    value: cl_signal.into(),
                    min: config.low() + 0.001,
                    max: config.center_high(),
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_cl_commit,
                }
            }
            Field { label: "CH".into(), for_id: Some("dz-ch".into()), error: err.clone(),
                NumberInput {
                    id: Some("dz-ch".into()),
                    value: ch_signal.into(),
                    min: config.center_low(),
                    max: config.high() - 0.001,
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_ch_commit,
                }
            }
            Field { label: "High".into(), for_id: Some("dz-high".into()), error: err.clone(),
                NumberInput {
                    id: Some("dz-high".into()),
                    value: high_signal.into(),
                    min: config.center_high() + 0.001,
                    max: 1.0,
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_high_commit,
                }
            }
            div { class: "if-deadzone__toolbar-spacer" }
            Button {
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                onclick: on_reset,
                "Reset"
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FieldId { Low, CL, CH, High }

#[expect(clippy::too_many_arguments, reason = "matches dispatch_stage_edit signature plus malformed_hints")]
fn commit_field(
    config: &DeadzoneConfig,
    field: FieldId,
    new_value: f64,
    actions: &[Action],
    stage_id: &StageId,
    mapping_key: &MappingKey,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
    malformed_hints: &mut Signal<std::collections::HashMap<StageId, String>>,
    config_signal: Signal<crate::context::ConfigSnapshot>,
) {
    let (low, cl, ch, high) = match field {
        FieldId::Low => (new_value, config.center_low(), config.center_high(), config.high()),
        FieldId::CL => (config.low(), new_value, config.center_high(), config.high()),
        FieldId::CH => (config.low(), config.center_low(), new_value, config.high()),
        FieldId::High => (config.low(), config.center_low(), config.center_high(), new_value),
    };
    let candidate = match DeadzoneConfig::new(low, cl, ch, high) {
        Ok(c) => c,
        Err(err) => {
            malformed_hints.write().insert(stage_id.clone(), err.to_string());
            return;
        }
    };
    let cfg_snap = config_signal.read();
    let name = cfg_snap.mapping_names.get(&mapping_key.1).cloned();
    drop(cfg_snap);
    let label_field = match field {
        FieldId::Low => "low",
        FieldId::CL => "center_low",
        FieldId::CH => "center_high",
        FieldId::High => "high",
    };
    let old_value = match field {
        FieldId::Low => config.low(),
        FieldId::CL => config.center_low(),
        FieldId::CH => config.center_high(),
        FieldId::High => config.high(),
    };
    let label = format!("deadzone: {label_field} {old_value:+.2} -> {new_value:+.2}");
    dispatch_stage_edit(
        actions,
        stage_id,
        Action::Deadzone { config: candidate },
        mapping_key,
        name,
        cmd_tx,
        undo_log,
        label,
    );
    malformed_hints.write().remove(stage_id);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo check -p inputforge-gui-dx`
Expected: clean (toolbar's behavioural tests live in `deadzone/tests.rs` written in Task 16; this task only verifies compilation).

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/toolbar.rs
git commit -m "feat(deadzone): toolbar with four numeric inputs and reset button"
```

---

## Task 14: Wire `DeadzoneBody` (state, signals, JS bridge, keyboard)

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/mod.rs`

Replace the stub `DeadzoneBody` with the full reactive component. Mirrors F10's `ResponseCurveBody` shape: `use_signal` for `BodyState`, `use_signal` for the working config, projection from `ConfigSnapshot`, JS bridge via `instruments::bridge::mount_mouse_bridge`, keyboard via `keyboard::handle_key`.

- [ ] **Step 1: Replace `DeadzoneBody` body**

```rust
#[component]
#[allow(unused_qualifications, reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners.")]
pub(crate) fn DeadzoneBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    config: DeadzoneConfig,
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let config_signal = ctx.config;
    let editor = use_context::<EditorState>();
    let mut undo_log = editor.undo_log;
    let mut malformed_hints = editor.malformed_hints;

    let mut body: Signal<state::BodyState> = use_signal(state::BodyState::default);
    let working_config: Signal<Option<DeadzoneConfig>> = use_signal(|| None);

    let time_baseline = use_signal(std::time::Instant::now);

    // Project the live config out of `ConfigSnapshot.selected_mapping_actions`.
    let cfg = config_signal.read().clone();
    let live_actions = cfg
        .selected_mapping_actions
        .clone()
        .unwrap_or_else(|| root_actions.clone());
    let live_config = project_stage_config(&live_actions, &stage_id, &config);

    // Live tracking dot via the shared instrument helper.
    let live_value: Option<f64> = instruments::live_axis::compute_live_axis_value(
        &stage_id, &mapping_key.1, &ctx, &live_actions,
    );

    // ----- JS-bridge mouse-event dispatch -----
    let plot_dom_id = instruments::bridge::stage_id_dom_id("if-deadzone-plot", &stage_id);
    let mapping_key_for_bridge = mapping_key.clone();
    let stage_id_for_bridge = stage_id.clone();
    let cmd_tx_for_bridge = ctx.commands.clone();
    let config_seed_for_bridge = config.clone();
    let dispatch = move |payload: instruments::bridge::BridgeEvent| {
        dispatch_bridge_event(
            &payload,
            body,
            working_config,
            config_signal,
            undo_log,
            malformed_hints,
            &mapping_key_for_bridge,
            &stage_id_for_bridge,
            &config_seed_for_bridge,
            &cmd_tx_for_bridge,
        );
    };
    let on_mounted = instruments::bridge::mount_mouse_bridge(plot_dom_id.clone(), dispatch);

    // ----- Keyboard -----
    let mapping_key_for_key = mapping_key.clone();
    let stage_id_for_key = stage_id.clone();
    let cmd_tx_for_key = ctx.commands.clone();
    let config_seed_for_key = config.clone();
    let on_key = move |evt: KeyboardEvent| {
        let key = match (evt.key(), evt.modifiers().shift()) {
            (Key::Tab, true) => keyboard::KeyInput::ShiftTab,
            (Key::Tab, false) => keyboard::KeyInput::Tab,
            (Key::ArrowLeft, shift) => keyboard::KeyInput::ArrowLeft { shift },
            (Key::ArrowRight, shift) => keyboard::KeyInput::ArrowRight { shift },
            (Key::ArrowUp, shift) => keyboard::KeyInput::ArrowUp { shift },
            (Key::ArrowDown, shift) => keyboard::KeyInput::ArrowDown { shift },
            (Key::Home, _) => keyboard::KeyInput::Home,
            (Key::End, _) => keyboard::KeyInput::End,
            (Key::Enter, _) => keyboard::KeyInput::Enter,
            (Key::Delete | Key::Backspace, _) => keyboard::KeyInput::Delete,
            (Key::Escape, _) => keyboard::KeyInput::Escape,
            _ => return,
        };
        if !matches!(key, keyboard::KeyInput::Tab | keyboard::KeyInput::ShiftTab) {
            evt.prevent_default();
        }
        #[allow(clippy::cast_possible_truncation, reason = "ms-since-mount fits u64 for any session")]
        let now_ms = std::time::Instant::now()
            .saturating_duration_since(*time_baseline.peek())
            .as_millis() as u64;
        let cfg = config_signal.read();
        let actions: Vec<Action> = cfg.selected_mapping_actions.clone().unwrap_or_default();
        let live_cfg = project_stage_config(&actions, &stage_id_for_key, &config_seed_for_key);
        let name = cfg.mapping_names.get(&mapping_key_for_key.1).cloned();
        drop(cfg);
        let (next, new_cfg, outcome, _) = keyboard::handle_key(body.peek().clone(), &live_cfg, key, now_ms);
        body.set(next);
        let Some(new) = new_cfg else { return };
        match outcome {
            Some(keyboard::KeyOutcome::PushUndo { label }) => {
                instruments::stage_dispatch::dispatch_stage_edit(
                    &actions,
                    &stage_id_for_key,
                    Action::Deadzone { config: new },
                    &mapping_key_for_key,
                    name,
                    &cmd_tx_for_key,
                    &mut undo_log,
                    label,
                );
            }
            Some(keyboard::KeyOutcome::MergeUndo) => {
                instruments::stage_dispatch::dispatch_stage_edit_no_undo(
                    &actions,
                    &stage_id_for_key,
                    Action::Deadzone { config: new },
                    &mapping_key_for_key,
                    name,
                    &cmd_tx_for_key,
                );
            }
            None => {} // Escape revert: body-local, no dispatch.
        }
    };

    let mut body_for_focusout = body;
    let on_focus_out = move |_| {
        body_for_focusout.with_mut(|s| {
            s.nudge_coalesce.reset();
        });
    };

    let snap = body.read();
    let dragging_attr = snap.dragging.is_some().to_string();
    let hovered_attr = snap.hovered_handle.is_some().to_string();
    drop(snap);

    rsx! {
        div { class: "if-deadzone",
            toolbar::Toolbar {
                config: live_config.clone(),
                stage_id: stage_id.clone(),
                root_actions: live_actions.clone(),
                mapping_key: mapping_key.clone(),
            }
            div {
                class: "if-deadzone__plot-frame",
                id: "{plot_dom_id}",
                tabindex: "0",
                "aria-label": "deadzone curve",
                "data-hovered": "{hovered_attr}",
                "data-dragging": "{dragging_attr}",
                onmounted: on_mounted,
                onkeydown: on_key,
                onfocusout: on_focus_out,
                { rendering::render_plot(&live_config, &body.read(), live_value) }
            }
        }
    }
}
```

Add the projection helper and the dispatcher:

```rust
fn project_stage_config(
    actions: &[Action],
    stage_id: &StageId,
    fallback: &DeadzoneConfig,
) -> DeadzoneConfig {
    use crate::frame::mapping_editor::pipeline::at_path;
    match at_path(actions, stage_id) {
        Some(Action::Deadzone { config }) => config.clone(),
        _ => fallback.clone(),
    }
}

#[expect(clippy::too_many_arguments, clippy::too_many_lines, reason = "Body-shaped dispatch keeps each per-event handler local.")]
fn dispatch_bridge_event(
    payload: &instruments::bridge::BridgeEvent,
    mut body: Signal<state::BodyState>,
    mut working_config: Signal<Option<DeadzoneConfig>>,
    config_signal: Signal<crate::context::ConfigSnapshot>,
    mut undo_log: Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
    mut malformed_hints: Signal<std::collections::HashMap<StageId, String>>,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    config_seed: &DeadzoneConfig,
    cmd_tx: &std::sync::mpsc::Sender<inputforge_core::engine::EngineCommand>,
) {
    if payload.rs <= 0.0 {
        return;
    }
    let rect = interaction::PlotRect {
        x: payload.rl,
        y: payload.rt,
        size: payload.rs,
    };

    match payload.kind.as_str() {
        "down" => {
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_config(actions, stage_id, config_seed);
            drop(cfg);
            let prev = body.peek().clone();
            let (next, _, _) = interaction::handle_pointer_down(prev, &live, (payload.x, payload.y), &rect);
            if next.dragging.is_some() {
                working_config.set(Some(live));
            }
            body.set(next);
        }
        "move" => {
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_config(actions, stage_id, config_seed);
            drop(cfg);
            let prev = body.peek().clone();
            let (next, new_cfg_opt, _) = interaction::handle_pointer_move(prev, &live, (payload.x, payload.y), &rect);
            if let Some(new_cfg) = new_cfg_opt {
                working_config.set(Some(new_cfg));
            }
            body.set(next);
        }
        "up" => {
            let prev = body.peek().clone();
            if prev.dragging.is_none() {
                return;
            }
            let cfg = config_signal.read();
            let actions = cfg.selected_mapping_actions.as_deref().unwrap_or(&[]);
            let live = project_stage_config(actions, stage_id, config_seed);
            let actions_snap = actions.to_vec();
            let name = cfg.mapping_names.get(&mapping_key.1).cloned();
            drop(cfg);
            let dragged = working_config.peek().clone().unwrap_or_else(|| live.clone());
            working_config.set(None);
            let (mut next, result, _) = interaction::handle_pointer_up(prev, &dragged);
            // Phantom-undo guard: a mousedown + mouseup with no intervening
            // move means `working_config` was never updated, so `dragged ==
            // live`. Without this guard the body would dispatch a no-op
            // `SetMapping` and record a `"deadzone: drag"` undo entry for a
            // user gesture that did not change anything. F10 has the
            // identical bug today; Task 17 backports the same guard.
            if dragged == live {
                body.set(next);
                return;
            }
            match result {
                Ok(valid) => {
                    instruments::stage_dispatch::dispatch_stage_edit(
                        &actions_snap,
                        stage_id,
                        Action::Deadzone { config: valid },
                        mapping_key,
                        name,
                        cmd_tx,
                        &mut undo_log,
                        "deadzone: drag".to_owned(),
                    );
                    malformed_hints.write().remove(stage_id);
                }
                Err(err) if err.is_empty() => {}
                Err(err) => {
                    let _revert = next.pre_drag_config.take();
                    malformed_hints.write().insert(stage_id.clone(), err);
                }
            }
            body.set(next);
        }
        _ => {} // dbl, ctx ignored: F11 has no double-click or right-click semantics.
    }
}
```

Add the imports at the top of `mod.rs`:

```rust
use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_body::instruments;
use crate::frame::mapping_editor::undo_log::StageId;

// Sibling submodules used by `DeadzoneBody` and `dispatch_bridge_event`.
// `keyboard::KeyInput`/`KeyOutcome` (on_key closure), `interaction::PlotRect`
// + `handle_pointer_*` (bridge dispatch), `rendering::render_plot` (rsx body),
// `state::BodyState` (signal type), `toolbar::Toolbar` (rsx child).
use super::deadzone::{interaction, keyboard, rendering, state, toolbar};
```

(Reading `super::deadzone::*` from inside `deadzone/mod.rs` itself is unusual; if Rust resolution objects, change to `use crate::frame::mapping_editor::pipeline::stage_body::deadzone::{interaction, keyboard, rendering, state, toolbar};` or rely on the in-module sibling resolution where the children are already declared by `pub(crate) mod ...;` at the top of this same file.)

- [ ] **Step 2: Run tests**

Run: `cargo check -p inputforge-gui-dx`
Expected: clean.

Run: `cargo test -p inputforge-gui-dx deadzone::`
Expected: all prior `deadzone::*` tests still green.

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/mod.rs
git commit -m "feat(deadzone): wire reactive body, JS bridge, keyboard dispatch"
```

---

## Task 15: Replace `Action::Deadzone` arms in the dispatcher

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs`
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/placeholders.rs` (drop `DeadzonePlaceholder`)

The two-arm replacement that activates F11 in the live UI. Per the spec mount-points section.

- [ ] **Step 1: Replace the dispatcher's Deadzone arm**

In `stage_body/mod.rs::StageBody`, replace:

```rust
Action::Deadzone { .. } => rsx! { placeholders::DeadzonePlaceholder {} },
```

with:

```rust
Action::Deadzone { config } => rsx! {
    deadzone::DeadzoneBody {
        mapping_key: mapping_key.clone(),
        stage_id: stage_id.clone(),
        config: config.clone(),
        root_actions: root_actions.clone(),
    }
},
```

- [ ] **Step 2: Replace the header right-slot arm**

In `header_right_slot`, replace:

```rust
Action::Deadzone { .. } => default_chevron(expanded),
```

with:

```rust
Action::Deadzone { config } => deadzone::thumbnail::header_thumbnail(config),
```

- [ ] **Step 3: Drop the placeholder**

In `placeholders.rs`, delete `DeadzonePlaceholder`. Search the crate for any remaining references with `cargo check -p inputforge-gui-dx`; the only previous use site was the dispatcher arm replaced above.

- [ ] **Step 4: Run tests**

Run: `cargo check -p inputforge-gui-dx`
Expected: clean.

Run: `cargo test -p inputforge-gui-dx`
Expected: full crate green.

- [ ] **Step 5: Manual smoke (human only, not for agent execution)**

Run: `dx run -p inputforge-app`. Then:
1. Create a new mapping with a `Deadzone` stage.
2. Expand the stage; the F11 body renders (zone bands visible, four handles).
3. Drag the `CenterLow` handle right; the curve updates, the engine receives `SetMapping`, the pipeline output reflects the new dead zone.
4. Press `Tab`; focus advances `Low` -> `CL` -> `CH` -> `High`.
5. Type `0.5` into the `Low` field, press Enter; validator rejects (engine error renders inline next to the field), no dispatch.
6. Click `Reset` on a non-default config; config snaps back to default, undo entry recorded.
7. Collapse the stage; the header right-slot shows the mini zone bar at 28x14.
8. Press `Ctrl+Z`; undo replays.
9. mousedown on a handle then mouseup at the same coordinates without moving: undo log unchanged (phantom-undo guard).

Quit when verified.

- [ ] **Step 6: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/placeholders.rs
git commit -m "feat(deadzone): replace placeholder with F11 body and thumbnail"
```

---

## Task 15.5: Extract shared SSR mount helper

**Files:**
- Create: `crates/inputforge-gui-dx/src/frame/mapping_editor/test_helpers.rs` (or a path consistent with existing test-utility conventions in this crate; verify by `Grep` for `#[cfg(test)] pub(crate) mod` siblings of `mapping_editor/mod.rs` before settling on the exact path)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs` (add `#[cfg(test)] pub(crate) mod test_helpers;`)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/tests.rs` (refactor 5 inline mount blocks at `:162-185`, `:237-258`, `:315-336`, `:479-500`, `:549-611` to call the shared helper)

This task is non-behavioural; it sets up Task 16 to be straightforward and removes 5 copies of the same context-stack construction from F10's tests. F11's `Task 16` consumes the same helper. No production code changes.

- [ ] **Step 1: Write the failing test for the helper**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/test_helpers.rs`:

```rust
// Rust guideline compliant 2026-05-03

//! Shared `#[cfg(test)]` mount harness for stage-body SSR tests. Builds a
//! minimal `AppContext` + `EditorState` provider stack and renders the
//! caller's RSX to a string. Used by F10 (`response_curve/tests.rs`) and F11
//! (`deadzone/tests.rs`) to keep the per-instrument tests focused on output
//! assertions instead of context construction.

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;

    #[test]
    fn helper_renders_trivial_body() {
        let html = mount_stage_body_test(|| rsx! { div { class: "probe", "ok" } });
        assert!(html.contains(r#"class="probe""#));
        assert!(html.contains("ok"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx mapping_editor::test_helpers::tests --no-run`
Expected: compile error: `cannot find function mount_stage_body_test`.

- [ ] **Step 3: Implement the helper**

Add the public surface plus the private `build_test_app_context` factory. Lift the `RawHandles { state, commands, settings }` + `MetaSnapshot` / `ConfigSnapshot` / `LiveSnapshot` Signal construction from F10's existing inline blocks (one canonical site to copy from: `response_curve/tests.rs:162-185`).

```rust
use dioxus::prelude::*;
use dioxus_ssr::render;

use crate::context::AppContext;
use crate::frame::mapping_editor::{use_editor_state_provider, use_live_capture_provider};

/// Mount a stage-body RSX expression in a test VirtualDom with the
/// canonical `AppContext` + `EditorState` + live-capture provider stack.
/// Returns the rendered HTML.
pub(crate) fn mount_stage_body_test<F>(render_fn: F) -> String
where
    F: 'static + Clone + Fn() -> Element,
{
    let mut dom = VirtualDom::new(move || {
        let ctx = build_test_app_context();
        use_context_provider(|| ctx);
        use_editor_state_provider();
        use_live_capture_provider();
        render_fn()
    });
    let _ = dom.rebuild_in_place();
    render(&dom)
}

/// Build a default `AppContext` for tests. Mirrors the inline construction
/// previously duplicated across F10's 5 SSR-test sites; centralised here so a
/// single change propagates to every body's tests.
fn build_test_app_context() -> AppContext {
    // ... lift verbatim from response_curve/tests.rs:162-185.
    // RawHandles { state, commands, settings } + MetaSnapshot/ConfigSnapshot/
    // LiveSnapshot signals + `AppContext { state, live, config, commands, ...
    // }` constructor. Read the actual block on the working tree before
    // copying; the type names may have drifted.
    todo!()
}
```

- [ ] **Step 4: Run helper test to verify it passes**

Run: `cargo test -p inputforge-gui-dx mapping_editor::test_helpers::tests`
Expected: 1 passed.

- [ ] **Step 5: Refactor F10's 5 SSR mount sites**

In `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/tests.rs`, replace each of the 5 inline mount blocks (around lines 162-185, 237-258, 315-336, 479-500, 549-611) with a call to `mount_stage_body_test`. Each refactored test shrinks from ~25 lines of context setup to a single `let html = mount_stage_body_test(move || rsx! { ResponseCurveBody { ... } });` line.

The helper takes only the inner RSX; the outer context stack is identical across all 5 sites (verify by reading the actual blocks). If a site has bespoke context (e.g. a non-default `AppContext.live` for the live-tracking-dot test), accept either of:

- **Option A**: extend `mount_stage_body_test` to accept an `AppContext` factory closure (`F2: Fn() -> AppContext`).
- **Option B**: keep the bespoke site inline and refactor only the 4 default sites.

Pick Option B unless the bespoke site is an exact one-Signal mutation of the default; the helper should stay simple.

- [ ] **Step 6: Verify F10 tests still pass**

Run: `cargo test -p inputforge-gui-dx response_curve::tests`
Expected: full module green; same test count as before the refactor.

Run: `cargo test -p inputforge-gui-dx`
Expected: full crate green.

- [ ] **Step 7: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/test_helpers.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/tests.rs
git commit -m "refactor(test): extract shared mount_stage_body_test helper"
```

---

## Task 16: SSR mount tests for `DeadzoneBody`

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/tests.rs`

End-to-end SSR tests for the assembled body. Mirrors F10's `tests.rs` shape.

- [ ] **Step 1: Replace the stub `tests.rs`**

```rust
// Rust guideline compliant 2026-05-03

//! SSR mount tests for `DeadzoneBody`. Mounts via the shared
//! `mount_stage_body_test` helper from Task 15.5; assertions target the
//! rendered HTML.

#[cfg(test)]
mod tests {
    use super::super::*;
    use dioxus::prelude::*;
    use inputforge_core::action::Action;
    use inputforge_core::processing::deadzone::DeadzoneConfig;
    use inputforge_core::types::InputAddress;

    use crate::frame::MappingKey;
    use crate::frame::mapping_editor::test_helpers::mount_stage_body_test;
    use crate::frame::mapping_editor::undo_log::{StageId, StageIdSegment};

    fn stage_id() -> StageId {
        StageId(vec![StageIdSegment::Index(0)])
    }

    fn mapping_key() -> MappingKey {
        ("default".into(), InputAddress::Unbound)
    }

    fn render_body(cfg: DeadzoneConfig) -> String {
        let key = mapping_key();
        let sid = stage_id();
        let cfg_for_actions = cfg.clone();
        mount_stage_body_test(move || rsx! {
            DeadzoneBody {
                mapping_key: key.clone(),
                stage_id: sid.clone(),
                config: cfg.clone(),
                root_actions: vec![Action::Deadzone { config: cfg_for_actions.clone() }],
            }
        })
    }

    #[test]
    fn default_config_renders_4_handles() {
        let html = render_body(DeadzoneConfig::default());
        let count = html.matches("if-deadzone__handle").count();
        assert_eq!(count, 4);
    }

    #[test]
    fn renders_zone_band_classes() {
        let cfg = DeadzoneConfig::new(-0.5, -0.1, 0.1, 0.5).expect("valid");
        let html = render_body(cfg);
        assert!(html.contains("if-deadzone__zone--sat"));
        assert!(html.contains("if-deadzone__zone--ramp"));
        assert!(html.contains("if-deadzone__zone--dead"));
    }

    #[test]
    fn no_live_dot_when_input_unbound() {
        let html = render_body(DeadzoneConfig::default());
        assert!(!html.contains("if-deadzone__live-dot\""));
    }

    #[test]
    fn renders_toolbar_inputs() {
        let html = render_body(DeadzoneConfig::default());
        assert!(html.contains(r#"id="dz-low""#));
        assert!(html.contains(r#"id="dz-cl""#));
        assert!(html.contains(r#"id="dz-ch""#));
        assert!(html.contains(r#"id="dz-high""#));
    }
}
```

The `mount_stage_body_test` helper (Task 15.5) provides the `AppContext` + `EditorState` + live-capture context stack; the closure passed in is the inner RSX. `MappingKey` is a tuple alias `(String, InputAddress)`, so the `("default".into(), Unbound)` tuple-literal form is required (NOT a tuple-struct constructor).

- [ ] **Step 2: Run tests**

Run: `cargo test -p inputforge-gui-dx deadzone::tests::tests`
Expected: 4 passed.

Run: `cargo test -p inputforge-gui-dx`
Expected: full crate green (re-run is the integration check after F11 lands).

- [ ] **Step 3: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/deadzone/tests.rs
git commit -m "test(deadzone): SSR mount tests covering body, toolbar, live dot gate"
```

---

## Task 17: Backport phantom-undo guard to F10

**Files:**
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs` (`up` arm of `dispatch_bridge_event`, around `:339-376`)
- Modify: `crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/tests.rs` (new failing test)

F11's Task 14 already added the `if dragged == live { ... return; }` guard to suppress the phantom `"deadzone: drag"` undo entry on a click-without-move. F10 still has the identical bug; this task backports the same guard so the two bodies stay symmetric.

- [ ] **Step 1: Write the failing test**

In `response_curve/tests.rs`, add a test that uses the shared `mount_stage_body_test` helper (Task 15.5) and simulates a mousedown + mouseup at the same coordinates on a curve anchor. Assert that `editor.undo_log.read().stacks.get(&key).map(|h| h.undo.len()).unwrap_or(0) == 0` after the round-trip.

(SSR-driven dispatch is fiddly; if synthesising the bridge events through the JS path is impractical, instead extract the up-arm match body into a pure helper `dispatch_pointer_up_into(undo_log: &mut UndoLog, working_curve: Option<ResponseCurve>, live_curve: ResponseCurve, ...)` mirroring Task 4's helper-form pattern, and write the test against that helper. The pure-helper approach is preferred; it keeps the test fast and deterministic.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p inputforge-gui-dx response_curve::tests::click_without_move_records_no_undo --no-run`, then run without `--no-run`.
Expected: test panics on the assertion (the guard is not yet present, so an undo entry is recorded).

- [ ] **Step 3: Implement the guard**

In `response_curve/mod.rs::dispatch_bridge_event`'s `up` arm, immediately after `let (mut next, result, _) = interaction::handle_pointer_up(prev, &dragged);` (and before the `match result` block), add the same shape Task 14 uses in F11:

```rust
// Phantom-undo guard: a mousedown + mouseup with no intervening move
// means `working_curve` was never updated, so `dragged == live`. Without
// this guard the body would dispatch a no-op `SetMapping` and record a
// `"curve: drag"` undo entry for a user gesture that did not change
// anything. F11 has the same guard in `deadzone/mod.rs` (Task 14).
if dragged == live {
    body.set(next);
    return;
}
```

(Verify the F10 variable names: the working copy may be named `working_curve` and the projected current value may be named `live` or `live_curve`; rename the equality operands accordingly. The two should be the same `ResponseCurve` shape.)

- [ ] **Step 4: Run tests**

Run: `cargo test -p inputforge-gui-dx response_curve::tests::click_without_move_records_no_undo`
Expected: 1 passed.

Run: `cargo test -p inputforge-gui-dx response_curve::`
Expected: full module green.

Run: `cargo test -p inputforge-gui-dx`
Expected: full crate green.

- [ ] **Step 5: Commit**

```bash
git add crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/mod.rs \
        crates/inputforge-gui-dx/src/frame/mapping_editor/pipeline/stage_body/response_curve/tests.rs
git commit -m "fix(response_curve): suppress phantom undo entry on click-without-move"
```

---

## Self-review checklist (run after Task 17 completes)

1. **Spec coverage:**
   - Q1 visualization (zone bands behind curve): Tasks 11 (rendering), 7 (state types).
   - Q2 toolbar layout (numeric row above plot): Task 13.
   - Q3 thumbnail (28x14 mini zone bar): Task 12.
   - Q4 no symmetric mode: state has no toggle (Task 7), keyboard has no paired-handle drag (Task 10).
   - Q5 shared instrument tokens: Tasks 1-5.
   - Q6 live tracking gate (top-level only): inherited from `instruments::live_axis` extracted in Task 2.
   - Q7 no position trail / snap: not present in any task; spec records as deferred.
   - Engine integration: Task 13 (`DeadzoneConfig::new` validation), Task 15 (`SetMapping` dispatch via `instruments::stage_dispatch`).
   - F2 NumberInput `oncommit`: Task 6.

2. **Placeholder scan:** No `TODO`, `TBD`, `Add error handling`, `Similar to Task N`, or naked `todo!()` survives past the task that introduces it. Each `todo!()` in Task 8's step 1 is replaced in step 3 of the same task. Task 15.5's `build_test_app_context` `todo!()` is filled in by lifting verbatim from `response_curve/tests.rs` during that same task's Step 3.

3. **Type consistency:** Names used consistently: `BodyState`, `DragInProgress`, `HandleId`, `KeyInput`, `KeyKind`, `KeyOutcome`, `KeyHandlerOut`, `PlotRect`, `BridgeEvent`, `NudgeCoalesce<K>` (generic, instantiated as `NudgeCoalesce<KeyKind>` per editor), `default_config`, `with_handle`, `adjacent_bounds`, `handle_positions`, `nearest_handle`, `dispatch_stage_edit`, `dispatch_stage_edit_into`, `dispatch_stage_edit_no_undo`, `compute_live_axis_value`, `mount_mouse_bridge`, `stage_id_dom_id`, `header_thumbnail`, `Toolbar`, `DeadzoneBody`, `mount_stage_body_test`, `INSTR_GLOW_STDDEV`. Glow filter id `if-instr-glow` consistent across CSS and Rust. CSS classes consistent: `if-deadzone__bg`, `__zone--sat/--ramp/--dead`, `__grid-major`, `__axis-cross`, `__identity`, `__path`, `__handle`, `__hover-ring`, `__drag-halo`, `__focus-ring`, `__live-guide`, `__live-dot-halo`, `__live-dot`, `__tick-label`, `__plot-frame`, `__plot`, `__toolbar`, `__toolbar-spacer`, plus `if-deadzone-thumb` for the standalone header thumbnail.

4. **Smoke vs manual partition:** every `dx run` invocation in this plan sits under a "Manual smoke (human only, not for agent execution)" sub-heading. Every `cargo` invocation sits under a "Run tests" sub-heading. No `dx run` carries the stale `--no-default-features --features gui-dioxus` flags (egui crate was removed; the flags fail). Tasks 3, 5, 15 (the three with manual smoke steps) all comply.

5. **Phantom-undo guard parity:** Task 14 (F11) and Task 17 (F10 backport) both contain the `if dragged == live { body.set(next); return; }` guard with the same wording, inserted in the corresponding `up` arm of `dispatch_bridge_event` immediately after `handle_pointer_up` and before the `match result` block.

6. **Test harness DRY:** F10 (`response_curve/tests.rs`) and F11 (`deadzone/tests.rs`) both call the shared `mount_stage_body_test` helper from `crate::frame::mapping_editor::test_helpers`. No body's tests inline the `RawHandles` / `MetaSnapshot` / `ConfigSnapshot` / `LiveSnapshot` construction.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-05-02-f11-deadzone-editor.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
