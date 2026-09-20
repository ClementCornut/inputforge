use super::*;
use crate::types::{AxisPolarity, DeviceId};

fn device() -> Device {
    Device {
        metadata: Metadata {
            keys: [300, 704].into(),
            abs_axes: [5, 16, 17].into(),
            ..Metadata::default()
        },
        classification: Classification {
            kind: Class::Controller,
            reasons: vec![],
        },
        identity: Identity {
            id: Some(DeviceId("evdev:v1:test".into())),
            quality: IdentityQuality::Serial,
        },
        axes: [5, 16, 17]
            .map(|code| AxisInfo {
                code,
                minimum: -1,
                maximum: 1,
                fuzz: 0,
                flat: 0,
                resolution: 0,
            })
            .into(),
        access: Access::Readable,
        issues: vec![],
    }
}

#[test]
fn bindings_preserve_sparse_controls_and_reconcile_changed_capabilities() {
    let mut device = device();
    let mut table = bindings::from_device(&device).unwrap();
    assert_eq!(table.buttons, [300, 704]);
    assert_eq!(table.hats, [0]);
    table.axes[0].polarity = AxisPolarity::Unipolar;
    device.axes.reverse();
    table.reconcile(&bindings::from_device(&device).unwrap(), true);
    device.axes[2].maximum = 2;
    table.reconcile(&bindings::from_device(&device).unwrap(), true);
    assert_eq!(table.axes[0].maximum, 2);
}

#[test]
fn bindings_reject_partial_hats_and_overflow() {
    let mut device = device();
    device.metadata.abs_axes.remove(&17);
    bindings::from_device(&device).unwrap_err();
    let mut device = self::device();
    device.metadata.keys = (256..512).collect();
    bindings::from_device(&device).unwrap_err();
}
