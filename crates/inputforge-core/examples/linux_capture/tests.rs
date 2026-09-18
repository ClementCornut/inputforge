use crate::{
    args::{Command, parse},
    render_state, write_state,
};
use inputforge_core::{
    device::evdev::{
        Access, Class, Classification, Device, Identity, IdentityQuality, Issue, Metadata,
    },
    types::DeviceId,
};
use std::io::{self, Write};

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn no_arguments_and_help_request_usage() {
    assert_eq!(parse(&strings(&[])).expect("no arguments"), Command::Help);
    assert_eq!(
        parse(&strings(&["--help"])).expect("help argument"),
        Command::Help
    );
}

#[test]
fn watch_accepts_no_options() {
    assert_eq!(parse(&strings(&["watch"])).expect("watch"), Command::Watch);
    assert!(parse(&strings(&["watch", "--device", "evdev:v1:a"])).is_err());
}

#[test]
fn capture_requires_a_bounded_duration() {
    for seconds in ["0", "61", "nope"] {
        assert!(
            parse(&strings(&[
                "capture",
                "--seconds",
                seconds,
                "--device",
                "evdev:v1:a",
            ]))
            .is_err(),
            "seconds={seconds}"
        );
    }
    assert!(parse(&strings(&["capture", "--device", "evdev:v1:a"])).is_err());
    for seconds in ["1", "60"] {
        assert!(
            parse(&strings(&[
                "capture",
                "--seconds",
                seconds,
                "--device",
                "evdev:v1:a",
            ]))
            .is_ok(),
            "seconds={seconds}"
        );
    }
}

#[test]
fn capture_requires_namespaced_device_ids() {
    for id in ["controller", "evdev:v1:"] {
        assert!(
            parse(&strings(&["capture", "--seconds", "5", "--device", id,])).is_err(),
            "id={id}"
        );
    }
    assert!(parse(&strings(&["capture", "--seconds", "5"])).is_err());
}

#[test]
fn capture_collects_each_selected_device() {
    assert_eq!(
        parse(&strings(&[
            "capture",
            "--seconds",
            "5",
            "--device",
            "evdev:v1:stick",
            "--device",
            "evdev:v1:pedals",
        ]))
        .expect("capture arguments"),
        Command::Capture {
            seconds: 5,
            devices: vec!["evdev:v1:stick".to_owned(), "evdev:v1:pedals".to_owned()],
        }
    );
}

#[test]
fn capture_rejects_missing_values_duplicate_duration_and_unknown_options() {
    for values in [
        vec!["capture", "--seconds"],
        vec!["capture", "--seconds", "5", "--device"],
        vec![
            "capture",
            "--seconds",
            "5",
            "--seconds",
            "6",
            "--device",
            "evdev:v1:a",
        ],
        vec![
            "capture",
            "--seconds",
            "5",
            "--device",
            "evdev:v1:a",
            "--all",
        ],
    ] {
        assert!(parse(&strings(&values)).is_err(), "values={values:?}");
    }
}

fn rejected_device() -> Device {
    Device {
        metadata: Metadata {
            node: "/dev/input/event4".into(),
            name: "unsafe\x1b[2J\nname".into(),
            ..Metadata::default()
        },
        classification: Classification {
            kind: Class::Ambiguous,
            reasons: vec!["conflicting joystick evidence\nnext".into()],
        },
        identity: Identity {
            id: Some(DeviceId("evdev:v1:test".into())),
            quality: IdentityQuality::Port,
        },
        axes: vec![],
        access: Access::Failed,
        issues: vec![Issue {
            operation: "open read-only".into(),
            path: "/dev/input/event4".into(),
            kind: io::ErrorKind::PermissionDenied,
            raw_os_error: Some(13),
            message: "denied\nnext".into(),
        }],
    }
}

#[test]
fn state_exposes_eligibility_access_and_escaped_diagnostics() {
    let output = render_state(&[rejected_device()], &[DeviceId("evdev:v1:test".into())]);

    assert!(output.contains("identity=Port class=Ambiguous access=Failed"));
    assert!(output.contains("reason=\"conflicting joystick evidence\\nnext\""));
    assert!(output.contains("kind=PermissionDenied errno=Some(13)"));
    assert!(output.contains("message=\"denied\\nnext\""));
    assert!(output.contains("captured: [\"evdev:v1:test\"]"));
    assert!(!output.contains('\x1b'));
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
fn state_output_failure_is_returned_without_advancing_snapshot() {
    let mut previous = String::new();
    let error = write_state(&[rejected_device()], &[], &mut BrokenWriter, &mut previous)
        .expect_err("writer failure");

    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert!(previous.is_empty());
}
