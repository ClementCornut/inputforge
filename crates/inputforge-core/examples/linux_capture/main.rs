mod args;

use args::{Command, USAGE, parse};
use std::{env, io::Write, process::ExitCode};

fn main() -> ExitCode {
    let arguments: Vec<_> = env::args().skip(1).collect();
    let command = match parse(&arguments) {
        Ok(command) => command,
        Err(error) => {
            write_error(&error);
            return ExitCode::from(2);
        }
    };
    if matches!(command, Command::Help) {
        return match writeln!(std::io::stdout().lock(), "{USAGE}") {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                write_error(&error);
                ExitCode::FAILURE
            }
        };
    }
    match run(command, std::io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            write_error(&format_args!("linux capture failed: {error}"));
            ExitCode::FAILURE
        }
    }
}

fn write_error(error: &impl std::fmt::Display) {
    let _ignored = writeln!(std::io::stderr().lock(), "{error}");
}

#[cfg(target_os = "linux")]
fn run(command: Command, mut writer: impl Write) -> Result<(), Box<dyn std::error::Error>> {
    use inputforge_core::{device::evdev::Capture, types::DeviceId};
    use std::time::Duration;

    let mut capture = Capture::new()?;
    match command {
        Command::Watch => watch(&mut capture, &mut writer),
        Command::Capture { seconds, devices } => {
            let devices: Vec<_> = devices.into_iter().map(DeviceId).collect();
            write_request(&devices, &mut writer)?;
            capture.acquire(devices)?;
            let result = capture_for(&mut capture, Duration::from_secs(seconds));
            let release = capture.release();
            result?;
            release?;
            let mut previous = String::new();
            write_changes(&capture, &mut writer, &mut previous)?;
            Ok(())
        }
        Command::Help => Ok(()),
    }
}

#[cfg(target_os = "linux")]
fn watch(
    capture: &mut inputforge_core::device::evdev::Capture,
    writer: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::{thread, time::Duration};

    let mut previous = String::new();
    write_changes(capture, writer, &mut previous)?;
    loop {
        thread::sleep(Duration::from_millis(100));
        let poll = capture.poll();
        let output = write_changes(capture, writer, &mut previous);
        poll?;
        output?;
    }
}

#[cfg(target_os = "linux")]
fn capture_for(
    capture: &mut inputforge_core::device::evdev::Capture,
    duration: std::time::Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::{thread, time::Instant};

    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        thread::sleep(std::time::Duration::from_millis(10));
        capture.poll()?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn write_request(
    devices: &[inputforge_core::types::DeviceId],
    writer: &mut impl Write,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "requesting capture: {:?}",
        devices.iter().map(|id| id.0.as_str()).collect::<Vec<_>>()
    )?;
    writer.flush()
}

#[cfg(target_os = "linux")]
fn write_changes(
    capture: &inputforge_core::device::evdev::Capture,
    writer: &mut impl Write,
    previous: &mut String,
) -> std::io::Result<()> {
    write_state(capture.devices(), capture.captured(), writer, previous)
}

#[cfg(target_os = "linux")]
fn write_state(
    devices: &[inputforge_core::device::evdev::Device],
    captured: &[inputforge_core::types::DeviceId],
    writer: &mut impl Write,
    previous: &mut String,
) -> std::io::Result<()> {
    let current = render_state(devices, captured);
    if *previous != current {
        writer.write_all(current.as_bytes())?;
        writer.flush()?;
        *previous = current;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[expect(
    clippy::unnecessary_debug_formatting,
    reason = "Escape device-controlled names and paths in terminal output"
)]
fn render_state(
    devices: &[inputforge_core::device::evdev::Device],
    captured: &[inputforge_core::types::DeviceId],
) -> String {
    let mut lines = vec![format!("inventory: {}", devices.len())];
    for device in devices {
        lines.push(format!(
            "  id={:?} identity={:?} class={:?} access={:?} name={:?} node={:?}",
            device.identity.id.as_ref().map(|id| &id.0),
            device.identity.quality,
            device.classification.kind,
            device.access,
            device.metadata.name,
            device.metadata.node
        ));
        lines.extend(
            device
                .classification
                .reasons
                .iter()
                .map(|reason| format!("    reason={reason:?}")),
        );
        lines.extend(device.issues.iter().map(|issue| {
            format!(
                "    issue operation={:?} path={:?} kind={:?} errno={:?} message={:?}",
                issue.operation, issue.path, issue.kind, issue.raw_os_error, issue.message
            )
        }));
    }
    lines.push(format!(
        "captured: {:?}",
        captured.iter().map(|id| id.0.as_str()).collect::<Vec<_>>()
    ));
    let mut output = lines.join("\n");
    output.push('\n');
    output
}

#[cfg(not(target_os = "linux"))]
fn run(_command: Command, _writer: impl Write) -> Result<(), Box<dyn std::error::Error>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "the linux-capture example requires Linux evdev hardware support",
    )
    .into())
}

#[cfg(all(test, target_os = "linux"))]
#[path = "tests.rs"]
mod tests;
