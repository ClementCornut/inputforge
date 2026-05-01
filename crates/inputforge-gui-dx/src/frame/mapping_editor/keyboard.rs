// Rust guideline compliant 2026-05-01

//! Editor-scoped keyboard handler for the F9 mapping editor.
//!
//! Architecture mirrors F8's `frame/mapping_list/keyboard.rs` pattern: a pure
//! `decide` dispatcher that is unit-tested independently of any Dioxus runtime,
//! plus a `use_kb_listener` hook that registers a JS keydown listener via
//! `document::eval` and routes events through `decide`.
//!
//! Keyboard surface covered (per spec line 393 + AC #21):
//! - `Ctrl+Z`: editor undo, falls through to native browser undo when focus is
//!   inside an `<input>` / `<textarea>` / `contenteditable`.
//! - `Ctrl+Shift+Z`: editor redo (always; browsers do not bind this in inputs).
//! - `Ctrl+Y`: editor redo (Windows convention).
//! - `Alt+Up` / `Alt+Down`: reorder the focused stage within its parent pipeline.
//! - `Shift+F10`: open the right-click stage menu at the focused stage's rect.

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;
use crate::frame::mapping_editor::EditorState;
use crate::frame::view_state::ViewState;

// ---------------------------------------------------------------------------
// Pure types
// ---------------------------------------------------------------------------

/// The action the keyboard handler should take.
///
/// Returned by [`decide`] which is pure and unit-testable without a runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KbIntent {
    /// Pop from the undo stack and dispatch `SetMapping`.
    Undo,
    /// Pop from the redo stack and dispatch `SetMapping`.
    Redo,
    /// Move the focused stage one slot earlier within its parent pipeline.
    StageMoveUp,
    /// Move the focused stage one slot later within its parent pipeline.
    StageMoveDown,
    /// Open the right-click menu at the focused stage's bounding rect.
    StageMenuOpen,
    /// Let the event reach native handlers unchanged.
    PassThrough,
}

/// Where focus lives relative to the editor boundary.
///
/// Classified by the JS focus probe in [`use_kb_listener`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusTarget {
    /// Focus is inside an `<input>`, `<textarea>`, or `[contenteditable]`.
    Input,
    /// Focus is on a `.if-stage` element carrying a `data-stage-id` attribute.
    Stage,
    /// Focus is anywhere else inside `.if-editor` (but not in an input or stage).
    Editor,
    /// Focus is outside `.if-editor` entirely.
    Outside,
}

// ---------------------------------------------------------------------------
// Pure dispatcher
// ---------------------------------------------------------------------------

/// Classify a keydown event into a [`KbIntent`].
///
/// This function is pure: it reads no globals, touches no signals, and
/// allocates nothing. All 12 unit tests exercise it directly.
///
/// # Arguments
///
/// - `key`: the `event.key` string as reported by the browser.
/// - `ctrl`: whether `Ctrl` (or `Meta` on macOS) is held.
/// - `shift`: whether `Shift` is held.
/// - `alt`: whether `Alt` is held.
/// - `focus`: where focus currently resides, as classified by the JS probe.
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
        // Ctrl+Z: editor undo unless inside <input> (browser native undo wins).
        ("z" | "Z", true, false, false) => match focus {
            FocusTarget::Input => KbIntent::PassThrough,
            _ => KbIntent::Undo,
        },
        // Ctrl+Shift+Z: editor redo; browsers do not bind this inside inputs.
        // Ctrl+Y: editor redo (Windows convention; browsers do not bind it).
        ("z" | "Z", true, true, false) | ("y" | "Y", true, false, false) => KbIntent::Redo,
        // Alt+Up / Alt+Down: stage reorder. Per spec line 393 + AC #21.
        // Only fires when a stage element is focused (NOT inside an input).
        ("ArrowUp", false, false, true) if focus == FocusTarget::Stage => KbIntent::StageMoveUp,
        ("ArrowDown", false, false, true) if focus == FocusTarget::Stage => KbIntent::StageMoveDown,
        // Shift+F10: open the right-click menu at the focused stage's rect.
        // Per spec line 391.
        ("F10", false, true, false) if focus == FocusTarget::Stage => KbIntent::StageMenuOpen,
        // Alt+Left / Alt+Right: deferred per spec line 393.
        _ => KbIntent::PassThrough,
    }
}

// ---------------------------------------------------------------------------
// Listener hook
// ---------------------------------------------------------------------------

