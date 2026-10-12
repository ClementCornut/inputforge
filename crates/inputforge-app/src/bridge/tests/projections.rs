use super::super::{config, live, meta, settings, time_labels::*};
use inputforge_core::pipeline::{ActionPathSegment, OutputDestination};
use inputforge_core::{
    action::ActionBranch,
    pipeline::OutputOwner,
    state::{
        AppState, DeviceState, EngineStatus, OutputActivityValue,
        ProfileOrigin as CoreProfileOrigin,
    },
    types::{
        AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, HatDirection, InputAddress, InputId,
        VJoyAxis, VirtualDeviceConfig,
    },
};
use inputforge_gui_dx::snapshots::*;
use std::{path::PathBuf, time::Instant};

#[test]
fn meta_projection_leaves_file_metadata_to_host() {
    let path = std::env::temp_dir().join(format!("inputforge-{}.json", ulid::Ulid::generate()));
    std::fs::write(&path, "{}").unwrap();
    let mut state =
        AppState::with_profile(profile_with_primary_merge_and_conditional_device_refs());
    state.profile_path = Some(path.clone());
    state.active_profile_origin = Some(CoreProfileOrigin::External);
    let meta = meta::project(&state);
    std::fs::remove_file(path).unwrap();
    assert_eq!(meta.profile_rows.len(), 1);
    assert_eq!(meta.profile_rows[0].last_edited_label, None);
}

fn test_device(id: &str, name: &str, axes: u8, buttons: u8, hats: u8) -> DeviceInfo {
    DeviceInfo {
        id: DeviceId(id.to_owned()),
        name: name.to_owned(),
        axes,
        buttons,
        hats,
        instance_path: None,
        axis_polarities: vec![AxisPolarity::Bipolar; usize::from(axes)],
    }
}

fn profile_with_primary_merge_and_conditional_device_refs() -> inputforge_core::profile::Profile {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::MergeOp;

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
    let device = DeviceId("dev-1".to_owned());
    let other = DeviceId("dev-2".to_owned());
    let third = DeviceId("dev-3".to_owned());
    let primary_button = InputAddress::Bound {
        device: device.clone(),
        input: InputId::Button { index: 0 },
    };
    let secondary_axis = InputAddress::Bound {
        device: device.clone(),
        input: InputId::Axis { index: 1 },
    };

    Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        vec![
            Mapping {
                input: primary_button.clone(),
                mode: "Default".to_owned(),
                name: Some("Primary fire".to_owned()),
                actions: vec![],
            },
            Mapping {
                input: InputAddress::Bound {
                    device: other,
                    input: InputId::Axis { index: 0 },
                },
                mode: "Default".to_owned(),
                name: Some("Merged axis".to_owned()),
                actions: vec![Action::MergeAxis {
                    second_input: secondary_axis,
                    operation: MergeOp::Average,
                }],
            },
            Mapping {
                input: InputAddress::Bound {
                    device: third,
                    input: InputId::Button { index: 0 },
                },
                mode: "Default".to_owned(),
                name: Some("Conditional fire".to_owned()),
                actions: vec![Action::Conditional {
                    condition: Condition::ButtonPressed {
                        input: primary_button,
                    },
                    if_true: vec![],
                    if_false: vec![],
                }],
            },
        ],
        vec![],
        "Default".to_owned(),
    )
}

#[test]
fn meta_snapshot_default_is_empty() {
    let m = MetaSnapshot::default();
    assert_eq!(m.engine_status, EngineStatus::Stopped);
    assert!(m.current_mode.is_empty());
    assert!(m.profile_name.is_none());
    assert!(m.profile_path.is_none());
    assert!(m.warnings.is_empty());
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
    assert!(c.device_panel_rows.is_empty());
}

#[test]
fn live_snapshot_default_is_empty() {
    let l = LiveSnapshot::default();
    assert!(l.device_inputs.is_empty());
    assert!(l.output_values.is_empty());
    assert!(l.output_activity.is_empty());
}

