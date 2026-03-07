// Rust guideline compliant 2026-03-06

//! Floating input viewer window showing live input/output values.
//!
//! Two-column layout inspired by `JoystickGremlin`: a left sidebar with
//! per-device visibility toggles and a main area with stacked instrument
//! panels for axes, buttons, and hats. Physical devices use blue/amber
//! axis bars; virtual (vJoy) devices use purple accents.

use std::collections::{HashMap, HashSet};

use egui::FontFamily;

use inputforge_core::state::DeviceState;
use inputforge_core::types::{AxisPolarity, DeviceId, VJoyAxis, VirtualDeviceConfig};

use inputforge_core::types::HatDirection;

use crate::app::{CachedState, DeviceInputSnapshot, VjoyOutputSnapshot};
use crate::panels::device_view::axis_label;
use crate::theme;
use crate::widgets::{axis_bar, button_grid, empty_state, hat_indicator, status_dot};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default window width in logical pixels.
const DEFAULT_WIDTH: f32 = 700.0;

/// Default window height in logical pixels.
const DEFAULT_HEIGHT: f32 = 550.0;

/// Minimum window width in logical pixels.
const MIN_WIDTH: f32 = 450.0;

/// Minimum window height in logical pixels.
const MIN_HEIGHT: f32 = 300.0;

/// Width of the left sidebar in logical pixels.
const SIDEBAR_WIDTH: f32 = 180.0;

/// Corner rounding for instrument panel frames in logical pixels.
const FRAME_ROUNDING: f32 = 4.0;

/// Stroke width for instrument panel frame borders.
const FRAME_STROKE_WIDTH: f32 = 1.0;

/// Inner margin for instrument panel frames in logical pixels.
const FRAME_INNER_MARGIN: f32 = 4.0;

/// Number of columns in the button grid.
const BUTTON_GRID_COLUMNS: usize = 8;

/// Spacing between axis bars in logical pixels.
const AXIS_BAR_SPACING: f32 = 1.0;

/// Spacing after an instrument section in logical pixels.
const SECTION_SPACING: f32 = 4.0;

/// Return a short label for a vJoy axis variant.
fn vjoy_axis_label(axis: VJoyAxis) -> &'static str {
    match axis {
        VJoyAxis::X => "X",
        VJoyAxis::Y => "Y",
        VJoyAxis::Z => "Z",
        VJoyAxis::Rx => "Rx",
        VJoyAxis::Ry => "Ry",
        VJoyAxis::Rz => "Rz",
        VJoyAxis::Slider0 => "Sl0",
        VJoyAxis::Slider1 => "Sl1",
    }
}

// ---------------------------------------------------------------------------
// State types
// ---------------------------------------------------------------------------

/// Identifies a device in the viewer, either physical or virtual.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum DeviceKey {
    /// A physical input device identified by its stable [`DeviceId`].
    Physical(DeviceId),
    /// A virtual vJoy device identified by its 1-based device ID.
    Virtual(u8),
}

/// Per-device visibility toggles for input categories.
#[derive(Clone, Debug, Default)]
struct VisibilityToggles {
    /// Whether axes are shown.
    axes: bool,
    /// Whether buttons are shown.
    buttons: bool,
    /// Whether hats are shown.
    hats: bool,
}

impl VisibilityToggles {
    /// Return `true` if at least one toggle is enabled.
    fn any_visible(&self) -> bool {
        self.axes || self.buttons || self.hats
    }
}

/// Persistent state for the input viewer window.
#[derive(Debug, Default)]
pub(crate) struct InputViewerWindowState {
    /// Per-device visibility toggles.
    device_visibility: HashMap<DeviceKey, VisibilityToggles>,
    /// Set of devices we have seen, used to auto-add new devices.
    known_devices: HashSet<DeviceKey>,
}

// ---------------------------------------------------------------------------
// Viewport
// ---------------------------------------------------------------------------

/// Viewport ID for the native input viewer window.
pub(crate) fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("input_viewer_window")
}

