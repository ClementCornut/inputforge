use std::process::{Command, Output};

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_inputforge-diagnostics"))
        .args(arguments)
        .output()
        .expect("diagnostic command must start")
}

#[test]
fn help_and_version_do_not_require_device_access() {
    let help = run(&["--help"]);
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    let text = String::from_utf8(help.stdout).expect("UTF-8 help");
    assert!(text.contains("Usage: inputforge-diagnostics"));
    assert!(text.contains("read-only Linux input"));
    let version = run(&["--version"]);
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).expect("UTF-8 version"),
        format!("inputforge-diagnostics {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn application_arguments_are_rejected_before_discovery() {
    for argument in [
        "--profile",
        "--enable",
        "--start-minimized",
        "--diagnose-linux",
    ] {
        let output = run(&[argument]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument"));
    }
}

#[cfg(not(target_os = "linux"))]
#[test]
fn unsupported_platform_exits_with_an_explicit_error() {
    let output = run(&[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("only available on Linux"));
}
