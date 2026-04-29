mod logic;

use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::view_state::{PanelSlot, ViewState};

use logic::{Tool, tool_active};

#[component]
pub(crate) fn ToolsCluster() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    let slot = use_memo(move || *view.panel_slot.read());
    let via_calib = use_memo(move || *view.via_calibration.read());
    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());

    let s = *slot.read();
    let v = *via_calib.read();
    let p = *has_profile.read();

    let mut panel = view.panel_slot;
    let mut via = view.via_calibration;

    // Capture per-button activeness so each click handler can decide
    // whether to toggle off (set panel_slot=None) or switch to the
    // target. Computed at render time from the same `tool_active`
    // matcher the buttons display, so visual + behavioral state stay
    // synchronized.
    let devices_active = tool_active(s, v, Tool::Devices);
    let calibration_active = tool_active(s, v, Tool::Calibration);
    let profiles_active = tool_active(s, v, Tool::Profiles);

    rsx! {
        nav { class: "if-tools-cluster", "aria-label": "Side panels",
            ToolButton {
                label: "Devices",
                active: devices_active,
                disabled: !p,
                onclick: move |_| {
                    if devices_active {
                        // Toggle off: close the panel. via_calibration
                        // is sticky-while-Devices-open per spec, so
                        // it stays as-is for the next time Devices opens.
                        panel.set(PanelSlot::None);
                    } else {
                        panel.set(PanelSlot::Devices);
                        via.set(false);
                    }
                },
            }
            ToolButton {
                label: "Calibration",
                active: calibration_active,
                disabled: !p,
                onclick: move |_| {
                    if calibration_active {
                        // Toggle off. Leave via_calibration true so the
                        // next Devices-open returns to Calibration view.
                        panel.set(PanelSlot::None);
                    } else {
                        panel.set(PanelSlot::Devices);
                        via.set(true);
                    }
                },
            }
            ToolButton {
                label: "Profiles",
                active: profiles_active,
                disabled: false,
                onclick: move |_| {
                    if profiles_active {
                        panel.set(PanelSlot::None);
                    } else {
                        panel.set(PanelSlot::Profiles);
                        via.set(false);
                    }
                },
            }
        }
    }
}

#[component]
fn ToolButton(
    label: String,
    active: bool,
    disabled: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "if-tools-cluster__button",
            disabled,
            "aria-pressed": "{active}",
            onclick,
            "{label}"
        }
    }
}
