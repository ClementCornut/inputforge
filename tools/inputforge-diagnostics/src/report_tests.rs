use super::run_with;
use inputforge_core::device::evdev::{
    Access, Class, Classification, Device, Identity, IdentityQuality, Issue, Metadata, Report,
};
use std::io::{self, Write};

#[test]
fn diagnostics_reports_empty_scan() {
    let mut output = Vec::new();
    run_with(|| Ok(Report::default()), &mut output).expect("empty scan succeeds");
    let text = String::from_utf8(output).expect("UTF-8 output");
    assert!(text.contains("0 event interfaces"));
    assert!(text.contains("No event streams read"));
    assert!(text.contains("write access and device creation NOT tested"));
}

fn denied_report() -> Report {
    Report {
        devices: vec![Device {
            metadata: Metadata {
                node: "/dev/input/event4".into(),
                name: "Stick\x1b[2J\nname".into(),
                ..Metadata::default()
            },
            classification: Classification {
                kind: Class::Controller,
                reasons: vec!["udev joystick".into()],
            },
            identity: Identity {
                id: None,
                quality: IdentityQuality::Ambiguous,
            },
            axes: vec![],
            access: Access::Failed,
            issues: vec![Issue {
                operation: "open read-only".into(),
                path: "/dev/input/event4".into(),
                kind: io::ErrorKind::PermissionDenied,
                raw_os_error: Some(13),
                message: "Permission denied".into(),
            }],
        }],
        ..Report::default()
    }
}

#[test]
fn per_device_denial_is_success_with_actionable_and_escaped_output() {
    let mut output = Vec::new();
    run_with(|| Ok(denied_report()), &mut output).expect("report includes failures");
    let text = String::from_utf8(output).expect("UTF-8 output");
    assert!(text.contains("PermissionDenied"));
    assert!(text.contains("errno=Some(13)"));
    assert!(text.contains("getfacl"));
    assert!(text.contains("uaccess"));
    assert!(!text.contains('\x1b'));
    assert!(text.contains("Ambiguous"));
}

#[derive(Debug)]
struct BrokenWriter;
impl Write for BrokenWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn scan_and_output_failures_propagate() {
    let error = run_with(|| Err(io::ErrorKind::PermissionDenied.into()), Vec::new())
        .expect_err("scan failed");
    assert!(error.to_string().contains("enumerate"));
    let error = run_with(|| Ok(Report::default()), BrokenWriter).expect_err("output failed");
    assert!(error.downcast_ref::<io::Error>().is_some());
}

#[test]
fn renders_native_axes_and_incomplete_hat_without_inventing_controls() {
    let mut report = denied_report();
    report.devices[0].metadata.abs_axes.insert(0x10);
    report.devices[0].metadata.keys.insert(0x2c0);
    report.devices[0]
        .axes
        .push(inputforge_core::device::evdev::AxisInfo {
            code: 0x10,
            minimum: -1,
            maximum: 1,
            fuzz: 0,
            flat: 0,
            resolution: 0,
        });
    let mut first = Vec::new();
    run_with(|| Ok(report.clone()), &mut first).expect("first report");
    let mut second = Vec::new();
    run_with(|| Ok(report), &mut second).expect("second report");
    assert_eq!(first, second);
    let text = String::from_utf8(first).expect("UTF-8 output");
    assert!(text.contains("KEY={2c0}"));
    assert!(text.contains("Hat pair 10/11: incomplete"));
    assert!(text.contains("ABS 10: min=-1 max=1"));
    assert!(!text.contains("ABS 11:"));
}
