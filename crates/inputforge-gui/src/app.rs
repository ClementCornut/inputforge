// Rust guideline compliant 2026-03-06

//! Main application struct implementing `eframe::App`.
//!
//! `InputForgeApp` holds cached copies of shared state, a command
//! channel to the engine, and per-frame GUI selection state.
//! The `update()` method renders panels in the required order:
//! `BottomPanel` -> `SidePanel` -> `CentralPanel` (last).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc;

use eframe::CreationContext;
use muda::{MenuEvent, MenuId};
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::pipeline::InputCache;
use inputforge_core::settings::AppSettings;
use inputforge_core::state::{AppState, DeviceState, EngineStatus};
use inputforge_core::types::{
    AxisPolarity, DeviceId, HatDirection, InputAddress, InputId, VJoyAxis, VirtualDeviceConfig,
};

use crate::panels;
use crate::panels::calibration_window::CalibrationWindowState;
use crate::panels::input_viewer_window::InputViewerWindowState;
use crate::panels::mapping_editor::MappingEditorState;
use crate::theme;
use crate::widgets::toast::{ToastLevel, ToastManager};

/// State for floating tool windows opened from the menu bar.
#[derive(Debug, Default)]
pub(crate) struct ToolWindowStates {
    /// Whether the calibration window is open.
    pub calibration_open: bool,
    /// Whether the input viewer window is open.
    pub input_viewer_open: bool,
}

/// Which view occupies the center panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CenterView {
    /// Live overview of all connected devices (default).
    DeviceOverview,
    /// Mapping editor for the selected device.
    MappingEditor,
    /// Mode tree display and selection.
    ModeEditor,
}

impl CenterView {
    /// All variants in tab-bar display order.
    pub(crate) const fn all() -> [Self; 3] {
        [Self::DeviceOverview, Self::MappingEditor, Self::ModeEditor]
    }
}

impl crate::widgets::tab_bar::TabItem for CenterView {
    fn label(self) -> &'static str {
        match self {
            Self::DeviceOverview => "Devices",
            Self::MappingEditor => "Mappings",
            Self::ModeEditor => "Modes",
        }
    }
}

/// Per-frame GUI selection state.
#[derive(Debug)]
pub(crate) struct GuiSelection {
    /// Index into `cached_devices` for the selected device.
    pub selected_device_idx: Option<usize>,
    /// The specific input selected within the device (for editing).
    pub selected_input: Option<InputId>,
    /// Which center panel view is active.
    pub center_view: CenterView,
}

impl Default for GuiSelection {
    fn default() -> Self {
        Self {
            selected_device_idx: None,
            selected_input: None,
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
    /// Axis values in `[-1.0, 1.0]` paired with their detected polarity.
    pub axes: Vec<(f64, AxisPolarity)>,
    /// Button pressed states.
    pub buttons: Vec<bool>,
    /// Hat switch directions.
    pub hats: Vec<HatDirection>,
}

/// Snapshot of a single vJoy device's output values.
///
/// Captured once per frame under a single read lock, then used for
/// rendering without further lock acquisitions.
#[derive(Debug, Clone)]
pub(crate) struct VjoyOutputSnapshot {
    /// Axis values in `[-1.0, 1.0]`, parallel to `VirtualDeviceConfig::axes`.
    pub axes: Vec<(VJoyAxis, f64)>,
    /// Button pressed states, length = `VirtualDeviceConfig::button_count`.
    pub buttons: Vec<bool>,
    /// Hat directions, length = `VirtualDeviceConfig::hat_count`.
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
    /// Per-vJoy-device output snapshots, parallel to `virtual_devices`.
    pub output_snapshots: Vec<VjoyOutputSnapshot>,
    /// Warnings from the engine (e.g., `HidHide` unavailable).
    pub warnings: Vec<String>,
    /// Set of input addresses that have mappings in the active profile.
    pub mapped_inputs: HashSet<InputAddress>,
    /// Mapping names keyed by input address (only for mappings with names).
    pub mapping_names: HashMap<InputAddress, String>,
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
            output_snapshots: Vec::new(),
            warnings: Vec::new(),
            mapped_inputs: HashSet::new(),
            mapping_names: HashMap::new(),
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
    /// Application-wide persistent settings (used by profile management window).
    #[expect(
        dead_code,
        reason = "will be read by profile management window (Task 10)"
    )]
    pub(crate) settings: AppSettings,
    /// Per-frame cached snapshot of shared state.
    pub(crate) cache: CachedState,
    /// GUI selection state (selected device, active view).
    pub(crate) selection: GuiSelection,
    /// Persistent state for the floating input viewer window.
    pub(crate) input_viewer_window_state: InputViewerWindowState,
    /// Persistent state for the mapping editor panel.
    pub(crate) mapping_editor_state: MappingEditorState,
    /// State for floating tool windows opened from the menu bar.
    pub(crate) tool_windows: ToolWindowStates,
    /// Persistent state for the calibration window.
    pub(crate) calibration_window_state: CalibrationWindowState,
    /// Toast notification manager for transient warnings.
    pub(crate) toast_manager: ToastManager,
    /// Pending input switch awaiting dirty-state confirmation.
    ///
    /// When the user selects a different input while the editor has unsaved
    /// changes, the target address is stored here. A modal dialog asks the
    /// user to save or discard before switching.
    ///
    /// The outer `Option` indicates whether a switch is pending; the inner
    /// `Option` carries the target (`None` = deselect, `Some` = new input).
    #[expect(clippy::option_option, reason = "outer=pending, inner=target")]
    pending_input_switch: Option<Option<InputAddress>>,
    /// Number of warnings seen so far (to detect new ones).
    last_warning_count: usize,
    /// Tray menu item IDs for polling `MenuEvent` while the GUI is open.
    tray_show_id: MenuId,
    tray_toggle_id: MenuId,
    tray_quit_id: MenuId,
}

