use std::{collections::BTreeMap, time::Instant};

use crate::types::{DeviceId, HatDirection};

/// A native control code, never a positional profile binding index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NativeControl {
    Key(u16),
    Abs(u16),
}

/// One ordered native state change within a complete event frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeChange {
    pub control: NativeControl,
    pub value: i32,
}

/// Current native values, keyed by supported evdev codes rather than enumeration order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeState {
    pub keys: BTreeMap<u16, bool>,
    pub axes: BTreeMap<u16, i32>,
}

/// Native hat components and a direction only when both components are valid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeHat {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub direction: Option<HatDirection>,
}

impl NativeState {
    /// Interpret native hat 0 through 3; absent or invalid components have no direction.
    #[must_use]
    pub fn hat(&self, index: u8) -> NativeHat {
        // Linux reserves ABS_HAT0X/Y through ABS_HAT3X/Y at 0x10..=0x17.
        let code = 0x10 + u16::from(index) * 2;
        let x = (index < 4).then(|| self.axes.get(&code).copied()).flatten();
        let y = (index < 4)
            .then(|| self.axes.get(&(code + 1)).copied())
            .flatten();
        let direction = match (x, y) {
            (Some(0), Some(0)) => Some(HatDirection::Center),
            (Some(0), Some(-1)) => Some(HatDirection::N),
            (Some(1), Some(-1)) => Some(HatDirection::NE),
            (Some(1), Some(0)) => Some(HatDirection::E),
            (Some(1), Some(1)) => Some(HatDirection::SE),
            (Some(0), Some(1)) => Some(HatDirection::S),
            (Some(-1), Some(1)) => Some(HatDirection::SW),
            (Some(-1), Some(0)) => Some(HatDirection::W),
            (Some(-1), Some(-1)) => Some(HatDirection::NW),
            _ => None,
        };
        NativeHat { x, y, direction }
    }
}

/// Why a complete sampled state replaces the consumer's prior state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotKind {
    Initial,
    Recovered,
}

/// Ordered stream delivery; snapshots and resets are not physical button edges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamUpdate {
    Frame {
        device: DeviceId,
        changes: Vec<NativeChange>,
        timestamp: Instant,
    },
    Snapshot {
        device: DeviceId,
        state: NativeState,
        kind: SnapshotKind,
        timestamp: Instant,
    },
    Reset {
        device: DeviceId,
    },
}

/// Whether every selected device is ready and additional bounded work remains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamStatus {
    pub ready: bool,
    /// Includes unread events, incomplete frames, initialization and recovery.
    pub pending: bool,
}
