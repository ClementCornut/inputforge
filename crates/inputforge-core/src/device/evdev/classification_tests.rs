use super::{Class, Metadata, classify};
use crate::types::DeviceDiagnostics;

const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;
const INPUT_PROP_ACCELEROMETER: u16 = 0x06;
const KEY_POWER: u16 = 0x74;
const BTN_JOYSTICK: u16 = 0x120;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_Z: u16 = 0x02;
const ABS_RX: u16 = 0x03;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;
const ABS_MISC: u16 = 0x28;

fn joystick() -> Metadata {
    let mut metadata = Metadata::default();
    metadata
        .properties
        .insert("ID_INPUT_JOYSTICK".into(), "1".into());
    metadata.event_types.extend([EV_KEY, EV_ABS]);
    metadata.keys.insert(BTN_JOYSTICK);
    metadata.abs_axes.extend([ABS_X, ABS_Y]);
    metadata
}

#[test]
fn virpil_event_nodes_are_controller_candidates() {
    for product in [0x83f4, 0x43f5] {
        let mut metadata = joystick();
        metadata.name = "VPC Stick MT-50CM2".into();
        metadata.diagnostics.vendor_id = Some(0x3344);
        metadata.diagnostics.product_id = Some(product);
        metadata.abs_axes = [0, 1, 2, 3, 4, 6].into_iter().collect();
        metadata.keys = (0x120..=0x12f).chain(0x2c0..=0x2cf).collect();

        assert!(matches!(classify(&metadata).kind, Class::Controller));
    }
}

#[test]
fn same_parent_joydev_handler_can_establish_candidacy() {
    let mut metadata = Metadata::default();
    metadata.joydev.push("/dev/input/js7".into());
    metadata.event_types.insert(EV_KEY);
    metadata.keys.insert(BTN_JOYSTICK);

    assert!(matches!(classify(&metadata).kind, Class::Controller));
}

#[test]
fn button_only_controller_is_supported() {
    let mut metadata = joystick();
    metadata.event_types.remove(&EV_ABS);
    metadata.abs_axes.clear();

    assert!(matches!(classify(&metadata).kind, Class::Controller));
}

#[test]
fn controls_without_joystick_evidence_are_ambiguous() {
    let mut metadata = Metadata::default();
    metadata.event_types.insert(EV_KEY);
    metadata.keys.insert(BTN_JOYSTICK);

    let result = classify(&metadata);
    assert!(matches!(result.kind, Class::Ambiguous));
    assert!(
        result
            .reasons
            .iter()
            .any(|reason| reason.contains("joystick"))
    );
}

#[test]
fn inputforge_signature_is_excluded_but_external_virtual_controller_is_allowed() {
    let mut own = joystick();
    own.phys = Some("inputforge/output/gamepad".into());
    assert!(matches!(classify(&own).kind, Class::Excluded));

    let mut external = joystick();
    external.bus = Some(0x06);
    external.phys = Some("external/virtual/gamepad".into());
    assert!(matches!(classify(&external).kind, Class::Controller));
}

#[test]
fn kernel_accelerometer_property_is_excluded() {
    let mut metadata = joystick();
    metadata.kernel_properties.insert(INPUT_PROP_ACCELEROMETER);

    assert!(matches!(classify(&metadata).kind, Class::Excluded));
}

#[test]
fn true_keyboard_pointer_touch_and_tablet_nodes_are_excluded() {
    for tag in [
        "ID_INPUT_KEYBOARD",
        "ID_INPUT_MOUSE",
        "ID_INPUT_POINTINGSTICK",
        "ID_INPUT_TOUCHPAD",
        "ID_INPUT_TOUCHSCREEN",
        "ID_INPUT_TABLET",
        "ID_INPUT_TABLET_PAD",
    ] {
        let mut metadata = joystick();
        metadata.properties.insert(tag.into(), "1".into());
        assert!(matches!(classify(&metadata).kind, Class::Excluded), "{tag}");
    }
}

