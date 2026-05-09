mod logic;

use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::view_state::{PanelSlot, ViewState};

use logic::{Tool, tool_active};

/// Decide the next `PanelSlot` when a tools-cluster button is clicked.
/// `current` is the current slot; `target` is the slot the button represents;
/// `target_active` is whether the button is currently lit. Active button
/// closes the slot; inactive button opens the target.
/// `current` is currently unused but kept in the signature for future variants
/// that may need it.
pub(crate) fn next_slot(current: PanelSlot, target: PanelSlot, target_active: bool) -> PanelSlot {
    let _ = current;
    if target_active {
        PanelSlot::None
    } else {
        target
    }
}

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
    let settings_active = tool_active(s, v, Tool::Settings);

    rsx! {
        nav { class: "if-tools-cluster", "aria-label": "Side panels",
            ToolButton {
                label: "Devices",
                active: devices_active,
                disabled: !p,
                disabled_reason: "Load a profile to inspect connected devices.",
                onclick: move |_| {
                    // via_calibration is sticky-while-Devices-open per spec;
                    // closing the panel leaves it as-is for the next open.
                    let next = next_slot(panel(), PanelSlot::Devices, devices_active);
                    panel.set(next);
                    if !devices_active {
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
                    let next = next_slot(panel(), PanelSlot::Profiles, profiles_active);
                    panel.set(next);
                    if !profiles_active {
                        via.set(false);
                    }
                },
            }
            ToolButton {
                label: "Settings",
                active: settings_active,
                disabled: false,
                disabled_reason: "",
                onclick: move |_| {
                    let next = next_slot(panel(), PanelSlot::Settings, settings_active);
                    panel.set(next);
                    if !settings_active {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_slot_active_button_closes() {
        assert_eq!(
            next_slot(PanelSlot::Settings, PanelSlot::Settings, true),
            PanelSlot::None
        );
    }

    #[test]
    fn next_slot_inactive_button_opens_target() {
        assert_eq!(
            next_slot(PanelSlot::None, PanelSlot::Settings, false),
            PanelSlot::Settings
        );
    }

    #[test]
    fn next_slot_replaces_other_panel() {
        assert_eq!(
            next_slot(PanelSlot::Devices, PanelSlot::Settings, false),
            PanelSlot::Settings
        );
        assert_eq!(
            next_slot(PanelSlot::Profiles, PanelSlot::Settings, false),
            PanelSlot::Settings
        );
    }
}
