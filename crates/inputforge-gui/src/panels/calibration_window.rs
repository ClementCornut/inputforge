// Rust guideline compliant 2026-03-06

//! Floating calibration window with two-column master-detail layout.
//!
//! Left column shows a scrollable list of axis bars for the selected device.
//! Right column shows a detailed calibration editor for the selected axis,
//! including raw/calibrated value bars, threshold editors, recording buttons,
//! and save/reset actions.

use std::sync::Arc;
use std::sync::mpsc;

use egui::FontFamily;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::processing::calibration::Calibration;
use inputforge_core::state::AppState;
use inputforge_core::types::AxisPolarity;

use crate::app::CachedState;
use crate::theme;
use crate::widgets::{axis_bar, calibration_editor};

/// Default window width in logical pixels.
const DEFAULT_WIDTH: f32 = 600.0;

/// Default window height in logical pixels.
const DEFAULT_HEIGHT: f32 = 450.0;

/// Minimum window width in logical pixels.
const MIN_WIDTH: f32 = 500.0;

/// Minimum window height in logical pixels.
const MIN_HEIGHT: f32 = 350.0;

/// Width of the left axis list column in logical pixels.
const LEFT_COLUMN_WIDTH: f32 = 180.0;

/// Width of the selected-axis accent border in logical pixels.
const ACCENT_BORDER_WIDTH: f32 = 2.0;

/// Drag speed for threshold value editors.
const DRAG_SPEED: f64 = 1.0;

/// Range limit for physical threshold drag values.
const RANGE_LIMIT: f64 = 100_000.0;

/// Viewport ID for the native calibration window.
pub(crate) fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("calibration_window")
}

/// Persistent state for the calibration window.
#[derive(Debug, Default)]
pub(crate) struct CalibrationWindowState {
    selected_device_idx: Option<usize>,
    selected_axis: Option<u8>,
    /// Per-axis editing state (rebuilt when device changes).
    axis_editors: Vec<AxisEditor>,
}

/// Per-axis editing state with working copy of calibration values.
#[derive(Debug, Clone)]
struct AxisEditor {
    /// Working copy of calibration values for editing.
    physical_min: f64,
    physical_center_low: f64,
    physical_center_high: f64,
    physical_max: f64,
    enabled: bool,
    with_center: bool,
    /// Recording state.
    recording_mode: RecordingMode,
    recorded_min: f64,
    recorded_max: f64,
    /// Whether this axis has unsaved changes.
    unsaved: bool,
}

impl Default for AxisEditor {
    fn default() -> Self {
        Self {
            physical_min: -1.0,
            physical_center_low: 0.0,
            physical_center_high: 0.0,
            physical_max: 1.0,
            enabled: true,
            with_center: false,
            recording_mode: RecordingMode::None,
            recorded_min: 0.0,
            recorded_max: 0.0,
            unsaved: false,
        }
    }
}

impl AxisEditor {
    /// Create an editor pre-populated from an existing calibration.
    fn from_calibration(cal: &Calibration) -> Self {
        let center_low = cal.physical_center_low();
        let center_high = cal.physical_center_high();
        let with_center = (center_low - center_high).abs() > f64::EPSILON;
        Self {
            physical_min: cal.physical_min(),
            physical_center_low: center_low,
            physical_center_high: center_high,
            physical_max: cal.physical_max(),
            enabled: cal.enabled(),
            with_center,
            recording_mode: RecordingMode::None,
            recorded_min: 0.0,
            recorded_max: 0.0,
            unsaved: false,
        }
    }

    /// Build a [`Calibration`] from the current editor values, or `None` if invalid.
    fn to_calibration(&self) -> Option<Calibration> {
        let (center_low, center_high) = if self.with_center {
            (self.physical_center_low, self.physical_center_high)
        } else {
            (0.0, 0.0)
        };
        Calibration::new(
            self.physical_min,
            center_low,
            center_high,
            self.physical_max,
            self.enabled,
        )
        .ok()
    }
}

/// Recording mode for live min/max capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordingMode {
    None,
    Center,
    Extrema,
}

