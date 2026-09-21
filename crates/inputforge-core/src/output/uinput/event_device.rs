use super::{
    Error,
    device::DeviceSpec,
    lifecycle::WRITE_LIMIT,
    native::{Handle, System},
};
use crate::{
    error::EngineError,
    output::{OutputFailure, OutputKind, OutputPhase},
};
use evdev::InputEvent;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(super) struct EventError {
    pub(super) phase: OutputPhase,
    pub(super) error: Error,
}

impl EventError {
    pub(super) fn into_engine(self, output: OutputKind) -> EngineError {
        EngineError::InjectionFailed {
            failure: OutputFailure {
                output,
                phase: self.phase,
                category: self.error.kind(),
                details: self.error.primary_details(),
                cleanup: self
                    .error
                    .cleanup_failures()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            },
        }
    }
}

#[derive(Debug)]
pub(super) struct EventDevice {
    pub(super) spec: DeviceSpec,
    pub(super) system: System,
    pub(super) handle: Option<Handle>,
    pub(super) valid: bool,
    pub(super) groups: BTreeMap<Vec<u16>, usize>,
    pub(super) claims: BTreeMap<u16, usize>,
    pub(super) possible: BTreeSet<u16>,
}

impl EventDevice {
    pub(super) fn new(spec: DeviceSpec) -> Self {
        Self::from_system(spec, System::Native)
    }

    #[cfg(test)]
    pub(super) fn with_system(spec: DeviceSpec, system: System) -> Self {
        Self::from_system(spec, system)
    }

    fn from_system(spec: DeviceSpec, system: System) -> Self {
        Self {
            spec,
            system,
            handle: None,
            valid: true,
            groups: BTreeMap::new(),
            claims: BTreeMap::new(),
            possible: BTreeSet::new(),
        }
    }

    pub(super) fn press(&mut self, ordered_codes: &[u16]) -> Result<(), EventError> {
        self.ensure_active(OutputPhase::Emission)?;
        let group = unique(ordered_codes);
        if let Some(count) = self.groups.get_mut(&group) {
            *count += 1;
            return Ok(());
        }
        self.possible.extend(group.iter().copied());
        let transitions: Vec<_> = group
            .iter()
            .copied()
            .filter(|code| !self.claims.contains_key(code))
            .collect();
        self.emit_keys(&transitions, 1, OutputPhase::Emission)?;
        self.groups.insert(group.clone(), 1);
        for code in group {
            *self.claims.entry(code).or_default() += 1;
        }
        Ok(())
    }

    pub(super) fn release(&mut self, ordered_codes: &[u16]) -> Result<(), EventError> {
        let group = unique(ordered_codes);
        let Some(&count) = self.groups.get(&group) else {
            return Ok(());
        };
        self.ensure_active(OutputPhase::Release)?;
        if count > 1 {
            self.groups.insert(group, count - 1);
            return Ok(());
        }
        let transitions: Vec<_> = group
            .iter()
            .rev()
            .copied()
            .filter(|code| self.claims.get(code) == Some(&1))
            .collect();
        self.emit_keys(&transitions, 0, OutputPhase::Release)?;
        self.groups.remove(&group);
        for code in group {
            if self.claims.get(&code) == Some(&1) {
                self.claims.remove(&code);
            } else if let Some(count) = self.claims.get_mut(&code) {
                *count -= 1;
            }
        }
        self.possible.retain(|code| self.claims.contains_key(code));
        Ok(())
    }

    pub(super) fn relative(&mut self, code: u16, value: i32) -> Result<(), EventError> {
        self.ensure_active(OutputPhase::Emission)?;
        self.emit(
            &[InputEvent::new(2, code, value), InputEvent::new(0, 0, 0)],
            OutputPhase::Emission,
        )
    }

    fn emit_keys(
        &mut self,
        codes: &[u16],
        value: i32,
        phase: OutputPhase,
    ) -> Result<(), EventError> {
        if codes.is_empty() {
            return Ok(());
        }
        let mut packet: Vec<_> = codes
            .iter()
            .map(|&code| InputEvent::new(1, code, value))
            .collect();
        packet.push(InputEvent::new(0, 0, 0));
        self.emit(&packet, phase)
    }

    fn emit(&mut self, packet: &[InputEvent], phase: OutputPhase) -> Result<(), EventError> {
        let deadline = self.system.now() + WRITE_LIMIT;
        if let Err(error) = self.write(packet, deadline) {
            self.valid = false;
            Err(EventError { phase, error })
        } else {
            Ok(())
        }
    }

    fn ensure_active(&self, phase: OutputPhase) -> Result<(), EventError> {
        if self.valid && self.handle.is_some() {
            Ok(())
        } else {
            Err(self.invalid(phase, "output is inactive or requires cleanup"))
        }
    }

    pub(super) fn invalid(&self, phase: OutputPhase, details: &str) -> EventError {
        EventError {
            phase,
            error: Error::invalid("event output", self.spec.key.slot(), details),
        }
    }
}

impl Drop for EventDevice {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn unique(codes: &[u16]) -> Vec<u16> {
    let mut seen = BTreeSet::new();
    codes
        .iter()
        .copied()
        .filter(|code| seen.insert(*code))
        .collect()
}
