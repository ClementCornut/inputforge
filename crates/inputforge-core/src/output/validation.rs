//! Controller capability validation shared with application workflows.
use super::traits::ControllerCapabilities;
use crate::{error::Result, profile::controllers::invalid, types::VirtualDeviceConfig};

/// Validate requested layouts against the output adapter's capability limits.
///
/// # Errors
/// Rejects button and hat counts outside backend capabilities.
/// Structural validation remains in `ControllerConfig::validate`.
pub fn validate_layout(
    configs: &[VirtualDeviceConfig],
    capabilities: ControllerCapabilities,
) -> Result<()> {
    for slot in configs {
        if slot.button_count < capabilities.min_buttons
            || slot.button_count > capabilities.max_buttons
            || slot.hat_count > capabilities.max_hats
        {
            return Err(invalid("Virtual controller exceeds backend capabilities"));
        }
    }
    Ok(())
}