/// JS that registers a `keydown` listener on `window`, classifies focus, and
/// posts `[key, ctrl, shift, alt, target]` tuples back to Rust via the Dioxus
/// eval bridge.
///
/// The listener runs in capture phase (`true` as third arg to
/// `addEventListener`) so it sees events before focussed inputs process them.
/// Rust calls `decide` and dispatches the returned intent synchronously.
///
/// The JS focus probe runs at keydown time (inside the listener), so the
/// classification is always fresh even if focus changed since mount.
///
/// Shutdown: the loop sends `"__shutdown__"` to the eval handle and waits for
/// an `"__ack__"` echo before exiting, mirroring the F8 pattern.
const KB_LISTENER_JS: &str = "\
const h = (ev) => {
  const el = document.activeElement;
  let target = 'Outside';
  if (el && el.closest('.if-editor')) {
    if (el.matches('input, textarea, [contenteditable]')) {
      target = 'Input';
    } else if (el.closest('.if-stage[data-stage-id]')) {
      target = 'Stage';
    } else {
      target = 'Editor';
    }
  }
  const ctrl  = ev.ctrlKey  ? 1 : 0;
  const shift = ev.shiftKey ? 1 : 0;
  const alt   = ev.altKey   ? 1 : 0;
  dioxus.send([ev.key, ctrl, shift, alt, target]);
};
window.addEventListener('keydown', h, true);
(async () => {
  while (true) {
    const msg = await dioxus.recv();
    if (msg === '__shutdown__') {
      window.removeEventListener('keydown', h, true);
      dioxus.send('__ack__');
      return;
    }
  }
})();
";

