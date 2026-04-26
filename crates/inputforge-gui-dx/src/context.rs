use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::pipeline::InputCache;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState, EngineStatus};
use inputforge_core::types::{
    AxisPolarity, HatDirection, InputAddress, InputId, VJoyAxis, VirtualDeviceConfig,
};

/// Raw signal-free handles installed via `LaunchBuilder::with_context`.
///
/// `Arc<AppSettings>` is a zero-cost read-only handle at F1; F14 will
/// unwind this wrapping when adding the mutation path.
#[derive(Clone, Debug)]
pub(crate) struct RawHandles {
    pub state: Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
    pub settings: Arc<AppSettings>,
}

/// Full per-window context: raw handles plus the three reactive signals.
///
/// Assembled inside `app_root` (signals must be created within the runtime).
#[derive(Clone, Debug)]
pub(crate) struct AppContext {
    pub state: Arc<RwLock<AppState>>,
    #[expect(dead_code, reason = "used in later tasks (engine command dispatch)")]
    pub commands: mpsc::Sender<EngineCommand>,
    #[expect(dead_code, reason = "used in later tasks (settings reads)")]
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

impl LiveSnapshot {
    /// Takes a pre-built `ConfigSnapshot` so device / virtual-device shape is
    /// read from a single coherent source.
    pub(crate) fn from_state(s: &AppState, cfg: &ConfigSnapshot) -> Self {
        let device_inputs: Vec<DeviceInputValues> = cfg
            .devices
            .iter()
            .map(|device| {
                let did = &device.info.id;
                DeviceInputValues {
                    axes: (0..device.info.axes)
                        .map(|i| {
                            let addr = InputAddress {
                                device: did.clone(),
                                input: InputId::Axis { index: i },
                            };
                            let pol = device
                                .info
                                .axis_polarities
                                .get(usize::from(i))
                                .copied()
                                .unwrap_or_default();
                            (s.input_cache.get_axis(&addr), pol)
                        })
                        .collect(),
                    buttons: (0..device.info.buttons)
                        .map(|i| {
                            let addr = InputAddress {
                                device: did.clone(),
                                input: InputId::Button { index: i },
                            };
                            s.input_cache.get_button(&addr)
                        })
                        .collect(),
                    hats: (0..device.info.hats)
                        .map(|i| {
                            let addr = InputAddress {
                                device: did.clone(),
                                input: InputId::Hat { index: i },
                            };
                            s.input_cache.get_hat(&addr)
                        })
                        .collect(),
                }
            })
            .collect();

        let output_values: Vec<VjoyOutputValues> = cfg
            .virtual_devices
            .iter()
            .map(|v| VjoyOutputValues {
                axes: v
                    .axes
                    .iter()
                    .map(|&a| (a, s.output_cache.get_axis(v.device_id, a)))
                    .collect(),
                buttons: (1..=v.button_count)
                    .map(|i| s.output_cache.get_button(v.device_id, i))
                    .collect(),
                hats: (0..v.hat_count)
                    .map(|i| s.output_cache.get_hat(v.device_id, i))
                    .collect(),
            })
            .collect();

        Self {
            device_inputs,
            output_values,
        }
    }
}

impl ConfigSnapshot {
    pub(crate) fn from_state(s: &AppState) -> Self {
        let mut mapped_inputs = HashSet::new();
        let mut mapping_names = HashMap::new();
        if let Some(profile) = &s.active_profile {
            for mapping in profile.mappings() {
                mapped_inputs.insert(mapping.input.clone());
                if let Some(name) = &mapping.name {
                    mapping_names.insert(mapping.input.clone(), name.clone());
                }
            }
        }
        Self {
            devices: s.devices.clone(),
            virtual_devices: s.virtual_devices.clone(),
            mapped_inputs,
            mapping_names,
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

    #[test]
    fn config_from_state_clones_devices_and_virtual_devices() {
        use inputforge_core::types::{DeviceId, DeviceInfo};

        let mut state = AppState::new();
        state.devices.push(DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Throttle".to_owned(),
                axes: 1,
                buttons: 0,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Unipolar],
            },
            connected: true,
        });
        state.virtual_devices.push(VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X],
            button_count: 4,
            hat_count: 0,
        });

        let cfg = ConfigSnapshot::from_state(&state);
        assert_eq!(cfg.devices.len(), 1);
        assert_eq!(cfg.devices[0].info.name, "Throttle");
        assert_eq!(cfg.virtual_devices.len(), 1);
        assert_eq!(cfg.virtual_devices[0].button_count, 4);
        assert!(cfg.mapped_inputs.is_empty()); // no profile loaded
        assert!(cfg.mapping_names.is_empty());
    }

    #[test]
    fn live_from_state_handles_empty_config() {
        let state = AppState::new();
        let cfg = ConfigSnapshot::from_state(&state);
        let live = LiveSnapshot::from_state(&state, &cfg);
        assert!(live.device_inputs.is_empty());
        assert!(live.output_values.is_empty());
    }

    #[test]
    fn live_from_state_reads_caches_per_device_shape() {
        use inputforge_core::state::DeviceState;
        use inputforge_core::types::{AxisValue, DeviceId, DeviceInfo, InputId, InputValue};

        let mut state = AppState::new();
        let did = DeviceId("dev-1".to_owned());

        state.devices.push(DeviceState {
            info: DeviceInfo {
                id: did.clone(),
                name: "Joystick".to_owned(),
                axes: 1,
                buttons: 1,
                hats: 1,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar],
            },
            connected: true,
        });
        state.virtual_devices.push(VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X],
            button_count: 1,
            hat_count: 1,
        });

        state.input_cache.update(
            &InputAddress {
                device: did.clone(),
                input: InputId::Axis { index: 0 },
            },
            &InputValue::Axis {
                value: AxisValue::new(0.5),
            },
        );
        state.input_cache.update(
            &InputAddress {
                device: did.clone(),
                input: InputId::Button { index: 0 },
            },
            &InputValue::Button { pressed: true },
        );
        state.input_cache.update(
            &InputAddress {
                device: did,
                input: InputId::Hat { index: 0 },
            },
            &InputValue::Hat {
                direction: HatDirection::N,
            },
        );

        state.output_cache.set_axis(1, VJoyAxis::X, -0.25);
        state.output_cache.set_button(1, 1, true);
        state.output_cache.set_hat(1, 0, HatDirection::SE);

        let cfg = ConfigSnapshot::from_state(&state);
        let live = LiveSnapshot::from_state(&state, &cfg);

        assert_eq!(live.device_inputs.len(), 1);
        assert_eq!(live.device_inputs[0].axes.len(), 1);
        assert!((live.device_inputs[0].axes[0].0 - 0.5).abs() < f64::EPSILON);
        assert_eq!(live.device_inputs[0].axes[0].1, AxisPolarity::Bipolar);
        assert_eq!(live.device_inputs[0].buttons, vec![true]);
        assert_eq!(live.device_inputs[0].hats, vec![HatDirection::N]);

        assert_eq!(live.output_values.len(), 1);
        assert!((live.output_values[0].axes[0].1 - (-0.25)).abs() < f64::EPSILON);
        assert_eq!(live.output_values[0].axes[0].0, VJoyAxis::X);
        assert_eq!(live.output_values[0].buttons, vec![true]);
        assert_eq!(live.output_values[0].hats, vec![HatDirection::SE]);
    }

    #[test]
    fn config_from_state_populates_mapped_inputs_and_names() {
        use inputforge_core::action::Mapping;
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let mut map = HashMap::new();
        map.insert("Default".to_owned(), vec![]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let device_id = DeviceId("dev-1".to_owned());
        let addr_named = InputAddress {
            device: device_id.clone(),
            input: InputId::Button { index: 0 },
        };
        let addr_unnamed = InputAddress {
            device: device_id,
            input: InputId::Button { index: 1 },
        };

        let mappings = vec![
            Mapping {
                input: addr_named.clone(),
                mode: "Default".to_owned(),
                name: Some("Fire".to_owned()),
                actions: vec![],
            },
            Mapping {
                input: addr_unnamed.clone(),
                mode: "Default".to_owned(),
                name: None,
                actions: vec![],
            },
        ];

        let profile = Profile::new(
            "TestProfile".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        let cfg = ConfigSnapshot::from_state(&state);
        assert_eq!(cfg.mapped_inputs.len(), 2);
        assert!(cfg.mapped_inputs.contains(&addr_named));
        assert!(cfg.mapped_inputs.contains(&addr_unnamed));
        assert_eq!(cfg.mapping_names.len(), 1);
        assert_eq!(cfg.mapping_names.get(&addr_named), Some(&"Fire".to_owned()));
        assert!(!cfg.mapping_names.contains_key(&addr_unnamed));
    }

    #[test]
    fn f1_readout_data_binding_contract() {
        use inputforge_core::state::{AppState, DeviceState, EngineStatus};
        use inputforge_core::types::{
            AxisPolarity, DeviceId, DeviceInfo, VJoyAxis, VirtualDeviceConfig,
        };

        let mut s = AppState::new();
        s.engine_status = EngineStatus::Running;
        "Combat".clone_into(&mut s.current_mode);
        s.warnings.push("low battery".to_owned());
        s.devices.push(DeviceState {
            info: DeviceInfo {
                id: DeviceId("dev-1".to_owned()),
                name: "Stick".to_owned(),
                axes: 2,
                buttons: 4,
                hats: 1,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar; 2],
            },
            connected: true,
        });
        s.virtual_devices.push(VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X, VJoyAxis::Y],
            button_count: 4,
            hat_count: 1,
        });

        let meta = MetaSnapshot::from_state(&s);
        let cfg = ConfigSnapshot::from_state(&s);

        // The exact six values F1Readout reads:
        assert_eq!(meta.engine_status, EngineStatus::Running);
        assert_eq!(meta.current_mode, "Combat");
        assert_eq!(meta.profile_name, None); // no profile loaded
        assert_eq!(cfg.devices.len(), 1);
        assert_eq!(cfg.virtual_devices.len(), 1);
        assert_eq!(meta.warnings.len(), 1);
    }
}