/// Show the calibration window as a native OS window.
pub(crate) fn show(
    ctx: &egui::Context,
    window_state: &mut CalibrationWindowState,
    open: &mut bool,
    cache: &CachedState,
    app_state: &Arc<RwLock<AppState>>,
    commands: &mpsc::Sender<EngineCommand>,
) {
    if !*open {
        return;
    }

    ctx.show_viewport_immediate(
        viewport_id(),
        egui::ViewportBuilder::default()
            .with_title("InputForge \u{2014} Calibration")
            .with_inner_size([DEFAULT_WIDTH, DEFAULT_HEIGHT])
            .with_min_inner_size([MIN_WIDTH, MIN_HEIGHT]),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                *open = false;
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                let colors = theme::colors(ui.ctx());

                // --- Device selector ---
                show_device_selector(ui, window_state, cache, app_state);

                ui.separator();

                let Some(device_idx) = window_state.selected_device_idx else {
                    ui.label(egui::RichText::new("Select a device above.").color(colors.text_dim));
                    return;
                };

                if device_idx >= cache.devices.len() {
                    window_state.selected_device_idx = None;
                    return;
                }

                let axis_count = cache.devices[device_idx].info.axes as usize;
                if axis_count == 0 {
                    ui.label(
                        egui::RichText::new("This device has no axes.").color(colors.text_dim),
                    );
                    return;
                }

                // --- Two-column layout ---
                let available = ui.available_size();
                let right_width =
                    (available.x - LEFT_COLUMN_WIDTH - ui.spacing().item_spacing.x).max(200.0);

                ui.horizontal(|ui| {
                    // Left column: axis list
                    ui.vertical(|ui| {
                        ui.set_width(LEFT_COLUMN_WIDTH);
                        show_axis_list(ui, window_state, cache, device_idx, axis_count, colors);
                    });

                    ui.separator();

                    // Right column: axis detail
                    ui.vertical(|ui| {
                        ui.set_width(right_width);
                        show_axis_detail(ui, window_state, cache, device_idx, colors, commands);
                    });
                });
            });
        },
    );
}

/// Render the device selector combobox.
fn show_device_selector(
    ui: &mut egui::Ui,
    window_state: &mut CalibrationWindowState,
    cache: &CachedState,
    app_state: &Arc<RwLock<AppState>>,
) {
    let selected_label = window_state
        .selected_device_idx
        .and_then(|idx| cache.devices.get(idx))
        .map_or_else(|| "-- Select Device --".to_owned(), |d| d.info.name.clone());

    let mut changed = false;
    egui::ComboBox::from_id_salt("Device")
        .selected_text(&selected_label)
        .show_ui(ui, |ui| {
            for (i, device) in cache.devices.iter().enumerate() {
                let is_selected = window_state.selected_device_idx == Some(i);
                if ui
                    .selectable_label(is_selected, &device.info.name)
                    .clicked()
                {
                    window_state.selected_device_idx = Some(i);
                    changed = true;
                }
            }
        });

    if changed {
        rebuild_axis_editors(window_state, cache, app_state);
    }
}

/// Rebuild axis editors when the selected device changes.
fn rebuild_axis_editors(
    window_state: &mut CalibrationWindowState,
    cache: &CachedState,
    app_state: &Arc<RwLock<AppState>>,
) {
    window_state.selected_axis = None;

    let Some(device_idx) = window_state.selected_device_idx else {
        window_state.axis_editors.clear();
        return;
    };

    let Some(device) = cache.devices.get(device_idx) else {
        window_state.axis_editors.clear();
        return;
    };

    let axis_count = device.info.axes as usize;
    let guard = app_state.read();

    window_state.axis_editors = (0..axis_count)
        .map(|i| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "axis index fits in u8 by hardware constraint"
            )]
            let axis_idx = i as u8;
            guard
                .calibrations
                .get(&device.info.id, axis_idx)
                .map_or_else(AxisEditor::default, AxisEditor::from_calibration)
        })
        .collect();
}

