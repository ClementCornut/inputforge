//! Positional auto-mapping logic. Pure functions only.
//!
//! Convention (locked in design Q4):
//! - Source axis index `i` maps to the `i`-th vJoy axis in
//!   `VirtualDeviceConfig.axes` order. The order is the canonical
//!   `VJoyAxis` enum order: X, Y, Z, Rx, Ry, Rz, Slider0, Slider1,
//!   subject to which slots vJoy actually exposes.
//! - Source button `i` (0-indexed) maps to vJoy button `i + 1`
//!   (1-indexed at the SDK layer). 0-vs-1 convention is intentional.
//! - Source hat `i` (0-indexed) maps to vJoy hat `i + 1`.
//! - Overflow (source has more inputs of a kind than the target):
//!   the row's auto-target is `None`.

use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis, VirtualDeviceConfig};

/// Return the auto-suggested target for source axis `i` against
/// `target`. `None` when `i >= target.axes.len()`.
pub(super) fn auto_axis_target(target: &VirtualDeviceConfig, i: usize) -> Option<OutputAddress> {
    let axis: VJoyAxis = *target.axes.get(i)?;
    Some(OutputAddress {
        device: target.device_id,
        output: OutputId::Axis { id: axis },
    })
}

/// Return the auto-suggested target for source button `i` against
/// `target`. `None` when `i >= target.button_count`.
pub(super) fn auto_button_target(target: &VirtualDeviceConfig, i: usize) -> Option<OutputAddress> {
    if i >= usize::from(target.button_count) {
        return None;
    }
    let id = u8::try_from(i + 1).ok()?;
    Some(OutputAddress {
        device: target.device_id,
        output: OutputId::Button { id },
    })
}

/// Return the auto-suggested target for source hat `i` against
/// `target`. `None` when `i >= target.hat_count`.
pub(super) fn auto_hat_target(target: &VirtualDeviceConfig, i: usize) -> Option<OutputAddress> {
    if i >= usize::from(target.hat_count) {
        return None;
    }
    let id = u8::try_from(i + 1).ok()?;
    Some(OutputAddress {
        device: target.device_id,
        output: OutputId::Hat { id },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> VirtualDeviceConfig {
        VirtualDeviceConfig {
            device_id: 1,
            axes: vec![
                VJoyAxis::X,
                VJoyAxis::Y,
                VJoyAxis::Z,
                VJoyAxis::Rx,
                VJoyAxis::Ry,
                VJoyAxis::Rz,
                VJoyAxis::Slider0,
                VJoyAxis::Slider1,
            ],
            button_count: 32,
            hat_count: 1,
        }
    }

    #[test]
    fn axis_index_zero_maps_to_x() {
        let t = auto_axis_target(&target(), 0).expect("axis target");
        assert!(matches!(t.output, OutputId::Axis { id: VJoyAxis::X }));
    }

    #[test]
    fn axis_index_seven_maps_to_slider1() {
        let t = auto_axis_target(&target(), 7).expect("axis target");
        assert!(matches!(
            t.output,
            OutputId::Axis {
                id: VJoyAxis::Slider1
            }
        ));
    }

    #[test]
    fn axis_overflow_returns_none() {
        let mut tgt = target();
        tgt.axes = vec![VJoyAxis::X];
        assert!(auto_axis_target(&tgt, 1).is_none());
    }

    #[test]
    fn button_zero_maps_to_button_one() {
        let t = auto_button_target(&target(), 0).expect("button target");
        assert!(matches!(t.output, OutputId::Button { id: 1 }));
    }

    #[test]
    fn button_overflow_returns_none() {
        let mut tgt = target();
        tgt.button_count = 4;
        assert!(auto_button_target(&tgt, 4).is_none());
    }

    #[test]
    fn hat_zero_maps_to_hat_one() {
        let t = auto_hat_target(&target(), 0).expect("hat target");
        assert!(matches!(t.output, OutputId::Hat { id: 1 }));
    }

    #[test]
    fn hat_overflow_returns_none() {
        let mut tgt = target();
        tgt.hat_count = 0;
        assert!(auto_hat_target(&tgt, 0).is_none());
    }
}
