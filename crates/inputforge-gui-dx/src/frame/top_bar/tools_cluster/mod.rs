mod logic;

use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::view_state::{PanelSlot, ViewState};

use logic::{Tool, tool_active};

#[component]
pub(crate) fn ToolsCluster() -> Element {
    tracing::trace!(target: "frame::render", region = "tools_cluster");
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
    let profiles_active = tool_active(s, v, Tool::Profiles);

    rsx! {
        nav { class: "if-tools-cluster", "aria-label": "Side panels",
            ToolButton {
                label: "Devices",
                active: devices_active,
                disabled: !p,
                disabled_reason: "Load a profile to inspect connected devices.",
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
                label: "Profiles",
                active: profiles_active,
                disabled: false,
                // Profiles is never disabled, the panel itself is the
                // discovery surface, so it must remain reachable.
                disabled_reason: "",
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
    // Surfaced as `title` (sighted hover) and read by AT as the button's
    // accessible description. Empty string ⇒ no description rendered.
    // Only meaningful while `disabled` is true; resting buttons have no
    // precondition to explain.
    disabled_reason: String,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let show_reason = disabled && !disabled_reason.is_empty();
    rsx! {
        button {
            r#type: "button",
            class: "if-tools-cluster__button",
            disabled,
            "aria-pressed": "{active}",
            title: if show_reason { disabled_reason.clone() } else { String::new() },
            onclick,
            "{label}"
        }
    }
}
