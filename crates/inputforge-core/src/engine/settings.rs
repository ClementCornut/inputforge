//! Settings deltas preserve queued fields and guard destructive buffer reductions at execution.
use super::{Engine, EngineCommand};
use crate::{profile::ProfileId, snapshot::SnapshotConfig, state::AppState};
use std::path::PathBuf;
/// Settings and active native profile context captured for a buffer change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneGuard {
    pub profile: Option<ProfileId>,
    pub path: Option<PathBuf>,
    pub config: SnapshotConfig,
    pub unpinned: usize,
}
impl PruneGuard {
    /// Capture the current profile, settings and count without filesystem access.
    #[must_use]
    pub fn capture(state: &AppState) -> Self {
        Self {
            profile: state.active_profile.as_ref().map(|p| p.id().clone()),
            path: state.profile_path.clone(),
            config: state.snapshot_config.clone(),
            unpinned: state
                .active_snapshot_rows
                .iter()
                .filter(|r| !r.pinned)
                .count(),
        }
    }
    /// Compare the full confirmation context with current authoritative state.
    #[must_use]
    pub fn matches(&self, state: &AppState) -> bool {
        self == &Self::capture(state)
    }
}
/// One settings field changed by the app; other fields remain authoritative at execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsChange {
    Max { value: usize, guard: PruneGuard },
    Skip(bool),
    Autostart(bool),
    Minimized(bool),
    DoubleTap(u64),
    LongPress(u64),
}
/// A settings mutation and its per-owner acknowledgement sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsRequest {
    pub owner: u128,
    pub serial: u64,
    pub change: SettingsChange,
}
impl Engine {
    pub(super) fn handle_settings(&mut self, request: SettingsRequest) {
        let command = self.settings_command(request.change);
        match command {
            Ok(command) => {
                if let Err(error) = self.handle_command(command) {
                    self.state
                        .write()
                        .warnings
                        .push(format!("Could not update settings: {error}"));
                }
            }
            Err(error) => self.state.write().warnings.push(error),
        }
        self.state.write().settings_ack = Some((request.owner, request.serial));
    }
    fn settings_command(&self, change: SettingsChange) -> Result<EngineCommand, String> {
        Ok(match change {
            SettingsChange::Max { value, guard } => {
                if !guard.matches(&self.state.read()) {
                    return Err("Snapshot buffer change cancelled: profile, count or settings changed. Cancel and try again.".into());
                }
                let mut config = self.settings.snapshot.clone();
                config.max_count = value;
                config.validate().map_err(|e| e.to_string())?;
                EngineCommand::SetSnapshotConfig { config }
            }
            SettingsChange::Skip(value) => {
                let mut config = self.settings.snapshot.clone();
                config.skip_if_unchanged = value;
                EngineCommand::SetSnapshotConfig { config }
            }
            SettingsChange::Autostart(enabled) => EngineCommand::SetAutostart { enabled },
            SettingsChange::Minimized(enabled) => {
                EngineCommand::SetStartMinimizedToTray { enabled }
            }
            SettingsChange::DoubleTap(threshold_ms) => {
                EngineCommand::SetDefaultDoubleTapThreshold { threshold_ms }
            }
            SettingsChange::LongPress(threshold_ms) => {
                EngineCommand::SetDefaultLongPressThreshold { threshold_ms }
            }
        })
    }
}
