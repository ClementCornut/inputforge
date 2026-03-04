// Rust guideline compliant 2026-03-03

//! Reusable custom-painted widgets for the `InputForge` GUI.
//!
//! - `action_card`: color-coded action card header row with category badge and control buttons
//! - `action_config`: per-variant configuration UI for each action type
//! - `axis_bar`: horizontal live axis indicator
//! - `button_grid`: grid of button state circles
//! - `calibration_editor`: visual calibration configuration editor
//! - `curve_editor`: interactive response curve editor with `egui_plot`
//! - `deadzone_editor`: visual deadzone configuration editor
//! - `empty_state`: centered placeholder for empty/unimplemented views
//! - `hat_indicator`: 8-way compass direction display
//! - `status_dot`: connection/status indicator dot

pub(crate) mod action_card;
pub(crate) mod action_config;
pub(crate) mod axis_bar;
pub(crate) mod button_grid;
pub(crate) mod calibration_editor;
pub(crate) mod curve_editor;
pub(crate) mod deadzone_editor;
pub(crate) mod empty_state;
pub(crate) mod hat_indicator;
pub(crate) mod status_dot;
