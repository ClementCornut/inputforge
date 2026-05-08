//! Pure logic for the tools-cluster active-state derivation.

use crate::frame::view_state::PanelSlot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Devices,
    // Button hidden until calibration UI is implemented; matcher arms and
    // tests below still exercise this variant so the state machine stays
    // intact for restoration.
    #[allow(
        dead_code,
        reason = "Calibration variant retained for F13 calibration UI (see docs/superpowers/specs/2026-04-24-egui-to-dioxus-rewrite-design.md section F12 calibration drill-in and 2026-04-29-f7-frame-design.md section 49 calibration sugar routing); matcher arms + tests keep the state machine intact for restoration"
    )]
    Calibration,
    Profiles,
}

/// Whether a given tool button should render as active.
///
/// Exclusive, at most one of (Devices, Calibration, Profiles) may be active
/// at any time. Calibration is "Devices opened with the calibration drill".
pub(crate) fn tool_active(slot: PanelSlot, via_calibration: bool, tool: Tool) -> bool {
    matches!(
        (slot, via_calibration, tool),
        (PanelSlot::Devices, false, Tool::Devices)
            | (PanelSlot::Devices, true, Tool::Calibration)
            | (PanelSlot::Profiles, _, Tool::Profiles)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_panel_with_via_calibration_false_lights_devices() {
        assert!(tool_active(PanelSlot::Devices, false, Tool::Devices));
        assert!(!tool_active(PanelSlot::Devices, false, Tool::Calibration));
        assert!(!tool_active(PanelSlot::Devices, false, Tool::Profiles));
    }

    #[test]
    fn devices_panel_with_via_calibration_true_lights_calibration() {
        assert!(!tool_active(PanelSlot::Devices, true, Tool::Devices));
        assert!(tool_active(PanelSlot::Devices, true, Tool::Calibration));
    }

    #[test]
    fn profiles_panel_lights_profiles_regardless_of_via_calibration() {
        assert!(tool_active(PanelSlot::Profiles, false, Tool::Profiles));
        assert!(tool_active(PanelSlot::Profiles, true, Tool::Profiles));
        assert!(!tool_active(PanelSlot::Profiles, true, Tool::Calibration));
    }

    #[test]
    fn no_panel_lights_nothing() {
        assert!(!tool_active(PanelSlot::None, false, Tool::Devices));
        assert!(!tool_active(PanelSlot::None, false, Tool::Calibration));
        assert!(!tool_active(PanelSlot::None, false, Tool::Profiles));
    }
}
