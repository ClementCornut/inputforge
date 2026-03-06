// Rust guideline compliant 2026-03-04

//! Main application struct implementing `eframe::App`.
//!
//! `InputForgeApp` holds cached copies of shared state, a command
//! channel to the engine, and per-frame GUI selection state.
//! The `update()` method renders panels in the required order:
//! `BottomPanel` -> `SidePanel` -> `CentralPanel` (last).

use std::sync::Arc;
use std::sync::mpsc;

use eframe::CreationContext;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::pipeline::InputCache;
use inputforge_core::state::{AppState, DeviceState, EngineStatus};
use inputforge_core::types::{DeviceId, HatDirection, InputAddress, InputId, VirtualDeviceConfig};

use crate::panels;
use crate::panels::calibration_window::CalibrationWindowState;
use crate::panels::input_monitor::InputMonitorState;
use crate::panels::mapping_editor::MappingEditorState;
use crate::theme;

/// State for floating tool windows opened from the menu bar.
#[derive(Debug, Default)]
pub(crate) struct ToolWindowStates {
    /// Whether the calibration window is open.
    pub calibration_open: bool,
}

/// Which view occupies the center panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CenterView {
    /// Live overview of all connected devices (default).
    DeviceOverview,
    /// Mapping editor for the selected device.
    MappingEditor,
    /// Real-time input event monitor.
    InputMonitor,
    /// Mode tree display and selection.
    ModeEditor,
}

impl CenterView {
    /// All variants in tab-bar display order.
    pub(crate) const fn all() -> [Self; 4] {
        [
            Self::DeviceOverview,
            Self::MappingEditor,
            Self::InputMonitor,
            Self::ModeEditor,
        ]
    }
}

impl crate::widgets::tab_bar::TabItem for CenterView {
    fn label(self) -> &'static str {
        match self {
            Self::DeviceOverview => "Devices",
            Self::MappingEditor => "Mappings",
            Self::InputMonitor => "Monitor",
            Self::ModeEditor => "Modes",
        }
    }
}

/// Per-frame GUI selection state.
#[derive(Debug)]
pub(crate) struct GuiSelection {
    /// Index into `cached_devices` for the selected device.
    pub selected_device_idx: Option<usize>,
    /// Which center panel view is active.
    pub center_view: CenterView,
}

impl Default for GuiSelection {
    fn default() -> Self {
        Self {
            selected_device_idx: None,
            center_view: CenterView::DeviceOverview,
        }
    }
}

/// Snapshot of a single device's live input values.
///
/// Captured once per frame under a single read lock, then used for
/// rendering without further lock acquisitions.
#[derive(Debug, Clone)]
pub(crate) struct DeviceInputSnapshot {
    /// Axis values in `[-1.0, 1.0]`.
    pub axes: Vec<f64>,
    /// Button pressed states.
    pub buttons: Vec<bool>,
    /// Hat switch directions.
    pub hats: Vec<HatDirection>,
}

/// Cached snapshot of shared state read each frame.
///
/// Populated by `refresh_cache()` with a brief read-lock on `AppState`.
#[derive(Debug)]
pub(crate) struct CachedState {
    pub devices: Vec<DeviceState>,
    /// Per-device input snapshots, parallel to `devices`.
    pub input_snapshots: Vec<DeviceInputSnapshot>,
    pub engine_status: EngineStatus,
    pub current_mode: String,
    pub profile_name: Option<String>,
    /// Discovered vJoy device configurations (empty until engine populates).
    pub virtual_devices: Vec<VirtualDeviceConfig>,
}

impl Default for CachedState {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            input_snapshots: Vec::new(),
            engine_status: EngineStatus::Stopped,
            current_mode: "Default".to_owned(),
            profile_name: None,
            virtual_devices: Vec::new(),
        }
    }
}

/// Top-level application driving the egui UI.
#[derive(Debug)]
pub struct InputForgeApp {
    /// Shared state with the engine thread.
    state: Arc<RwLock<AppState>>,
    /// Channel to send commands to the engine.
    commands: mpsc::Sender<EngineCommand>,
    /// Per-frame cached snapshot of shared state.
    pub(crate) cache: CachedState,
    /// GUI selection state (selected device, active view).
    pub(crate) selection: GuiSelection,
    /// Persistent state for the input monitor panel.
    pub(crate) monitor_state: InputMonitorState,
    /// Persistent state for the mapping editor panel.
    pub(crate) mapping_editor_state: MappingEditorState,
    /// State for floating tool windows opened from the menu bar.
    pub(crate) tool_windows: ToolWindowStates,
    /// Persistent state for the calibration window.
    pub(crate) calibration_window_state: CalibrationWindowState,
}