fn test_output_owner() -> OutputOwner {
    OutputOwner {
        profile: "memory-profile".to_owned(),
        mode: "Default".to_owned(),
        input: InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index: 0 },
        },
        action_path: vec![
            ActionPathSegment::Index(0),
            ActionPathSegment::Branch(ActionBranch::TapSingle),
            ActionPathSegment::Index(0),
        ],
        destination: OutputDestination::Keyboard(inputforge_core::types::KeyCombo {
            key: inputforge_core::types::PhysicalKey::Space,
            modifiers: vec![],
        }),
        behavior: inputforge_core::action::OutputBehavior::Pulse,
    }
}

#[test]
fn live_snapshot_from_state_at_includes_latched_output_activity() {
    let now = Instant::now();
    let mut state = AppState::new();
    let owner = test_output_owner();
    state.output_activity.record(
        owner.clone(),
        OutputActivityValue::Keyboard(true),
        now,
        true,
    );
    state.output_activity.record(
        owner.clone(),
        OutputActivityValue::Keyboard(false),
        now,
        true,
    );

    let live = live::project_at(&state, &ConfigSnapshot::default(), now);

    assert_eq!(live.output_values, Vec::new());
    assert_eq!(
        live.output_activity,
        vec![OutputActivitySnapshot {
            owner,
            value: OutputActivityValue::Keyboard(true),
        }]
    );
}

#[test]
fn live_snapshot_from_state_at_omits_expired_output_activity() {
    let now = Instant::now();
    let mut state = AppState::new();
    let owner = test_output_owner();
    state
        .output_activity
        .record(owner, OutputActivityValue::Keyboard(true), now, true);
    state.output_activity.record(
        test_output_owner(),
        OutputActivityValue::Keyboard(false),
        now,
        true,
    );

    let live = live::project_at(
        &state,
        &ConfigSnapshot::default(),
        now + inputforge_core::state::OUTPUT_ACTIVITY_PREVIEW_LATCH
            + std::time::Duration::from_millis(1),
    );

    assert!(live.output_activity.is_empty());
}

#[test]
fn meta_from_state_extracts_lifecycle_fields() {
    let mut state = AppState::new();
    state.engine_status = EngineStatus::Running;
    state.current_mode = "FlightAssist".to_owned();
    state.warnings.push("HidHide unavailable".to_owned());
    state.profile_path = Some(PathBuf::from("/tmp/profile.json"));

    let meta = meta::project(&state);
    assert_eq!(meta.engine_status, EngineStatus::Running);
    assert_eq!(meta.current_mode, "FlightAssist");
    assert_eq!(meta.profile_name, None); // active_profile is None
    assert_eq!(meta.profile_path, Some(PathBuf::from("/tmp/profile.json")));
    assert_eq!(meta.warnings, vec!["HidHide unavailable".to_owned()]);
}

#[test]
fn meta_from_state_with_active_profile_maps_name() {
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

    let profile = Profile::new(
        "Hornet".to_owned(),
        vec![],
        modes,
        vec![],
        vec![],
        "Default".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let meta = meta::project(&state);
    assert_eq!(meta.profile_name, Some("Hornet".to_owned()));
}

#[test]
fn config_from_state_clones_devices_and_virtual_devices() {
    use inputforge_core::types::{DeviceDiagnostics, DeviceId, DeviceInfo};

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
        diagnostics: DeviceDiagnostics::default(),
    });
    state.virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X],
        button_count: 4,
        hat_count: 0,
    });

    let cfg = config::project(&state, None);
    assert_eq!(cfg.devices.len(), 1);
    assert_eq!(cfg.devices[0].info.name, "Throttle");
    assert_eq!(cfg.virtual_devices.len(), 1);
    assert_eq!(cfg.virtual_devices[0].button_count, 4);
    assert!(cfg.mapped_inputs.is_empty()); // no profile loaded
    assert!(cfg.mapping_names.is_empty());
}