/// Show the input viewer as a native OS window.
///
/// If `!*open`, returns immediately. Otherwise renders a two-column
/// layout: sidebar with device toggles, and main area with live
/// instrument panels for all visible devices.
pub(crate) fn show(
    ctx: &egui::Context,
    window_state: &mut InputViewerWindowState,
    open: &mut bool,
    cache: &CachedState,
) {
    if !*open {
        return;
    }

    ctx.show_viewport_immediate(
        viewport_id(),
        egui::ViewportBuilder::default()
            .with_title("InputForge \u{2014} Input Viewer")
            .with_inner_size([DEFAULT_WIDTH, DEFAULT_HEIGHT])
            .with_min_inner_size([MIN_WIDTH, MIN_HEIGHT]),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                *open = false;
            }

            // Auto-register newly discovered devices.
            register_new_devices(window_state, cache);

            let colors = theme::colors(ctx);

            // Single CentralPanel with horizontal strip to avoid SidePanel
            // rendering artifacts in child viewports.
            egui::CentralPanel::default().show(ctx, |ui| {
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::exact(SIDEBAR_WIDTH))
                    .size(egui_extras::Size::remainder())
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            show_sidebar(ui, window_state, cache, colors);
                        });
                        strip.cell(|ui| {
                            show_main_area(ui, window_state, cache, colors);
                        });
                    });
            });
        },
    );
}

// ---------------------------------------------------------------------------
// Device registration
// ---------------------------------------------------------------------------

/// Auto-register any devices present in the cache that are not yet known.
///
/// Newly discovered devices get default visibility toggles (all enabled).
fn register_new_devices(state: &mut InputViewerWindowState, cache: &CachedState) {
    for device in &cache.devices {
        let key = DeviceKey::Physical(device.info.id.clone());
        if state.known_devices.insert(key.clone()) {
            state
                .device_visibility
                .insert(key, VisibilityToggles::default());
        }
    }

    for vdev in &cache.virtual_devices {
        let key = DeviceKey::Virtual(vdev.device_id);
        if state.known_devices.insert(key.clone()) {
            state
                .device_visibility
                .insert(key, VisibilityToggles::default());
        }
    }
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

/// Render the left sidebar with device visibility toggles.
fn show_sidebar(
    ui: &mut egui::Ui,
    state: &mut InputViewerWindowState,
    cache: &CachedState,
    colors: &theme::ThemeColors,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // --- Physical devices ---
        ui.label(
            egui::RichText::new("Physical")
                .family(FontFamily::Name("SemiBold".into()))
                .color(colors.text_dim),
        );

        for device in &cache.devices {
            let key = DeviceKey::Physical(device.info.id.clone());
            show_sidebar_device(ui, state, &key, device, colors);
        }

        ui.separator();

        // --- Virtual devices ---
        ui.label(
            egui::RichText::new("Virtual")
                .family(FontFamily::Name("SemiBold".into()))
                .color(colors.text_dim),
        );

        for vdev in &cache.virtual_devices {
            let key = DeviceKey::Virtual(vdev.device_id);
            let name = format!("vJoy {}", vdev.device_id);
            show_sidebar_device_virtual(ui, state, &key, &name, vdev, colors);
        }
    });
}

/// Render a sidebar entry for a physical device with collapsing toggles.
fn show_sidebar_device(
    ui: &mut egui::Ui,
    state: &mut InputViewerWindowState,
    key: &DeviceKey,
    device: &DeviceState,
    colors: &theme::ThemeColors,
) {
    let id = ui.make_persistent_id(format!("sidebar_{key:?}"));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            let dot_color = if device.connected {
                colors.live
            } else {
                colors.error
            };
            status_dot::status_dot(ui, dot_color, device.connected);
            ui.label(egui::RichText::new(&device.info.name).color(colors.text));
        })
        .body(|ui| {
            if let Some(toggles) = state.device_visibility.get_mut(key) {
                if device.info.axes > 0 {
                    ui.checkbox(&mut toggles.axes, "Axes");
                }
                if device.info.buttons > 0 {
                    ui.checkbox(&mut toggles.buttons, "Buttons");
                }
                if device.info.hats > 0 {
                    ui.checkbox(&mut toggles.hats, "Hats");
                }
            }
        });
}