impl InputForgeApp {
    /// Create the application, apply theme, and initialize cached state.
    ///
    /// Must be called with the `CreationContext` from `eframe::run_native`
    /// so that fonts and visuals are set up before the first frame.
    #[must_use]
    pub fn new(
        cc: &CreationContext<'_>,
        state: Arc<RwLock<AppState>>,
        commands: mpsc::Sender<EngineCommand>,
    ) -> Self {
        theme::setup(&cc.egui_ctx);

        let mut app = Self {
            state,
            commands,
            cache: CachedState::default(),
            selection: GuiSelection::default(),
            monitor_state: InputMonitorState::new(),
            mapping_editor_state: MappingEditorState::new(),
            tool_windows: ToolWindowStates::default(),
            calibration_window_state: CalibrationWindowState::default(),
        };
        app.refresh_cache();
        app
    }

    /// Read shared state briefly and clone into cached fields.
    ///
    /// The read lock is held only for the duration of the clone and
    /// snapshot operations. All rendering uses the cached data without
    /// further lock acquisitions.
    pub(crate) fn refresh_cache(&mut self) {
        let guard = self.state.read();
        self.cache.devices = guard.devices.clone();
        self.cache.engine_status = guard.engine_status;
        self.cache.current_mode.clone_from(&guard.current_mode);
        self.cache.profile_name = guard.active_profile.as_ref().map(|p| p.name().to_owned());
        self.cache
            .virtual_devices
            .clone_from(&guard.virtual_devices);

        // Snapshot all device input values under the same lock.
        self.cache.input_snapshots = self
            .cache
            .devices
            .iter()
            .map(|device| snapshot_device_inputs(&device.info.id, device, &guard))
            .collect();
        // Guard dropped here.

        // Validate selected_device_idx after device list may have changed.
        if let Some(idx) = self.selection.selected_device_idx {
            if idx >= self.cache.devices.len() {
                self.selection.selected_device_idx = None;
            }
        }
    }

    /// Send a command to the engine thread.
    ///
    /// Errors are logged at `warn` level but otherwise silently dropped
    /// to avoid disrupting the UI frame.
    pub(crate) fn send_command(&self, cmd: EngineCommand) {
        if let Err(e) = self.commands.send(cmd) {
            tracing::warn!(error = %e, "failed to send engine command");
        }
    }
}

/// Read all input values for a device from the shared state guard.
///
/// Called under the existing read lock in `refresh_cache()` to avoid
/// additional lock acquisitions during rendering.
fn snapshot_device_inputs(
    device_id: &DeviceId,
    device: &DeviceState,
    guard: &AppState,
) -> DeviceInputSnapshot {
    let axes: Vec<f64> = (0..device.info.axes)
        .map(|i| {
            let addr = InputAddress {
                device: device_id.clone(),
                input: InputId::Axis { index: i },
            };
            guard.input_cache.get_axis(&addr)
        })
        .collect();

    let buttons: Vec<bool> = (0..device.info.buttons)
        .map(|i| {
            let addr = InputAddress {
                device: device_id.clone(),
                input: InputId::Button { index: i },
            };
            guard.input_cache.get_button(&addr)
        })
        .collect();

    let hats: Vec<HatDirection> = (0..device.info.hats)
        .map(|i| {
            let addr = InputAddress {
                device: device_id.clone(),
                input: InputId::Hat { index: i },
            };
            guard.input_cache.get_hat(&addr)
        })
        .collect();

    DeviceInputSnapshot {
        axes,
        buttons,
        hats,
    }
}