#[test]
fn config_snapshot_merges_live_and_remembered_device_rows() {
    let live = DeviceState {
        info: test_device("dev-live", "Live Wheel", 4, 12, 1),
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    };
    let remembered = test_device("dev-old", "Old Pedals", 3, 0, 0);
    let mut state = AppState::new();
    state.devices.push(live);
    state
        .device_aliases
        .insert(DeviceId("dev-live".to_owned()), "Rig Wheel".to_owned());
    state.device_registry.insert(
        DeviceId("dev-old".to_owned()),
        inputforge_core::settings::DeviceRecord {
            axis_settings: Vec::new(),
            info: remembered,
            diagnostics: DeviceDiagnostics::default(),
            last_seen_unix_ms: Some(1),
        },
    );

    let snapshot = config::project(&state, None);

    assert_eq!(snapshot.device_panel_rows.len(), 2);
    assert_eq!(snapshot.device_panel_rows[0].display_name, "Rig Wheel");
    assert!(snapshot.device_panel_rows[0].connected);
    assert_eq!(snapshot.device_panel_rows[1].display_name, "Old Pedals");
    assert!(!snapshot.device_panel_rows[1].connected);
}

#[test]
fn removed_device_is_hidden_only_while_disconnected_in_its_profile() {
    let profile = inputforge_core::profile::Profile::from_toml(
        "removed_devices = ['old-stick']\nmodes = ['Default']\n[profile]\nid = 'test-profile'\nname = 'P'\nstartup_mode = 'Default'",
    ).unwrap();
    let mut state = AppState::with_profile(profile);
    let info = test_device("old-stick", "Old Stick", 6, 32, 0);
    state.device_registry.insert(
        info.id.clone(),
        inputforge_core::settings::DeviceRecord {
            axis_settings: vec![],
            info: info.clone(),
            diagnostics: DeviceDiagnostics::default(),
            last_seen_unix_ms: None,
        },
    );
    assert_eq!(config::project(&state, None).device_panel_rows.len(), 0);

    state.devices.push(DeviceState {
        info,
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });
    assert_eq!(config::project(&state, None).device_panel_rows.len(), 1);
    state.devices[0].connected = false;
    assert_eq!(config::project(&state, None).device_panel_rows.len(), 0);

    state.active_profile = Some(inputforge_core::profile::Profile::new(
        "Other profile".to_owned(),
        vec![],
        inputforge_core::mode::Modes::new(vec!["Default".to_owned()]).unwrap(),
        vec![],
        vec![],
        "Default".to_owned(),
    ));
    assert_eq!(config::project(&state, None).device_panel_rows.len(), 1);
}

#[test]
fn removed_device_reappears_if_a_mapping_references_it() {
    let profile = profile_with_primary_merge_and_conditional_device_refs();
    let toml = format!(
        "removed_devices = ['dev-1']\n{}",
        profile.to_toml().unwrap()
    );
    let mut state =
        AppState::with_profile(inputforge_core::profile::Profile::from_toml(&toml).unwrap());
    let info = test_device("dev-1", "Old Stick", 6, 32, 0);
    state.device_registry.insert(
        info.id.clone(),
        inputforge_core::settings::DeviceRecord {
            info,
            axis_settings: vec![],
            diagnostics: DeviceDiagnostics::default(),
            last_seen_unix_ms: None,
        },
    );
    let snapshot = config::project(&state, None);
    assert_eq!(snapshot.device_panel_rows.len(), 1);
    assert_eq!(snapshot.device_panel_rows[0].usage.primary_mappings, 1);
    assert_eq!(snapshot.device_panel_rows[0].usage.secondary_mappings, 2);
}