/// Render a sidebar entry for a virtual device with collapsing toggles.
fn show_sidebar_device_virtual(
    ui: &mut egui::Ui,
    state: &mut InputViewerWindowState,
    key: &DeviceKey,
    name: &str,
    config: &VirtualDeviceConfig,
    colors: &theme::ThemeColors,
) {
    let id = ui.make_persistent_id(format!("sidebar_{key:?}"));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.label(egui::RichText::new(name).color(colors.text));
        })
        .body(|ui| {
            if let Some(toggles) = state.device_visibility.get_mut(key) {
                if !config.axes.is_empty() {
                    ui.checkbox(&mut toggles.axes, "Axes");
                }
                if config.button_count > 0 {
                    ui.checkbox(&mut toggles.buttons, "Buttons");
                }
                if config.hat_count > 0 {
                    ui.checkbox(&mut toggles.hats, "Hats");
                }
            }
        });
}

// ---------------------------------------------------------------------------
// Main area
// ---------------------------------------------------------------------------

/// Render the main area with stacked device instrument panels.
fn show_main_area(
    ui: &mut egui::Ui,
    state: &InputViewerWindowState,
    cache: &CachedState,
    colors: &theme::ThemeColors,
) {
    let no_devices = cache.devices.is_empty() && cache.virtual_devices.is_empty();
    if no_devices {
        empty_state::empty_state(ui, "No devices detected");
        return;
    }

    // Check if any device has at least one visible toggle.
    let any_visible = has_any_visible_device(state);
    if !any_visible {
        empty_state::empty_state(ui, "Toggle devices in the sidebar");
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let mut first = true;

            // Physical devices.
            for device in &cache.devices {
                let key = DeviceKey::Physical(device.info.id.clone());
                let toggles = match state.device_visibility.get(&key) {
                    Some(t) if t.any_visible() => t,
                    _ => continue,
                };

                // Find the matching snapshot by DeviceId (not index).
                let snapshot = cache
                    .devices
                    .iter()
                    .zip(&cache.input_snapshots)
                    .find(|(d, _)| d.info.id == device.info.id)
                    .map(|(_, s)| s);

                if !first {
                    ui.separator();
                }
                first = false;

                show_physical_device_section(ui, device, snapshot, toggles, colors);
            }

            // Virtual devices.
            for vdev in &cache.virtual_devices {
                let key = DeviceKey::Virtual(vdev.device_id);
                let toggles = match state.device_visibility.get(&key) {
                    Some(t) if t.any_visible() => t,
                    _ => continue,
                };

                // Find the matching snapshot by device_id (not index).
                let snapshot = cache
                    .virtual_devices
                    .iter()
                    .zip(&cache.output_snapshots)
                    .find(|(v, _)| v.device_id == vdev.device_id)
                    .map(|(_, s)| s);

                if !first {
                    ui.separator();
                }
                first = false;

                show_virtual_device_section(ui, vdev, snapshot, toggles, colors);
            }
        });
}

/// Return `true` if at least one device has any visible toggle enabled.
fn has_any_visible_device(state: &InputViewerWindowState) -> bool {
    state
        .device_visibility
        .values()
        .any(VisibilityToggles::any_visible)
}

// ---------------------------------------------------------------------------
// Physical device section
// ---------------------------------------------------------------------------

/// Render a physical device's section header and instrument panels.
fn show_physical_device_section(
    ui: &mut egui::Ui,
    device: &DeviceState,
    snapshot: Option<&DeviceInputSnapshot>,
    toggles: &VisibilityToggles,
    colors: &theme::ThemeColors,
) {
    // Section header.
    ui.horizontal(|ui| {
        let dot_color = if device.connected {
            colors.live
        } else {
            colors.error
        };
        status_dot::status_dot(ui, dot_color, device.connected);

        ui.label(
            egui::RichText::new(&device.info.name)
                .family(FontFamily::Name("SemiBold".into()))
                .color(colors.text),
        );

        // Right-aligned INPUT tag.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("INPUT").color(colors.primary));
        });
    });

    if !device.connected {
        ui.label(
            egui::RichText::new("Disconnected")
                .color(colors.error)
                .italics(),
        );
        return;
    }

    let Some(snapshot) = snapshot else {
        return;
    };

    // Determine which sections to show and whether to use side-by-side layout.
    let show_axes = toggles.axes && !snapshot.axes.is_empty();
    let show_buttons = toggles.buttons && !snapshot.buttons.is_empty();
    let show_hats = toggles.hats && !snapshot.hats.is_empty();

    if show_axes {
        instrument_frame(ui, colors, |ui| {
            for (i, &(value, polarity)) in snapshot.axes.iter().enumerate() {
                axis_bar::axis_bar(ui, &axis_label(i), value, polarity);
                ui.add_space(AXIS_BAR_SPACING);
            }
        });
        ui.add_space(SECTION_SPACING);
    }

    show_buttons_and_hats(
        ui,
        &snapshot.buttons,
        &snapshot.hats,
        show_buttons,
        show_hats,
        colors,
    );
}

