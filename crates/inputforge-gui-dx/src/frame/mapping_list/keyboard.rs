//! Pure keyboard-routing logic for the F8 rail.
//!
//! `handle_key` takes the current state (visible filtered rows,
//! current selection, capture-armed, filter-focused, query-empty) and
//! returns an `Intent` that the `mod.rs` orchestrator translates into
//! signal writes. Splitting the routing decision out lets us unit-test
//! the boundary cases without a Dioxus runtime.

use inputforge_core::types::InputAddress;

/// Keys F8 cares about. Dioxus 0.7's `Key` enum carries platform-specific
/// variants; we narrow to the F8 vocabulary here so the unit tests can
/// drive `handle_key` with stable inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Key {
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    /// Cmd-F (macOS) or Ctrl-F (Windows/Linux). Caller normalizes.
    FilterShortcut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct State<'a> {
    pub visible_rows: &'a [&'a (String, InputAddress)],
    pub selected: Option<(&'a str, &'a InputAddress)>,
    pub capture_armed: bool,
    pub filter_focused: bool,
    pub filter_query_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Intent {
    /// Move selection to this row.
    Select((String, InputAddress)),
    /// Focus `[data-editor-focus]` (F9 owns the attached element).
    FocusEditor,
    /// Focus the filter input.
    FocusFilter,
    /// Clear filter query and unfocus.
    ClearFilter,
    /// Do nothing (key not handled in this context).
    NoOp,
}

#[allow(
    clippy::match_same_arms,
    reason = "Each key arm is a distinct routing rule; merging Escape+Enter NoOp arms would hide which key the rail rejects in which context."
)]
pub(crate) fn handle_key(key: Key, state: State<'_>) -> Intent {
    // Capture-armed always shadows Up/Down (and Enter/Esc which the
    // primitive owns). Filter-focused Esc with non-empty query clears.
    if state.capture_armed && matches!(key, Key::ArrowUp | Key::ArrowDown) {
        return Intent::NoOp;
    }
    match key {
        Key::FilterShortcut => Intent::FocusFilter,
        Key::Escape if state.filter_focused && !state.filter_query_empty => Intent::ClearFilter,
        Key::Escape => Intent::NoOp,
        Key::Enter if state.selected.is_some() => Intent::FocusEditor,
        Key::Enter => Intent::NoOp,
        Key::ArrowDown | Key::ArrowUp => {
            if state.visible_rows.is_empty() {
                return Intent::NoOp;
            }
            let len = state.visible_rows.len();
            let next_idx = match (key, state.selected) {
                (Key::ArrowDown, None) => 0,
                (Key::ArrowUp, None) => len - 1,
                (Key::ArrowDown, Some((sel_mode, sel_input))) => {
                    let cur = state
                        .visible_rows
                        .iter()
                        .position(|(m, i)| m == sel_mode && i == sel_input)
                        .unwrap_or(0);
                    (cur + 1) % len
                }
                (Key::ArrowUp, Some((sel_mode, sel_input))) => {
                    let cur = state
                        .visible_rows
                        .iter()
                        .position(|(m, i)| m == sel_mode && i == sel_input)
                        .unwrap_or(0);
                    (cur + len - 1) % len
                }
                _ => unreachable!(),
            };
            let (m, i) = state.visible_rows[next_idx].clone();
            Intent::Select((m, i))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{DeviceId, InputId};

    fn addr(id: u8) -> InputAddress {
        InputAddress {
            device: DeviceId("dev".to_owned()),
            input: InputId::Button { index: id },
        }
    }

    #[test]
    fn down_selects_first_when_nothing_selected() {
        let rows = [
            ("Default".to_owned(), addr(0)),
            ("Default".to_owned(), addr(1)),
        ];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        match handle_key(Key::ArrowDown, state) {
            Intent::Select((m, i)) => {
                assert_eq!(m, "Default");
                assert_eq!(i, addr(0));
            }
            other => panic!("expected Select(first), got {other:?}"),
        }
    }

    #[test]
    fn up_selects_last_when_nothing_selected() {
        let rows = [
            ("Default".to_owned(), addr(0)),
            ("Default".to_owned(), addr(1)),
        ];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        match handle_key(Key::ArrowUp, state) {
            Intent::Select((_, i)) => assert_eq!(i, addr(1)),
            other => panic!("expected Select(last), got {other:?}"),
        }
    }

    #[test]
    fn down_wraps_at_boundary() {
        let rows = [
            ("Default".to_owned(), addr(0)),
            ("Default".to_owned(), addr(1)),
        ];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let mode = "Default".to_owned();
        let last = addr(1);
        let state = State {
            visible_rows: &row_refs,
            selected: Some((&mode, &last)),
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        match handle_key(Key::ArrowDown, state) {
            Intent::Select((_, i)) => assert_eq!(i, addr(0)),
            other => panic!("expected wrap to first, got {other:?}"),
        }
    }

    #[test]
    fn capture_armed_disables_up_down() {
        let rows = [("Default".to_owned(), addr(0))];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: true,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::ArrowDown, state), Intent::NoOp);
        assert_eq!(handle_key(Key::ArrowUp, state), Intent::NoOp);
    }

    #[test]
    fn enter_with_selection_focuses_editor() {
        let rows = [("Default".to_owned(), addr(0))];
        let row_refs: Vec<&(String, InputAddress)> = rows.iter().collect();
        let mode = "Default".to_owned();
        let sel = addr(0);
        let state = State {
            visible_rows: &row_refs,
            selected: Some((&mode, &sel)),
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::Enter, state), Intent::FocusEditor);
    }

    #[test]
    fn enter_with_no_selection_is_noop() {
        let row_refs: Vec<&(String, InputAddress)> = Vec::new();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::Enter, state), Intent::NoOp);
    }

    #[test]
    fn cmd_f_focuses_filter() {
        let row_refs: Vec<&(String, InputAddress)> = Vec::new();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::FilterShortcut, state), Intent::FocusFilter);
    }

    #[test]
    fn esc_on_filter_with_query_clears() {
        let row_refs: Vec<&(String, InputAddress)> = Vec::new();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: true,
            filter_query_empty: false,
        };
        assert_eq!(handle_key(Key::Escape, state), Intent::ClearFilter);
    }

    #[test]
    fn esc_on_rail_with_empty_filter_is_noop() {
        let row_refs: Vec<&(String, InputAddress)> = Vec::new();
        let state = State {
            visible_rows: &row_refs,
            selected: None,
            capture_armed: false,
            filter_focused: false,
            filter_query_empty: true,
        };
        assert_eq!(handle_key(Key::Escape, state), Intent::NoOp);
    }
}
