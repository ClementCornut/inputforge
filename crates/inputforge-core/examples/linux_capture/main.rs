mod args;
#[cfg(target_os = "linux")]
mod report;
#[cfg(target_os = "linux")]
mod stream;

#[cfg(target_os = "linux")]
use report::write_changes;

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
        Command::Stream { seconds, devices } => {
            let devices: Vec<_> = devices.into_iter().map(DeviceId).collect();
            write_request(&devices, &mut writer)?;
            capture.acquire(devices)?;
            stream::run(&mut capture, Duration::from_secs(seconds), &mut writer)
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