// ---------------------------------------------------------------------------
// Virtual device section
// ---------------------------------------------------------------------------

/// Render a virtual device's section header and instrument panels.
fn show_virtual_device_section(
    ui: &mut egui::Ui,
    config: &VirtualDeviceConfig,
    snapshot: Option<&VjoyOutputSnapshot>,
    toggles: &VisibilityToggles,
    colors: &theme::ThemeColors,
) {
    // Section header.
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("vJoy {}", config.device_id))
                .family(FontFamily::Name("SemiBold".into()))
                .color(colors.text),
        );

        // Right-aligned OUTPUT tag.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("OUTPUT").color(colors.special));
        });
    });

    let Some(snapshot) = snapshot else {
        return;
    };

    let show_axes = toggles.axes && !snapshot.axes.is_empty();
    let show_buttons = toggles.buttons && !snapshot.buttons.is_empty();
    let show_hats = toggles.hats && !snapshot.hats.is_empty();

    if show_axes {
        instrument_frame(ui, colors, |ui| {
            for &(axis, value) in &snapshot.axes {
                axis_bar::axis_bar_colored(
                    ui,
                    vjoy_axis_label(axis),
                    value,
                    AxisPolarity::Bipolar,
                    colors.special,
                    colors.special,
                );
                ui.add_space(AXIS_BAR_SPACING);
            }
        });
        ui.add_space(SECTION_SPACING);
    }

    show_buttons_and_hats(
        ui,
        &snapshot.buttons,
        &snapshot.hats,
        show_buttons,
        show_hats,
        colors,
    );
}

// ---------------------------------------------------------------------------
// Shared button + hat rendering
// ---------------------------------------------------------------------------

/// Render button grid and hat indicators with optional side-by-side layout.
///
/// When both `show_buttons` and `show_hats` are true, renders them in a
/// two-column layout. Otherwise renders each independently at full width.
fn show_buttons_and_hats(
    ui: &mut egui::Ui,
    buttons: &[bool],
    hats: &[HatDirection],
    show_buttons: bool,
    show_hats: bool,
    colors: &theme::ThemeColors,
) {
    if show_buttons && show_hats {
        ui.columns(2, |columns| {
            instrument_frame(&mut columns[0], colors, |ui| {
                button_grid::button_grid(ui, buttons, BUTTON_GRID_COLUMNS);
            });
            instrument_frame(&mut columns[1], colors, |ui| {
                show_hat_row(ui, hats);
            });
        });
        ui.add_space(SECTION_SPACING);
    } else {
        if show_buttons {
            instrument_frame(ui, colors, |ui| {
                button_grid::button_grid(ui, buttons, BUTTON_GRID_COLUMNS);
            });
            ui.add_space(SECTION_SPACING);
        }
        if show_hats {
            instrument_frame(ui, colors, |ui| {
                show_hat_row(ui, hats);
            });
            ui.add_space(SECTION_SPACING);
        }
    }
}

/// Render a horizontal row of hat indicators with direction tooltips.
fn show_hat_row(ui: &mut egui::Ui, hats: &[HatDirection]) {
    ui.horizontal(|ui| {
        for &dir in hats {
            hat_indicator::hat_indicator(ui, dir)
                .on_hover_text(hat_indicator::direction_label(dir));
            ui.add_space(SECTION_SPACING);
        }
    });
}

// ---------------------------------------------------------------------------
// Instrument frame helper
// ---------------------------------------------------------------------------