#[test]
fn config_snapshot_counts_primary_merge_and_conditional_usage() {
    let mut state =
        AppState::with_profile(profile_with_primary_merge_and_conditional_device_refs());
    state.devices.push(DeviceState {
        info: test_device("dev-1", "Stick", 6, 32, 1),
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });

    let snapshot = config::project(&state, None);
    let row = snapshot
        .device_panel_rows
        .iter()
        .find(|row| row.device_id == DeviceId("dev-1".to_owned()))
        .expect("device row");

    assert_eq!(row.usage.primary_mappings, 1);
    assert_eq!(row.usage.secondary_mappings, 2);
    assert_eq!(row.usage.axes.mapped, 1);
    assert_eq!(row.usage.buttons.mapped, 1);
    assert_eq!(row.usage.hats.mapped, 0);
}

#[test]
fn config_snapshot_counts_same_device_merge_axis_as_secondary_usage() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::MergeOp;

    let device = DeviceId("dev-1".to_owned());
    let primary = InputAddress::Bound {
        device: device.clone(),
        input: InputId::Button { index: 0 },
    };
    let merge_axis = InputAddress::Bound {
        device: device.clone(),
        input: InputId::Axis { index: 0 },
    };
    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        vec![Mapping {
            input: primary,
            mode: "Default".to_owned(),
            name: Some("Toe brake blend".to_owned()),
            actions: vec![Action::MergeAxis {
                second_input: merge_axis,
                operation: MergeOp::Average,
            }],
        }],
        vec![],
        "Default".to_owned(),
    );
    let mut state = AppState::with_profile(profile);
    state.devices.push(DeviceState {
        info: test_device("dev-1", "Sim Pedals", 4, 12, 0),
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });

    let snapshot = config::project(&state, None);
    let row = snapshot
        .device_panel_rows
        .iter()
        .find(|row| row.device_id == device)
        .expect("device row");

    assert_eq!(row.usage.primary_mappings, 1);
    assert_eq!(row.usage.secondary_mappings, 1);
    assert_eq!(row.usage.axes.mapped, 1);
}

#[test]
fn live_from_state_handles_empty_config() {
    let state = AppState::new();
    let cfg = config::project(&state, None);
    let live = live::project(&state, &cfg);
    assert!(live.device_inputs.is_empty());
    assert!(live.output_values.is_empty());
}

#[test]
fn live_from_state_reads_caches_per_device_shape() {
    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{
        AxisValue, DeviceDiagnostics, DeviceId, DeviceInfo, InputId, InputValue,
    };

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
        diagnostics: DeviceDiagnostics::default(),
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
    state.output_cache.set_hat(1, 1, HatDirection::SE);

    let cfg = config::project(&state, None);
    let live = live::project(&state, &cfg);

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
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId};

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

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

    let cfg = config::project(&state, None);
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
        AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, VJoyAxis, VirtualDeviceConfig,
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
        diagnostics: DeviceDiagnostics::default(),
    });
    s.virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X, VJoyAxis::Y],
        button_count: 4,
        hat_count: 1,
    });

    let meta = meta::project(&s);
    let cfg = config::project(&s, None);

    // The exact six snapshot fields the placeholder shell surfaces consume:
    assert_eq!(meta.engine_status, EngineStatus::Running);
    assert_eq!(meta.current_mode, "Combat");
    assert_eq!(meta.profile_name, None); // no profile loaded
    assert_eq!(cfg.devices.len(), 1);
    assert_eq!(cfg.virtual_devices.len(), 1);
    assert_eq!(meta.warnings.len(), 1);
}

#[test]
fn meta_from_state_projects_modes_and_startup_mode() {
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;

    let modes = Modes::new(vec![
        "Default".to_owned(),
        "Combat".to_owned(),
        "Landing".to_owned(),
    ])
    .unwrap();
    let profile = Profile::new(
        "Modal".to_owned(),
        vec![],
        modes,
        vec![],
        vec![],
        "Combat".to_owned(),
    );
    let state = AppState::with_profile(profile);
    let meta = meta::project(&state);
    assert_eq!(meta.modes.len(), 3);
    assert_eq!(meta.modes[0], "Default", "DFS pre-order: root first");
    assert_eq!(meta.startup_mode, Some("Combat".to_owned()));
}

