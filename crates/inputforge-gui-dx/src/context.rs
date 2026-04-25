use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState, EngineStatus};
use inputforge_core::types::{
    AxisPolarity, HatDirection, InputAddress, VJoyAxis, VirtualDeviceConfig,
};

/// Raw signal-free handles installed via `LaunchBuilder::with_context`.
///
/// `Arc<AppSettings>` is a zero-cost read-only handle at F1; F14 will
/// unwind this wrapping when adding the mutation path.
#[derive(Clone, Debug)]
#[expect(
    dead_code,
    reason = "constructed in Task 8 (bridge) and Task 9 (app_root)"
)]
pub(crate) struct RawHandles {
    pub state: Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
    pub settings: Arc<AppSettings>,
}

/// Full per-window context: raw handles plus the three reactive signals.
///
/// Assembled inside `app_root` (signals must be created within the runtime).
#[derive(Clone, Debug)]
#[expect(dead_code, reason = "constructed in Task 9 (app_root)")]
pub(crate) struct AppContext {
    pub state: Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
    pub settings: Arc<AppSettings>,
    pub meta: Signal<MetaSnapshot>,
    pub config: Signal<ConfigSnapshot>,
    pub live: Signal<LiveSnapshot>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MetaSnapshot {
    pub engine_status: EngineStatus,
    pub current_mode: String,
    pub profile_name: Option<String>,
    pub profile_path: Option<PathBuf>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ConfigSnapshot {
    pub devices: Vec<DeviceState>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    pub mapped_inputs: HashSet<InputAddress>,
    pub mapping_names: HashMap<InputAddress, String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LiveSnapshot {
    pub device_inputs: Vec<DeviceInputValues>,
    pub output_values: Vec<VjoyOutputValues>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DeviceInputValues {
    pub axes: Vec<(f64, AxisPolarity)>,
    pub buttons: Vec<bool>,
    pub hats: Vec<HatDirection>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct VjoyOutputValues {
    pub axes: Vec<(VJoyAxis, f64)>,
    pub buttons: Vec<bool>,
    pub hats: Vec<HatDirection>,
}

impl MetaSnapshot {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "called from bridge polling task in Task 8")
    )]
    pub(crate) fn from_state(s: &AppState) -> Self {
        Self {
            engine_status: s.engine_status,
            current_mode: s.current_mode.clone(),
            profile_name: s.active_profile.as_ref().map(|p| p.name().to_owned()),
            profile_path: s.profile_path.clone(),
            warnings: s.warnings.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_snapshot_default_is_empty() {
        let m = MetaSnapshot::default();
        assert_eq!(m.engine_status, EngineStatus::Stopped);
        assert!(m.current_mode.is_empty());
        assert!(m.profile_name.is_none());
        assert!(m.profile_path.is_none());
        assert!(m.warnings.is_empty());
    }

    #[test]
    fn config_snapshot_default_is_empty() {
        let c = ConfigSnapshot::default();
        assert!(c.devices.is_empty());
        assert!(c.virtual_devices.is_empty());
        assert!(c.mapped_inputs.is_empty());
        assert!(c.mapping_names.is_empty());
    }

    #[test]
    fn live_snapshot_default_is_empty() {
        let l = LiveSnapshot::default();
        assert!(l.device_inputs.is_empty());
        assert!(l.output_values.is_empty());
    }

    #[test]
    fn meta_from_state_extracts_lifecycle_fields() {
        let mut state = AppState::new();
        state.engine_status = EngineStatus::Running;
        state.current_mode = "FlightAssist".to_owned();
        state.warnings.push("HidHide unavailable".to_owned());
        state.profile_path = Some(PathBuf::from("/tmp/profile.json"));

        let meta = MetaSnapshot::from_state(&state);
        assert_eq!(meta.engine_status, EngineStatus::Running);
        assert_eq!(meta.current_mode, "FlightAssist");
        assert_eq!(meta.profile_name, None); // active_profile is None
        assert_eq!(meta.profile_path, Some(PathBuf::from("/tmp/profile.json")));
        assert_eq!(meta.warnings, vec!["HidHide unavailable".to_owned()]);
    }

    #[test]
    fn meta_from_state_with_active_profile_maps_name() {
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;

        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let profile = Profile::new(
            "Hornet".to_owned(),
            vec![],
            modes,
            vec![],
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let meta = MetaSnapshot::from_state(&state);
        assert_eq!(meta.profile_name, Some("Hornet".to_owned()));
    }
}
