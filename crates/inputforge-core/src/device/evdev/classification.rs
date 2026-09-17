use super::{Class, Classification, Metadata};

const INPUT_PROP_ACCELEROMETER: u16 = 0x06;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const BTN_JOYSTICK: u16 = 0x120;
const BTN_GAMEPAD_LAST: u16 = 0x13f;
const BTN_DPAD_UP: u16 = 0x220;
const BTN_DPAD_RIGHT: u16 = 0x223;
const BTN_TRIGGER_HAPPY: u16 = 0x2c0;
const BTN_TRIGGER_HAPPY_LAST: u16 = 0x2e7;
const ABS_BRAKE: u16 = 0x0a;

const NON_CONTROLLER_TAGS: [&str; 7] = [
    "ID_INPUT_KEYBOARD",
    "ID_INPUT_MOUSE",
    "ID_INPUT_POINTINGSTICK",
    "ID_INPUT_TOUCHPAD",
    "ID_INPUT_TOUCHSCREEN",
    "ID_INPUT_TABLET",
    "ID_INPUT_TABLET_PAD",
];

pub(super) fn classify(metadata: &Metadata) -> Classification {
    if metadata
        .phys
        .as_deref()
        .is_some_and(|phys| phys.starts_with("inputforge/"))
    {
        return result(Class::Excluded, "InputForge virtual output interface");
    }
    if metadata
        .kernel_properties
        .contains(&INPUT_PROP_ACCELEROMETER)
    {
        return result(Class::Excluded, "kernel INPUT_PROP_ACCELEROMETER property");
    }
    if let Some(tag) = NON_CONTROLLER_TAGS
        .iter()
        .find(|tag| property_is_one(metadata, tag))
    {
        return result(Class::Excluded, &format!("udev {tag}=1 input interface"));
    }

    let candidate = property_is_one(metadata, "ID_INPUT_JOYSTICK") || !metadata.joydev.is_empty();
    if !candidate {
        return result(
            Class::Ambiguous,
            "missing joystick tag or same-parent joydev handler",
        );
    }

    let acceleration_conflict = property_is_one(metadata, "ID_INPUT_ACCELEROMETER");
    if is_thrustmaster_pedals(metadata) {
        let mut classification = result(
            Class::Controller,
            "narrow Thrustmaster 044f:b371 pedal capability signature",
        );
        if acceleration_conflict {
            classification
                .reasons
                .push("accepted despite conflicting udev accelerometer tag".into());
        }
        return classification;
    }
    if acceleration_conflict {
        return result(Class::Ambiguous, "conflicting udev accelerometer tag");
    }
    if !metadata.rel_axes.is_empty() {
        return result(Class::Ambiguous, "mixed relative and controller controls");
    }
    if (!metadata.keys.is_empty() && !metadata.event_types.contains(&EV_KEY))
        || (!metadata.abs_axes.is_empty() && !metadata.event_types.contains(&EV_ABS))
    {
        return result(
            Class::Ambiguous,
            "inconsistent event and control capability bitmaps",
        );
    }

    let has_button = metadata.keys.iter().any(|key| is_controller_button(*key));
    let has_axis = metadata.abs_axes.iter().any(|axis| *axis <= ABS_BRAKE);
    if !has_button && !has_axis {
        return result(Class::Ambiguous, "missing compatible controller controls");
    }

    result(
        Class::Controller,
        "joystick evidence and compatible controller controls",
    )
}

fn property_is_one(metadata: &Metadata, name: &str) -> bool {
    metadata
        .properties
        .get(name)
        .is_some_and(|value| value == "1")
}

fn is_controller_button(key: u16) -> bool {
    (BTN_JOYSTICK..=BTN_GAMEPAD_LAST).contains(&key)
        || (BTN_DPAD_UP..=BTN_DPAD_RIGHT).contains(&key)
        || (BTN_TRIGGER_HAPPY..=BTN_TRIGGER_HAPPY_LAST).contains(&key)
}

fn is_thrustmaster_pedals(metadata: &Metadata) -> bool {
    let expected_axes = [0, 1, 2].into_iter().collect();
    metadata.diagnostics.vendor_id == Some(0x044f)
        && metadata.diagnostics.product_id == Some(0xb371)
        && metadata.abs_axes == expected_axes
        && metadata.event_types.contains(&EV_ABS)
        && metadata.keys.is_empty()
        && metadata.rel_axes.is_empty()
}

fn result(kind: Class, reason: &str) -> Classification {
    Classification {
        kind,
        reasons: vec![reason.into()],
    }
}