#[test]
fn meta_from_state_with_no_profile_has_empty_modes_and_no_startup() {
    let state = AppState::new();
    let meta = meta::project(&state);
    assert!(meta.modes.is_empty());
    assert!(meta.startup_mode.is_none());
}

#[test]
fn config_snapshot_populates_mappings_with_no_glyphs() {
    use inputforge_core::action::Mapping;
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId};

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

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
    let cfg = config::project(&state, None);

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
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId, MergeOp};

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

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
    let cfg = config::project(&state, None);

    let s = &cfg.mappings[0];
    assert_eq!(s.glyphs.merge_secondary.as_ref(), Some(&secondary));
    assert!(s.glyphs.first_input_predicate.is_none());
}

#[test]
fn config_snapshot_glyph_walker_finds_input_conditional() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId};

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

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
    let cfg = config::project(&state, None);

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
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId, MergeOp};

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

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
    let cfg = config::project(&state, None);

    let s = &cfg.mappings[0];
    assert_eq!(s.glyphs.merge_secondary.as_ref(), Some(&secondary));
    assert!(s.glyphs.first_input_predicate.is_some());
}

#[test]
fn config_snapshot_glyph_walker_recurses_through_composite_conditions() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId};

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

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
    let cfg = config::project(&state, None);

    let s = &cfg.mappings[0];
    assert!(
        s.glyphs.first_input_predicate.is_some(),
        "walker must recurse through Not -> Any -> All -> ButtonReleased"
    );
}

#[test]
fn config_snapshot_glyph_walker_descends_into_nested_actions() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId, MergeOp};

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

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
    let cfg = config::project(&state, None);

    let s = &cfg.mappings[0];
    assert_eq!(
        s.glyphs.merge_secondary.as_ref(),
        Some(&secondary),
        "walker must descend into Conditional.if_true to find MergeAxis"
    );
}

#[test]
fn mapping_summary_referenced_devices_dedupes_and_ignores_unbound() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{DeviceId, InputAddress, InputId, MergeOp};

    let dev_a = DeviceId("dev-a".to_owned());
    let primary = InputAddress::Bound {
        device: dev_a.clone(),
        input: InputId::Axis { index: 0 },
    };
    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        vec![Mapping {
            input: primary.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![
                Action::MergeAxis {
                    second_input: InputAddress::Unbound,
                    operation: MergeOp::Average,
                },
                Action::Conditional {
                    condition: Condition::ButtonPressed { input: primary },
                    if_true: vec![],
                    if_false: vec![],
                },
            ],
        }],
        vec![],
        "Default".to_owned(),
    );

    let cfg = config::project(&AppState::with_profile(profile), None);
    assert_eq!(cfg.mappings[0].referenced_devices, vec![dev_a]);
}

#[test]
fn mapping_summary_finds_first_vjoy_output_in_preorder() {
    use inputforge_core::action::{Action, Condition, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, OutputAddress, OutputId, VJoyAxis,
    };

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
    let input = InputAddress::Bound {
        device: DeviceId("stick".to_owned()),
        input: InputId::Axis { index: 0 },
    };
    let true_output = OutputAddress {
        device: 2,
        output: OutputId::Axis { id: VJoyAxis::Y },
    };
    let false_output = OutputAddress {
        device: 3,
        output: OutputId::Axis { id: VJoyAxis::Z },
    };
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        vec![Mapping {
            input: input.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::Conditional {
                condition: Condition::ButtonPressed { input },
                if_true: vec![Action::MapToVJoy {
                    output: true_output.clone(),
                }],
                if_false: vec![Action::MapToVJoy {
                    output: false_output,
                }],
            }],
        }],
        vec![],
        "Default".to_owned(),
    );

    let cfg = config::project(&AppState::with_profile(profile), None);
    assert_eq!(
        cfg.mappings[0].first_vjoy_output.as_ref(),
        Some(&true_output)
    );
}

