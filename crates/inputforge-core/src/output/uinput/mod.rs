//! Bounded Linux virtual-controller output, independent of the engine.
//!
//! Configure while inactive, call [`Output::create`], stage values, then flush.
//! Slots and output buttons/hats are one-based. Flush may contain several packets;
//! each hat pair is coherent, but an entire controller or device set is not atomic.
//! Explicit release attempts neutralization and reports cleanup failures. Drop and
//! process death destroy devices without guaranteeing neutral delivery to consumers.
//! No method captures physical input or changes host permissions.

mod config;
mod device;
mod error;
mod event_device;
mod event_device_lifecycle;
mod key_codes;
mod keyboard;
mod lifecycle;
mod mouse;
mod native;
mod state;
#[cfg(test)]
pub(crate) mod tests;

#[doc(inline)]
pub use config::default_config;
#[doc(inline)]
pub use error::Error;
pub use keyboard::Keyboard;
pub use mouse::Mouse;

use crate::types::{HatDirection, VJoyAxis, VirtualDeviceConfig};
use lifecycle::Held;
use native::System;

/// Owns an explicitly configured set of Linux virtual controllers.
///
/// Construction and configuration are hardware-free. Creation and runtime failures
/// release every descriptor before returning. A runtime failure invalidates this
/// owner; construct a fresh owner to retry. No operation logs while owning devices.
#[derive(Debug)]
pub struct Output {
    configs: Vec<VirtualDeviceConfig>,
    held: Vec<Held>,
    system: System,
    valid: bool,
}

impl Output {
    /// Validate configurations without opening any device.
    ///
    /// # Errors
    /// Rejects empty sets, duplicate/out-of-range slots or axes, buttons outside
    /// 2..=53, and more than four hats. Slots are unique within this owner only.
    pub fn new(configs: Vec<VirtualDeviceConfig>) -> Result<Self, Error> {
        Ok(Self {
            configs: config::validate(configs)?,
            held: vec![],
            system: System::Native,
            valid: true,
        })
    }

    /// Replace the configuration while inactive, without touching hardware.
    ///
    /// # Errors
    /// Rejects active/invalidated owners or invalid configurations without mutation.
    pub fn configure(&mut self, configs: Vec<VirtualDeviceConfig>) -> Result<(), Error> {
        self.ensure_valid()?;
        if self.is_active() {
            return Err(Error::invalid("configure", None, "output is active"));
        }
        self.configs = config::validate(configs)?;
        Ok(())
    }

    /// Return configurations in ascending slot order.
    #[must_use]
    pub fn configs(&self) -> &[VirtualDeviceConfig] {
        &self.configs
    }

    /// Return whether this owner holds created output devices.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.held.is_empty()
    }

    /// Create all devices, await event-node readiness, and submit neutral state.
    ///
    /// # Errors
    /// Rejects active/invalidated owners. Setup, readiness, timeout and initial-write
    /// failures roll back every created device. Access failures require host setup
    /// outside this API; creation never changes permissions or retries automatically.
    pub fn create(&mut self) -> Result<(), Error> {
        self.create_all()
    }

    /// Stage a normalized axis value; nonfinite values become numerical center.
    ///
    /// # Errors
    /// Rejects inactive owners, unknown slots, or undeclared axes without mutation.
    pub fn set_axis(&mut self, slot: u8, axis: VJoyAxis, value: f64) -> Result<(), Error> {
        self.state_mut(slot)?
            .axis(axis, value)
            .map_err(|e| Error::io("set axis", Some(slot), "", e))
    }

    /// Stage a one-based button address without performing I/O.
    ///
    /// # Errors
    /// Rejects inactive owners, unknown slots, or undeclared buttons without mutation.
    pub fn set_button(&mut self, slot: u8, button: u8, pressed: bool) -> Result<(), Error> {
        self.state_mut(slot)?
            .button(button, pressed)
            .map_err(|e| Error::io("set button", Some(slot), "", e))
    }

    /// Stage both components of a one-based hat address together.
    ///
    /// # Errors
    /// Rejects inactive owners, unknown slots, or undeclared hats without mutation.
    pub fn set_hat(&mut self, slot: u8, hat: u8, direction: HatDirection) -> Result<(), Error> {
        self.state_mut(slot)?
            .hat(hat, direction)
            .map_err(|e| Error::io("set hat", Some(slot), "", e))
    }

    /// Submit changed encoded values with bounded packet writes.
    ///
    /// Setters coalesce to final state, so a press/release before a flush is no pulse.
    /// Success means submission, not consumer acknowledgement or whole-set atomicity.
    /// # Errors
    /// Inactive/invalidated owners fail. A write failure invalidates the owner and
    /// closes every device, retaining primary and secondary cleanup errors.
    pub fn flush(&mut self) -> Result<(), Error> {
        self.flush_all(false)
    }

    /// Replace pending state with centered axes, released buttons and centered hats.
    ///
    /// # Errors
    /// Returns the same errors and performs the same failure cleanup as [`Self::flush`].
    pub fn reset(&mut self) -> Result<(), Error> {
        self.ensure_active()?;
        for held in &mut self.held {
            held.state.neutral();
        }
        self.flush()
    }

    /// Attempt neutralization, then destroy and close every owned device.
    ///
    /// Repeated release succeeds. Pending application state is never flushed.
    /// # Errors
    /// Reports neutralization/destruction failures after closing all descriptors.
    pub fn release(&mut self) -> Result<(), Error> {
        self.release_all(None)
    }

    fn ensure_valid(&self) -> Result<(), Error> {
        if self.valid {
            Ok(())
        } else {
            Err(Error::invalid(
                "output",
                None,
                "owner invalidated; construct a fresh owner",
            ))
        }
    }
    fn ensure_active(&self) -> Result<(), Error> {
        self.ensure_valid()?;
        if self.is_active() {
            Ok(())
        } else {
            Err(Error::invalid("output", None, "output is inactive"))
        }
    }
    fn state_mut(&mut self, slot: u8) -> Result<&mut state::State, Error> {
        self.ensure_active()?;
        self.held
            .iter_mut()
            .find(|h| h.slot == slot)
            .map(|h| &mut h.state)
            .ok_or_else(|| Error::invalid("address", Some(slot), "unknown output slot"))
    }
}

mod sink;
pub use sink::UinputSink;
