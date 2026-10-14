use super::support::{change, click, mount};
use inputforge_core::engine::EngineCommand;
#[test]
fn row_target_reset_and_group_controls_forward_real_app_intents() {
    let (mut dom, _, rx) = mount();
    change(&dom, "bulk-map-row-A-0", "axis:Y");
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains(">Reset</button>"));
    click(&dom, "Reset");
    dom.render_immediate_to_vec();
    assert!(!dioxus_ssr::render(&dom).contains(">Reset</button>"));
    change(&dom, "bulk-map-row-H-0", "(do not map)");
    dom.render_immediate_to_vec();
    click(&dom, "Apply 12 mappings");
    assert!(
        matches!(rx.try_recv().unwrap(), EngineCommand::SetMappingsBulk { entries, .. } if entries.len() == 12 && !entries.iter().any(|e| matches!(e.input, inputforge_core::types::InputAddress::Bound { input: inputforge_core::types::InputId::Hat { .. }, .. })))
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        EngineCommand::MappingEditBarrier { .. }
    ));
}
#[test]
fn invalid_target_has_visible_error_and_valid_edit_clears_it() {
    let (mut dom, _, rx) = mount();
    change(&dom, "bulk-map-row-A-0", "button:1");
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("role=\"alert\""));
    change(&dom, "bulk-map-row-A-0", "axis:Y");
    dom.render_immediate_to_vec();
    assert!(!dioxus_ssr::render(&dom).contains("role=\"alert\""));
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}

#[test]
fn metadata_and_collision_controls_select_current_source_target_and_modes() {
    use inputforge_core::{
        mode::Modes,
        profile::Profile,
        types::{DeviceId, InputAddress, InputId, OutputId, VJoyAxis},
    };
    let mut state = super::super::tests::seeded_state(true);
    state.active_profile = Some(Profile::new(
        "T".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Other".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    ));
    let mut source = state.devices[0].clone();
    source.info.id = DeviceId("second".into());
    source.info.name = "Wheel".into();
    source.info.axes = 1;
    source.info.buttons = 1;
    source.info.hats = 0;
    state.devices.push(source);
    let mut target = state.virtual_devices[0].clone();
    target.device_id = 2;
    target.axes = vec![VJoyAxis::Slider0];
    target.button_count = 1;
    state.virtual_devices.push(target);
    state.active_profile.as_mut().unwrap().set_mapping(
        &InputAddress::Bound {
            device: DeviceId("second".into()),
            input: InputId::Axis { index: 0 },
        },
        "Default",
        None,
        vec![],
    );
    let (mut dom, _, rx) = super::support::mount_state(state);
    for (id, value) in [
        ("bulk-map-source", "second"),
        ("bulk-map-target", "2"),
        ("bulk-map-mode", "Other"),
        ("bulk-map-all-modes", "true"),
    ] {
        change(&dom, id, value);
        dom.render_immediate_to_vec();
    }
    assert!(dioxus_ssr::render(&dom).contains("Apply 3 mappings"));
    click(&dom, "Replace all conflicts");
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("Apply 4 mappings"));
    click(&dom, "Skip all conflicts");
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("Apply 3 mappings"));
    click(&dom, "Replace");
    dom.render_immediate_to_vec();
    click(&dom, "Apply 4 mappings");
    let EngineCommand::SetMappingsBulk {
        entries,
        snapshot_label,
    } = rx.try_recv().unwrap()
    else {
        panic!("batch expected")
    };
    assert_eq!(snapshot_label, "Wheel · Device 2");
    assert_eq!(entries.len(), 4);
    assert!(entries.iter().all(|e| e.output.device == 2
        && matches!(&e.input, InputAddress::Bound { device, .. } if device.0 == "second")));
    assert_eq!(entries.iter().filter(|e| e.mode == "Other").count(), 2);
    assert!(matches!(
        entries[0].output.output,
        OutputId::Axis {
            id: VJoyAxis::Slider0
        }
    ));
}