#[test]
fn config_snapshot_glyph_walker_descends_into_gesture_branches() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use inputforge_core::types::{
        DeviceId, InputAddress, InputId, MergeOp, OutputAddress, OutputId, VJoyAxis,
    };

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();
    let primary = InputAddress::Bound {
        device: DeviceId("stick".to_owned()),
        input: InputId::Button { index: 0 },
    };
    let secondary = InputAddress::Bound {
        device: DeviceId("pedals".to_owned()),
        input: InputId::Axis { index: 1 },
    };
    let output = OutputAddress {
        device: 1,
        output: OutputId::Axis { id: VJoyAxis::X },
    };
    let profile = Profile::new(
        "P".to_owned(),
        vec![],
        modes,
        vec![Mapping {
            input: primary.clone(),
            mode: "Default".to_owned(),
            name: None,
            actions: vec![Action::TapGesture {
                threshold_ms: 300,
                fire_single_immediately: false,
                single_tap: Vec::new(),
                double_tap: vec![
                    Action::MergeAxis {
                        second_input: secondary.clone(),
                        operation: MergeOp::Average,
                    },
                    Action::MapToVJoy {
                        output: output.clone(),
                    },
                ],
            }],
        }],
        vec![],
        "Default".to_owned(),
    );

    let cfg = config::project(&AppState::with_profile(profile), None);
    let summary = &cfg.mappings[0];

    assert_eq!(summary.glyphs.merge_secondary.as_ref(), Some(&secondary));
    assert_eq!(
        summary.referenced_devices,
        vec![DeviceId("stick".to_owned()), DeviceId("pedals".to_owned())]
    );
    assert_eq!(summary.first_vjoy_output.as_ref(), Some(&output));
}

#[test]
fn config_from_state_with_selection_clones_actions() {
    use inputforge_core::action::{Action, Mapping};
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::types::{DeviceId, InputId};

    let modes = Modes::new(vec!["Default".to_owned()]).unwrap();

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
    let cfg = config::project(&state, sel.as_ref());

    assert_eq!(cfg.selected_mapping_actions.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        cfg.selected_mapping_key.as_ref(),
        Some(&("Default".to_owned(), addr.clone()))
    );
}

#[test]
fn config_from_state_without_selection_actions_none() {
    let state = AppState::new();
    let cfg = config::project(&state, None);
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
    let cfg = config::project(&app, stale_sel.as_ref());
    assert!(cfg.selected_mapping_actions.is_none());
    assert_eq!(cfg.selected_mapping_key, stale_sel);
}

fn at(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .expect("rfc3339 timestamp")
        .with_timezone(&chrono::Utc)
}

#[test]
fn relative_time_under_one_minute_is_just_now() {
    let now = at("2026-05-07T12:00:00Z");
    let taken = at("2026-05-07T11:59:30Z");
    assert_eq!(format_relative_at(taken, now), "just now");
}

#[test]
fn relative_time_under_one_hour_is_minutes_ago() {
    let now = at("2026-05-07T12:00:00Z");
    assert_eq!(
        format_relative_at(at("2026-05-07T11:55:00Z"), now),
        "5m ago"
    );
    assert_eq!(
        format_relative_at(at("2026-05-07T11:00:30Z"), now),
        "59m ago"
    );
}

#[test]
fn relative_time_under_one_day_is_hours_ago() {
    let now = at("2026-05-07T12:00:00Z");
    assert_eq!(
        format_relative_at(at("2026-05-07T10:00:00Z"), now),
        "2h ago"
    );
    assert_eq!(
        format_relative_at(at("2026-05-06T12:30:00Z"), now),
        "23h ago"
    );
}

