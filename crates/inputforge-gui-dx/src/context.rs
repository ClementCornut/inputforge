use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::pipeline::InputCache;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState, EngineStatus, ForcedMode};
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
    pub mode_force: Option<ForcedMode>,
    /// DFS pre-order names. Hierarchy queries (parent, descendants) are
    /// not surfaced through this field, components requiring tree shape
    /// read `ctx.state.active_profile.modes()` directly. The split is
    /// deliberate: this snapshot is cheap to clone-on-tick, and the
    /// only F7 consumer that needs hierarchy (delete-confirm preview)
    /// reads from raw state at dialog-open time, which is rare enough
    /// not to warrant projecting an `Arc<ModeTree>` here.
    pub modes: Vec<String>,
    pub startup_mode: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ConfigSnapshot {
    pub devices: Vec<DeviceState>,
    pub virtual_devices: Vec<VirtualDeviceConfig>,
    pub mapped_inputs: HashSet<InputAddress>,
    pub mapping_names: HashMap<InputAddress, String>,
    pub mappings: Vec<MappingSummary>,
    /// Cloned `Vec<Action>` for the currently-selected mapping, if any.
    /// Cheap because only one mapping's actions are cloned per tick.
    pub selected_mapping_actions: Option<Vec<inputforge_core::action::Action>>,
    /// The (mode, input) key recorded at the same tick. Allows the editor
    /// to detect cross-window conflicts: selection still refers to a key
    /// that the engine no longer holds.
    pub selected_mapping_key: Option<crate::frame::MappingKey>,
}

/// One row's worth of state for the F8 mapping list. Populated by
/// `ConfigSnapshot::from_state` once per polling tick from the active
/// profile's `Mapping` entries; consumers in `frame::mapping_list` read
/// these as constant-time field accesses without re-walking the action
/// tree.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MappingSummary {
    pub input: InputAddress,
    pub mode: String,
    pub name: Option<String>,
    pub glyphs: GlyphFlags,
}

