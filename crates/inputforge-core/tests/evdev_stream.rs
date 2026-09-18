#![cfg(all(target_os = "linux", feature = "evdev-input"))]

use inputforge_core::device::evdev::{AxisInfo, NativeState};
use inputforge_core::types::HatDirection;

#[test]
fn native_hats_preserve_absent_components_and_never_reuse_invalid_direction() {
    let mut state = NativeState::default();
    state.axes.insert(0x10, -1);
    let hat = state.hat(0);
    assert_eq!(hat.x, Some(-1));
    assert_eq!(hat.y, None);
    assert_eq!(hat.direction, None);
    for (x, y, direction) in [
        (0, 0, HatDirection::Center),
        (-1, -1, HatDirection::NW),
        (0, -1, HatDirection::N),
        (1, -1, HatDirection::NE),
        (1, 0, HatDirection::E),
        (1, 1, HatDirection::SE),
        (0, 1, HatDirection::S),
        (-1, 1, HatDirection::SW),
        (-1, 0, HatDirection::W),
    ] {
        state.axes.insert(0x10, x);
        state.axes.insert(0x11, y);
        assert_eq!(state.hat(0).direction, Some(direction));
    }
    state.axes.insert(0x11, 2);
    assert_eq!(state.hat(0).direction, None);
    state.axes.insert(0x18, 1);
    assert_eq!(state.hat(4).x, None);
}

#[test]
fn normalization_preserves_out_of_range_values_and_handles_extreme_ranges() {
    let mut axis = AxisInfo {
        code: 0x28,
        minimum: 0,
        maximum: 60000,
        fuzz: 1,
        flat: 100,
        resolution: 3,
    };
    for (raw, expected) in [(0, -1.0), (30000, 0.0), (60000, 1.0), (90000, 2.0)] {
        assert_eq!(axis.normalize(raw), Some(expected));
    }
    axis.minimum = i32::MIN;
    axis.maximum = i32::MAX;
    assert_eq!(axis.normalize(i32::MIN), Some(-1.0));
    assert_eq!(axis.normalize(i32::MAX), Some(1.0));
    axis.minimum = axis.maximum;
    assert_eq!(axis.normalize(0), None);
    axis.maximum = -1;
    assert_eq!(axis.normalize(0), None);
}