/// Render the scrollable axis list in the left column.
fn show_axis_list(
    ui: &mut egui::Ui,
    window_state: &mut CalibrationWindowState,
    cache: &CachedState,
    device_idx: usize,
    axis_count: usize,
    colors: &theme::ThemeColors,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let snapshot = cache.input_snapshots.get(device_idx);

        for i in 0..axis_count {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "axis index fits in u8 by hardware constraint"
            )]
            let axis_idx = i as u8;
            let is_selected = window_state.selected_axis == Some(axis_idx);

            let value = snapshot
                .and_then(|s| s.axes.get(i))
                .copied()
                .unwrap_or((0.0, AxisPolarity::Bipolar))
                .0;
            let label = format!("A{i}");

            let response = ui
                .horizontal(|ui| {
                    // Selected axis accent border.
                    if is_selected {
                        let rect = ui.available_rect_before_wrap();
                        let accent_rect = egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(ACCENT_BORDER_WIDTH, rect.height().max(18.0)),
                        );
                        ui.painter().rect_filled(accent_rect, 0.0, colors.primary);
                        ui.add_space(ACCENT_BORDER_WIDTH + 2.0);
                    }

                    axis_bar::axis_bar(ui, &label, value, AxisPolarity::Bipolar);
                })
                .response;

            if response.interact(egui::Sense::click()).clicked() {
                window_state.selected_axis = Some(axis_idx);
            }
        }
    });
}

/// Render the selected axis detail in the right column.
fn show_axis_detail(
    ui: &mut egui::Ui,
    window_state: &mut CalibrationWindowState,
    cache: &CachedState,
    device_idx: usize,
    colors: &theme::ThemeColors,
    commands: &mpsc::Sender<EngineCommand>,
) {
    let Some(axis_idx) = window_state.selected_axis else {
        ui.label(egui::RichText::new("Select an axis from the list.").color(colors.text_dim));
        return;
    };

    let editor_idx = axis_idx as usize;
    if editor_idx >= window_state.axis_editors.len() {
        ui.label(egui::RichText::new("Axis not available.").color(colors.text_dim));
        return;
    }

    let raw_value = cache
        .input_snapshots
        .get(device_idx)
        .and_then(|s| s.axes.get(editor_idx))
        .copied()
        .unwrap_or((0.0, AxisPolarity::Bipolar))
        .0;

    // --- Section label ---
    ui.label(
        egui::RichText::new(format!("Axis {axis_idx}"))
            .family(FontFamily::Name("SemiBold".into()))
            .size(16.0),
    );

    ui.add_space(4.0);

    // --- Raw value bar ---
    axis_bar::axis_bar_colored(
        ui,
        "Raw",
        raw_value,
        AxisPolarity::Bipolar,
        colors.indicator_idle,
        colors.text_dim,
    );

    // --- Calibrated value bar ---
    let editor = &window_state.axis_editors[editor_idx];
    let calibrated = editor
        .to_calibration()
        .map_or(raw_value, |cal| cal.apply(raw_value));
    axis_bar::axis_bar_colored(
        ui,
        "Cal",
        calibrated,
        AxisPolarity::Bipolar,
        colors.live,
        colors.live,
    );

    ui.add_space(4.0);

    // --- Calibration zone bar ---
    if let Some(cal) = editor.to_calibration() {
        calibration_editor::paint_calibration_bar(ui, &cal, Some(raw_value));
    }

    ui.add_space(8.0);

    // --- Threshold editors ---
    let editor = &mut window_state.axis_editors[editor_idx];

    let mut changed = false;
    egui::Grid::new(ui.id().with("cal_thresholds"))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Min").color(colors.text_dim));
            changed |= ui
                .add(
                    egui::DragValue::new(&mut editor.physical_min)
                        .range(-RANGE_LIMIT..=RANGE_LIMIT)
                        .speed(DRAG_SPEED),
                )
                .changed();
            ui.end_row();

            if editor.with_center {
                ui.label(egui::RichText::new("Ctr Low").color(colors.text_dim));
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut editor.physical_center_low)
                            .range(-RANGE_LIMIT..=RANGE_LIMIT)
                            .speed(DRAG_SPEED),
                    )
                    .changed();
                ui.end_row();

                ui.label(egui::RichText::new("Ctr High").color(colors.text_dim));
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut editor.physical_center_high)
                            .range(-RANGE_LIMIT..=RANGE_LIMIT)
                            .speed(DRAG_SPEED),
                    )
                    .changed();
                ui.end_row();
            }

            ui.label(egui::RichText::new("Max").color(colors.text_dim));
            changed |= ui
                .add(
                    egui::DragValue::new(&mut editor.physical_max)
                        .range(-RANGE_LIMIT..=RANGE_LIMIT)
                        .speed(DRAG_SPEED),
                )
                .changed();
            ui.end_row();

            ui.label(egui::RichText::new("With Center").color(colors.text_dim));
            changed |= ui.checkbox(&mut editor.with_center, "").changed();
            ui.end_row();

            ui.label(egui::RichText::new("Enabled").color(colors.text_dim));
            changed |= ui.checkbox(&mut editor.enabled, "").changed();
            ui.end_row();
        });

    if changed {
        editor.unsaved = true;
    }

    ui.add_space(8.0);

    // --- Recording buttons ---
    show_recording_buttons(ui, editor, raw_value, colors);

    ui.add_space(8.0);

    // --- Action buttons ---
    show_action_buttons(
        ui,
        window_state,
        device_idx,
        editor_idx,
        colors,
        cache,
        commands,
    );
}

