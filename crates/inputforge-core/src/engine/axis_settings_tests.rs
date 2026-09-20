use super::*;
use crate::{
    device::MockInputSource,
    output::{MockKeyboardSink, MockMouseSink, MockOutputSink},
    settings::{AxisSetting, DeviceRecord},
    types::{AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo},
};

fn device() -> DeviceInfo {
    DeviceInfo {
        id: DeviceId("test-controller".into()),
        name: "Test".into(),
        axes: 1,
        buttons: 0,
        hats: 0,
        instance_path: None,
        axis_polarities: vec![],
    }
}
fn engine(settings: AppSettings, path: PathBuf) -> Engine {
    let (_tx, rx) = mpsc::channel();
    Engine::new(
        Box::new(MockInputSource {
            devices: vec![device()],
            ..Default::default()
        }),
        Box::new(MockOutputSink::new()),
        Box::new(MockKeyboardSink::new()),
        Box::new(MockMouseSink::new()),
        Arc::new(RwLock::new(AppState::new())),
        rx,
        settings,
        path,
        Box::new(inputforge_autostart::mock::MockAutostart::default()),
    )
}
fn settings() -> AppSettings {
    let mut settings = AppSettings::default();
    settings.device_registry.insert(
        device().id,
        DeviceRecord {
            info: device(),
            diagnostics: DeviceDiagnostics::default(),
            last_seen_unix_ms: None,
            axis_settings: vec![],
        },
    );
    settings
}
fn axis(e: &Engine) -> &AxisSetting {
    &e.settings.device_registry[&device().id].axis_settings[0]
}

#[test]
fn detection_override_auto_and_redetection_survive_restart_without_profile() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.toml");
    let mut e = engine(settings(), path.clone());
    e.observe_axis(&device().id, 0, -1.0).unwrap();
    e.observe_axis(&device().id, 0, 0.0).unwrap();
    assert_eq!(axis(&e).detected, Some(AxisPolarity::Unipolar));
    e.set_axis_polarity(&device().id, 0, Some(AxisPolarity::Bipolar))
        .unwrap();
    drop(e);
    let mut e = engine(AppSettings::load_from(&path), path);
    assert_eq!(axis(&e).effective(), AxisPolarity::Bipolar);
    e.set_axis_polarity(&device().id, 0, None).unwrap();
    assert_eq!(axis(&e).effective(), AxisPolarity::Unipolar);
    e.detect_axis(&device().id, 0).unwrap();
    assert_eq!(axis(&e).detected, None);
    e.observe_axis(&device().id, 0, 0.0).unwrap();
    assert_eq!(axis(&e).effective(), AxisPolarity::Bipolar);
}

#[test]
fn failed_save_keeps_previous_axis_settings_and_reports_failure() {
    let temp = tempfile::tempdir().unwrap();
    let mut e = engine(settings(), temp.path().into()); // A directory cannot replace a settings file.
    assert!(
        e.set_axis_polarity(&device().id, 0, Some(AxisPolarity::Unipolar))
            .is_err()
    );
    assert!(
        e.settings.device_registry[&device().id]
            .axis_settings
            .is_empty()
    );
    assert!(
        e.state
            .read()
            .warnings
            .iter()
            .any(|w| w.contains("Could not save axis settings"))
    );
}

#[test]
fn legacy_axis_choices_import_once_and_existing_controller_setting_wins() {
    let temp = tempfile::tempdir().unwrap();
    for existing in [false, true] {
        let mut settings = settings();
        if existing {
            settings
                .device_registry
                .get_mut(&device().id)
                .unwrap()
                .axis_settings
                .push(AxisSetting {
                    code: 0,
                    detected: Some(AxisPolarity::Bipolar),
                    override_polarity: None,
                });
        }
        let mut e = engine(settings, temp.path().join("settings.toml"));
        let mut profile = crate::profile::Profile::new(
            "Legacy".into(),
            vec![],
            crate::mode::Modes::new(vec!["Default".into()]).unwrap(),
            vec![],
            vec![],
            "Default".into(),
        );
        let mut binding = e.input.binding_table(&device().id).unwrap().unwrap();
        binding.axes[0].polarity = AxisPolarity::Unipolar;
        profile
            .set_controllers(crate::profile::controllers::ControllerConfig {
                legacy_axis_settings: true,
                selected: vec![device().id],
                bindings: vec![binding],
                ..Default::default()
            })
            .unwrap();
        e.state.write().active_profile = Some(profile);
        e.sync_controller_bindings().unwrap();
        assert_eq!(
            axis(&e).effective(),
            if existing {
                AxisPolarity::Bipolar
            } else {
                AxisPolarity::Unipolar
            }
        );
        assert!(
            !e.state
                .read()
                .active_profile
                .as_ref()
                .unwrap()
                .controllers()
                .unwrap()
                .legacy_axis_settings
        );
    }
}
