use super::{Access, Class, Device};
use crate::{
    error::Result,
    profile::controllers::{AxisBinding, DeviceBinding, invalid},
    types::AxisPolarity,
};

pub(super) fn from_device(device: &Device) -> Result<DeviceBinding> {
    if device.classification.kind != Class::Controller || device.access != Access::Readable {
        return Err(invalid(format!(
            "{}: controller capabilities unavailable; restore access and Refresh",
            device.metadata.node.display()
        )));
    }
    let id = device
        .identity
        .id
        .clone()
        .ok_or_else(|| invalid("ambiguous controller identity"))?;
    let mut axes = Vec::new();
    let mut hats = Vec::new();
    for code in &device.metadata.abs_axes {
        let info = device
            .axes
            .iter()
            .find(|a| a.code == *code)
            .ok_or_else(|| invalid(format!("{}: missing ABS {code} metadata", id.0)))?;
        if (16..=23).contains(code) {
            let pair = (*code - 16) / 2;
            let x = 16 + pair * 2;
            if !device.metadata.abs_axes.contains(&x)
                || !device.metadata.abs_axes.contains(&(x + 1))
                || info.minimum != -1
                || info.maximum != 1
            {
                return Err(invalid(format!(
                    "{}: incomplete or invalid native hat {pair}",
                    id.0
                )));
            }
            if *code == x {
                hats.push(u8::try_from(pair).expect("four hat pairs"));
            }
        } else {
            axes.push(AxisBinding {
                code: *code,
                minimum: info.minimum,
                maximum: info.maximum,
                polarity: AxisPolarity::Bipolar,
            });
        }
    }
    let binding = DeviceBinding {
        observed_layout: None,
        device: id,
        axes,
        buttons: device.metadata.keys.iter().copied().collect(),
        hats,
        unavailable: Vec::new(),
    };
    binding.validate()?;
    Ok(binding)
}