/// Render the recording toggle buttons (Center / Extrema).
fn show_recording_buttons(
    ui: &mut egui::Ui,
    editor: &mut AxisEditor,
    raw_value: f64,
    colors: &theme::ThemeColors,
) {
    ui.horizontal(|ui| {
        // Record Center
        let is_center = editor.recording_mode == RecordingMode::Center;
        let center_text = egui::RichText::new("Record Center");
        let center_text = if is_center {
            center_text.color(colors.live)
        } else {
            center_text
        };

        if ui.selectable_label(is_center, center_text).clicked() {
            if is_center {
                editor.recording_mode = RecordingMode::None;
            } else {
                editor.recording_mode = RecordingMode::Center;
                editor.recorded_min = raw_value;
                editor.recorded_max = raw_value;
            }
        }

        // Record Extrema
        let is_extrema = editor.recording_mode == RecordingMode::Extrema;
        let extrema_text = egui::RichText::new("Record Extrema");
        let extrema_text = if is_extrema {
            extrema_text.color(colors.live)
        } else {
            extrema_text
        };

        if ui.selectable_label(is_extrema, extrema_text).clicked() {
            if is_extrema {
                editor.recording_mode = RecordingMode::None;
            } else {
                editor.recording_mode = RecordingMode::Extrema;
                editor.recorded_min = raw_value;
                editor.recorded_max = raw_value;
            }
        }
    });

    // Apply recording each frame while active.
    match editor.recording_mode {
        RecordingMode::Center => {
            editor.recorded_min = editor.recorded_min.min(raw_value);
            editor.recorded_max = editor.recorded_max.max(raw_value);
            editor.physical_center_low = editor.recorded_min;
            editor.physical_center_high = editor.recorded_max;
            editor.unsaved = true;
        }
        RecordingMode::Extrema => {
            editor.recorded_min = editor.recorded_min.min(raw_value);
            editor.recorded_max = editor.recorded_max.max(raw_value);
            editor.physical_min = editor.recorded_min;
            editor.physical_max = editor.recorded_max;
            editor.unsaved = true;
        }
        RecordingMode::None => {}
    }
}

