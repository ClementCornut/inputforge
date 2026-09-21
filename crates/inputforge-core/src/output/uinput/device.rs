use super::config;
use crate::types::VirtualDeviceConfig;
use evdev::{BusType, InputId, KeyCode, RelativeAxisCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum DeviceKey {
    Controller(u8),
    Keyboard,
    Mouse,
}

impl DeviceKey {
    pub(super) const fn slot(self) -> Option<u8> {
        match self {
            Self::Controller(slot) => Some(slot),
            Self::Keyboard | Self::Mouse => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AbsoluteAxis {
    pub(super) code: u16,
    pub(super) minimum: i32,
    pub(super) maximum: i32,
}

impl From<(u16, i32, i32)> for AbsoluteAxis {
    fn from((code, minimum, maximum): (u16, i32, i32)) -> Self {
        Self {
            code,
            minimum,
            maximum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeviceSpec {
    pub(super) key: DeviceKey,
    pub(super) name: String,
    pub(super) phys: String,
    pub(super) id: InputId,
    pub(super) keys: Vec<u16>,
    pub(super) relatives: Vec<u16>,
    pub(super) absolutes: Vec<AbsoluteAxis>,
    pub(super) required_class: &'static str,
}

impl DeviceSpec {
    pub(super) fn controller(controller: &VirtualDeviceConfig) -> Self {
        let mut absolutes: Vec<_> = config::axes(controller)
            .into_iter()
            .map(AbsoluteAxis::from)
            .collect();
        absolutes.sort_unstable_by_key(|axis| axis.code);
        Self {
            key: DeviceKey::Controller(controller.device_id),
            name: config::name(controller.device_id),
            phys: config::phys(controller.device_id),
            id: InputId::new(BusType::BUS_VIRTUAL, 0, u16::from(controller.device_id), 1),
            keys: (1..=controller.button_count)
                .map(|button| config::button_code(button).expect("validated button"))
                .collect(),
            relatives: Vec::new(),
            absolutes,
            required_class: "ID_INPUT_JOYSTICK",
        }
    }

    pub(super) fn keyboard(mut keys: Vec<u16>) -> Self {
        keys.sort_unstable();
        keys.dedup();
        Self {
            key: DeviceKey::Keyboard,
            name: "InputForge Keyboard".to_owned(),
            phys: "inputforge/output/keyboard".to_owned(),
            id: InputId::new(BusType::BUS_VIRTUAL, 0, 0, 1),
            keys,
            relatives: Vec::new(),
            absolutes: Vec::new(),
            required_class: "ID_INPUT_KEYBOARD",
        }
    }

    pub(super) fn mouse() -> Self {
        Self {
            key: DeviceKey::Mouse,
            name: "InputForge Mouse".to_owned(),
            phys: "inputforge/output/mouse".to_owned(),
            id: InputId::new(BusType::BUS_VIRTUAL, 0, 0, 1),
            keys: vec![
                KeyCode::BTN_LEFT.0,
                KeyCode::BTN_RIGHT.0,
                KeyCode::BTN_MIDDLE.0,
                KeyCode::BTN_SIDE.0,
                KeyCode::BTN_EXTRA.0,
            ],
            relatives: vec![RelativeAxisCode::REL_WHEEL.0],
            absolutes: Vec::new(),
            required_class: "ID_INPUT_MOUSE",
        }
    }

    pub(super) fn events(&self) -> Vec<u16> {
        let mut events = vec![0];
        if !self.keys.is_empty() {
            events.push(1);
        }
        if !self.relatives.is_empty() {
            events.push(2);
        }
        if !self.absolutes.is_empty() {
            events.push(3);
        }
        events
    }
}
