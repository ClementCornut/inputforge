use super::{AxisInfo, NativeControl, NativeState, SnapshotKind, StreamUpdate};
use crate::{
    device::InputUpdate,
    error::Result,
    profile::controllers::{DeviceBinding, invalid},
    types::{AxisValue, DeviceId, InputAddress, InputEvent, InputId, InputValue},
};
use std::{collections::HashMap, time::Instant};

pub(super) fn translate(
    tables: &[DeviceBinding],
    states: &mut HashMap<DeviceId, NativeState>,
    update: StreamUpdate,
) -> Result<InputUpdate> {
    let device = match &update {
        StreamUpdate::Frame { device, .. }
        | StreamUpdate::Snapshot { device, .. }
        | StreamUpdate::Reset { device } => device,
    };
    let table = tables
        .iter()
        .find(|t| &t.device == device)
        .ok_or_else(|| invalid(format!("{}: no frozen control table", device.0)))?;
    match update {
        StreamUpdate::Reset { device } => {
            states.remove(&device);
            Ok(InputUpdate::Reset { device })
        }
        StreamUpdate::Snapshot {
            device,
            state,
            kind,
            timestamp,
        } => {
            let values = snapshot_values(table, &state, timestamp)?;
            states.insert(device.clone(), state);
            Ok(InputUpdate::Snapshot {
                device,
                values,
                recovered: kind == SnapshotKind::Recovered,
            })
        }
        StreamUpdate::Frame {
            device,
            changes,
            timestamp,
        } => {
            let state = states
                .get_mut(&device)
                .ok_or_else(|| invalid("frame received before snapshot"))?;
            let mut events = Vec::new();
            let mut changed_hats = Vec::new();
            for change in changes {
                match change.control {
                    NativeControl::Abs(code) if (16..=23).contains(&code) => {
                        let pair = u8::try_from((code - 16) / 2).expect("four hat pairs");
                        if !table.hats.contains(&pair) || !(-1..=1).contains(&change.value) {
                            return Err(invalid("invalid native hat value/control"));
                        }
                        state.axes.insert(code, change.value);
                        if !changed_hats.contains(&pair) {
                            changed_hats.push(pair);
                        }
                    }
                    control => {
                        events.push(event(table, control, change.value, timestamp)?);
                        match control {
                            NativeControl::Key(code) => {
                                state.keys.insert(code, change.value != 0);
                            }
                            NativeControl::Abs(code) => {
                                state.axes.insert(code, change.value);
                            }
                        }
                    }
                }
            }
            append_hats(table, state, &changed_hats, timestamp, &mut events)?;
            Ok(InputUpdate::Frame(events))
        }
    }
}

fn snapshot_values(
    table: &DeviceBinding,
    state: &NativeState,
    timestamp: Instant,
) -> Result<Vec<InputEvent>> {
    let mut values = Vec::new();
    for (index, code) in table.buttons.iter().enumerate() {
        if !table.resolves(&InputId::Button {
            index: u8::try_from(index).expect("validated button count"),
        }) {
            continue;
        }
        let pressed = state
            .keys
            .get(code)
            .ok_or_else(|| invalid("incomplete key snapshot"))?;
        values.push(event(
            table,
            NativeControl::Key(*code),
            i32::from(*pressed),
            timestamp,
        )?);
    }
    for (index, axis) in table.axes.iter().enumerate() {
        if !table.resolves(&InputId::Axis {
            index: u8::try_from(index).expect("validated axis count"),
        }) {
            continue;
        }
        let value = state
            .axes
            .get(&axis.code)
            .ok_or_else(|| invalid("incomplete axis snapshot"))?;
        values.push(event(
            table,
            NativeControl::Abs(axis.code),
            *value,
            timestamp,
        )?);
    }
    append_hats(table, state, &table.hats, timestamp, &mut values)?;
    Ok(values)
}

fn event(
    table: &DeviceBinding,
    control: NativeControl,
    raw: i32,
    timestamp: Instant,
) -> Result<InputEvent> {
    let (input, value) = match control {
        NativeControl::Key(code) => {
            let index = table
                .buttons
                .iter()
                .position(|c| *c == code)
                .ok_or_else(|| invalid(format!("unknown KEY {code}")))?;
            if !(0..=1).contains(&raw) {
                return Err(invalid("invalid key value"));
            }
            (
                InputId::Button {
                    index: u8::try_from(index)
                        .map_err(|error| invalid(format!("button overflow: {error}")))?,
                },
                InputValue::Button { pressed: raw != 0 },
            )
        }
        NativeControl::Abs(code) => {
            let index = table
                .axes
                .iter()
                .position(|a| a.code == code)
                .ok_or_else(|| invalid(format!("unknown ABS {code}")))?;
            let axis = &table.axes[index];
            let metadata = AxisInfo {
                code,
                minimum: axis.minimum,
                maximum: axis.maximum,
                fuzz: 0,
                flat: 0,
                resolution: 0,
            };
            let value = metadata
                .normalize(raw)
                .ok_or_else(|| invalid("invalid normalization range"))?;
            (
                InputId::Axis {
                    index: u8::try_from(index)
                        .map_err(|error| invalid(format!("axis overflow: {error}")))?,
                },
                InputValue::Axis {
                    value: AxisValue::raw(value),
                    polarity: axis.polarity,
                },
            )
        }
    };
    Ok(InputEvent {
        source: InputAddress::Bound {
            device: table.device.clone(),
            input,
        },
        value,
        timestamp,
    })
}

fn append_hats(
    table: &DeviceBinding,
    state: &NativeState,
    hats: &[u8],
    timestamp: Instant,
    out: &mut Vec<InputEvent>,
) -> Result<()> {
    for hat in hats {
        let index = table
            .hats
            .iter()
            .position(|h| h == hat)
            .ok_or_else(|| invalid("unknown hat"))?;
        if !table.resolves(&InputId::Hat {
            index: u8::try_from(index).expect("four hats"),
        }) {
            continue;
        }
        let direction = state
            .hat(*hat)
            .direction
            .ok_or_else(|| invalid("incomplete or invalid hat snapshot"))?;
        out.push(InputEvent {
            source: InputAddress::Bound {
                device: table.device.clone(),
                input: InputId::Hat {
                    index: u8::try_from(index).expect("four hats"),
                },
            },
            value: InputValue::Hat { direction },
            timestamp,
        });
    }
    Ok(())
}
