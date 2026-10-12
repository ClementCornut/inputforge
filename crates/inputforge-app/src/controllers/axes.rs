//! Global physical axis settings, independent of profile and virtual-layout editing locks.
use anyhow::{Result, bail};
use inputforge_core::{
    engine::EngineCommand,
    profile::controllers::DeviceBinding,
    state::AppState,
    types::{AxisPolarity, DeviceId, InputId},
};
use inputforge_gui_dx::controllers::AxisRow;

fn tables(state: &AppState) -> impl Iterator<Item = &DeviceBinding> {
    state.session.bindings.iter().chain(
        state
            .active_profile
            .iter()
            .filter_map(|profile| profile.controllers())
            .flat_map(|config| &config.bindings)
            .filter(|saved| {
                !state
                    .session
                    .bindings
                    .iter()
                    .any(|live| live.device == saved.device)
            }),
    )
}

pub(super) fn rows(state: &AppState) -> Vec<AxisRow> {
    tables(state)
        .filter(|table| table.validate().is_ok())
        .flat_map(|table| {
            table.axes.iter().enumerate().filter_map(|(index, axis)| {
                let index = u8::try_from(index).ok()?;
                let setting = state.device_registry.get(&table.device).and_then(|record| {
                    record
                        .axis_settings
                        .iter()
                        .find(|setting| setting.code == axis.code)
                });
                Some(AxisRow {
                    device: table.device.clone(),
                    index,
                    code: axis.code,
                    override_polarity: setting.and_then(|setting| setting.override_polarity),
                    detected: setting.and_then(|setting| setting.detected),
                    can_override: !state.session.offline,
                    can_detect: !state.session.offline
                        && state.session.monitored.contains(&table.device)
                        && table.resolves(&InputId::Axis { index }),
                })
            })
        })
        .collect()
}

pub(super) fn resolve(
    state: &AppState,
    device: DeviceId,
    axis: u8,
    code: u16,
    value: Option<&str>,
) -> Result<Option<EngineCommand>> {
    if state.session.offline {
        return Ok(None);
    }
    let Some(table) = tables(state).find(|table| table.device == device) else {
        return Ok(None);
    };
    table.validate()?;
    if table
        .axes
        .get(usize::from(axis))
        .is_none_or(|current| current.code != code)
    {
        return Ok(None);
    }
    if let Some(value) = value {
        let polarity = match value {
            "auto" => None,
            "centered" => Some(AxisPolarity::Bipolar),
            "minimum" => Some(AxisPolarity::Unipolar),
            _ => bail!("Invalid axis interpretation: {value}"),
        };
        return Ok(Some(EngineCommand::SetAxisPolarity {
            device,
            axis,
            polarity,
        }));
    }
    Ok((state.session.monitored.contains(&device)
        && table.resolves(&InputId::Axis { index: axis }))
    .then_some(EngineCommand::DetectAxis { device, axis }))
}