/// Pre-computed glyph state for a `MappingSummary`. The walker stops on
/// the first match per glyph, so both fields hold the *first*
/// occurrence found by depth-first traversal of the action tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct GlyphFlags {
    /// `Some(addr)` if the action tree contains an `Action::MergeAxis`
    /// whose `second_input` is `addr`, the secondary input shown after
    /// the gold `+` glyph.
    pub merge_secondary: Option<InputAddress>,
    /// `Some(addr)` if the action tree contains an `Action::Conditional`
    /// whose `condition` references at least one `InputAddress` (via
    /// `ButtonPressed | ButtonReleased | AxisInRange | HatDirection`,
    /// possibly nested under `All | Any | Not`). The violet `+` glyph
    /// hover-tooltip in `row.rs` runs this address through
    /// `source_label::format` to produce the human-readable predicate
    /// label (identical path to `merge_secondary`).
    pub first_input_predicate: Option<InputAddress>,
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
            mode_force: s.mode_force.clone(),
            modes: s
                .active_profile
                .as_ref()
                .map(|p| {
                    p.modes()
                        .all_modes()
                        .into_iter()
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            startup_mode: s
                .active_profile
                .as_ref()
                .map(|p| p.settings().startup_mode().to_owned()),
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
                            let addr = InputAddress::Bound {
                                device: did.clone(),
                                input: InputId::Axis { index: i },
                            };
                            // Polarity source: device.info.axis_polarities
                            // (the lazy-classification table updated on
                            // re-probe). Fall back to Bipolar when the
                            // device entry is short, matching pre-Task-1
                            // behavior. The cache's polarity tag is
                            // unused here intentionally.
                            let (value, _cache_polarity) = s.input_cache.get_axis(&addr);
                            let pol = device
                                .info
                                .axis_polarities
                                .get(usize::from(i))
                                .copied()
                                .unwrap_or_default();
                            (value, pol)
                        })
                        .collect(),
                    buttons: (0..device.info.buttons)
                        .map(|i| {
                            let addr = InputAddress::Bound {
                                device: did.clone(),
                                input: InputId::Button { index: i },
                            };
                            s.input_cache.get_button(&addr)
                        })
                        .collect(),
                    hats: (0..device.info.hats)
                        .map(|i| {
                            let addr = InputAddress::Bound {
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

/// Walk an action tree in depth-first order, recording the first
/// `MergeAxis::second_input` and the first input-bearing `Conditional`
/// predicate. Returns early once both glyphs are populated, or after a
/// full traversal (whichever comes first).
fn derive_glyphs(actions: &[inputforge_core::action::Action]) -> GlyphFlags {
    let mut out = GlyphFlags::default();
    walk_actions(actions, &mut out);
    out
}

fn walk_actions(actions: &[inputforge_core::action::Action], out: &mut GlyphFlags) {
    use inputforge_core::action::Action;
    for action in actions {
        if out.merge_secondary.is_some() && out.first_input_predicate.is_some() {
            return;
        }
        match action {
            Action::MergeAxis { second_input, .. } => {
                if out.merge_secondary.is_none() {
                    out.merge_secondary = Some(second_input.clone());
                }
            }
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                if out.first_input_predicate.is_none() {
                    if let Some(addr) = first_input_predicate(condition) {
                        out.first_input_predicate = Some(addr);
                    }
                }
                walk_actions(if_true, out);
                walk_actions(if_false, out);
            }
            _ => {}
        }
    }
}

/// Recurse through `All | Any | Not` composites until an input-bearing
/// leaf (`ButtonPressed | ButtonReleased | AxisInRange | HatDirection`)
/// is found.
fn first_input_predicate(condition: &inputforge_core::action::Condition) -> Option<InputAddress> {
    use inputforge_core::action::Condition;
    match condition {
        Condition::ButtonPressed { input }
        | Condition::ButtonReleased { input }
        | Condition::AxisInRange { input, .. }
        | Condition::HatDirection { input, .. } => Some(input.clone()),
        Condition::All { conditions } | Condition::Any { conditions } => {
            conditions.iter().find_map(first_input_predicate)
        }
        Condition::Not { condition } => first_input_predicate(condition),
    }
}

impl ConfigSnapshot {
    pub(crate) fn from_state(s: &AppState, selection: Option<&crate::frame::MappingKey>) -> Self {
        let mut mapped_inputs = HashSet::new();
        let mut mapping_names = HashMap::new();
        let mut mappings = Vec::new();
        let mut selected_mapping_actions: Option<Vec<inputforge_core::action::Action>> = None;
        if let Some(profile) = &s.active_profile {
            for mapping in profile.mappings() {
                mapped_inputs.insert(mapping.input.clone());
                if let Some(name) = &mapping.name {
                    mapping_names.insert(mapping.input.clone(), name.clone());
                }
                mappings.push(MappingSummary {
                    input: mapping.input.clone(),
                    mode: mapping.mode.clone(),
                    name: mapping.name.clone(),
                    glyphs: derive_glyphs(&mapping.actions),
                });
                if let Some((sel_mode, sel_input)) = selection {
                    if mapping.mode == *sel_mode && mapping.input == *sel_input {
                        selected_mapping_actions = Some(mapping.actions.clone());
                    }
                }
            }
        }
        Self {
            devices: s.devices.clone(),
            virtual_devices: s.virtual_devices.clone(),
            mapped_inputs,
            mapping_names,
            mappings,
            selected_mapping_actions,
            selected_mapping_key: selection.cloned(),
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
        assert!(m.mode_force.is_none());
        assert!(m.modes.is_empty());
        assert!(m.startup_mode.is_none());
    }

    #[test]
    fn config_snapshot_default_is_empty() {
        let c = ConfigSnapshot::default();
        assert!(c.devices.is_empty());
        assert!(c.virtual_devices.is_empty());
        assert!(c.mapped_inputs.is_empty());
        assert!(c.mapping_names.is_empty());
        assert!(c.mappings.is_empty());
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

        let cfg = ConfigSnapshot::from_state(&state, None);
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
        let cfg = ConfigSnapshot::from_state(&state, None);
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
            &InputAddress::Bound {
                device: did.clone(),
                input: InputId::Axis { index: 0 },
            },
            &InputValue::Axis {
                value: AxisValue::new(0.5),
                polarity: AxisPolarity::Bipolar,
            },
        );
        state.input_cache.update(
            &InputAddress::Bound {
                device: did.clone(),
                input: InputId::Button { index: 0 },
            },
            &InputValue::Button { pressed: true },
        );
        state.input_cache.update(
            &InputAddress::Bound {
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

        let cfg = ConfigSnapshot::from_state(&state, None);
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
        let addr_named = InputAddress::Bound {
            device: device_id.clone(),
            input: InputId::Button { index: 0 },
        };
        let addr_unnamed = InputAddress::Bound {
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

        let cfg = ConfigSnapshot::from_state(&state, None);
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
        let cfg = ConfigSnapshot::from_state(&s, None);

        // The exact six snapshot fields the placeholder shell surfaces consume:
        assert_eq!(meta.engine_status, EngineStatus::Running);
        assert_eq!(meta.current_mode, "Combat");
        assert_eq!(meta.profile_name, None); // no profile loaded
        assert_eq!(cfg.devices.len(), 1);
        assert_eq!(cfg.virtual_devices.len(), 1);
        assert_eq!(meta.warnings.len(), 1);
    }

    #[test]
    fn meta_from_state_projects_mode_force() {
        use inputforge_core::state::ForcedMode;

        let mut state = AppState::new();
        state.mode_force = Some(ForcedMode {
            mode: "Combat".to_owned(),
        });
        let meta = MetaSnapshot::from_state(&state);
        assert_eq!(
            meta.mode_force.as_ref().map(|f| f.mode.as_str()),
            Some("Combat")
        );
    }

    #[test]
    fn meta_from_state_projects_modes_and_startup_mode() {
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;

        let mut map = HashMap::new();
        map.insert(
            "Default".to_owned(),
            vec!["Combat".to_owned(), "Landing".to_owned()],
        );
        let modes = ModeTree::from_adjacency(&map).unwrap();
        let profile = Profile::new(
            "Modal".to_owned(),
            vec![],
            modes,
            vec![],
            vec![],
            "Combat".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let meta = MetaSnapshot::from_state(&state);
        assert_eq!(meta.modes.len(), 3);
        assert_eq!(meta.modes[0], "Default", "DFS pre-order: root first");
        assert_eq!(meta.startup_mode, Some("Combat".to_owned()));
    }

    #[test]
    fn meta_from_state_with_no_profile_has_empty_modes_and_no_startup() {
        let state = AppState::new();
        let meta = MetaSnapshot::from_state(&state);
        assert!(meta.modes.is_empty());
        assert!(meta.startup_mode.is_none());
    }

    #[test]
    fn config_snapshot_populates_mappings_with_no_glyphs() {
        use inputforge_core::action::Mapping;
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let addr = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let mappings = vec![Mapping {
            input: addr.clone(),
            mode: "Default".to_owned(),
            name: Some("Fire".to_owned()),
            actions: vec![],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        assert_eq!(cfg.mappings.len(), 1);
        let s = &cfg.mappings[0];
        assert_eq!(s.input, addr);
        assert_eq!(s.mode, "Default");
        assert_eq!(s.name.as_deref(), Some("Fire"));
        assert!(s.glyphs.merge_secondary.is_none());
        assert!(s.glyphs.first_input_predicate.is_none());
    }

    #[test]
    fn config_snapshot_glyph_walker_finds_merge_axis() {
        use inputforge_core::action::{Action, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId, MergeOp};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let primary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let secondary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        };

        let mappings = vec![Mapping {
            input: primary.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::MergeAxis {
                second_input: secondary.clone(),
                operation: MergeOp::Average,
            }],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert_eq!(s.glyphs.merge_secondary.as_ref(), Some(&secondary));
        assert!(s.glyphs.first_input_predicate.is_none());
    }

    #[test]
    fn config_snapshot_glyph_walker_finds_input_conditional() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let trigger = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let predicate = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 1 },
        };

        let mappings = vec![Mapping {
            input: trigger.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: predicate.clone(),
                },
                if_true: vec![],
                if_false: Vec::new(),
            }],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert!(s.glyphs.merge_secondary.is_none());
        assert!(
            s.glyphs.first_input_predicate.is_some(),
            "input-bearing Conditional must populate first_input_predicate"
        );
    }

    #[test]
    fn config_snapshot_glyph_walker_finds_both_glyphs() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId, MergeOp};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let primary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let secondary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        let predicate = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };

        let mappings = vec![Mapping {
            input: primary.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![
                Action::MergeAxis {
                    second_input: secondary.clone(),
                    operation: MergeOp::Average,
                },
                Action::Conditional {
                    condition: Condition::ButtonPressed {
                        input: predicate.clone(),
                    },
                    if_true: vec![],
                    if_false: Vec::new(),
                },
            ],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert_eq!(s.glyphs.merge_secondary.as_ref(), Some(&secondary));
        assert!(s.glyphs.first_input_predicate.is_some());
    }

    #[test]
    fn config_snapshot_glyph_walker_recurses_through_composite_conditions() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let trigger = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let nested_predicate = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 5 },
        };

        let nested_condition = Condition::Not {
            condition: Box::new(Condition::Any {
                conditions: vec![Condition::All {
                    conditions: vec![Condition::ButtonReleased {
                        input: nested_predicate.clone(),
                    }],
                }],
            }),
        };

        let mappings = vec![Mapping {
            input: trigger.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Conditional {
                condition: nested_condition,
                if_true: vec![],
                if_false: Vec::new(),
            }],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert!(
            s.glyphs.first_input_predicate.is_some(),
            "walker must recurse through Not -> Any -> All -> ButtonReleased"
        );
    }

    #[test]
    fn config_snapshot_glyph_walker_descends_into_nested_actions() {
        use inputforge_core::action::{Action, Condition, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId, MergeOp};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let primary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 0 },
        };
        let secondary = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        let predicate = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };

        let mappings = vec![Mapping {
            input: primary.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Conditional {
                condition: Condition::ButtonPressed {
                    input: predicate.clone(),
                },
                if_true: vec![Action::MergeAxis {
                    second_input: secondary.clone(),
                    operation: MergeOp::Average,
                }],
                if_false: Vec::new(),
            }],
        }];

        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);
        let cfg = ConfigSnapshot::from_state(&state, None);

        let s = &cfg.mappings[0];
        assert_eq!(
            s.glyphs.merge_secondary.as_ref(),
            Some(&secondary),
            "walker must descend into Conditional.if_true to find MergeAxis"
        );
    }

    #[test]
    fn config_from_state_with_selection_clones_actions() {
        use inputforge_core::action::{Action, Mapping};
        use inputforge_core::mode::ModeTree;
        use inputforge_core::profile::Profile;
        use inputforge_core::types::{DeviceId, InputId};

        let map = HashMap::from([("Default".to_owned(), vec![])]);
        let modes = ModeTree::from_adjacency(&map).unwrap();

        let addr = InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        };
        let mappings = vec![Mapping {
            input: addr.clone(),
            mode: "Default".to_owned(),
            name: Some("Fire".to_owned()),
            actions: vec![Action::Invert],
        }];
        let profile = Profile::new(
            "P".to_owned(),
            vec![],
            modes,
            mappings,
            vec![],
            "Default".to_owned(),
        );
        let state = AppState::with_profile(profile);

        let sel = Some(("Default".to_owned(), addr.clone()));
        let cfg = ConfigSnapshot::from_state(&state, sel.as_ref());

        assert_eq!(cfg.selected_mapping_actions.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            cfg.selected_mapping_key.as_ref(),
            Some(&("Default".to_owned(), addr.clone()))
        );
    }

    #[test]
    fn config_from_state_without_selection_actions_none() {
        let state = AppState::new();
        let cfg = ConfigSnapshot::from_state(&state, None);
        assert!(cfg.selected_mapping_actions.is_none());
        assert!(cfg.selected_mapping_key.is_none());
    }

    #[test]
    fn config_from_state_with_stale_selection_actions_none_key_present() {
        use inputforge_core::types::{DeviceId, InputId};

        let app = AppState::new();
        let stale_sel = Some((
            "Default".to_owned(),
            InputAddress::Bound {
                device: DeviceId("nonexistent".to_owned()),
                input: InputId::Button { index: 99 },
            },
        ));
        let cfg = ConfigSnapshot::from_state(&app, stale_sel.as_ref());
        assert!(cfg.selected_mapping_actions.is_none());
        assert_eq!(cfg.selected_mapping_key, stale_sel);
    }
}
