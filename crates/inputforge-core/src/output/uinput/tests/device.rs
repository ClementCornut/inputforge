use super::super::{
    config,
    device::{DeviceKey, DeviceSpec},
    native::readiness::{Metadata, verify},
};
use crate::output::uinput::default_config;
use evdev::{AbsInfo, BusType, InputId, KeyCode, RelativeAxisCode};

#[test]
fn controller_spec_preserves_existing_identity_and_capabilities() {
    let mut config = default_config(2);
    config.axes.reverse();
    let spec = DeviceSpec::controller(&config);

    assert_eq!(spec.key, DeviceKey::Controller(2));
    assert_eq!(spec.name, "InputForge Virtual Controller 2");
    assert_eq!(spec.phys, "inputforge/controller/v1/slot/2");
    assert_eq!(spec.id, InputId::new(BusType::BUS_VIRTUAL, 0, 2, 1));
    assert_eq!(
        spec.keys,
        (1..=config.button_count)
            .map(|button| config::button_code(button).unwrap())
            .collect::<Vec<_>>()
    );
    let mut axes = config::axes(&config);
    axes.sort_unstable_by_key(|axis| axis.0);
    assert_eq!(
        spec.absolutes,
        axes.into_iter()
            .map(|(code, minimum, maximum)| (code, minimum, maximum).into())
            .collect::<Vec<_>>()
    );
    assert!(spec.relatives.is_empty());
    assert_eq!(spec.required_class, "ID_INPUT_JOYSTICK");
}

#[test]
fn keyboard_spec_advertises_exact_keys_and_keyboard_class() {
    let mut keys = vec![
        KeyCode::KEY_ESC.0,
        KeyCode::KEY_A.0,
        KeyCode::KEY_RIGHTALT.0,
        KeyCode::KEY_KPENTER.0,
    ];
    let spec = DeviceSpec::keyboard(keys.clone());
    keys.sort_unstable();

    assert_eq!(spec.key, DeviceKey::Keyboard);
    assert_eq!(spec.name, "InputForge Keyboard");
    assert_eq!(spec.phys, "inputforge/output/keyboard");
    assert_eq!(spec.id, InputId::new(BusType::BUS_VIRTUAL, 0, 0, 1));
    assert_eq!(spec.keys, keys);
    assert!(spec.relatives.is_empty());
    assert!(spec.absolutes.is_empty());
    assert_eq!(spec.events(), vec![0, 1]);
    assert_eq!(spec.required_class, "ID_INPUT_KEYBOARD");
}

#[test]
fn mouse_spec_advertises_five_buttons_wheel_and_no_movement() {
    let spec = DeviceSpec::mouse();

    assert_eq!(spec.key, DeviceKey::Mouse);
    assert_eq!(spec.name, "InputForge Mouse");
    assert_eq!(spec.phys, "inputforge/output/mouse");
    assert_eq!(spec.id, InputId::new(BusType::BUS_VIRTUAL, 0, 0, 1));
    assert_eq!(
        spec.keys,
        [
            KeyCode::BTN_LEFT.0,
            KeyCode::BTN_RIGHT.0,
            KeyCode::BTN_MIDDLE.0,
            KeyCode::BTN_SIDE.0,
            KeyCode::BTN_EXTRA.0,
        ]
    );
    assert_eq!(spec.relatives, [RelativeAxisCode::REL_WHEEL.0]);
    assert!(spec.absolutes.is_empty());
    assert_eq!(spec.events(), vec![0, 1, 2]);
    assert_eq!(spec.required_class, "ID_INPUT_MOUSE");
}

#[test]
fn readiness_rejects_wrong_identity_capabilities_or_class() {
    let spec = DeviceSpec::mouse();
    let metadata = || Metadata {
        name: Some(spec.name.clone()),
        phys: Some(spec.phys.clone()),
        id: spec.id.clone(),
        keys: spec.keys.clone(),
        relatives: spec.relatives.clone(),
        axes: spec
            .absolutes
            .iter()
            .map(|axis| {
                (
                    axis.code,
                    AbsInfo::new(0, axis.minimum, axis.maximum, 0, 0, 0),
                )
            })
            .collect(),
        events: spec.events(),
        class: Some((spec.required_class.to_owned(), "1".to_owned())),
    };
    verify(&metadata(), &spec).unwrap();

    let mutations: [fn(&mut Metadata); 6] = [
        |actual| actual.name = Some("wrong".to_owned()),
        |actual| {
            actual.keys.pop();
        },
        |actual| actual.relatives.push(RelativeAxisCode::REL_X.0),
        |actual| actual.events.push(3),
        |actual| actual.class = None,
        |actual| actual.class = Some(("ID_INPUT_KEYBOARD".to_owned(), "1".to_owned())),
    ];
    for mutate in mutations {
        let mut actual = metadata();
        mutate(&mut actual);
        assert_eq!(
            verify(&actual, &spec).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }
}