#[test]
fn relative_time_under_one_week_is_days_ago() {
    let now = at("2026-05-07T12:00:00Z");
    assert_eq!(
        format_relative_at(at("2026-05-05T12:00:00Z"), now),
        "2d ago"
    );
    assert_eq!(
        format_relative_at(at("2026-05-01T12:00:00Z"), now),
        "6d ago"
    );
}

#[test]
fn relative_time_one_week_or_older_falls_back_to_iso_date() {
    let now = at("2026-05-07T12:00:00Z");
    assert_eq!(
        format_relative_at(at("2026-04-30T12:00:00Z"), now),
        "2026-04-30"
    );
}

#[test]
fn relative_time_negative_delta_collapses_to_iso_date() {
    // Clock skew: snapshot timestamp is in the future relative to
    // `now`. We avoid showing "in 2m" (jarring); we show the date.
    let now = at("2026-05-07T12:00:00Z");
    assert_eq!(
        format_relative_at(at("2026-05-07T12:02:00Z"), now),
        "2026-05-07"
    );
}

#[test]
fn absolute_time_uses_local_clock_and_seasonal_offset_without_subseconds() {
    use chrono::TimeZone;

    for (month, day, hour, expected_clock) in [
        (10, 12, 22, "2026-10-12 22:30"),
        (1, 12, 22, "2026-01-12 22:30"),
        (5, 8, 0, "2026-05-08 00:30"),
    ] {
        let local = chrono::Local
            .with_ymd_and_hms(2026, month, day, hour, 30, 59)
            .single()
            .expect("unambiguous local timestamp");
        let taken = local.with_timezone(&chrono::Utc) + chrono::Duration::nanoseconds(659_262_500);
        assert_eq!(
            format_absolute_time(taken),
            format!("{expected_clock} {}", local.offset())
        );
    }
}

#[test]
fn device_display_name_returns_alias_when_present() {
    let mut state = AppState::new();
    state.devices.push(DeviceState {
        info: test_device("dev-live", "Generic Joystick", 4, 16, 0),
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });
    state
        .device_aliases
        .insert(DeviceId("dev-live".to_owned()), "Rig Wheel".to_owned());
    let snapshot = config::project(&state, None);
    assert_eq!(
        snapshot.device_display_name(&DeviceId("dev-live".to_owned())),
        "Rig Wheel"
    );
}

#[test]
fn device_display_name_falls_back_to_hardware_name_when_alias_blank() {
    let mut state = AppState::new();
    state.devices.push(DeviceState {
        info: test_device("dev-1", "Generic HID Joystick", 0, 0, 0),
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });
    state
        .device_aliases
        .insert(DeviceId("dev-1".to_owned()), "   ".to_owned());
    let snapshot = config::project(&state, None);
    assert_eq!(
        snapshot.device_display_name(&DeviceId("dev-1".to_owned())),
        "Generic HID Joystick"
    );
}

#[test]
fn device_display_name_returns_alias_for_remembered_disconnected_device() {
    let mut state = AppState::new();
    state.device_registry.insert(
        DeviceId("dev-old".to_owned()),
        inputforge_core::settings::DeviceRecord {
            axis_settings: Vec::new(),
            info: test_device("dev-old", "Old Pedals", 0, 0, 0),
            diagnostics: DeviceDiagnostics::default(),
            last_seen_unix_ms: Some(1),
        },
    );
    state
        .device_aliases
        .insert(DeviceId("dev-old".to_owned()), "Track Pedals".to_owned());
    // Not in s.devices: the device is remembered but disconnected.
    let snapshot = config::project(&state, None);
    assert_eq!(
        snapshot.device_display_name(&DeviceId("dev-old".to_owned())),
        "Track Pedals"
    );
}

#[test]
fn device_display_name_returns_id_for_unknown_device() {
    let state = AppState::new();
    let snapshot = config::project(&state, None);
    // Snapshot has not seen this device anywhere; fall back to id.
    assert_eq!(
        snapshot.device_display_name(&DeviceId("ghost".to_owned())),
        "ghost"
    );
}

