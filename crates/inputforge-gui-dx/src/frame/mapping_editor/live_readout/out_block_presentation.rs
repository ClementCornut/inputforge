//! Readout display labels and presentation helpers.
use crate::icons::Icon as IconKind;
use inputforge_core::action::OutputBehavior;
pub(super) fn row_wrap_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-row-wrap if-editor__readout-row-wrap--frozen"
    } else {
        "if-editor__readout-row-wrap"
    }
}

pub(super) fn chevron_icon(expanded: bool) -> IconKind {
    if expanded {
        IconKind::ChevronDown
    } else {
        IconKind::ChevronRight
    }
}

pub(super) fn chevron_label(expanded: bool) -> &'static str {
    if expanded {
        "Collapse output merge preview"
    } else {
        "Expand output merge preview"
    }
}

pub(super) fn keyboard_row_class(frozen: bool) -> &'static str {
    if frozen {
        "if-editor__readout-row if-editor__readout-row--kb if-editor__readout-row--frozen"
    } else {
        "if-editor__readout-row if-editor__readout-row--kb"
    }
}

pub(super) fn format_behavior(behavior: OutputBehavior) -> &'static str {
    match behavior {
        OutputBehavior::Hold => "Hold",
        OutputBehavior::Pulse => "Pulse",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::mapping_editor::live_readout::value_helpers::hat_glyph_for;
    use crate::readouts::ExpandState;
    use inputforge_core::types::HatDirection;

    #[test]
    fn hat_glyph_for_maps_all_directions() {
        assert_eq!(hat_glyph_for(HatDirection::N), '\u{2191}');
        assert_eq!(hat_glyph_for(HatDirection::NE), '\u{2197}');
        assert_eq!(hat_glyph_for(HatDirection::E), '\u{2192}');
        assert_eq!(hat_glyph_for(HatDirection::SE), '\u{2198}');
        assert_eq!(hat_glyph_for(HatDirection::S), '\u{2193}');
        assert_eq!(hat_glyph_for(HatDirection::SW), '\u{2199}');
        assert_eq!(hat_glyph_for(HatDirection::W), '\u{2190}');
        assert_eq!(hat_glyph_for(HatDirection::NW), '\u{2196}');
        assert_eq!(hat_glyph_for(HatDirection::Center), '\u{00b7}');
    }

    #[test]
    fn keyboard_row_classes_include_kind_and_frozen_state() {
        assert_eq!(
            keyboard_row_class(false),
            "if-editor__readout-row if-editor__readout-row--kb"
        );
        assert_eq!(
            keyboard_row_class(true),
            "if-editor__readout-row if-editor__readout-row--kb if-editor__readout-row--frozen"
        );
    }

    #[test]
    fn expand_state_applies_per_output_state() {
        let state = ExpandState {
            per_output: vec![false, true],
            ..ExpandState::default()
        };

        assert!(!state.is_expanded(0));
        assert!(state.is_expanded(1));
        assert!(!state.is_expanded(2));
    }

    #[test]
    fn row_wrap_class_and_chevron_state_track_state() {
        assert_eq!(row_wrap_class(false), "if-editor__readout-row-wrap");
        assert_eq!(
            row_wrap_class(true),
            "if-editor__readout-row-wrap if-editor__readout-row-wrap--frozen"
        );
        assert_eq!(chevron_icon(false), IconKind::ChevronRight);
        assert_eq!(chevron_icon(true), IconKind::ChevronDown);
        assert_eq!(chevron_label(false), "Expand output merge preview");
        assert_eq!(chevron_label(true), "Collapse output merge preview");
    }
}
