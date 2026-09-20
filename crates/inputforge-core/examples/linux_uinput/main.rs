mod args;
#[cfg(target_os = "linux")]
mod run;

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
    match run_command(command, std::io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            write_error(&error);
            ExitCode::FAILURE
        }
    }
}

fn write_error(error: &impl std::fmt::Display) {
    let _ignored = writeln!(std::io::stderr().lock(), "{error}");
}

#[cfg(target_os = "linux")]
fn run_command(command: Command, mut writer: impl Write) -> Result<(), String> {
    use inputforge_core::{
        output::uinput::{Output, default_config},
        types::VirtualDeviceConfig,
    };
    use run::execute;
    use std::time::Duration;

    let Command::Run {
        scenario,
        slots,
        seconds,
        buttons,
        hats,
        axes,
    } = command
    else {
        return Ok(());
    };
    let configs: Vec<VirtualDeviceConfig> = slots
        .iter()
        .map(|&slot| {
            let mut config = default_config(slot);
            config.axes.clone_from(&axes);
            config.button_count = buttons;
            config.hat_count = hats;
            config
        })
        .collect();
    let mut output = Output::new(configs).map_err(|error| error.to_string())?;
    writeln!(
        writer,
        "requesting Linux uinput {scenario:?}: slots={slots:?} seconds={seconds} axes={axes:?} buttons={buttons} hats={hats}",
    )
    .and_then(|()| writer.flush())
    .map_err(|error| error.to_string())?;

    let outcome = execute(
        &mut output,
        scenario,
        &slots,
        &axes,
        buttons,
        hats,
        Duration::from_secs(seconds),
    );
    let report = writeln!(
        writer,
        "Linux uinput ownership ended: slots={slots:?}; reset and release attempted"
    )
    .and_then(|()| writer.flush())
    .err();
    combine_errors(outcome, report)
}

#[cfg(target_os = "linux")]
fn combine_errors<E: std::fmt::Display>(
    outcome: run::Outcome<E>,
    report: Option<std::io::Error>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    if let Some(error) = outcome.primary {
        errors.push(format!("linux uinput failed: {error}"));
    }
    if let Some(error) = outcome.reset {
        errors.push(format!("linux uinput reset failed: {error}"));
    }
    if let Some(error) = outcome.release {
        errors.push(format!("linux uinput release failed: {error}"));
    }
    if let Some(error) = report {
        errors.push(format!("post-release report failed: {error}"));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[cfg(not(target_os = "linux"))]
fn run_command(_command: Command, _writer: impl Write) -> Result<(), String> {
    Err("the linux-uinput example requires Linux uinput support".to_owned())
}

#[cfg(all(test, target_os = "linux"))]
#[path = "tests.rs"]
mod tests;
