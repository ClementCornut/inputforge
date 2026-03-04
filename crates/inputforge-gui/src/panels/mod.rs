// Rust guideline compliant 2026-03-03

//! UI panel modules for the three-panel application layout.
//!
//! - [`center_panel`]: Routes between overview, editor, monitor, and mode views.
//! - [`device_view`]: Per-device collapsible section with live axis, button, and hat data.
//! - [`input_monitor`]: Real-time scrollable event log with filtering and pause controls.
//! - [`left_panel`]: Resizable sidebar with device tree.
//! - [`mapping_editor`]: Action pipeline editor with arrow-button card reordering.
//! - [`mode_editor`]: Hierarchical mode tree display and selection.
//! - [`status_bar`]: Bottom status bar with engine state and mode badge.

pub(crate) mod center_panel;
pub(crate) mod device_view;
pub(crate) mod input_monitor;
pub(crate) mod left_panel;
pub(crate) mod mapping_editor;
pub(crate) mod mode_editor;
pub(crate) mod status_bar;
