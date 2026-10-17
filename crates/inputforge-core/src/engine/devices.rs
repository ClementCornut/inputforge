//! Captured profile and native device identity guard inspector commands at execution.
use super::{Engine, EngineCommand};
use crate::{
    profile::ProfileId,
    state::AppState,
    types::{DeviceId, DeviceInfo},
};
use std::path::PathBuf;
/// Device and profile identity captured when selecting an inspector row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTarget {
    pub profile: Option<ProfileId>,
    pub path: Option<PathBuf>,
    pub info: DeviceInfo,
    pub serial: Option<String>,
}
impl DeviceTarget {
    /// Capture native identity from live metadata, falling back to remembered metadata.
    #[must_use]
    pub fn capture(state: &AppState, device: &DeviceId) -> Option<Self> {
        let (info, diagnostics) = state
            .devices
            .iter()
            .find(|d| &d.info.id == device)
            .map(|d| (&d.info, &d.diagnostics))
            .or_else(|| {
                state
                    .device_registry
                    .get(device)
                    .map(|d| (&d.info, &d.diagnostics))
            })?;
        Some(Self {
            profile: state.active_profile.as_ref().map(|p| p.id().clone()),
            path: state.profile_path.clone(),
            info: info.clone(),
            serial: diagnostics.serial.clone(),
        })
    }
    /// Check current profile and device identity without filesystem access.
    #[must_use]
    pub fn matches(&self, state: &AppState) -> bool {
        Self::capture(state, &self.info.id).as_ref() == Some(self)
    }
}
/// Existing inspector mutation to execute after identity checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceAction {
    Alias(Option<String>),
    Remove,
}
/// A guarded inspector mutation using the existing engine transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRequest {
    pub target: DeviceTarget,
    pub action: DeviceAction,
}
impl Engine {
    pub(super) fn handle_device(&mut self, request: DeviceRequest) {
        if !request.target.matches(&self.state.read()) {
            self.state.write().warnings.push("Device action cancelled: profile or device changed. Reselect the device and try again.".into());
            return;
        }
        let device = request.target.info.id;
        let command = match request.action {
            DeviceAction::Alias(alias) => EngineCommand::SetDeviceAlias { device, alias },
            DeviceAction::Remove => EngineCommand::RemoveDeviceFromProfile { device },
        };
        if let Err(error) = self.handle_command(command) {
            self.state
                .write()
                .warnings
                .push(format!("Could not update device: {error}"));
        }
    }
}
