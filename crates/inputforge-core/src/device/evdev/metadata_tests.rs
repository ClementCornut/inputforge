use super::read_input;
use crate::device::evdev::Metadata;
use std::{fs, io, path::Path};

fn fixture(parent: &Path) -> Metadata {
    fs::create_dir_all(parent.join("id")).expect("id directory");
    fs::create_dir_all(parent.join("capabilities")).expect("capability directory");
    for (name, value) in [
        ("name", "Test pedals\n"),
        ("phys", "usb-test/input0\n"),
        ("uniq", "\n"),
        ("id/bustype", "0003"),
        ("id/vendor", "044f"),
        ("id/product", "b371"),
        ("id/version", "0100"),
        ("properties", "0"),
        ("capabilities/ev", "9"),
        ("capabilities/key", "0"),
        ("capabilities/abs", "7"),
        ("capabilities/rel", "0"),
        ("event7", ""),
        ("js0", ""),
        ("json", ""),
        ("js", ""),
    ] {
        fs::write(parent.join(name), value).expect("fixture attribute");
    }
    Metadata {
        node: parent.join("event7"),
        ..Metadata::default()
    }
}

#[test]
fn reads_native_ids_and_only_same_parent_joydev_handlers() {
    let directory = tempfile::tempdir().expect("temporary fixture");
    let metadata = fixture(directory.path());
    let (metadata, issues) = read_input(directory.path(), metadata, vec![]);
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(metadata.diagnostics.vendor_id, Some(0x044f));
    assert_eq!(metadata.diagnostics.product_id, Some(0xb371));
    assert_eq!(metadata.abs_axes, [0, 1, 2].into());
    assert_eq!(metadata.joydev, ["/dev/input/js0"]);
    assert!(metadata.unique.is_none());
}

#[test]
fn malformed_capability_and_missing_node_retain_exact_failures() {
    let directory = tempfile::tempdir().expect("temporary fixture");
    let metadata = fixture(directory.path());
    fs::write(directory.path().join("capabilities/abs"), "not a bitmap")
        .expect("malformed fixture");
    fs::remove_file(&metadata.node).expect("simulate disappearance");
    let (_, issues) = read_input(directory.path(), metadata, vec![]);
    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].kind, io::ErrorKind::InvalidData);
    assert_eq!(issues[0].path, directory.path().join("capabilities/abs"));
    assert_eq!(issues[1].operation, "stat event node");
    assert_eq!(issues[1].raw_os_error, Some(2));
}

#[test]
fn preserves_kernel_string_whitespace_except_the_sysfs_added_newline() {
    let directory = tempfile::tempdir().expect("temporary fixture");
    let metadata = fixture(directory.path());
    for (name, value) in [
        ("name", "  Padded controller \n"),
        ("phys", " usb/controller \n"),
        ("uniq", " serial \n"),
    ] {
        fs::write(directory.path().join(name), value).expect("padded fixture");
    }
    let (metadata, issues) = read_input(directory.path(), metadata, vec![]);
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(metadata.name, "  Padded controller ");
    assert_eq!(metadata.phys.as_deref(), Some(" usb/controller "));
    assert_eq!(metadata.unique.as_deref(), Some(" serial "));
}