#[test]
fn device_display_name_connected_device_overrides_registry_record() {
    // Same DeviceId in both `device_registry` (with stale name)
    // and `s.devices` (live with current name). The connected
    // entry must win on collision so the user sees the
    // up-to-date hardware identity, not the stale one.
    let mut state = AppState::new();
    state.devices.push(DeviceState {
        info: test_device("dev-1", "Live Stick", 0, 0, 0),
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });
    state.device_registry.insert(
        DeviceId("dev-1".to_owned()),
        inputforge_core::settings::DeviceRecord {
            axis_settings: Vec::new(),
            info: test_device("dev-1", "Stale Stick", 0, 0, 0),
            diagnostics: DeviceDiagnostics::default(),
            last_seen_unix_ms: Some(1),
        },
    );
    let snapshot = config::project(&state, None);
    assert_eq!(
        snapshot.device_display_name(&DeviceId("dev-1".to_owned())),
        "Live Stick"
    );
}

#[test]
fn settings_snapshot_default_is_zero_count() {
    let snap = SettingsSnapshot::default();
    assert_eq!(snap.unpinned_snapshot_count, 0);
    assert_eq!(snap.default_double_tap_threshold_ms, 500);
    assert_eq!(snap.default_long_press_threshold_ms, 500);
}

#[test]
fn settings_snapshot_from_state_no_profile_yields_zero_count() {
    use inputforge_core::state::AppState;
    let state = AppState::new();
    let snap = settings::project(&state);
    assert_eq!(
        snap.unpinned_snapshot_count, 0,
        "no profile loaded must yield 0 unpinned"
    );
    assert_eq!(snap.snapshot, state.snapshot_config);
}

#[test]
fn settings_snapshot_from_state_mirrors_snapshot_config_field() {
    use inputforge_core::snapshot::SnapshotConfig;
    use inputforge_core::state::AppState;

    let mut state = AppState::new();
    state.snapshot_config = SnapshotConfig {
        max_count: 42,
        skip_if_unchanged: false,
    };

    let snap = settings::project(&state);
    assert_eq!(snap.snapshot.max_count, 42);
    assert!(!snap.snapshot.skip_if_unchanged);
    assert_eq!(snap.unpinned_snapshot_count, 0);
}

#[test]
fn settings_snapshot_from_state_counts_unpinned_active_rows() {
    use chrono::DateTime;
    use inputforge_core::snapshot::{SnapshotId, SnapshotKind};
    use inputforge_core::state::{ActiveSnapshotRow, AppState};
    use ulid::Ulid;

    fn row(pinned: bool) -> ActiveSnapshotRow {
        ActiveSnapshotRow {
            id: SnapshotId(Ulid::nil()),
            kind: SnapshotKind::Manual,
            label: None,
            taken_at: DateTime::from_timestamp(0, 0).unwrap(),
            pinned,
        }
    }

    let mut state = AppState::new();
    state.active_snapshot_rows = vec![row(true), row(false), row(true), row(false)];

    let snap = settings::project(&state);
    assert_eq!(
        snap.unpinned_snapshot_count, 2,
        "should count only unpinned rows"
    );
}

#[test]
fn settings_snapshot_from_state_mirrors_startup() {
    use inputforge_core::settings::StartupSettings;
    use inputforge_core::state::AppState;

    let mut state = AppState::new();
    state.startup = StartupSettings {
        launch_at_startup: true,
        start_minimized_to_tray: true,
    };
    let snap = settings::project(&state);
    assert_eq!(snap.startup, state.startup);
}

#[test]
fn settings_snapshot_from_state_mirrors_gesture_defaults() {
    let mut state = AppState::new();
    state.default_double_tap_threshold_ms = 325;
    state.default_long_press_threshold_ms = 725;

    let snapshot = settings::project(&state);

    assert_eq!(snapshot.default_double_tap_threshold_ms, 325);
    assert_eq!(snapshot.default_long_press_threshold_ms, 725);
}
