use crate::args::Scenario;
use inputforge_core::output::uinput::{Error, Output};
use inputforge_core::types::{HatDirection, VJoyAxis};
use std::{
    thread,
    time::{Duration, Instant},
};

pub(super) trait DeviceOwner {
    type Error;

    fn create(&mut self) -> Result<(), Self::Error>;
    fn set_axis(&mut self, slot: u8, axis: VJoyAxis, value: f64) -> Result<(), Self::Error>;
    fn set_button(&mut self, slot: u8, button: u8, pressed: bool) -> Result<(), Self::Error>;
    fn set_hat(&mut self, slot: u8, hat: u8, direction: HatDirection) -> Result<(), Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
    fn reset(&mut self) -> Result<(), Self::Error>;
    fn release(&mut self) -> Result<(), Self::Error>;
}

impl DeviceOwner for Output {
    type Error = Error;

    fn create(&mut self) -> Result<(), Self::Error> {
        Output::create(self)
    }

    fn set_axis(&mut self, slot: u8, axis: VJoyAxis, value: f64) -> Result<(), Self::Error> {
        Output::set_axis(self, slot, axis, value)
    }

    fn set_button(&mut self, slot: u8, button: u8, pressed: bool) -> Result<(), Self::Error> {
        Output::set_button(self, slot, button, pressed)
    }

    fn set_hat(&mut self, slot: u8, hat: u8, direction: HatDirection) -> Result<(), Self::Error> {
        Output::set_hat(self, slot, hat, direction)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Output::flush(self)
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        Output::reset(self)
    }

    fn release(&mut self) -> Result<(), Self::Error> {
        Output::release(self)
    }
}

#[derive(Debug)]
pub(super) struct Outcome<E> {
    pub(super) primary: Option<E>,
    pub(super) reset: Option<E>,
    pub(super) release: Option<E>,
}

pub(super) fn execute<O: DeviceOwner>(
    owner: &mut O,
    scenario: Scenario,
    slots: &[u8],
    axes: &[VJoyAxis],
    buttons: u8,
    hats: u8,
    duration: Duration,
) -> Outcome<O::Error> {
    let deadline = Instant::now() + duration;
    let mut primary = owner.create().err();
    let mut reset = None;
    if primary.is_none() {
        primary = drive(owner, scenario, slots, axes, buttons, hats, deadline).err();
        reset = owner.reset().err();
    }
    let release = owner.release().err();
    Outcome {
        primary,
        reset,
        release,
    }
}

fn drive<O: DeviceOwner>(
    owner: &mut O,
    scenario: Scenario,
    slots: &[u8],
    axes: &[VJoyAxis],
    buttons: u8,
    hats: u8,
    deadline: Instant,
) -> Result<(), O::Error> {
    match scenario {
        Scenario::Neutral => wait_until(deadline),
        Scenario::Hold => {
            if Instant::now() < deadline {
                stage_hold(owner, slots, axes, buttons, hats)?;
                owner.flush()?;
                wait_until(deadline);
            }
        }
        Scenario::Exercise => {
            let mut phase = 0;
            while Instant::now() < deadline {
                stage_exercise(owner, slots, axes, buttons, hats, phase)?;
                owner.flush()?;
                phase += 1;
                wait_until(deadline.min(Instant::now() + Duration::from_millis(250)));
            }
        }
    }
    Ok(())
}

fn stage_hold<O: DeviceOwner>(
    owner: &mut O,
    slots: &[u8],
    axes: &[VJoyAxis],
    buttons: u8,
    hats: u8,
) -> Result<(), O::Error> {
    for &slot in slots {
        for (index, &axis) in axes.iter().enumerate() {
            let sign = if (usize::from(slot) + index) % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            owner.set_axis(slot, axis, sign * 0.5)?;
        }
        for button in 1..=buttons {
            owner.set_button(slot, button, button == 1)?;
        }
        for hat in 1..=hats {
            owner.set_hat(slot, hat, hold_hat(slot, hat))?;
        }
    }
    Ok(())
}

fn stage_exercise<O: DeviceOwner>(
    owner: &mut O,
    slots: &[u8],
    axes: &[VJoyAxis],
    buttons: u8,
    hats: u8,
    phase: usize,
) -> Result<(), O::Error> {
    const VALUES: [f64; 4] = [-1.0, -0.5, 0.5, 1.0];
    const DIRECTIONS: [HatDirection; 4] = [
        HatDirection::N,
        HatDirection::E,
        HatDirection::S,
        HatDirection::W,
    ];
    for &slot in slots {
        for (index, &axis) in axes.iter().enumerate() {
            owner.set_axis(slot, axis, VALUES[(phase + index + usize::from(slot)) % 4])?;
        }
        let pressed = u8::try_from((phase + usize::from(slot)) % usize::from(buttons))
            .expect("remainder is smaller than the u8 button count")
            + 1;
        for button in 1..=buttons {
            owner.set_button(slot, button, button == pressed)?;
        }
        for hat in 1..=hats {
            let direction = DIRECTIONS[(phase + usize::from(slot) + usize::from(hat) - 1) % 4];
            owner.set_hat(slot, hat, direction)?;
        }
    }
    Ok(())
}

fn hold_hat(slot: u8, hat: u8) -> HatDirection {
    const DIRECTIONS: [HatDirection; 4] = [
        HatDirection::N,
        HatDirection::E,
        HatDirection::S,
        HatDirection::W,
    ];
    DIRECTIONS[(usize::from(slot) + usize::from(hat) - 2) % 4]
}

fn wait_until(deadline: Instant) {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if !remaining.is_zero() {
        thread::sleep(remaining);
    }
}
