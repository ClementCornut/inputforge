//! Copy policy exercises actual mounted dispatch while replacing only native clipboard I/O.
use super::*;
use inputforge_core::{
    settings::DeviceRecord,
    types::{DeviceId, DeviceInfo},
};
fn root() -> Element {
    rsx! {div{}}
}
#[test]
fn copy_writes_exactly_once_releases_engine_lock_and_reports_errors() {
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    dom.in_scope(ScopeId::ROOT, || {
        let mut state = AppState::new();
        let id = DeviceId("stick".into());
        state.device_registry.insert(
            id.clone(),
            DeviceRecord {
                info: DeviceInfo {
                    id,
                    name: "Stick".into(),
                    axes: 0,
                    buttons: 1,
                    hats: 0,
                    instance_path: None,
                    axis_polarities: vec![],
                },
                diagnostics: inputforge_core::types::DeviceDiagnostics::default(),
                axis_settings: vec![],
                last_seen_unix_ms: None,
            },
        );
        let state = Arc::new(RwLock::new(state));
        let (tx, _rx) = mpsc::channel();
        let c = Controller::new(Arc::clone(&state), tx);
        let view = Signal::new(View::default());
        let owner = c.mount(view);
        let workflow = view.peek().workflow;
        assert!(c.dispatch(owner, workflow, Intent::Input("retained draft".into())));
        for fail in [false, true] {
            let writes = std::cell::Cell::new(0);
            let result = c.dispatch_with(
                owner,
                workflow,
                Intent::Copy("Device ID: stick".into()),
                |text| {
                    assert_eq!(text, "Device ID: stick");
                    assert!(
                        state.try_write().is_some(),
                        "engine guard held during clipboard write"
                    );
                    writes.set(writes.get() + 1);
                    if fail { Err("denied".into()) } else { Ok(()) }
                },
            );
            assert_eq!(result, !fail);
            assert_eq!(writes.get(), 1);
            assert_eq!(view.peek().error.is_some(), fail);
            assert_eq!(view.peek().draft, "retained draft");
        }
        c.unmount(owner);
        assert!(
            !c.dispatch_with(owner, workflow, Intent::Copy("old".into()), |_| panic!(
                "obsolete clipboard write"
            ))
        );
    });
}
