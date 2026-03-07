// Rust guideline compliant 2026-03-07

//! Shared device-related utilities used across multiple panels.
//!
//! Contains axis label helpers used by the input viewer window,
//! left panel, and mapping editor.

use std::borrow::Cow;

/// HID standard axis names for indices 0–7.
///
/// Maps to the standard HID usage page ordering: X, Y, Z, then rotational
/// axes, then slider and dial.  Uses abbreviated forms to fit the 40 px
/// label area.
pub(crate) const HID_AXIS_LABELS: [&str; 8] =
    ["X", "Y", "Z", "Rot X", "Rot Y", "Rot Z", "Sldr", "Dial"];

/// Return a human-readable label for the given 0-based axis index.
///
/// Indices 0–7 use HID standard names; higher indices fall back to
/// `Ax {index}`.
pub(crate) fn axis_label(index: usize) -> Cow<'static, str> {
    if index < HID_AXIS_LABELS.len() {
        Cow::Borrowed(HID_AXIS_LABELS[index])
    } else {
        Cow::Owned(format!("Ax {index}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_label_hid_names() {
        assert_eq!(axis_label(0), "X");
        assert_eq!(axis_label(2), "Z");
        assert_eq!(axis_label(3), "Rot X");
        assert_eq!(axis_label(6), "Sldr");
        assert_eq!(axis_label(7), "Dial");
    }

    #[test]
    fn axis_label_beyond_table_falls_back() {
        assert_eq!(axis_label(8), "Ax 8");
        assert_eq!(axis_label(99), "Ax 99");
    }
}