/// Render the Reset and Save action buttons.
fn show_action_buttons(
    ui: &mut egui::Ui,
    window_state: &mut CalibrationWindowState,
    device_idx: usize,
    editor_idx: usize,
    colors: &theme::ThemeColors,
    cache: &CachedState,
    commands: &mpsc::Sender<EngineCommand>,
) {
    let editor = &mut window_state.axis_editors[editor_idx];

    ui.horizontal(|ui| {
        if ui.button("Reset").clicked() {
            *editor = AxisEditor::default();
            editor.unsaved = true;
        }

        if editor.unsaved {
            let save_button = egui::Button::new(egui::RichText::new("Save").color(colors.text))
                .fill(colors.warning);

            if ui.add(save_button).clicked() {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "axis index fits in u8 by hardware constraint"
                )]
                let axis_idx = editor_idx as u8;
                if let Some(cal) = editor.to_calibration() {
                    if let Some(device) = cache.devices.get(device_idx) {
                        let cmd = EngineCommand::SetCalibration {
                            device: device.info.id.clone(),
                            axis: axis_idx,
                            calibration: cal,
                        };
                        if let Err(e) = commands.send(cmd) {
                            tracing::warn!(error = %e, "failed to send SetCalibration command");
                        }
                        if let Err(e) = commands.send(EngineCommand::SaveCalibrations) {
                            tracing::warn!(error = %e, "failed to send SaveCalibrations command");
                        }
                        editor.unsaved = false;
                    }
                }
            }
        } else {
            // Disabled-looking save button when no changes.
            ui.add_enabled(false, egui::Button::new("Save"));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_editor_default_values() {
        let editor = AxisEditor::default();
        assert!((editor.physical_min - (-1.0)).abs() < f64::EPSILON);
        assert!(editor.physical_center_low.abs() < f64::EPSILON);
        assert!(editor.physical_center_high.abs() < f64::EPSILON);
        assert!((editor.physical_max - 1.0).abs() < f64::EPSILON);
        assert!(editor.enabled);
        assert!(!editor.with_center);
        assert_eq!(editor.recording_mode, RecordingMode::None);
        assert!(editor.recorded_min.abs() < f64::EPSILON);
        assert!(editor.recorded_max.abs() < f64::EPSILON);
        assert!(!editor.unsaved);
    }

    #[test]
    fn axis_editor_to_calibration_default_is_valid() {
        let editor = AxisEditor::default();
        let cal = editor.to_calibration();
        assert!(
            cal.is_some(),
            "default editor should produce a valid calibration"
        );
        let cal = cal.unwrap();
        assert!((cal.physical_min() - (-1.0)).abs() < f64::EPSILON);
        assert!((cal.physical_max() - 1.0).abs() < f64::EPSILON);
        assert!(cal.enabled());
    }

    #[test]
    fn axis_editor_to_calibration_without_center() {
        let mut editor = AxisEditor::default();
        editor.with_center = false;
        let cal = editor.to_calibration();
        assert!(cal.is_some());
        let cal = cal.unwrap();
        assert!((cal.physical_center_low()).abs() < f64::EPSILON);
        assert!((cal.physical_center_high()).abs() < f64::EPSILON);
    }

    #[test]
    fn axis_editor_to_calibration_invalid_returns_none() {
        let mut editor = AxisEditor::default();
        // min >= center_low violates invariant.
        editor.physical_min = 1.0;
        editor.physical_center_low = 0.0;
        assert!(editor.to_calibration().is_none());
    }

    #[test]
    fn axis_editor_from_calibration_roundtrip() {
        let cal = Calibration::new(-500.0, -10.0, 10.0, 500.0, true).unwrap();
        let editor = AxisEditor::from_calibration(&cal);
        assert!((editor.physical_min - (-500.0)).abs() < f64::EPSILON);
        assert!((editor.physical_center_low - (-10.0)).abs() < f64::EPSILON);
        assert!((editor.physical_center_high - 10.0).abs() < f64::EPSILON);
        assert!((editor.physical_max - 500.0).abs() < f64::EPSILON);
        assert!(editor.enabled);
        assert!(editor.with_center);
        assert!(!editor.unsaved);
    }

    #[test]
    fn axis_editor_from_calibration_zero_width_center() {
        let cal = Calibration::new(-1.0, 0.0, 0.0, 1.0, true).unwrap();
        let editor = AxisEditor::from_calibration(&cal);
        // Zero-width center band -> with_center is false.
        assert!(!editor.with_center);
    }

    #[test]
    fn recording_mode_variants_are_distinct() {
        assert_ne!(RecordingMode::None, RecordingMode::Center);
        assert_ne!(RecordingMode::None, RecordingMode::Extrema);
        assert_ne!(RecordingMode::Center, RecordingMode::Extrema);
    }

    #[test]
    fn calibration_window_state_default() {
        let state = CalibrationWindowState::default();
        assert!(state.selected_device_idx.is_none());
        assert!(state.selected_axis.is_none());
        assert!(state.axis_editors.is_empty());
    }

    #[test]
    fn constants_are_positive() {
        assert!(DEFAULT_WIDTH > 0.0);
        assert!(DEFAULT_HEIGHT > 0.0);
        assert!(MIN_WIDTH > 0.0);
        assert!(MIN_HEIGHT > 0.0);
        assert!(LEFT_COLUMN_WIDTH > 0.0);
        assert!(ACCENT_BORDER_WIDTH > 0.0);
        assert!(DRAG_SPEED > 0.0);
        assert!(RANGE_LIMIT > 0.0);
    }
}