#[test]
fn generic_acceleration_conflict_remains_ambiguous() {
    let mut metadata = joystick();
    metadata.joydev.push("/dev/input/js0".into());
    metadata
        .properties
        .insert("ID_INPUT_ACCELEROMETER".into(), "1".into());

    let result = classify(&metadata);
    assert!(matches!(result.kind, Class::Ambiguous));
    assert!(
        result
            .reasons
            .iter()
            .any(|reason| reason.contains("acceler"))
    );
}

#[test]
fn narrow_thrustmaster_pedals_override_udev_acceleration_conflict() {
    let mut metadata = Metadata {
        name: "Thrustmaster Sim Pedals".into(),
        diagnostics: DeviceDiagnostics {
            vendor_id: Some(0x044f),
            product_id: Some(0xb371),
            ..DeviceDiagnostics::default()
        },
        ..Metadata::default()
    };
    metadata
        .properties
        .insert("ID_INPUT_ACCELEROMETER".into(), "1".into());
    metadata.joydev.push("/dev/input/js0".into());
    metadata.event_types.insert(EV_ABS);
    metadata.abs_axes.extend([ABS_X, ABS_Y, ABS_Z]);

    let result = classify(&metadata);
    assert!(matches!(result.kind, Class::Controller));
    assert!(result.reasons.iter().any(|reason| reason.contains("udev")));
    assert!(
        result
            .reasons
            .iter()
            .any(|reason| reason.contains("acceler"))
    );
}

#[test]
fn thrustmaster_override_rejects_near_matches() {
    let mut metadata = Metadata {
        name: "Thrustmaster Sim Pedals".into(),
        diagnostics: DeviceDiagnostics {
            vendor_id: Some(0x044f),
            product_id: Some(0xb371),
            ..DeviceDiagnostics::default()
        },
        ..Metadata::default()
    };
    metadata.properties.extend([
        ("ID_INPUT_JOYSTICK".into(), "1".into()),
        ("ID_INPUT_ACCELEROMETER".into(), "1".into()),
    ]);
    metadata.event_types.extend([EV_ABS, EV_REL]);
    metadata.abs_axes.extend([ABS_X, ABS_Y, ABS_Z, ABS_RX]);
    metadata.rel_axes.insert(0);

    assert!(matches!(classify(&metadata).kind, Class::Ambiguous));
}

#[test]
fn system_keys_and_hats_do_not_prove_monsgeek_capture_eligibility() {
    let mut metadata = Metadata {
        name: "MonsGeek Keyboard".into(),
        ..Metadata::default()
    };
    metadata
        .properties
        .insert("ID_INPUT_JOYSTICK".into(), "1".into());
    metadata.event_types.extend([EV_KEY, EV_ABS]);
    metadata.keys.insert(KEY_POWER);
    metadata.abs_axes.extend([ABS_HAT0X, ABS_HAT0Y, ABS_MISC]);

    assert!(matches!(classify(&metadata).kind, Class::Ambiguous));
}

#[test]
fn high_keyboard_codes_are_not_controller_buttons() {
    for key in [0x161, 0x1b6] {
        let mut metadata = Metadata::default();
        metadata
            .properties
            .insert("ID_INPUT_JOYSTICK".into(), "1".into());
        metadata.event_types.insert(EV_KEY);
        metadata.keys.insert(key);

        assert!(matches!(classify(&metadata).kind, Class::Ambiguous));
    }
}

#[test]
fn missing_controls_explain_ambiguity() {
    let mut metadata = Metadata::default();
    metadata
        .properties
        .insert("ID_INPUT_JOYSTICK".into(), "1".into());

    let result = classify(&metadata);
    assert!(matches!(result.kind, Class::Ambiguous));
    assert!(
        result
            .reasons
            .iter()
            .any(|reason| reason.contains("control"))
    );
}

#[test]
fn capability_sets_without_matching_event_types_are_ambiguous() {
    let mut metadata = joystick();
    metadata.event_types.clear();

    let result = classify(&metadata);
    assert!(matches!(result.kind, Class::Ambiguous));
    assert!(
        result
            .reasons
            .iter()
            .any(|reason| reason.contains("inconsistent"))
    );
}
