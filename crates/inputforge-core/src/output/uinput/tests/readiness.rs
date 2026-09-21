use super::{Metadata, verify};
use crate::output::uinput::{
    default_config,
    device::DeviceSpec,
    native::sysfs::{Attributes, parse},
};
use evdev::{AbsInfo, BusType, InputId};

fn metadata() -> Metadata {
    Metadata {
        name: Some("InputForge Virtual Controller 2".into()),
        phys: Some("inputforge/controller/v1/slot/2".into()),
        id: InputId::new(BusType::BUS_VIRTUAL, 0, 2, 1),
        keys: (0x120..=0x12b)
            .chain([0x12f])
            .chain(0x2c0..=0x2d2)
            .collect(),
        relatives: vec![],
        axes: (0..8)
            .map(|code| (code, AbsInfo::new(0, -32767, 32767, 0, 0, 0)))
            .chain((0x10..=0x17).map(|code| (code, AbsInfo::new(0, -1, 1, 0, 0, 0))))
            .collect(),
        events: vec![0, 1, 3],
        class: Some(("ID_INPUT_JOYSTICK".into(), "1".into())),
    }
}

#[test]
fn readiness_requires_exact_identity_keys_axes_ranges_and_event_classes() {
    let config = default_config(2);
    let spec = DeviceSpec::controller(&config);
    verify(&metadata(), &spec).unwrap();
    let mutations: [fn(&mut Metadata); 9] = [
        |m| m.name = Some("different controller".into()),
        |m| m.phys = None,
        |m| m.id = InputId::new(BusType::BUS_VIRTUAL, 0, 3, 1),
        |m| {
            m.keys.pop();
        },
        |m| m.keys.push(0x130),
        |m| {
            m.axes.pop();
        },
        |m| m.axes[0].1 = AbsInfo::new(0, 0, 65535, 0, 0, 0),
        |m| m.axes[8].1 = AbsInfo::new(0, -1, 1, 1, 0, 0),
        |m| m.events.push(0x15),
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

#[test]
fn readiness_accepts_buttons_only_and_ignores_current_axis_values() {
    let mut actual = metadata();
    actual.axes[0].1 = AbsInfo::new(123, -32767, 32767, 0, 0, 0);
    verify(&actual, &DeviceSpec::controller(&default_config(2))).unwrap();
    let mut config = default_config(2);
    config.axes.clear();
    config.hat_count = 0;
    config.button_count = 2;
    actual.axes.clear();
    actual.keys = vec![0x120, 0x121];
    actual.events = vec![0, 1];
    verify(&actual, &DeviceSpec::controller(&config)).unwrap();
}

#[test]
fn sysfs_metadata_parses_exact_identity_hex_ids_and_kernel_bitmap_order() {
    let actual = parse(&Attributes {
        name: "InputForge Keyboard",
        phys: "inputforge/output/keyboard",
        bustype: "0006",
        vendor: "0000",
        product: "0000",
        version: "0001",
        events: "3",
        keys: "1 40000002",
        relatives: "0",
        absolutes: "0",
        class: Some(("ID_INPUT_KEYBOARD", "1")),
    })
    .expect("valid sysfs metadata");

    let spec = DeviceSpec::keyboard(vec![1, 30, 64]);
    verify(&actual, &spec).expect("exact sysfs metadata");
}

#[test]
fn sysfs_metadata_rejects_malformed_ids_bitmaps_and_absolute_axes() {
    let attributes = |bustype, keys, absolutes| Attributes {
        name: "InputForge Keyboard",
        phys: "inputforge/output/keyboard",
        bustype,
        vendor: "0000",
        product: "0000",
        version: "0001",
        events: "3",
        keys,
        relatives: "0",
        absolutes,
        class: Some(("ID_INPUT_KEYBOARD", "1")),
    };

    for invalid in [
        attributes("not-hex", "40000002", "0"),
        attributes("0006", "not-hex", "0"),
        attributes("0006", "40000002", "1"),
    ] {
        assert_eq!(
            parse(&invalid).err().expect("invalid metadata").kind(),
            std::io::ErrorKind::InvalidData
        );
    }
}
