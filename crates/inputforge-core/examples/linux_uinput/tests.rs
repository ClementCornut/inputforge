use crate::args::{Command, Scenario, parse};
use crate::{
    combine_errors,
    run::{DeviceOwner, Outcome, execute},
};
use inputforge_core::types::VJoyAxis;
use std::time::Duration;

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn no_arguments_and_help_never_request_hardware() {
    assert_eq!(parse(&strings(&[])).expect("no arguments"), Command::Help);
    assert_eq!(
        parse(&strings(&["--help"])).expect("help argument"),
        Command::Help
    );
}

#[test]
fn run_requires_explicit_slots_and_bounded_duration() {
    for arguments in [
        vec!["neutral", "--seconds", "1"],
        vec!["neutral", "--slot", "1"],
        vec!["neutral", "--slot", "0", "--seconds", "1"],
        vec!["neutral", "--slot", "1", "--seconds", "0"],
        vec!["neutral", "--slot", "1", "--seconds", "61"],
    ] {
        assert!(
            parse(&strings(&arguments)).is_err(),
            "arguments={arguments:?}"
        );
    }
}

#[test]
fn repeated_slots_and_capabilities_are_parsed() {
    assert_eq!(
        parse(&strings(&[
            "exercise",
            "--slot",
            "2",
            "--slot",
            "5",
            "--seconds",
            "3",
            "--buttons",
            "53",
            "--hats",
            "0",
            "--axes",
            "X,Y,Rz",
        ]))
        .expect("valid exercise"),
        Command::Run {
            scenario: Scenario::Exercise,
            slots: vec![2, 5],
            seconds: 3,
            buttons: 53,
            hats: 0,
            axes: vec![VJoyAxis::X, VJoyAxis::Y, VJoyAxis::Rz],
        }
    );
}

#[test]
fn axes_may_be_explicitly_empty() {
    let command = parse(&strings(&[
        "hold",
        "--slot",
        "1",
        "--seconds",
        "1",
        "--axes",
        "none",
    ]))
    .expect("axes none");
    let Command::Run { axes, .. } = command else {
        panic!("expected run command");
    };
    assert!(axes.is_empty());
}

#[test]
fn optional_capabilities_use_acceptance_defaults() {
    let command = parse(&strings(&["neutral", "--slot", "1", "--seconds", "60"]))
        .expect("default capabilities");
    let Command::Run {
        scenario,
        buttons,
        hats,
        axes,
        ..
    } = command
    else {
        panic!("expected run command");
    };
    assert_eq!(scenario, Scenario::Neutral);
    assert_eq!(buttons, 32);
    assert_eq!(hats, 4);
    assert_eq!(axes.len(), 8);
}

#[test]
fn invalid_ranges_axes_duplicates_and_options_are_rejected() {
    for arguments in [
        vec!["neutral", "--slot", "1", "--seconds", "1", "--buttons", "1"],
        vec![
            "neutral",
            "--slot",
            "1",
            "--seconds",
            "1",
            "--buttons",
            "54",
        ],
        vec!["neutral", "--slot", "1", "--seconds", "1", "--hats", "5"],
        vec!["neutral", "--slot", "1", "--seconds", "1", "--axes", "X,X"],
        vec!["neutral", "--slot", "1", "--seconds", "1", "--axes", "x"],
        vec!["neutral", "--slot", "17", "--seconds", "1"],
        vec!["neutral", "--slot", "1", "--seconds", "1", "--axes", "RX"],
        vec!["neutral", "--slot", "1", "--seconds", "1", "--wat", "2"],
        vec!["neutral", "--slot", "1", "--slot", "1", "--seconds", "1"],
    ] {
        assert!(
            parse(&strings(&arguments)).is_err(),
            "arguments={arguments:?}"
        );
    }
}

#[test]
fn missing_values_and_duplicate_single_options_are_rejected() {
    for arguments in [
        vec!["neutral", "--slot"],
        vec!["neutral", "--slot", "1", "--seconds"],
        vec!["neutral", "--slot", "1", "--seconds", "1", "--seconds", "2"],
        vec![
            "neutral",
            "--slot",
            "1",
            "--seconds",
            "1",
            "--axes",
            "none",
            "--axes",
            "X",
        ],
    ] {
        assert!(
            parse(&strings(&arguments)).is_err(),
            "arguments={arguments:?}"
        );
    }
}

#[derive(Default)]
struct FakeOwner {
    events: Vec<&'static str>,
    fail_flush: bool,
    fail_reset: bool,
    fail_release: bool,
}

impl DeviceOwner for FakeOwner {
    type Error = &'static str;

    fn create(&mut self) -> Result<(), Self::Error> {
        self.events.push("create");
        Ok(())
    }

    fn set_axis(&mut self, _slot: u8, _axis: VJoyAxis, _value: f64) -> Result<(), Self::Error> {
        self.events.push("axis");
        Ok(())
    }

    fn set_button(&mut self, _slot: u8, _button: u8, _pressed: bool) -> Result<(), Self::Error> {
        self.events.push("button");
        Ok(())
    }

    fn set_hat(
        &mut self,
        _slot: u8,
        _hat: u8,
        _direction: inputforge_core::types::HatDirection,
    ) -> Result<(), Self::Error> {
        self.events.push("hat");
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.events.push("flush");
        if self.fail_flush {
            Err("flush")
        } else {
            Ok(())
        }
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        self.events.push("reset");
        if self.fail_reset {
            Err("reset")
        } else {
            Ok(())
        }
    }

    fn release(&mut self) -> Result<(), Self::Error> {
        self.events.push("release");
        if self.fail_release {
            Err("release")
        } else {
            Ok(())
        }
    }
}

#[test]
fn output_failure_resets_and_releases_before_returning() {
    let mut owner = FakeOwner {
        fail_flush: true,
        ..FakeOwner::default()
    };
    let outcome = execute(
        &mut owner,
        Scenario::Hold,
        &[1],
        &[VJoyAxis::X],
        2,
        1,
        // Flush fails immediately; allow ample time for scheduler delays before it.
        Duration::from_secs(10),
    );

    assert_eq!(outcome.primary, Some("flush"));
    assert_eq!(outcome.reset, None);
    assert_eq!(outcome.release, None);
    assert_eq!(owner.events.last(), Some(&"release"));
    assert!(owner.events.ends_with(&["reset", "release"]));
}

#[test]
fn primary_and_release_failures_are_both_preserved() {
    let mut owner = FakeOwner {
        fail_flush: true,
        fail_reset: true,
        fail_release: true,
        ..FakeOwner::default()
    };
    let outcome = execute(
        &mut owner,
        Scenario::Hold,
        &[1],
        &[VJoyAxis::X],
        2,
        1,
        // Flush fails immediately; allow ample time for scheduler delays before it.
        Duration::from_secs(10),
    );

    assert_eq!(outcome.primary, Some("flush"));
    assert_eq!(outcome.reset, Some("reset"));
    assert_eq!(outcome.release, Some("release"));
    assert_eq!(owner.events.last(), Some(&"release"));
}

#[test]
fn run_release_and_post_release_report_failures_are_all_preserved() {
    let error = combine_errors(
        Outcome {
            primary: Some("run"),
            reset: Some("reset"),
            release: Some("release"),
        },
        Some(std::io::ErrorKind::BrokenPipe.into()),
    )
    .expect_err("combined failure");

    assert!(error.contains("linux uinput failed: run"));
    assert!(error.contains("linux uinput reset failed: reset"));
    assert!(error.contains("linux uinput release failed: release"));
    assert!(error.contains("post-release report failed: broken pipe"));
}
