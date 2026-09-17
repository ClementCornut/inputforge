#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const UNAVAILABLE_MESSAGE: &str = "Linux input and output backends are unavailable in Slice 1; evdev and uinput arrive in later slices.";

// This bounds regressions that accidentally reach a blocking GUI or tray path.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(5);

static NEXT_TEMP_HOME: AtomicU64 = AtomicU64::new(0);

struct TempHome {
    path: PathBuf,
}

impl TempHome {
    fn new() -> Self {
        let sequence = NEXT_TEMP_HOME.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "inputforge-linux-startup-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("temporary XDG root must be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        match std::fs::remove_dir_all(&self.path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("temporary XDG root must be removed: {error}"),
        }
    }
}

fn run_inputforge(arguments: &[&str]) -> Output {
    let isolated_home = TempHome::new();

    let mut child = Command::new(env!("CARGO_BIN_EXE_inputforge"))
        .args(arguments)
        .env("XDG_CONFIG_HOME", isolated_home.path().join("config"))
        .env("XDG_DATA_HOME", isolated_home.path().join("data"))
        .env("XDG_CACHE_HOME", isolated_home.path().join("cache"))
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("RUST_LOG")
        .env_remove("RUST_BACKTRACE")
        .env_remove("RUST_LIB_BACKTRACE")
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("LD_PRELOAD")
        .env_remove("LD_AUDIT")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("inputforge process must start");

    let deadline = Instant::now() + PROCESS_TIMEOUT;

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child
                    .wait_with_output()
                    .expect("completed process output must be readable");
                assert!(
                    isolated_home
                        .path()
                        .read_dir()
                        .expect("temporary XDG root must be readable")
                        .next()
                        .is_none(),
                    "startup must not create config, data, or cache files"
                );
                return output;
            }
            Ok(None) => {}
            Err(error) => panic!("failed to poll inputforge process: {error}"),
        }

        if Instant::now() >= deadline {
            child.kill().expect("timed-out process must be terminated");
            let output = child
                .wait_with_output()
                .expect("timed-out process output must be readable");
            panic!(
                "inputforge exceeded five seconds\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn no_arguments_fail_before_ui_with_stable_message() {
    let output = run_inputforge(&[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout must be UTF-8"),
        ""
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr must be UTF-8"),
        format!("Error: {UNAVAILABLE_MESSAGE}\n")
    );
}

#[test]
fn help_succeeds_before_preflight() {
    let output = run_inputforge(&["--help"]);
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("stderr must be UTF-8");

    assert!(stdout.contains("Usage: inputforge [OPTIONS]"));
    assert!(stdout.contains("virtual devices"));
    assert!(!stdout.contains("--diagnose-linux"));
    assert!(!stdout.contains("virtual vJoy devices"));
    assert!(!stdout.contains(UNAVAILABLE_MESSAGE));
    assert_eq!(stderr, "");
}

#[test]
fn version_succeeds_before_preflight() {
    let output = run_inputforge(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout must be UTF-8"),
        format!("inputforge {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr must be UTF-8"),
        ""
    );
}

#[test]
fn diagnostics_flag_belongs_to_the_standalone_tool() {
    let output = run_inputforge(&["--diagnose-linux"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument"));
}
