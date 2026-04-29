use dioxus::prelude::*;

use crate::frame::view_state::{PanelSlot as PanelSlotEnum, ViewState};

const PANEL_SLOT_CSS: Asset = asset!("/assets/frame/panel_slot.css");

#[component]
pub(crate) fn PanelSlot() -> Element {
    let view = use_context::<ViewState>();
    let slot = use_memo(move || *view.panel_slot.read());

    rsx! {
        Stylesheet { href: PANEL_SLOT_CSS }
        match *slot.read() {
            PanelSlotEnum::None => rsx! {},
            PanelSlotEnum::Devices => rsx! {
                aside {
                    class: "if-panel-slot if-panel-slot--devices",
                    "aria-label": "Devices panel",
                    "Devices panel — F12 owns content"
                }
            },
            PanelSlotEnum::Profiles => rsx! {
                aside {
                    class: "if-panel-slot if-panel-slot--profiles",
                    "aria-label": "Profiles panel",
                    "Profiles panel — F13 owns content"
                }
            },
        }
    }
}
