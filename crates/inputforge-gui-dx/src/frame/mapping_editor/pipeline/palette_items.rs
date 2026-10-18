//! Palette categories and the physical-key prompt.
use crate::mapping_edits::StageKind;
use dioxus::prelude::*;
pub(super) struct PaletteItem {
    pub(super) label: &'static str,
    pub(super) kind: StageKind,
}

pub(super) const PROCESSING_ITEMS: &[PaletteItem] = &[
    PaletteItem {
        label: "Invert",
        kind: StageKind::Invert,
    },
    PaletteItem {
        label: "Deadzone",
        kind: StageKind::Deadzone,
    },
    PaletteItem {
        label: "Response curve",
        kind: StageKind::ResponseCurve,
    },
];

pub(super) const OUTPUT_ITEMS: &[PaletteItem] = &[
    PaletteItem {
        label: "Map to mouse",
        kind: StageKind::Mouse,
    },
    PaletteItem {
        label: "Merge axis",
        kind: StageKind::MergeAxis,
    },
];

pub(super) const CONTROL_ITEMS: &[PaletteItem] = &[
    PaletteItem {
        label: "Conditional",
        kind: StageKind::Conditional,
    },
    PaletteItem {
        label: "Change mode",
        kind: StageKind::ChangeMode,
    },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub(super) fn KeyboardCapturePrompt(onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        crate::components::KeyCaptureButton {
            value: "Press the physical key or shortcut to map. Esc cancels.",
            aria_label: "Press the physical key or shortcut to map. Escape cancels.",
            listening: true, autofocus: true, onclick,
        }
    }
}
