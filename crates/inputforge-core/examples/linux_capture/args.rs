pub(super) const USAGE: &str = "Usage:\n  linux-capture watch\n  linux-capture capture --seconds <1..60> --device <evdev:v1:ID> [--device <evdev:v1:ID> ...]\n  linux-capture stream --seconds <1..60> --device <evdev:v1:ID> [--device <evdev:v1:ID> ...]";

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Command {
    Help,
    Watch,
    Capture { seconds: u64, devices: Vec<String> },
    Stream { seconds: u64, devices: Vec<String> },
}

pub(super) fn parse(arguments: &[String]) -> Result<Command, String> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Ok(Command::Help);
    };
    match command {
        "-h" | "--help" if arguments.len() == 1 => Ok(Command::Help),
        "watch" if arguments.len() == 1 => Ok(Command::Watch),
        "capture" | "stream" => parse_capture(&arguments[1..], command == "stream"),
        _ => Err(format!("invalid arguments\n{USAGE}")),
    }
}

fn parse_capture(arguments: &[String], streaming: bool) -> Result<Command, String> {
    let mut seconds = None;
    let mut devices = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        let option = arguments[index].as_str();
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {option}\n{USAGE}"))?;
        match option {
            "--seconds" if seconds.is_none() => {
                let value = value.parse::<u64>().map_err(|error| {
                    format!("seconds must be an integer from 1 to 60: {error}\n{USAGE}")
                })?;
                if !(1..=60).contains(&value) {
                    return Err(format!("seconds must be from 1 to 60\n{USAGE}"));
                }
                seconds = Some(value);
            }
            "--device" if valid_device_id(value) => devices.push(value.clone()),
            "--device" => {
                return Err(format!(
                    "device ID must use the evdev:v1: namespace\n{USAGE}"
                ));
            }
            _ => return Err(format!("invalid option {option}\n{USAGE}")),
        }
        index += 2;
    }
    let seconds = seconds.ok_or_else(|| format!("--seconds is required\n{USAGE}"))?;
    if devices.is_empty() {
        return Err(format!("at least one --device is required\n{USAGE}"));
    }
    if streaming {
        Ok(Command::Stream { seconds, devices })
    } else {
        Ok(Command::Capture { seconds, devices })
    }
}

fn valid_device_id(value: &str) -> bool {
    value
        .strip_prefix("evdev:v1:")
        .is_some_and(|suffix| !suffix.is_empty())
}
