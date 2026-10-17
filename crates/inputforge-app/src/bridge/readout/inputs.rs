//! Prepare typed values and rail intensity from one coherent physical sample.
use inputforge_core::{
    processing::into_natural_domain,
    types::{HatDirection, InputAddress, InputId},
};
use inputforge_gui_dx::{
    readouts::{AxisDisplay, InputReadout, ReadoutDisplay},
    snapshots::{ConfigSnapshot, LiveSnapshot},
};
pub(super) fn project(cfg: &ConfigSnapshot, live: &mut LiveSnapshot) {
    for (device, values) in cfg.devices.iter().zip(&live.device_inputs) {
        let axes = values
            .axes
            .iter()
            .enumerate()
            .map(|(index, &(value, polarity))| {
                (
                    InputId::Axis {
                        index: u8::try_from(index).expect("sampled axes fit device count"),
                    },
                    ReadoutDisplay::Axis(AxisDisplay {
                        value: into_natural_domain(value, polarity),
                        polarity,
                    }),
                )
            });
        let buttons = values.buttons.iter().enumerate().map(|(index, &pressed)| {
            (
                InputId::Button {
                    index: u8::try_from(index).expect("sampled buttons fit device count"),
                },
                ReadoutDisplay::Button { pressed },
            )
        });
        let hats = values.hats.iter().enumerate().map(|(index, &direction)| {
            (
                InputId::Hat {
                    index: u8::try_from(index).expect("sampled hats fit device count"),
                },
                ReadoutDisplay::Hat { direction },
            )
        });
        for (input, display) in axes.chain(buttons).chain(hats) {
            let intensity = match display {
                ReadoutDisplay::Axis(value) => value.value.abs().clamp(0.0, 1.0),
                ReadoutDisplay::Button { pressed } => f64::from(pressed),
                ReadoutDisplay::Hat { direction } => f64::from(direction != HatDirection::Center),
            };
            live.input_readouts.insert(
                InputAddress::Bound {
                    device: device.info.id.clone(),
                    input,
                },
                InputReadout { display, intensity },
            );
        }
    }
}
pub(super) fn missing(address: &InputAddress) -> InputReadout {
    let display = match address.input_id() {
        Some(InputId::Button { .. }) => ReadoutDisplay::Button { pressed: false },
        Some(InputId::Hat { .. }) => ReadoutDisplay::Hat {
            direction: HatDirection::Center,
        },
        _ => ReadoutDisplay::default(),
    };
    InputReadout {
        display,
        intensity: 0.0,
    }
}
