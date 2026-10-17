//! Device requests cannot follow queued loads or native identity replacements.
use super::{dev_id, make_engine_with_simple_disk_profile, make_profile, simple_modes};
use crate::engine::{DeviceAction, DeviceRequest, DeviceTarget, EngineCommand};
use crate::types::DeviceDiagnostics;
fn request(s: &mut crate::state::AppState, action: DeviceAction) -> EngineCommand {
    s.device_registry
        .entry(dev_id())
        .or_insert_with(|| crate::settings::DeviceRecord {
            info: crate::types::DeviceInfo {
                id: dev_id(),
                name: "Stick".into(),
                axes: 1,
                buttons: 1,
                hats: 0,
                instance_path: Some("native".into()),
                axis_polarities: vec![],
            },
            diagnostics: DeviceDiagnostics::default(),
            axis_settings: vec![],
            last_seen_unix_ms: None,
        });
    EngineCommand::Device(Box::new(DeviceRequest {
        target: DeviceTarget::capture(s, &dev_id()).unwrap(),
        action,
    }))
}
#[test]
fn queued_profile_load_rejects_alias_and_removal_of_old_device() {
    for action in [
        DeviceAction::Alias(Some("Old edit".into())),
        DeviceAction::Remove,
    ] {
        let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
        let cmd = request(&mut state.write(), action);
        let other = path.with_file_name("Bravo.toml");
        make_profile(simple_modes(), vec![]).save(&other).unwrap();
        let bytes = std::fs::read(&other).unwrap();
        tx.send(EngineCommand::LoadProfile(other.clone())).unwrap();
        tx.send(cmd).unwrap();
        engine.tick().unwrap();
        assert_eq!(std::fs::read(other).unwrap(), bytes);
        assert!(!state.read().device_aliases.contains_key(&dev_id()));
        assert!(
            state
                .read()
                .warnings
                .iter()
                .any(|w| w.contains("Device action cancelled"))
        );
    }
}
#[test]
fn changed_or_missing_native_device_rejects_alias_before_settings_write() {
    for missing in [false, true] {
        let (mut engine, state, tx, _dir, _path) = make_engine_with_simple_disk_profile();
        let cmd = request(&mut state.write(), DeviceAction::Alias(Some("No".into())));
        if missing {
            state.write().devices.clear();
            state.write().device_registry.clear();
        } else {
            state
                .write()
                .device_registry
                .get_mut(&dev_id())
                .unwrap()
                .info
                .instance_path = Some("replacement".into());
        }
        tx.send(cmd).unwrap();
        engine.tick().unwrap();
        assert!(!state.read().device_aliases.contains_key(&dev_id()));
        assert_ne!(state.read().warnings, Vec::<String>::new());
    }
}
#[test]
fn failed_alias_save_keeps_confirmed_settings_and_reports_failure() {
    let (mut engine, state, tx, dir, _path) = make_engine_with_simple_disk_profile();
    let cmd = request(
        &mut state.write(),
        DeviceAction::Alias(Some("Unsaved".into())),
    );
    engine.settings_path = dir.path().to_path_buf();
    tx.send(cmd).unwrap();
    engine.tick().unwrap();
    assert!(!state.read().device_aliases.contains_key(&dev_id()));
    assert!(!engine.settings.device_aliases.contains_key(&dev_id()));
    assert!(
        state
            .read()
            .warnings
            .iter()
            .any(|w| w.contains("Could not update device"))
    );
}
#[test]
fn removal_rechecks_connected_and_new_nested_dependencies_at_execution() {
    use crate::{
        action::{Action, Condition},
        types::{InputAddress, InputId},
    };
    for connected in [false, true] {
        let (mut engine, state, tx, _dir, path) = make_engine_with_simple_disk_profile();
        let cmd = request(&mut state.write(), DeviceAction::Remove);
        let before = std::fs::read(&path).unwrap();
        if connected {
            let info = state.read().device_registry[&dev_id()].info.clone();
            state.write().devices.push(crate::state::DeviceState {
                info,
                connected: true,
                diagnostics: DeviceDiagnostics::default(),
            });
        } else {
            state.write().active_profile.as_mut().unwrap().set_mapping(
                &InputAddress::Unbound,
                "Default",
                None,
                vec![Action::Conditional {
                    condition: Condition::Not {
                        condition: Box::new(Condition::ButtonPressed {
                            input: InputAddress::Bound {
                                device: dev_id(),
                                input: InputId::Button { index: 0 },
                            },
                        }),
                    },
                    if_true: vec![],
                    if_false: vec![],
                }],
            );
        }
        tx.send(cmd).unwrap();
        engine.tick().unwrap();
        assert_eq!(std::fs::read(path).unwrap(), before);
        assert!(
            !state
                .read()
                .active_profile
                .as_ref()
                .unwrap()
                .removed_devices()
                .contains(&dev_id())
        );
        assert_ne!(state.read().warnings, Vec::<String>::new());
    }
}
#[test]
fn current_alias_request_reuses_existing_global_persistence() {
    let (mut engine, state, tx, _dir, _path) = make_engine_with_simple_disk_profile();
    tx.send(request(
        &mut state.write(),
        DeviceAction::Alias(Some("  Saved alias  ".into())),
    ))
    .unwrap();
    engine.tick().unwrap();
    assert_eq!(state.read().device_aliases[&dev_id()], "Saved alias");
    let persisted = crate::settings::AppSettings::load_from(&engine.settings_path);
    assert_eq!(persisted.device_aliases[&dev_id()], "Saved alias");
}
