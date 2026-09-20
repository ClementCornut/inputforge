use super::*;

#[test]
fn layout_edits_wait_for_explicit_apply_while_selection_does_not_recreate_outputs() {
    let (mut e, tx, s, _temp) = harness();
    tx.send(EngineCommand::Activate).unwrap();
    e.tick().unwrap();
    tx.send(EngineCommand::Deactivate).unwrap();
    e.tick().unwrap();
    s.lock().calls.clear();
    e.handle_command(EngineCommand::SelectControllers(vec![]))
        .unwrap();
    let mut config = e
        .state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .controllers()
        .unwrap()
        .clone();
    config.virtual_devices[0].button_count = 3;
    e.handle_command(EngineCommand::SetControllerConfig(config))
        .unwrap();
    assert!(s.lock().calls.is_empty());
    let error = e.handle_command(EngineCommand::Activate).unwrap_err();
    assert!(error.to_string().contains("Apply controller changes"));
    assert!(e.state.read().session.output_active);
    e.handle_command(EngineCommand::ApplyControllerChanges)
        .unwrap();
    assert_eq!(s.lock().calls, ["release", "stop", "start", "neutral"]);
    assert_eq!(e.read_status(), EngineStatus::Stopped);
    s.lock().calls.clear();
    e.handle_command(EngineCommand::Activate).unwrap();
    assert!(s.lock().calls.is_empty());
    assert_eq!(e.read_status(), EngineStatus::Running);
}

#[test]
fn profile_load_reuses_matching_layout_and_requires_apply_for_changed_layout() {
    let (mut e, _tx, s, temp) = harness();
    e.handle_command(EngineCommand::Activate).unwrap();
    let mut profile = e.state.read().active_profile.clone().unwrap();
    let path = temp.path().join("replacement.toml");
    profile.save(&path).unwrap();
    s.lock().calls.clear();
    e.handle_command(EngineCommand::LoadProfile(path.clone()))
        .unwrap();
    assert_eq!(e.read_status(), EngineStatus::Stopped);
    assert!(!s.lock().calls.iter().any(|call| call == "stop"));
    e.handle_command(EngineCommand::Activate).unwrap();
    assert!(!s.lock().calls.iter().any(|call| call == "start"));
    let mut config = profile.controllers().unwrap().clone();
    config.virtual_devices[0].button_count += 1;
    profile.set_controllers(config).unwrap();
    profile.save(&path).unwrap();
    e.handle_command(EngineCommand::LoadProfile(path)).unwrap();
    assert!(e.handle_command(EngineCommand::Activate).is_err());
    assert!(e.state.read().session.output_active);
    e.handle_command(EngineCommand::ApplyControllerChanges)
        .unwrap();
    assert_eq!(e.read_status(), EngineStatus::Stopped);
}

#[test]
fn fixed_output_publishes_native_layout_and_reuses_compatible_slot() {
    let native = VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::Y, VJoyAxis::X, VJoyAxis::Z],
        button_count: 8,
        hat_count: 2,
    };
    let (mut e, _tx, s, temp) = fixed_harness(vec![native.clone()]);
    let mut profile = e.state.read().active_profile.clone().unwrap();
    let mut config = profile.controllers().unwrap().clone();
    config.virtual_devices[0] = VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X, VJoyAxis::Y],
        button_count: 4,
        hat_count: 1,
    };
    profile.set_controllers(config).unwrap();
    let path = temp.path().join("fixed.toml");
    profile.save(&path).unwrap();

    e.handle_command(EngineCommand::LoadProfile(path)).unwrap();
    assert_eq!(
        e.state.read().virtual_devices,
        std::slice::from_ref(&native)
    );

    e.handle_command(EngineCommand::Activate).unwrap();
    assert_eq!(
        e.state.read().session.output_layout,
        std::slice::from_ref(&native)
    );
    e.handle_command(EngineCommand::Deactivate).unwrap();
    s.lock().calls.clear();
    e.handle_command(EngineCommand::SelectControllers(vec![]))
        .unwrap();
    assert_eq!(
        e.state.read().virtual_devices,
        std::slice::from_ref(&native)
    );
    e.handle_command(EngineCommand::Activate).unwrap();
    assert!(!s.lock().calls.iter().any(|call| call == "start"));
}

#[test]
fn reconfiguration_rejects_running_and_invalid_layouts_before_destroying_outputs() {
    let (mut e, _tx, s, _temp) = harness();
    e.handle_command(EngineCommand::Activate).unwrap();
    s.lock().calls.clear();
    assert!(
        e.handle_command(EngineCommand::ApplyControllerChanges)
            .is_err()
    );
    assert!(s.lock().calls.is_empty());
    e.handle_command(EngineCommand::Deactivate).unwrap();
    let mut config = e
        .state
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .controllers()
        .unwrap()
        .clone();
    config.virtual_devices[0].button_count = 3;
    e.handle_command(EngineCommand::SetControllerConfig(config))
        .unwrap();
    s.lock().start_failures_remaining = 1;
    assert!(
        e.handle_command(EngineCommand::ApplyControllerChanges)
            .is_err()
    );
    assert_eq!(e.read_status(), EngineStatus::Faulted);
    assert!(!e.state.read().session.output_active);
    e.handle_command(EngineCommand::ApplyControllerChanges)
        .unwrap();
    assert_eq!(e.read_status(), EngineStatus::Stopped);
    assert!(e.state.read().session.output_active);
}