/// Install an editor-scoped keydown listener that routes through [`decide`].
///
/// Mount exactly once from [`super::MappingEditor`]. The hook respects Dioxus
/// hook ordering rules: it allocates its guard signals unconditionally and only
/// spawns the async listener once (guarded by `mounted`).
///
/// # Dispatch behavior
///
/// | Intent | Action |
/// |---|---|
/// | `Undo` | Pops undo entry, dispatches `SetMapping` with `mapping_before`. |
/// | `Redo` | Pops redo entry, dispatches `SetMapping` with `mapping_before`. |
/// | `StageMoveUp` | Stub: logs to console (Task 41 smoke). |
/// | `StageMoveDown` | Stub: logs to console (Task 41 smoke). |
/// | `StageMenuOpen` | Stub: logs to console (Task 41 smoke). |
/// | `PassThrough` | No-op; native handlers proceed. |
#[expect(
    clippy::too_many_lines,
    reason = "The hook is a single logical unit: signal allocation, effect mount, \
              and the async listener loop. Splitting it across helpers would \
              break Dioxus hook ordering rules and scatter the signal lifetimes."
)]
pub(crate) fn use_kb_listener() {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let mut editor = use_context::<EditorState>();

    // Guard: install the JS listener only once per component mount.
    let kb_listener_mounted: Signal<bool> = use_signal(|| false);
    let kb_shutdown_signal: Signal<bool> = use_signal(|| false);

    use_effect(move || {
        let mut mounted = kb_listener_mounted;
        if *mounted.peek() {
            return;
        }
        mounted.set(true);
        let mut sd = kb_shutdown_signal;
        sd.set(false);

        // Clone all context handles the spawned future needs.
        let cmd_tx = ctx.commands.clone();
        let view_sel = view.selected_mapping;

        spawn(async move {
            let mut handle = document::eval(KB_LISTENER_JS);

            loop {
                if *kb_shutdown_signal.peek() {
                    let _ = handle.send("__shutdown__".to_owned());
                    let _ = handle.recv::<String>().await;
                    break;
                }

                // Each message is [key, ctrl, shift, alt, target].
                let Ok((key_str, ctrl_u8, shift_u8, alt_u8, target_str)) =
                    handle.recv::<(String, u8, u8, u8, String)>().await
                else {
                    // Eval bridge closed (e.g. component unmounted).
                    break;
                };

                let focus = match target_str.as_str() {
                    "Input" => FocusTarget::Input,
                    "Stage" => FocusTarget::Stage,
                    "Editor" => FocusTarget::Editor,
                    _ => FocusTarget::Outside,
                };

                let intent = decide(&key_str, ctrl_u8 != 0, shift_u8 != 0, alt_u8 != 0, focus);

                match intent {
                    KbIntent::PassThrough => {
                        // Let the event reach native handlers unchanged.
                    }

                    KbIntent::Undo => {
                        let Some(key) = view_sel.peek().clone() else {
                            continue;
                        };
                        let Some(entry) = editor.undo_log.write().undo(&key) else {
                            continue;
                        };
                        let m = &entry.mapping_before;
                        if cmd_tx
                            .send(EngineCommand::SetMapping {
                                input: m.input.clone(),
                                mode: m.mode.clone(),
                                name: m.name.clone(),
                                actions: m.actions.clone(),
                            })
                            .is_err()
                        {
                            tracing::warn!(
                                target: "f9::keyboard",
                                action = "undo_dropped_offline",
                                "undo dropped: engine channel disconnected"
                            );
                            // Known concern (documented in plan): we do NOT
                            // roll back the undo pop; the entry has already
                            // been pushed to the redo stack by UndoLog::undo.
                        }
                    }

                    KbIntent::Redo => {
                        let Some(key) = view_sel.peek().clone() else {
                            continue;
                        };
                        let Some(entry) = editor.undo_log.write().redo(&key) else {
                            continue;
                        };
                        let m = &entry.mapping_before;
                        if cmd_tx
                            .send(EngineCommand::SetMapping {
                                input: m.input.clone(),
                                mode: m.mode.clone(),
                                name: m.name.clone(),
                                actions: m.actions.clone(),
                            })
                            .is_err()
                        {
                            tracing::warn!(
                                target: "f9::keyboard",
                                action = "redo_dropped_offline",
                                "redo dropped: engine channel disconnected"
                            );
                        }
                    }

                    // Stubs: StageMoveUp/Down and StageMenuOpen dispatch is
                    // exercised in Task 41's manual smoke run. The JS-side
                    // data-stage-id read and structural mutation helpers
                    // (remove_at_path + insert_at_path) require a second
                    // async eval round-trip that ships in a follow-on task.
                    KbIntent::StageMoveUp => {
                        tracing::debug!(
                            target: "f9::keyboard",
                            action = "stage_move_up_stub",
                            "Alt+Up: stage move up (stub, Task 41)"
                        );
                    }
                    KbIntent::StageMoveDown => {
                        tracing::debug!(
                            target: "f9::keyboard",
                            action = "stage_move_down_stub",
                            "Alt+Down: stage move down (stub, Task 41)"
                        );
                    }
                    KbIntent::StageMenuOpen => {
                        // Read the stage-id from the focused element and
                        // populate EditorState.stage_menu. The bounding-rect
                        // fetch requires a second eval call; stub logs for now.
                        tracing::debug!(
                            target: "f9::keyboard",
                            action = "stage_menu_open_stub",
                            "Shift+F10: stage menu open (stub, Task 41)"
                        );
                        let _ = editor.stage_menu;
                    }
                }
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_z_inside_input_passes_through() {
        assert_eq!(
            decide("z", true, false, false, FocusTarget::Input),
            KbIntent::PassThrough
        );
    }

    #[test]
    fn ctrl_z_in_editor_drives_undo() {
        assert_eq!(
            decide("z", true, false, false, FocusTarget::Editor),
            KbIntent::Undo
        );
    }

    #[test]
    fn ctrl_z_on_focused_stage_drives_undo() {
        assert_eq!(
            decide("z", true, false, false, FocusTarget::Stage),
            KbIntent::Undo
        );
    }

    #[test]
    fn ctrl_shift_z_inside_input_drives_redo() {
        assert_eq!(
            decide("z", true, true, false, FocusTarget::Input),
            KbIntent::Redo
        );
    }

    #[test]
    fn ctrl_y_in_editor_drives_redo() {
        assert_eq!(
            decide("y", true, false, false, FocusTarget::Editor),
            KbIntent::Redo
        );
    }

    #[test]
    fn outside_editor_passes_through() {
        assert_eq!(
            decide("z", true, false, false, FocusTarget::Outside),
            KbIntent::PassThrough
        );
    }

    #[test]
    fn unrelated_key_passes_through() {
        assert_eq!(
            decide("a", false, false, false, FocusTarget::Editor),
            KbIntent::PassThrough
        );
    }

    #[test]
    fn alt_up_on_stage_moves_up() {
        assert_eq!(
            decide("ArrowUp", false, false, true, FocusTarget::Stage),
            KbIntent::StageMoveUp
        );
    }

    #[test]
    fn alt_down_on_stage_moves_down() {
        assert_eq!(
            decide("ArrowDown", false, false, true, FocusTarget::Stage),
            KbIntent::StageMoveDown
        );
    }

    #[test]
    fn alt_up_in_input_passes_through() {
        // Do not intercept Alt+Up inside text fields; native cursor movement wins.
        assert_eq!(
            decide("ArrowUp", false, false, true, FocusTarget::Input),
            KbIntent::PassThrough
        );
    }

    #[test]
    fn shift_f10_on_stage_opens_menu() {
        assert_eq!(
            decide("F10", false, true, false, FocusTarget::Stage),
            KbIntent::StageMenuOpen
        );
    }

    #[test]
    fn alt_left_right_deferred_pass_through() {
        // Per spec line 393, Alt+Left/Right is deferred to impeccable:harden.
        // The handler MUST pass these through unchanged.
        assert_eq!(
            decide("ArrowLeft", false, false, true, FocusTarget::Stage),
            KbIntent::PassThrough
        );
        assert_eq!(
            decide("ArrowRight", false, false, true, FocusTarget::Stage),
            KbIntent::PassThrough
        );
    }
}
