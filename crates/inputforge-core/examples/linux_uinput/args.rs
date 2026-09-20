use inputforge_core::types::VJoyAxis;

pub(super) const USAGE: &str = "Usage:\n  linux-uinput <neutral|exercise|hold> --slot <1..16> [--slot <1..16> ...] --seconds <1..60> [--buttons <2..53>] [--hats <0..4>] [--axes <X,Y,Z,Rx,Ry,Rz,Slider0,Slider1|none>]";

const ALL_AXES: [VJoyAxis; 8] = [
    VJoyAxis::X,
    VJoyAxis::Y,
    VJoyAxis::Z,
    VJoyAxis::Rx,
    VJoyAxis::Ry,
    VJoyAxis::Rz,
    VJoyAxis::Slider0,
    VJoyAxis::Slider1,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scenario {
    Neutral,
    Exercise,
    Hold,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Command {
    Help,
    Run {
        scenario: Scenario,
        slots: Vec<u8>,
        seconds: u64,
        buttons: u8,
        hats: u8,
        axes: Vec<VJoyAxis>,
    },
}

pub(super) fn parse(arguments: &[String]) -> Result<Command, String> {
    let Some(name) = arguments.first().map(String::as_str) else {
        return Ok(Command::Help);
    };
    if matches!(name, "-h" | "--help") && arguments.len() == 1 {
        return Ok(Command::Help);
    }
    let scenario = match name {
        "neutral" => Scenario::Neutral,
        "exercise" => Scenario::Exercise,
        "hold" => Scenario::Hold,
        _ => return Err(invalid("expected neutral, exercise, or hold")),
    };
    parse_run(scenario, &arguments[1..])
}

fn parse_run(scenario: Scenario, arguments: &[String]) -> Result<Command, String> {
    let mut slots = Vec::new();
    let mut seconds = None;
    let mut buttons = None;
    let mut hats = None;
    let mut axes = None;
    let mut index = 0;
    while index < arguments.len() {
        let option = arguments[index].as_str();
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| invalid(&format!("missing value for {option}")))?;
        match option {
            "--slot" => {
                let slot = ranged_u8("slot", value, 1, 16)?;
                if slots.contains(&slot) {
                    return Err(invalid("each --slot must be unique"));
                }
                slots.push(slot);
            }
            "--seconds" if seconds.is_none() => {
                seconds = Some(u64::from(ranged_u8("seconds", value, 1, 60)?));
            }
            "--buttons" if buttons.is_none() => {
                buttons = Some(ranged_u8("buttons", value, 2, 53)?);
            }
            "--hats" if hats.is_none() => hats = Some(ranged_u8("hats", value, 0, 4)?),
            "--axes" if axes.is_none() => axes = Some(parse_axes(value)?),
            _ => return Err(invalid(&format!("invalid or duplicate option {option}"))),
        }
        index += 2;
    }
    if slots.is_empty() {
        return Err(invalid("at least one --slot is required"));
    }
    Ok(Command::Run {
        scenario,
        slots,
        seconds: seconds.ok_or_else(|| invalid("--seconds is required"))?,
        buttons: buttons.unwrap_or(32),
        hats: hats.unwrap_or(4),
        axes: axes.unwrap_or_else(|| ALL_AXES.to_vec()),
    })
}

fn parse_axes(value: &str) -> Result<Vec<VJoyAxis>, String> {
    if value == "none" {
        return Ok(Vec::new());
    }
    let mut selected = Vec::new();
    for axis_name in value.split(',') {
        let parsed = match axis_name {
            "X" => VJoyAxis::X,
            "Y" => VJoyAxis::Y,
            "Z" => VJoyAxis::Z,
            "Rx" => VJoyAxis::Rx,
            "Ry" => VJoyAxis::Ry,
            "Rz" => VJoyAxis::Rz,
            "Slider0" => VJoyAxis::Slider0,
            "Slider1" => VJoyAxis::Slider1,
            _ => return Err(invalid(&format!("unknown axis {axis_name}"))),
        };
        if selected.contains(&parsed) {
            return Err(invalid(&format!("duplicate axis {axis_name}")));
        }
        selected.push(parsed);
    }
    Ok(selected)
}

fn ranged_u8(name: &str, value: &str, min: u8, max: u8) -> Result<u8, String> {
    let value = value
        .parse::<u8>()
        .map_err(|error| invalid(&format!("{name} must be an integer: {error}")))?;
    if !(min..=max).contains(&value) {
        return Err(invalid(&format!("{name} must be from {min} to {max}")));
    }
    Ok(value)
}

fn invalid(message: &str) -> String {
    format!("{message}\n{USAGE}")
}
