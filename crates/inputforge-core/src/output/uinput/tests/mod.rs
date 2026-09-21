mod device;
mod encoding;
mod event_device;
mod failures;
pub(super) mod fixtures;
mod keyboard;
mod lifecycle;
mod mouse;
mod writes;

#[cfg(feature = "evdev-input")]
pub(crate) fn discovery_metadata(slot: u8) -> crate::device::evdev::Metadata {
    let cfg = super::default_config(slot);
    crate::device::evdev::Metadata {
        node: format!("/dev/input/event{slot}").into(),
        name: super::config::name(slot),
        phys: Some(super::config::phys(slot)),
        bus: Some(6),
        event_types: [0, 1, 3].into_iter().collect(),
        keys: (1..=cfg.button_count)
            .map(|button| super::config::button_code(button).unwrap())
            .collect(),
        abs_axes: super::config::axes(&cfg)
            .into_iter()
            .map(|axis| axis.0)
            .collect(),
        properties: [("ID_INPUT_JOYSTICK".into(), "1".into())]
            .into_iter()
            .collect(),
        ..crate::device::evdev::Metadata::default()
    }
}
mod adapter;
