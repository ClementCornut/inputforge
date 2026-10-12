//! Primary RSX dev-loop harness.
//!
//! Builds a mock `AppState` with seeded device / virtual-device / profile
//! entries, wraps it in `Arc<RwLock<_>>`, builds a drop-channel
//! `mpsc::Sender<EngineCommand>` whose receiver is leaked, and calls
//! the editor renderer. No engine thread, no tray, no profile I/O,
//! no `HidHide` scan, predictable seeded data, hot-reload safe.
//!
//! Run via:
//!     dx serve --example `bridge_demo` --platform desktop

use std::sync::{Arc, mpsc};

use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::state::{AppState, DeviceState, EngineStatus};
use inputforge_core::types::{
    AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, VJoyAxis, VirtualDeviceConfig,
};
use inputforge_gui_dx::routing::{Action, Bindings, View};

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let mut state = AppState::new();
    state.engine_status = EngineStatus::Running;
    state.session.ready = true;
    state.session.monitored = vec![DeviceId("demo-stick".to_owned())];
    state.session.keyboard_supported = true;
    state.session.mouse_supported = true;
    "Demo".clone_into(&mut state.current_mode);
    state
        .warnings
        .push("This is a seeded demo, no engine attached.".to_owned());

    state.devices.push(DeviceState {
        info: DeviceInfo {
            id: DeviceId("demo-stick".to_owned()),
            name: "Demo Stick".to_owned(),
            axes: 4,
            buttons: 12,
            hats: 1,
            instance_path: None,
            axis_polarities: vec![AxisPolarity::Bipolar; 4],
        },
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });

    state.virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X, VJoyAxis::Y, VJoyAxis::Rz],
        button_count: 8,
        hat_count: 1,
    });

    let state = Arc::new(RwLock::new(state));

    // Drop-channel: the receiver is leaked so engine sends don't error.
    let (commands, rx) = mpsc::channel::<EngineCommand>();
    Box::leak(Box::new(rx));

    dioxus::LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new().with_disable_drag_drop_handler(true))
        .with_context((state, commands))
        .launch(demo_root);
}

fn demo_root() -> dioxus::prelude::Element {
    let (state, commands) =
        dioxus::prelude::use_context::<(Arc<RwLock<AppState>>, mpsc::Sender<EngineCommand>)>();
    let routing = Bindings {
        project: |_| View {
            toggle: Some(Action::Stop),
            stop: Some(Action::Stop),
            recovery: Some(Action::Refresh),
            refresh: Some(Action::Refresh),
            ..View::default()
        },
        dispatch: dioxus::prelude::Callback::new(|_| {}),
    };
    inputforge_gui_dx::render_editor(state, commands, routing)
}
