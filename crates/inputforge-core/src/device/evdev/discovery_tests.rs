use super::{Access, Class, Issue, Metadata, discovery::assemble};
use std::{io, path::PathBuf};

#[cfg(feature = "uinput-output")]
#[test]
fn generated_uinput_devices_are_excluded_before_event_node_probes() {
    let entries = (1..=16)
        .map(|slot| {
            (
                crate::output::uinput::tests::discovery_metadata(slot),
                vec![],
            )
        })
        .collect();
    let report = assemble(entries, |_| {
        panic!("must not open InputForge virtual output")
    });
    assert_eq!(report.devices.len(), 16);
    for device in report.devices {
        assert_eq!(device.classification.kind, Class::Excluded);
        assert_eq!(device.access, Access::NotAttempted);
    }
}

fn candidate(node: &str) -> Metadata {
    let mut metadata = Metadata {
        node: node.into(),
        name: "Test stick".into(),
        ..Metadata::default()
    };
    metadata
        .properties
        .insert("ID_INPUT_JOYSTICK".into(), "1".into());
    metadata.event_types.insert(1);
    metadata.keys.insert(0x120);
    metadata
}

#[test]
fn retains_denied_and_disconnected_nodes_and_continues_scan() {
    let entries = [
        "/dev/input/event9",
        "/dev/input/event1",
        "/dev/input/event4",
    ]
    .into_iter()
    .map(|path| (candidate(path), vec![]))
    .collect();
    let report = assemble(entries, |metadata| {
        let errno = match metadata.node.to_str().expect("test path") {
            "/dev/input/event1" => 13,
            "/dev/input/event4" => 19,
            _ => return Ok(vec![]),
        };
        Err(Issue::new(
            "open read-only",
            metadata.node.clone(),
            &io::Error::from_raw_os_error(errno),
        ))
    });
    assert_eq!(report.devices.len(), 3);
    assert_eq!(
        report.devices[0].metadata.node,
        PathBuf::from("/dev/input/event1")
    );
    assert_eq!(report.devices[0].access, Access::Failed);
    assert_eq!(report.devices[0].issues[0].raw_os_error, Some(13));
    assert_eq!(report.devices[1].issues[0].raw_os_error, Some(19));
    assert_eq!(report.devices[2].access, Access::Readable);
}

#[test]
fn excluded_and_ambiguous_nodes_are_never_probed() {
    let mut keyboard = candidate("/dev/input/event1");
    keyboard
        .properties
        .insert("ID_INPUT_KEYBOARD".into(), "1".into());
    let unknown = Metadata::default();
    let report = assemble(vec![(keyboard, vec![]), (unknown, vec![])], |_| {
        panic!("must not open")
    });
    assert!(
        report
            .devices
            .iter()
            .all(|device| device.access == Access::NotAttempted)
    );
}

#[test]
fn incomplete_metadata_prevents_opening_and_preserves_issue() {
    let metadata = candidate("/dev/input/event1");
    let issue = Issue::new(
        "capabilities",
        "/sys/example",
        &io::Error::new(io::ErrorKind::InvalidData, "bad bitmap"),
    );
    let report = assemble(vec![(metadata, vec![issue])], |_| {
        panic!("metadata incomplete")
    });
    assert_eq!(report.devices[0].classification.kind, Class::Ambiguous);
    assert_eq!(report.devices[0].issues[0].message, "bad bitmap");
}

#[test]
fn missing_node_does_not_erase_controller_classification() {
    let metadata = candidate("/dev/input/event1");
    let issue = Issue::new(
        "stat event node",
        &metadata.node,
        &io::Error::from_raw_os_error(2),
    );
    let report = assemble(vec![(metadata, vec![issue])], |metadata| {
        Err(Issue::new(
            "open read-only",
            &metadata.node,
            &io::Error::from_raw_os_error(2),
        ))
    });
    assert_eq!(report.devices[0].classification.kind, Class::Controller);
    assert_eq!(report.devices[0].access, Access::Failed);
    assert_eq!(report.devices[0].issues.len(), 2);
}