impl eframe::App for InputForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh_cache();

        // Request continuous repainting so live data animates at ~60 fps.
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // Panel ordering: BottomPanel -> SidePanel -> CentralPanel (last).
        panels::status_bar::show(ctx, &self.cache);
        panels::left_panel::show(ctx, &self.cache, &mut self.selection);
        panels::center_panel::show(
            ctx,
            &self.cache,
            &mut self.selection,
            &mut self.monitor_state,
            &mut self.mapping_editor_state,
            &mut self.tool_windows,
        );

        panels::calibration_window::show(
            ctx,
            &mut self.calibration_window_state,
            &mut self.tool_windows.calibration_open,
            &self.cache,
            &self.state,
            &self.commands,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_selection_defaults_to_device_overview() {
        let sel = GuiSelection::default();
        assert_eq!(sel.center_view, CenterView::DeviceOverview);
        assert!(sel.selected_device_idx.is_none());
    }

    #[test]
    fn cached_state_defaults_to_stopped() {
        let cache = CachedState::default();
        assert_eq!(cache.engine_status, EngineStatus::Stopped);
        assert!(cache.devices.is_empty());
        assert_eq!(cache.current_mode, "Default");
        assert!(cache.profile_name.is_none());
    }

    #[test]
    fn refresh_cache_reads_shared_state() {
        let state = Arc::new(RwLock::new(AppState::new()));
        let (tx, _rx) = mpsc::channel();

        let mut app = InputForgeApp {
            state: Arc::clone(&state),
            commands: tx,
            cache: CachedState::default(),
            selection: GuiSelection::default(),
            monitor_state: InputMonitorState::new(),
            mapping_editor_state: MappingEditorState::new(),
            tool_windows: ToolWindowStates::default(),
            calibration_window_state: CalibrationWindowState::default(),
        };

        // Mutate shared state.
        {
            let mut guard = state.write();
            guard.engine_status = EngineStatus::Running;
            guard.current_mode = "Combat".to_owned();
        }

        app.refresh_cache();

        assert_eq!(app.cache.engine_status, EngineStatus::Running);
        assert_eq!(app.cache.current_mode, "Combat");
    }

    #[test]
    fn refresh_cache_clears_stale_device_selection() {
        use inputforge_core::types::{DeviceId, DeviceInfo};

        let state = Arc::new(RwLock::new(AppState::new()));
        let (tx, _rx) = mpsc::channel();

        let mut app = InputForgeApp {
            state: Arc::clone(&state),
            commands: tx,
            cache: CachedState::default(),
            selection: GuiSelection::default(),
            monitor_state: InputMonitorState::new(),
            mapping_editor_state: MappingEditorState::new(),
            tool_windows: ToolWindowStates::default(),
            calibration_window_state: CalibrationWindowState::default(),
        };

        // Add one device.
        {
            let mut guard = state.write();
            guard.devices.push(DeviceState {
                info: DeviceInfo {
                    id: DeviceId("dev-0".to_owned()),
                    name: "Joystick".to_owned(),
                    axes: 0,
                    buttons: 0,
                    hats: 0,
                    instance_path: None,
                },
                connected: true,
            });
        }

        app.refresh_cache();
        app.selection.selected_device_idx = Some(0);

        // Remove the device.
        {
            let mut guard = state.write();
            guard.devices.clear();
        }

        app.refresh_cache();
        assert!(app.selection.selected_device_idx.is_none());
    }

    #[test]
    fn snapshot_device_inputs_returns_defaults() {
        let state = AppState::new();
        let device = DeviceState {
            info: inputforge_core::types::DeviceInfo {
                id: DeviceId("test".to_owned()),
                name: "Test".to_owned(),
                axes: 3,
                buttons: 12,
                hats: 1,
                instance_path: None,
            },
            connected: true,
        };

        let snap = snapshot_device_inputs(&device.info.id, &device, &state);
        assert_eq!(snap.axes.len(), 3);
        assert_eq!(snap.buttons.len(), 12);
        assert_eq!(snap.hats.len(), 1);

        for &v in &snap.axes {
            assert!(v.abs() < f64::EPSILON);
        }
        for &b in &snap.buttons {
            assert!(!b);
        }
        for &h in &snap.hats {
            assert_eq!(h, HatDirection::Center);
        }
    }

    /// Guard ensuring `CenterView::all()` stays in sync with the enum.
    ///
    /// The `match` forces a compile error when a new variant is added
    /// without updating this test (and presumably `all()`).
    #[test]
    fn center_view_all_is_exhaustive() {
        let all = CenterView::all();
        // If you add a variant, update `all()` AND add it to the match below.
        for view in &all {
            match view {
                CenterView::DeviceOverview
                | CenterView::MappingEditor
                | CenterView::InputMonitor
                | CenterView::ModeEditor => {}
            }
        }
        // The array length must match the variant count.
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn snapshot_device_inputs_reads_cached_values() {
        use inputforge_core::types::{AxisValue, InputValue};

        let mut state = AppState::new();
        let device = DeviceState {
            info: inputforge_core::types::DeviceInfo {
                id: DeviceId("test-dev".to_owned()),
                name: "Test".to_owned(),
                axes: 3,
                buttons: 12,
                hats: 1,
                instance_path: None,
            },
            connected: true,
        };

        let axis_addr = InputAddress {
            device: DeviceId("test-dev".to_owned()),
            input: InputId::Axis { index: 1 },
        };
        state.input_cache.update(
            &axis_addr,
            &InputValue::Axis {
                value: AxisValue::new(0.75),
            },
        );

        let btn_addr = InputAddress {
            device: DeviceId("test-dev".to_owned()),
            input: InputId::Button { index: 3 },
        };
        state
            .input_cache
            .update(&btn_addr, &InputValue::Button { pressed: true });

        let hat_addr = InputAddress {
            device: DeviceId("test-dev".to_owned()),
            input: InputId::Hat { index: 0 },
        };
        state.input_cache.update(
            &hat_addr,
            &InputValue::Hat {
                direction: HatDirection::NE,
            },
        );

        let snap = snapshot_device_inputs(&device.info.id, &device, &state);
        assert!((snap.axes[1] - 0.75).abs() < f64::EPSILON);
        assert!(snap.buttons[3]);
        assert_eq!(snap.hats[0], HatDirection::NE);
    }
}
