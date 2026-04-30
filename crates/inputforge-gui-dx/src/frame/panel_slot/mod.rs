use dioxus::prelude::*;

use crate::frame::view_state::{PanelSlot as PanelSlotEnum, ViewState};

const PANEL_SLOT_CSS: Asset = asset!("/assets/frame/panel_slot.css");

#[component]
pub(crate) fn PanelSlot() -> Element {
    let view = use_context::<ViewState>();
    let slot = use_memo(move || *view.panel_slot.read());
    let via_calib = use_memo(move || *view.via_calibration.read());

    rsx! {
        Stylesheet { href: PANEL_SLOT_CSS }
        match *slot.read() {
            PanelSlotEnum::None => rsx! {},
            PanelSlotEnum::Devices => {
                // F7 placeholder swaps title to reflect the active tool
                // (Devices vs Calibration drill). F12 will replace this
                // file end-to-end when it lands; the placeholder just
                // stops lying about which mode is active.
                let calib = *via_calib.read();
                let title = if calib { "Calibration" } else { "Devices" };
                let body = if calib {
                    "F12 owns content (calibration)"
                } else {
                    "F12 owns content"
                };
                rsx! {
                    aside {
                        class: "if-panel-slot if-panel-slot--devices",
                        "aria-label": "{title} panel",
                        header { class: "if-panel-slot__header",
                            div { class: "if-panel-slot__caption", "Panel · F12" }
                            h2 { class: "if-panel-slot__title", "{title}" }
                        }
                        div { class: "if-panel-slot__body", "{body}" }
                    }
                }
            },
            PanelSlotEnum::Profiles => rsx! {
                aside {
                    class: "if-panel-slot if-panel-slot--profiles",
                    "aria-label": "Profiles panel",
                    header { class: "if-panel-slot__header",
                        div { class: "if-panel-slot__caption", "Panel · F13" }
                        h2 { class: "if-panel-slot__title", "Profiles" }
                    }
                    div { class: "if-panel-slot__body", "F13 owns content" }
                }
            },
        }
    }
}
