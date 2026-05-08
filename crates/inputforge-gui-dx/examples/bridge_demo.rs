//! Primary RSX dev-loop harness.
//!
//! Builds a mock `AppState` with seeded device / virtual-device / profile
//! entries, wraps it in `Arc<RwLock<_>>`, builds a drop-channel
//! `mpsc::Sender<EngineCommand>` whose receiver is leaked, and calls
//! `launch_gui` directly. No engine thread, no tray, no profile I/O,
//! no `HidHide` scan, predictable seeded data, hot-reload safe.
//!
//! Run via:
//!     dx serve --example `bridge_demo` --platform desktop

use std::sync::{Arc, mpsc};

use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState, EngineStatus};
use inputforge_core::types::{
    AxisPolarity, DeviceDiagnostics, DeviceId, DeviceInfo, VJoyAxis, VirtualDeviceConfig,
};

fn main() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let mut state = AppState::new();
    state.engine_status = EngineStatus::Running;
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

    // Stub menu IDs, wired through to `use_muda_event_handler` at F3, but
    // harmless here because no real tray icon is registered in this example,
    // so muda never emits `MenuEvent`s matching these IDs.
    let menu_ids = (
        muda::MenuId::new("show-gui"),
        muda::MenuId::new("toggle-activation"),
        muda::MenuId::new("quit"),
    );

    // Stub toggle item; no real tray exists in this example, so the label
    // sync effect just mutates a detached `MenuItem` that nothing renders.
    let toggle_item = muda::MenuItem::new("Activate", true, None);

    inputforge_gui_dx::launch_gui(
        state,
        commands,
        menu_ids,
        toggle_item,
        AppSettings::default(),
        false,
    )
}