/// Action chosen in the dirty-state confirmation dialog.
enum DirtyAction {
    None,
    Discard,
    Save,
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
        tray_menu_ids: (MenuId, MenuId, MenuId),
        settings: AppSettings,
    ) -> Self {
        theme::setup(&cc.egui_ctx);

        let mut app = Self {
            state,
            commands,
            settings,
            cache: CachedState::default(),
            selection: GuiSelection::default(),
            input_viewer_window_state: InputViewerWindowState::default(),
            mapping_editor_state: MappingEditorState::new(),
            tool_windows: ToolWindowStates::default(),
            calibration_window_state: CalibrationWindowState::default(),
            toast_manager: ToastManager::default(),
            pending_input_switch: None,
            last_warning_count: 0,
            tray_show_id: tray_menu_ids.0,
            tray_toggle_id: tray_menu_ids.1,
            tray_quit_id: tray_menu_ids.2,
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

        // Snapshot all vJoy output values under the same lock.
        self.cache.output_snapshots = self
            .cache
            .virtual_devices
            .iter()
            .map(|vdev| snapshot_vjoy_outputs(vdev, &guard))
            .collect();

        self.cache.warnings.clone_from(&guard.warnings);

        // Cache which inputs have mappings for left panel indicators.
        self.cache.mapped_inputs.clear();
        self.cache.mapping_names.clear();
        if let Some(profile) = &guard.active_profile {
            for mapping in profile.mappings() {
                self.cache.mapped_inputs.insert(mapping.input.clone());
                if let Some(name) = &mapping.name {
                    self.cache
                        .mapping_names
                        .insert(mapping.input.clone(), name.clone());
                }
            }
        }
        // Guard dropped here.

        // Push any new warnings to the toast manager.
        if self.cache.warnings.len() > self.last_warning_count {
            for msg in &self.cache.warnings[self.last_warning_count..] {
                self.toast_manager.push(msg.clone(), ToastLevel::Warning);
            }
            self.last_warning_count = self.cache.warnings.len();
        }

        // Validate selected_device_idx after device list may have changed.
        if let Some(idx) = self.selection.selected_device_idx {
            if idx >= self.cache.devices.len() {
                self.selection.selected_device_idx = None;
                self.selection.selected_input = None;
            }
        }
    }

    /// Drain queued tray menu events and act on them.
    ///
    /// While the GUI is open, eframe owns the Win32 message pump.
    /// Tray menu clicks are dispatched by eframe and pushed to the
    /// `MenuEvent` channel, so we poll it here each frame.
    ///
    /// Returns `true` if the GUI should close (quit requested).
    fn process_tray_events(&mut self, ctx: &egui::Context) -> bool {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.tray_quit_id {
                self.state.write().quit_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return true;
            } else if event.id == self.tray_toggle_id {
                let cmd = match self.cache.engine_status {
                    EngineStatus::Running => EngineCommand::Deactivate,
                    EngineStatus::Paused | EngineStatus::Stopped => EngineCommand::Activate,
                };
                let _ = self.commands.send(cmd);
            } else if event.id == self.tray_show_id {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
        false
    }

    /// Render the unsaved-changes confirmation dialog.
    ///
    /// Shown when `pending_input_switch` is `Some`, meaning the user tried
    /// to switch inputs while the editor had unsaved changes.
    fn show_dirty_confirmation(&mut self, ctx: &egui::Context) {
        if self.pending_input_switch.is_none() {
            return;
        }

        let mut action = DirtyAction::None;

        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("You have unsaved changes to the current mapping.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Discard").clicked() {
                        action = DirtyAction::Discard;
                    }
                    if ui.button("Save").clicked() {
                        action = DirtyAction::Save;
                    }
                });
            });

        match action {
            DirtyAction::None => {}
            DirtyAction::Discard => {
                let target = self.pending_input_switch.take().flatten();
                self.switch_to_input(target);
            }
            DirtyAction::Save => {
                if let Some(editing_addr) = self.mapping_editor_state.editing().cloned() {
                    let _ = self.commands.send(EngineCommand::SetMapping {
                        input: editing_addr,
                        mode: self.cache.current_mode.clone(),
                        name: self.mapping_editor_state.take_name(),
                        actions: self.mapping_editor_state.take_actions(),
                    });
                }
                let target = self.pending_input_switch.take().flatten();
                self.switch_to_input(target);
            }
        }
    }

    /// Load the mapping for the given input address into the editor.
    ///
    /// Briefly acquires a read lock on shared state to look up the
    /// mapping in the active profile. If no mapping exists, loads
    /// an empty pipeline.
    fn switch_to_input(&mut self, target: Option<InputAddress>) {
        match target {
            Some(addr) => {
                let guard = self.state.read();
                let (name, actions) = guard
                    .active_profile
                    .as_ref()
                    .and_then(|p| p.find_mapping(&addr, &self.cache.current_mode))
                    .map(|m| (m.name.clone(), m.actions.clone()))
                    .unwrap_or_default();
                drop(guard);
                self.mapping_editor_state.load(addr, name, actions);
            }
            None => {
                self.mapping_editor_state.clear();
            }
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
    let axes: Vec<(f64, AxisPolarity)> = (0..device.info.axes)
        .map(|i| {
            let addr = InputAddress {
                device: device_id.clone(),
                input: InputId::Axis { index: i },
            };
            let polarity = device
                .info
                .axis_polarities
                .get(usize::from(i))
                .copied()
                .unwrap_or_default();
            (guard.input_cache.get_axis(&addr), polarity)
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

/// Read all output values for a vJoy device from the shared state guard.
///
/// Called under the existing read lock in `refresh_cache()` to avoid
/// additional lock acquisitions during rendering.
fn snapshot_vjoy_outputs(config: &VirtualDeviceConfig, guard: &AppState) -> VjoyOutputSnapshot {
    let axes: Vec<(VJoyAxis, f64)> = config
        .axes
        .iter()
        .map(|&axis| (axis, guard.output_cache.get_axis(config.device_id, axis)))
        .collect();

    let buttons: Vec<bool> = (1..=config.button_count)
        .map(|i| guard.output_cache.get_button(config.device_id, i))
        .collect();

    let hats: Vec<HatDirection> = (0..config.hat_count)
        .map(|i| guard.output_cache.get_hat(config.device_id, i))
        .collect();

    VjoyOutputSnapshot {
        axes,
        buttons,
        hats,
    }
}

impl eframe::App for InputForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh_cache();

        // Detect input selection changes and load the corresponding mapping.
        let target = match (
            self.selection.selected_device_idx,
            &self.selection.selected_input,
        ) {
            (Some(idx), Some(input_id)) if idx < self.cache.devices.len() => {
                let device_id = self.cache.devices[idx].info.id.clone();
                Some(InputAddress {
                    device: device_id,
                    input: input_id.clone(),
                })
            }
            _ => None,
        };
        if target.as_ref() != self.mapping_editor_state.editing()
            && self.pending_input_switch.is_none()
        {
            if self.mapping_editor_state.is_dirty() {
                self.pending_input_switch = Some(target);
            } else {
                self.switch_to_input(target);
            }
        }

        if self.process_tray_events(ctx) {
            return;
        }

        // Request continuous repainting so live data animates at ~60 fps.
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // Panel ordering: BottomPanel -> SidePanel -> CentralPanel (last).
        panels::status_bar::show(ctx, &self.cache);
        panels::left_panel::show(ctx, &self.cache, &mut self.selection);
        panels::center_panel::show(
            ctx,
            &self.cache,
            &mut self.selection,
            &mut self.mapping_editor_state,
            &mut self.tool_windows,
            &self.commands,
        );

        panels::calibration_window::show(
            ctx,
            &mut self.calibration_window_state,
            &mut self.tool_windows.calibration_open,
            &self.cache,
            &self.state,
            &self.commands,
        );

        panels::input_viewer_window::show(
            ctx,
            &mut self.input_viewer_window_state,
            &mut self.tool_windows.input_viewer_open,
            &self.cache,
        );

        // Show dirty-state confirmation dialog if pending.
        self.show_dirty_confirmation(ctx);

        // Render toast overlays on top of all panels.
        self.toast_manager.show(ctx);
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
        assert!(sel.selected_input.is_none());
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
            settings: AppSettings::default(),
            cache: CachedState::default(),
            selection: GuiSelection::default(),
            input_viewer_window_state: InputViewerWindowState::default(),
            mapping_editor_state: MappingEditorState::new(),
            tool_windows: ToolWindowStates::default(),
            calibration_window_state: CalibrationWindowState::default(),
            toast_manager: ToastManager::default(),
            pending_input_switch: None,
            last_warning_count: 0,
            tray_show_id: MenuId::new("test-show"),
            tray_toggle_id: MenuId::new("test-toggle"),
            tray_quit_id: MenuId::new("test-quit"),
        };

        // Mutate shared state.
        {
            let mut guard = state.write();
            guard.engine_status = EngineStatus::Running;
            guard.current_mode = "Combat".to_owned();
        };

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
            settings: AppSettings::default(),
            cache: CachedState::default(),
            selection: GuiSelection::default(),
            input_viewer_window_state: InputViewerWindowState::default(),
            mapping_editor_state: MappingEditorState::new(),
            tool_windows: ToolWindowStates::default(),
            calibration_window_state: CalibrationWindowState::default(),
            toast_manager: ToastManager::default(),
            pending_input_switch: None,
            last_warning_count: 0,
            tray_show_id: MenuId::new("test-show"),
            tray_toggle_id: MenuId::new("test-toggle"),
            tray_quit_id: MenuId::new("test-quit"),
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
                    axis_polarities: vec![],
                },
                connected: true,
            });
        };

        app.refresh_cache();
        app.selection.selected_device_idx = Some(0);

        // Remove the device.
        {
            let mut guard = state.write();
            guard.devices.clear();
        };

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
                axis_polarities: vec![],
            },
            connected: true,
        };

        let snap = snapshot_device_inputs(&device.info.id, &device, &state);
        assert_eq!(snap.axes.len(), 3);
        assert_eq!(snap.buttons.len(), 12);
        assert_eq!(snap.hats.len(), 1);

        for &(v, polarity) in &snap.axes {
            assert!(v.abs() < f64::EPSILON);
            assert_eq!(polarity, AxisPolarity::Bipolar);
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
                CenterView::DeviceOverview | CenterView::MappingEditor | CenterView::ModeEditor => {
                }
            }
        }
        // The array length must match the variant count.
        assert_eq!(all.len(), 3);
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
                axis_polarities: vec![],
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
        assert!((snap.axes[1].0 - 0.75).abs() < f64::EPSILON);
        assert!(snap.buttons[3]);
        assert_eq!(snap.hats[0], HatDirection::NE);
    }

    #[test]
    fn vjoy_output_snapshot_returns_defaults() {
        let state = AppState::new();
        let config = VirtualDeviceConfig {
            device_id: 1,
            axes: vec![VJoyAxis::X, VJoyAxis::Y],
            button_count: 4,
            hat_count: 1,
        };
        let snap = snapshot_vjoy_outputs(&config, &state);
        assert_eq!(snap.axes.len(), 2);
        assert_eq!(snap.buttons.len(), 4);
        assert_eq!(snap.hats.len(), 1);
        for &(_, v) in &snap.axes {
            assert!(v.abs() < f64::EPSILON);
        }
        for &b in &snap.buttons {
            assert!(!b);
        }
        for &h in &snap.hats {
            assert_eq!(h, HatDirection::Center);
        }
    }
}
