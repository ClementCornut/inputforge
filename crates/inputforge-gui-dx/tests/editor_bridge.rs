//! The public renderer installs the app poller once and owns its lifetime.
use dioxus::prelude::*;
use inputforge_core::{
    mode::Modes,
    profile::Profile,
    settings::{AxisSetting, DeviceRecord},
    state::AppState,
    types::{AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo},
};
use inputforge_gui_dx::{EditorNavigation, PanelSlot, render_editor, routing::Bindings};
use parking_lot::RwLock;
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, mpsc},
    time::Duration,
};

#[derive(Clone, Props)]
struct EditorProps {
    state: Arc<RwLock<AppState>>,
    starts: Rc<Cell<usize>>,
}
impl PartialEq for EditorProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state) && Rc::ptr_eq(&self.starts, &other.starts)
    }
}
fn editor(props: EditorProps) -> Element {
    let navigation = EditorNavigation {
        panel: use_signal(|| PanelSlot::Devices),
        calibration: use_signal(|| false),
    };
    use_context_provider(|| navigation);
    let (commands, _) = mpsc::channel();
    let capture = use_hook(|| {
        inputforge_app::live_capture::Controller::new(Arc::clone(&props.state), commands.clone())
    });
    let tick_capture = capture.clone();
    let state = Arc::clone(&props.state);
    let start = Callback::new(move |publisher| {
        props.starts.set(props.starts.get() + 1);
        inputforge_app::bridge::spawn_polling_task(
            Arc::clone(&state),
            publisher,
            tick_capture.clone(),
        );
    });
    let controllers =
        inputforge_app::controllers::bindings(Arc::clone(&props.state), commands.clone());
    let keyboard =
        use_hook(|| inputforge_app::keyboard_capture::Controller::new(Arc::clone(&props.state)));
    let profiles = use_hook(|| {
        inputforge_app::profiles::Controller::new(Arc::clone(&props.state), commands.clone())
    });
    let settings = use_hook(|| {
        inputforge_app::settings::Controller::new(Arc::clone(&props.state), commands.clone())
    });
    render_editor(
        inputforge_gui_dx::RawHandles {
            state: Arc::clone(&props.state),
            commands: (commands).clone(),
        },
        Bindings {
            dispatch: Callback::new(|_| {}),
        },
        controllers,
        capture.bindings(),
        keyboard.bindings(),
        use_hook(|| {
            inputforge_app::mapping_edits::Controller::new(
                Arc::clone(&props.state),
                (commands).clone(),
            )
        })
        .bindings(),
        profiles.bindings(),
        settings.bindings(),
        start,
    )
}
fn profile(name: &str) -> Profile {
    Profile::new(
        name.into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    )
}
async fn drive(dom: &mut VirtualDom, millis: u64) {
    tokio::time::advance(Duration::from_millis(millis)).await;
    dom.render_immediate_to_vec();
}
fn fixture() -> (Arc<RwLock<AppState>>, DeviceId) {
    let mut state = AppState::with_profile(profile("Initial fixture"));
    state
        .virtual_devices
        .push(inputforge_core::types::VirtualDeviceConfig {
            device_id: 3,
            axes: vec![],
            button_count: 8,
            hat_count: 1,
        });
    let axis_device = DeviceId("polling-axis".into());
    state.session.bindings = vec![inputforge_core::profile::controllers::DeviceBinding {
        device: axis_device.clone(),
        observed_layout: None,
        unavailable: vec![],
        buttons: vec![],
        hats: vec![],
        axes: vec![inputforge_core::profile::controllers::AxisBinding {
            code: 24,
            minimum: -100,
            maximum: 100,
            polarity: AxisPolarity::Bipolar,
        }],
    }];
    state.device_registry.insert(
        axis_device.clone(),
        DeviceRecord {
            info: DeviceInfo {
                id: axis_device.clone(),
                name: "Polling stick".into(),
                axes: 1,
                buttons: 0,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![],
            },
            diagnostics: DeviceDiagnostics::default(),
            last_seen_unix_ms: None,
            axis_settings: vec![AxisSetting {
                code: 24,
                override_polarity: None,
                detected: None,
            }],
        },
    );
    (Arc::new(RwLock::new(state)), axis_device)
}
#[test]
#[expect(
    clippy::await_holding_lock,
    reason = "hold the writer through a tick to verify nonblocking contention skips"
)]
fn renderer_preserves_tick_cadence_skips_contention_and_releases_polling_on_drop() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (state, axis_device) = fixture();
        let starts = Rc::new(Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            editor,
            EditorProps {
                state: Arc::clone(&state),
                starts: Rc::clone(&starts),
            },
        );
        dom.rebuild_in_place();
        drive(&mut dom, 0).await;
        assert!(dioxus_ssr::render(&dom).contains("Initial fixture"));
        assert!(dioxus_ssr::render(&dom).contains("Virtual controller 3"));
        assert!(dioxus_ssr::render(&dom).contains("Not detected yet"));
        let mut next = profile("Next fixture");
        next.set_modes(Modes::new(vec!["Default".into(), "Prepared mode".into()]).unwrap());
        state.write().active_profile = Some(next);
        state.write().virtual_devices[0].device_id = 4;
        state
            .write()
            .device_registry
            .get_mut(&axis_device)
            .unwrap()
            .axis_settings[0]
            .detected = Some(AxisPolarity::Unipolar);
        drive(&mut dom, 15).await;
        assert!(!dioxus_ssr::render(&dom).contains("Next fixture"));
        assert!(!dioxus_ssr::render(&dom).contains("data-mode=\"Prepared mode\""));
        assert!(!dioxus_ssr::render(&dom).contains("Virtual controller 4"));
        assert!(!dioxus_ssr::render(&dom).contains("Detected: rests at minimum"));
        drive(&mut dom, 1).await;
        assert!(dioxus_ssr::render(&dom).contains("Next fixture"));
        assert!(dioxus_ssr::render(&dom).contains("data-mode=\"Prepared mode\""));
        assert!(dioxus_ssr::render(&dom).contains("Virtual controller 4"));
        assert!(dioxus_ssr::render(&dom).contains("Detected: rests at minimum"));
        dom.mark_dirty(ScopeId::ROOT);
        dom.render_immediate_to_vec();
        assert_eq!(starts.get(), 1, "rerender must not install a second poller");
        let blocked_html = {
            let mut guard = state.write();
            let mut recovered = profile("Recovered fixture");
            recovered
                .set_modes(Modes::new(vec!["Default".into(), "Recovered mode".into()]).unwrap());
            guard.active_profile = Some(recovered);
            guard
                .device_registry
                .get_mut(&axis_device)
                .unwrap()
                .axis_settings[0]
                .detected = Some(AxisPolarity::Bipolar);
            drive(&mut dom, 16).await;
            dioxus_ssr::render(&dom)
        };
        assert!(!blocked_html.contains("Recovered fixture"));
        assert!(!blocked_html.contains("data-mode=\"Recovered mode\""));
        assert!(!blocked_html.contains("Detected: centered"));
        drive(&mut dom, 16).await;
        assert!(dioxus_ssr::render(&dom).contains("Recovered fixture"));
        assert!(dioxus_ssr::render(&dom).contains("data-mode=\"Recovered mode\""));
        assert!(dioxus_ssr::render(&dom).contains("Detected: centered"));
        drop(dom);
        assert_eq!(
            Arc::strong_count(&state),
            1,
            "scope teardown must release every engine handle"
        );
        let mut remount = VirtualDom::new_with_props(
            editor,
            EditorProps {
                state: Arc::clone(&state),
                starts: Rc::clone(&starts),
            },
        );
        remount.rebuild_in_place();
        drive(&mut remount, 0).await;
        assert_eq!(starts.get(), 2);
        assert!(dioxus_ssr::render(&remount).contains("Recovered fixture"));
        assert!(dioxus_ssr::render(&remount).contains("data-mode=\"Recovered mode\""));
        assert!(dioxus_ssr::render(&remount).contains("Detected: centered"));
        drop(remount);
        assert_eq!(Arc::strong_count(&state), 1);
    });
}

#[path = "editor_bridge/batch.rs"]
mod batch;
