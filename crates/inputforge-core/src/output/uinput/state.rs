use super::config;
use crate::types::{HatDirection, VJoyAxis, VirtualDeviceConfig};
use evdev::InputEvent;
use std::{collections::BTreeMap, io};

#[derive(Debug)]
pub(super) struct State {
    desired: BTreeMap<(u16, u16), i32>,
    sent: BTreeMap<(u16, u16), i32>,
}

impl State {
    pub(super) fn new(config: &VirtualDeviceConfig) -> Self {
        let desired: BTreeMap<_, _> = config::axes(config)
            .into_iter()
            .map(|(code, _, _)| ((3, code), 0))
            .chain(
                (1..=config.button_count)
                    .map(|id| ((1, config::button_code(id).expect("validated button")), 0)),
            )
            .collect();
        Self {
            sent: desired.clone(),
            desired,
        }
    }

    fn set(&mut self, key: (u16, u16), value: i32) -> io::Result<()> {
        let entry = self.desired.get_mut(&key).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "undeclared output address")
        })?;
        *entry = value;
        Ok(())
    }

    pub(super) fn axis(&mut self, axis: VJoyAxis, value: f64) -> io::Result<()> {
        let value = if value.is_finite() {
            value.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "finite rounded value is within -32767..=32767"
        )]
        let raw = (value * 32767.0).round() as i32;
        self.set((3, config::axis_code(axis)), raw)
    }

    pub(super) fn button(&mut self, button: u8, pressed: bool) -> io::Result<()> {
        let code = config::button_code(button)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "button must be 1..=53"))?;
        self.set((1, code), i32::from(pressed))
    }

    pub(super) fn hat(&mut self, hat: u8, direction: HatDirection) -> io::Result<()> {
        use HatDirection::{Center, E, N, NE, NW, S, SE, SW, W};
        if !(1..=4).contains(&hat) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "hat must be 1..=4",
            ));
        }
        let code = 0x10 + u16::from(hat - 1) * 2;
        let (x, y) = match direction {
            Center => (0, 0),
            N => (0, -1),
            NE => (1, -1),
            E => (1, 0),
            SE => (1, 1),
            S => (0, 1),
            SW => (-1, 1),
            W => (-1, 0),
            NW => (-1, -1),
        };
        self.set((3, code), x)?;
        self.set((3, code + 1), y)
    }

    pub(super) fn neutral(&mut self) {
        self.desired.values_mut().for_each(|v| *v = 0);
    }
    pub(super) fn commit(&mut self) {
        self.sent.clone_from(&self.desired);
    }

    pub(super) fn packets(&self, force: bool) -> Vec<Vec<InputEvent>> {
        let changed = |key: &(u16, u16)| force || self.desired.get(key) != self.sent.get(key);
        let mut absolute = Vec::new();
        let mut buttons = Vec::new();
        for (&(kind, code), &value) in &self.desired {
            let include = if kind == 3 && (0x10..=0x17).contains(&code) {
                let x = code & !1;
                changed(&(3, x)) || changed(&(3, x + 1))
            } else {
                changed(&(kind, code))
            };
            if include {
                let target = if kind == 3 {
                    &mut absolute
                } else {
                    &mut buttons
                };
                target.push(InputEvent::new(kind, code, value));
            }
        }
        let mut packets = Vec::new();
        // input_estimate_events_per_packet reserves all ABS + SYN + seven KEY/MSC records.
        // Avoid kernel-inserted report boundaries; all hat components stay in the first packet.
        for chunk in buttons.chunks(7) {
            let mut packet = std::mem::take(&mut absolute);
            packet.extend_from_slice(chunk);
            packet.push(InputEvent::new(0, 0, 0));
            packets.push(packet);
        }
        if !absolute.is_empty() {
            absolute.push(InputEvent::new(0, 0, 0));
            packets.push(absolute);
        }
        packets
    }
}
