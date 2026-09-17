use std::path::PathBuf;

use super::{
    Access, Class, Classification, Device, Identity, IdentityQuality, Metadata, identify,
    mark_collisions,
};
use crate::types::DeviceDiagnostics;

fn usb_metadata() -> Metadata {
    Metadata {
        bus: Some(0x03),
        name: "VPC Stick".into(),
        interface: Some("01".into()),
        diagnostics: DeviceDiagnostics {
            vendor_id: Some(0x3344),
            product_id: Some(0x83f4),
            ..DeviceDiagnostics::default()
        },
        ..Metadata::default()
    }
}

fn device(id: Option<&str>, quality: IdentityQuality) -> Device {
    Device {
        metadata: Metadata::default(),
        classification: Classification {
            kind: Class::Controller,
            reasons: Vec::new(),
        },
        identity: Identity {
            id: id.map(|id| crate::types::DeviceId(id.into())),
            quality,
        },
        axes: Vec::new(),
        access: Access::NotAttempted,
        issues: Vec::new(),
    }
}

#[test]
fn serial_identity_ignores_runtime_node_and_port_movement() {
    let mut first = usb_metadata();
    first.node = PathBuf::from("/dev/input/event4");
    first.stable_path = Some("pci-0000:00:14.0-usb-0:3:1.0".into());
    first.diagnostics.serial = Some("ABC123".into());

    let mut moved = first.clone();
    moved.node = PathBuf::from("/dev/input/event19");
    moved.stable_path = Some("pci-0000:00:14.0-usb-0:8:1.0".into());

    let first = identify(&first);
    let moved = identify(&moved);
    assert!(matches!(first.quality, IdentityQuality::Serial));
    assert_eq!(first.id, moved.id);
    assert!(
        first
            .id
            .as_ref()
            .is_some_and(|id| id.0.starts_with("evdev:v1:"))
    );
}

#[test]
fn evdev_unique_can_supply_serial_quality() {
    let mut metadata = usb_metadata();
    metadata.unique = Some("stick-serial-42".into());

    assert!(matches!(
        identify(&metadata).quality,
        IdentityQuality::Serial
    ));
}

#[test]
fn placeholder_usb_serial_falls_through_to_evdev_unique() {
    let mut metadata = usb_metadata();
    metadata.diagnostics.serial = Some("noserial".into());
    metadata.unique = Some("stick-serial-42".into());

    assert!(matches!(
        identify(&metadata).quality,
        IdentityQuality::Serial
    ));
}

#[test]
fn blank_and_noserial_values_are_ignored() {
    for serial in ["", "   ", "noserial", "NO_SERIAL"] {
        let mut metadata = usb_metadata();
        metadata.diagnostics.serial = Some(serial.into());
        let identity = identify(&metadata);
        assert!(identity.id.is_none(), "{serial:?}");
        assert!(matches!(identity.quality, IdentityQuality::Ambiguous));
    }
}

#[test]
fn composite_interfaces_receive_distinct_serial_identities() {
    let mut first = usb_metadata();
    first.diagnostics.serial = Some("ABC123".into());
    let mut second = first.clone();
    second.interface = Some("02".into());

    assert_ne!(identify(&first).id, identify(&second).id);
}

#[test]
fn serial_without_interface_discriminator_is_insufficient() {
    let mut metadata = usb_metadata();
    metadata.interface = None;
    metadata.diagnostics.serial = Some("ABC123".into());

    let identity = identify(&metadata);
    assert!(identity.id.is_none());
    assert!(matches!(identity.quality, IdentityQuality::Ambiguous));
}

#[test]
fn stable_topology_provides_port_quality_and_tracks_port_movement() {
    let mut first = usb_metadata();
    first.stable_path = Some("pci-0000:00:14.0-usb-0:3:1.0".into());
    let mut moved = first.clone();
    moved.stable_path = Some("pci-0000:00:14.0-usb-0:8:1.0".into());

    let first = identify(&first);
    let moved = identify(&moved);
    assert!(matches!(first.quality, IdentityQuality::Port));
    assert_ne!(first.id, moved.id);
}

#[test]
fn stable_identity_is_independent_of_discovery_order() {
    let mut first = usb_metadata();
    first.stable_path = Some("pci-0000:00:14.0-usb-0:3:1.0".into());
    let mut second = usb_metadata();
    second.stable_path = Some("pci-0000:00:14.0-usb-0:8:1.0".into());

    let forward = [identify(&first).id, identify(&second).id];
    let reverse = [identify(&second).id, identify(&first).id];
    assert_eq!(forward[0], reverse[1]);
    assert_eq!(forward[1], reverse[0]);
}

#[test]
fn runtime_input_names_are_not_identity_material() {
    let mut metadata = usb_metadata();
    metadata.phys = Some("usb-0000:00:14.0-3/input7".into());
    metadata.node = PathBuf::from("/dev/input/event12");

    let identity = identify(&metadata);
    assert!(identity.id.is_none());
    assert!(matches!(identity.quality, IdentityQuality::Ambiguous));
}

#[test]
fn missing_vid_pid_or_bus_prevents_serial_identity() {
    let mut metadata = usb_metadata();
    metadata.diagnostics.serial = Some("ABC123".into());
    metadata.diagnostics.vendor_id = None;

    assert!(identify(&metadata).id.is_none());

    metadata.diagnostics.vendor_id = Some(0x3344);
    metadata.bus = None;
    assert!(identify(&metadata).id.is_none());
}

#[test]
fn duplicate_derived_ids_are_cleared_for_every_colliding_node() {
    let mut devices = [
        device(Some("evdev:v1:duplicate"), IdentityQuality::Serial),
        device(Some("evdev:v1:unique"), IdentityQuality::Port),
        device(Some("evdev:v1:duplicate"), IdentityQuality::Serial),
    ];

    mark_collisions(&mut devices);

    assert!(devices[0].identity.id.is_none());
    assert!(matches!(
        devices[0].identity.quality,
        IdentityQuality::Ambiguous
    ));
    assert!(
        devices[0]
            .classification
            .reasons
            .iter()
            .any(|reason| reason.contains("duplicate"))
    );
    assert_eq!(
        devices[1].identity.id.as_ref().map(|id| id.0.as_str()),
        Some("evdev:v1:unique")
    );
    assert!(devices[2].identity.id.is_none());
    assert!(matches!(
        devices[2].identity.quality,
        IdentityQuality::Ambiguous
    ));
    assert!(
        devices[2]
            .classification
            .reasons
            .iter()
            .any(|reason| reason.contains("duplicate"))
    );
}
