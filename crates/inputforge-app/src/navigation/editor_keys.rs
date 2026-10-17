//! Scoped editor key interpretation.
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