/// Wrap content in a recessed instrument panel frame.
///
/// Uses `colors.crust` fill, `colors.surface0` border with
/// [`FRAME_STROKE_WIDTH`], [`FRAME_ROUNDING`] corner radius,
/// and [`FRAME_INNER_MARGIN`] padding.
fn instrument_frame(
    ui: &mut egui::Ui,
    colors: &theme::ThemeColors,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::NONE
        .fill(colors.crust)
        .stroke(egui::Stroke::new(FRAME_STROKE_WIDTH, colors.surface0))
        .corner_radius(FRAME_ROUNDING)
        .inner_margin(FRAME_INNER_MARGIN)
        .show(ui, |ui| {
            add_contents(ui);
        });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_viewer_window_state_default_is_empty() {
        let state = InputViewerWindowState::default();
        assert!(state.device_visibility.is_empty());
        assert!(state.known_devices.is_empty());
    }

    #[test]
    fn visibility_toggles_default_all_false() {
        let toggles = VisibilityToggles::default();
        assert!(!toggles.axes);
        assert!(!toggles.buttons);
        assert!(!toggles.hats);
    }

    #[test]
    fn visibility_toggles_default_none_visible() {
        let toggles = VisibilityToggles::default();
        assert!(!toggles.any_visible());
    }

    #[test]
    fn visibility_toggles_any_visible_all_off() {
        let toggles = VisibilityToggles {
            axes: false,
            buttons: false,
            hats: false,
        };
        assert!(!toggles.any_visible());
    }

    #[test]
    fn visibility_toggles_any_visible_partial() {
        let toggles = VisibilityToggles {
            axes: false,
            buttons: true,
            hats: false,
        };
        assert!(toggles.any_visible());
    }

    #[test]
    fn device_key_physical_and_virtual_are_distinct_in_hashmap() {
        let mut map = HashMap::new();
        let physical = DeviceKey::Physical(DeviceId("dev-1".to_owned()));
        let virtual_key = DeviceKey::Virtual(1);

        map.insert(physical.clone(), "physical");
        map.insert(virtual_key.clone(), "virtual");

        assert_eq!(map.len(), 2);
        assert_eq!(map[&physical], "physical");
        assert_eq!(map[&virtual_key], "virtual");
    }

    #[test]
    fn device_key_physical_equality() {
        let a = DeviceKey::Physical(DeviceId("same".to_owned()));
        let b = DeviceKey::Physical(DeviceId("same".to_owned()));
        assert_eq!(a, b);
    }

    #[test]
    fn device_key_virtual_equality() {
        let a = DeviceKey::Virtual(3);
        let b = DeviceKey::Virtual(3);
        assert_eq!(a, b);
    }

    #[test]
    fn device_key_physical_vs_virtual_not_equal() {
        let physical = DeviceKey::Physical(DeviceId("1".to_owned()));
        let virtual_key = DeviceKey::Virtual(1);
        assert_ne!(physical, virtual_key);
    }

    #[test]
    fn vjoy_axis_label_x() {
        assert_eq!(vjoy_axis_label(VJoyAxis::X), "X");
    }

    #[test]
    fn vjoy_axis_label_y() {
        assert_eq!(vjoy_axis_label(VJoyAxis::Y), "Y");
    }

    #[test]
    fn vjoy_axis_label_z() {
        assert_eq!(vjoy_axis_label(VJoyAxis::Z), "Z");
    }

    #[test]
    fn vjoy_axis_label_rx() {
        assert_eq!(vjoy_axis_label(VJoyAxis::Rx), "Rx");
    }

    #[test]
    fn vjoy_axis_label_ry() {
        assert_eq!(vjoy_axis_label(VJoyAxis::Ry), "Ry");
    }

    #[test]
    fn vjoy_axis_label_rz() {
        assert_eq!(vjoy_axis_label(VJoyAxis::Rz), "Rz");
    }

    #[test]
    fn vjoy_axis_label_slider0() {
        assert_eq!(vjoy_axis_label(VJoyAxis::Slider0), "Sl0");
    }

    #[test]
    fn vjoy_axis_label_slider1() {
        assert_eq!(vjoy_axis_label(VJoyAxis::Slider1), "Sl1");
    }

    #[test]
    fn vjoy_axis_label_all_variants() {
        let all = [
            (VJoyAxis::X, "X"),
            (VJoyAxis::Y, "Y"),
            (VJoyAxis::Z, "Z"),
            (VJoyAxis::Rx, "Rx"),
            (VJoyAxis::Ry, "Ry"),
            (VJoyAxis::Rz, "Rz"),
            (VJoyAxis::Slider0, "Sl0"),
            (VJoyAxis::Slider1, "Sl1"),
        ];
        for (axis, expected) in all {
            assert_eq!(vjoy_axis_label(axis), expected, "mismatch for {axis:?}");
        }
    }

    #[test]
    fn axis_label_hid_names() {
        assert_eq!(axis_label(0), "X");
        assert_eq!(axis_label(3), "Rot X");
        assert_eq!(axis_label(7), "Dial");
    }

    #[test]
    fn axis_label_beyond_table_falls_back() {
        assert_eq!(axis_label(8), "Ax 8");
        assert_eq!(axis_label(99), "Ax 99");
    }

    const _: () = assert!(DEFAULT_WIDTH > 0.0);
    const _: () = assert!(DEFAULT_HEIGHT > 0.0);
    const _: () = assert!(MIN_WIDTH > 0.0);
    const _: () = assert!(MIN_HEIGHT > 0.0);
    const _: () = assert!(SIDEBAR_WIDTH > 0.0);
    const _: () = assert!(FRAME_ROUNDING > 0.0);
    const _: () = assert!(MIN_WIDTH < DEFAULT_WIDTH);
    const _: () = assert!(MIN_HEIGHT < DEFAULT_HEIGHT);

    #[test]
    fn register_new_devices_adds_physical() {
        use inputforge_core::types::{DeviceId, DeviceInfo};

        let mut state = InputViewerWindowState::default();
        let cache = CachedState {
            devices: vec![DeviceState {
                info: DeviceInfo {
                    id: DeviceId("test-dev".to_owned()),
                    name: "Test Joystick".to_owned(),
                    axes: 3,
                    buttons: 12,
                    hats: 1,
                    instance_path: None,
                    axis_polarities: vec![],
                },
                connected: true,
            }],
            input_snapshots: vec![],
            engine_status: inputforge_core::state::EngineStatus::Stopped,
            current_mode: "Default".to_owned(),
            profile_name: None,
            profile_path: None,
            virtual_devices: vec![],
            output_snapshots: vec![],
            warnings: vec![],
            mapped_inputs: std::collections::HashSet::new(),
            mapping_names: std::collections::HashMap::new(),
        };

        register_new_devices(&mut state, &cache);

        let key = DeviceKey::Physical(DeviceId("test-dev".to_owned()));
        assert!(state.known_devices.contains(&key));
        assert!(state.device_visibility.contains_key(&key));
    }

    #[test]
    fn register_new_devices_adds_virtual() {
        let mut state = InputViewerWindowState::default();
        let cache = CachedState {
            devices: vec![],
            input_snapshots: vec![],
            engine_status: inputforge_core::state::EngineStatus::Stopped,
            current_mode: "Default".to_owned(),
            profile_name: None,
            profile_path: None,
            virtual_devices: vec![VirtualDeviceConfig {
                device_id: 1,
                axes: vec![VJoyAxis::X],
                button_count: 4,
                hat_count: 0,
            }],
            output_snapshots: vec![],
            warnings: vec![],
            mapped_inputs: std::collections::HashSet::new(),
            mapping_names: std::collections::HashMap::new(),
        };

        register_new_devices(&mut state, &cache);

        let key = DeviceKey::Virtual(1);
        assert!(state.known_devices.contains(&key));
        assert!(state.device_visibility.contains_key(&key));
    }

    #[test]
    fn register_new_devices_idempotent() {
        let mut state = InputViewerWindowState::default();
        let cache = CachedState {
            devices: vec![],
            input_snapshots: vec![],
            engine_status: inputforge_core::state::EngineStatus::Stopped,
            current_mode: "Default".to_owned(),
            profile_name: None,
            profile_path: None,
            virtual_devices: vec![VirtualDeviceConfig {
                device_id: 2,
                axes: vec![],
                button_count: 0,
                hat_count: 1,
            }],
            output_snapshots: vec![],
            warnings: vec![],
            mapped_inputs: std::collections::HashSet::new(),
            mapping_names: std::collections::HashMap::new(),
        };

        register_new_devices(&mut state, &cache);
        let key = DeviceKey::Virtual(2);

        // Modify the toggles.
        if let Some(t) = state.device_visibility.get_mut(&key) {
            t.axes = false;
        }

        // Re-register: should NOT overwrite existing toggles.
        register_new_devices(&mut state, &cache);
        let toggles = &state.device_visibility[&key];
        assert!(!toggles.axes, "re-registration should not reset toggles");
    }
}
