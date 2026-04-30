use dioxus::prelude::*;

use crate::frame::view_state::{PanelSlot as PanelSlotEnum, ViewState};

const PANEL_SLOT_CSS: Asset = asset!("/assets/frame/panel_slot.css");

#[component]
pub(crate) fn PanelSlot() -> Element {
    let view = use_context::<ViewState>();
    let slot = use_memo(move || *view.panel_slot.read());
    let via_calib = use_memo(move || *view.via_calibration.read());

    let s = *slot.read();
    if matches!(s, PanelSlotEnum::None) {
        return rsx! { Stylesheet { href: PANEL_SLOT_CSS } };
    }

    // Single stable <aside> across Devices ⇄ Profiles ⇄ Calibration.
    // Hoisting the element outside the match keeps Dioxus's diff at the
    // text-node level on tool swap, so the entrance keyframe only fires
    // on the genuine None → Some open — not on every Some → Some swap
    // (which previously read as a close-and-reopen animation). F12/F13
    // will rewrite this file end-to-end; the placeholder just gives the
    // chrome a faithful "which tool is active" readout in the meantime.
    let calib = *via_calib.read();
    let (caption, title, body, aria) = match s {
        PanelSlotEnum::Devices if calib => (
            "Panel · F12",
            "Calibration",
            "F12 owns content (calibration)",
            "Calibration panel",
        ),
        PanelSlotEnum::Devices => (
            "Panel · F12",
            "Devices",
            "F12 owns content",
            "Devices panel",
        ),
        PanelSlotEnum::Profiles => (
            "Panel · F13",
            "Profiles",
            "F13 owns content",
            "Profiles panel",
        ),
        PanelSlotEnum::None => unreachable!("None branch returned above"),
    };

    rsx! {
        Stylesheet { href: PANEL_SLOT_CSS }
        aside {
            class: "if-panel-slot",
            "aria-label": "{aria}",
            header { class: "if-panel-slot__header",
                div { class: "if-panel-slot__caption", "{caption}" }
                h2 { class: "if-panel-slot__title", "{title}" }
            }
            div { class: "if-panel-slot__body", "{body}" }
        }
    }
}
