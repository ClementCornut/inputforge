//! Engine-confirmed ownership and capture readiness, independent of cached values.
use super::MappingIssue;
use crate::output::traits::ControllerCapabilities;
use crate::types::{DeviceId, VirtualDeviceConfig};
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent backend, readiness, output ownership and thread availability facts"
)]
pub struct SessionState {
    pub keyboard_supported: bool,
    pub mouse_supported: bool,
    pub controller_capabilities: ControllerCapabilities,
    pub bindings: Vec<crate::profile::controllers::DeviceBinding>,
    pub monitored: Vec<DeviceId>,
    pub mapping_issues: Vec<MappingIssue>,
    pub ready: bool,
    pub generation: u64,
    pub captured: Vec<DeviceId>,
    pub output_active: bool,
    /// Layout currently owned by the output adapter, retained while stopped.
    pub output_layout: Vec<VirtualDeviceConfig>,
    pub notice: Option<String>,
    pub error: Option<String>,
    pub offline: bool,
}

impl SessionState {
    /// Whether retained controllers differ from the desired profile layout.
    #[must_use]
    pub fn requires_output_reconfiguration(&self, desired: &[VirtualDeviceConfig]) -> bool {
        self.output_active
            && (self.output_layout.len() != desired.len()
                || desired.iter().any(|next| {
                    !self.output_layout.iter().any(|current| {
                        if self.controller_capabilities.configurable {
                            current == next
                        } else {
                            current.supports(next)
                        }
                    })
                }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::VJoyAxis;

    fn layout(axes: Vec<VJoyAxis>, buttons: u8, hats: u8) -> VirtualDeviceConfig {
        VirtualDeviceConfig {
            device_id: 1,
            axes,
            button_count: buttons,
            hat_count: hats,
        }
    }

    #[test]
    fn retained_layout_comparison_uses_fixed_containment_and_configurable_exactness() {
        let native = layout(vec![VJoyAxis::Y, VJoyAxis::X, VJoyAxis::Z], 8, 2);
        let desired = layout(vec![VJoyAxis::X, VJoyAxis::Y], 4, 1);
        let mut state = SessionState {
            output_active: true,
            output_layout: vec![native],
            ..SessionState::default()
        };

        assert!(!state.requires_output_reconfiguration(std::slice::from_ref(&desired)));
        assert!(state.requires_output_reconfiguration(&[layout(vec![VJoyAxis::Rx], 4, 1)]));

        state.controller_capabilities.configurable = true;
        state.output_layout = vec![desired.clone()];
        assert!(state.requires_output_reconfiguration(&[layout(
            vec![VJoyAxis::Y, VJoyAxis::X],
            4,
            1,
        )]));
        assert!(!state.requires_output_reconfiguration(&[desired]));
    }
}
