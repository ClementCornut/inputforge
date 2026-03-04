// Rust guideline compliant 2026-03-03

//! Real-time input event monitor panel.
//!
//! Displays a scrollable, filterable table of input events captured
//! from all connected devices. Events are stored in a ring buffer
//! capped at [`MAX_ENTRIES`] entries.
//!
//! The panel exposes [`InputMonitorState`] for persistent state and
//! a [`show`] function to render the toolbar and event table.

use std::collections::VecDeque;

use egui_extras::{Column, TableBuilder};

use crate::theme;

/// Maximum number of entries kept in the ring buffer.
const MAX_ENTRIES: usize = 500;

/// Ring buffer entry for monitored input events.
#[derive(Debug, Clone)]
pub(crate) struct MonitorEntry {
    /// Millisecond timestamp since engine start.
    pub timestamp_ms: u64,
    /// Human-readable device name that generated the event.
    pub device_name: String,
    /// Label of the input (e.g. "Axis 0", "Button 3").
    pub input_label: String,
    /// Formatted value text (e.g. "0.75", "Pressed").
    pub value_text: String,
    /// Active mode when the event was captured.
    pub mode: String,
}

/// State for the input monitor panel.
///
/// Holds a ring buffer of [`MonitorEntry`] items and toolbar toggles
/// for pause, auto-scroll, and text filtering.
#[derive(Debug)]
pub(crate) struct InputMonitorState {
    entries: VecDeque<MonitorEntry>,
    auto_scroll: bool,
    paused: bool,
    filter_text: String,
    /// Row count from the previous frame, used to detect new entries
    /// so `scroll_to_row` is only called when data actually changed.
    last_row_count: usize,
}

impl Default for InputMonitorState {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            auto_scroll: true,
            paused: false,
            filter_text: String::new(),
            last_row_count: 0,
        }
    }
}

impl InputMonitorState {
    /// Create an empty monitor state with auto-scroll enabled and unpaused.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Append an entry to the ring buffer.
    ///
    /// If the monitor is paused, the entry is silently discarded.
    /// If the buffer exceeds [`MAX_ENTRIES`], the oldest entry is removed.
    pub(crate) fn push(&mut self, entry: MonitorEntry) {
        if self.paused {
            return;
        }
        self.entries.push_back(entry);
        while self.entries.len() > MAX_ENTRIES {
            self.entries.pop_front();
        }
    }

    /// Remove all entries from the buffer.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    /// Return an iterator over entries matching the current filter text.
    ///
    /// Matching is case-insensitive against `device_name`, `input_label`,
    /// and `mode`. If the filter text is empty, all entries are returned.
    pub(crate) fn filtered_entries(&self) -> impl Iterator<Item = &MonitorEntry> {
        let filter = self.filter_text.to_lowercase();
        self.entries.iter().filter(move |e| {
            if filter.is_empty() {
                return true;
            }
            e.device_name.to_lowercase().contains(&filter)
                || e.input_label.to_lowercase().contains(&filter)
                || e.mode.to_lowercase().contains(&filter)
        })
    }
}

/// Render the input monitor panel with toolbar and event table.
pub(crate) fn show(ui: &mut egui::Ui, state: &mut InputMonitorState) {
    show_toolbar(ui, state);

    if state.paused {
        let colors = theme::colors(ui.ctx());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("PAUSED").color(colors.warning).strong());
        });
    }

    ui.separator();
    let row_count = show_table(ui, state);
    state.last_row_count = row_count;
}

/// Render the toolbar row: pause, auto-scroll, filter, clear, count.
fn show_toolbar(ui: &mut egui::Ui, state: &mut InputMonitorState) {
    ui.horizontal(|ui| {
        // Pause / Resume button.
        let pause_label = if state.paused { "Resume" } else { "Pause" };
        if ui.button(pause_label).clicked() {
            state.paused = !state.paused;
            if state.paused {
                // Disable auto-scroll when pausing so the user can scroll freely.
                state.auto_scroll = false;
            } else {
                // Restore auto-scroll when resuming so the live tail resumes.
                state.auto_scroll = true;
            }
        }

        ui.separator();

        // Auto-scroll toggle.
        ui.checkbox(&mut state.auto_scroll, "Auto-scroll");

        ui.separator();

        // Filter text input.
        ui.label("Filter:");
        ui.add(
            egui::TextEdit::singleline(&mut state.filter_text)
                .desired_width(150.0)
                .hint_text("device or input..."),
        );

        ui.separator();

        // Clear button.
        if ui.button("Clear").clicked() {
            state.clear();
        }

        ui.separator();

        // Entry count label.
        let total = state.entries.len();
        let colors = theme::colors(ui.ctx());
        ui.label(egui::RichText::new(format!("{total} / {MAX_ENTRIES}")).color(colors.text_dim));
    });
}

/// Render the event table using `egui_extras::TableBuilder`.
///
/// Returns the current filtered row count so the caller can update
/// [`InputMonitorState::last_row_count`] after the borrow on `state`
/// (via `filtered_entries`) is released.
fn show_table(ui: &mut egui::Ui, state: &InputMonitorState) -> usize {
    let colors = theme::colors(ui.ctx());
    // Collect filtered entries so we can index into them for the table body.
    let filtered: Vec<&MonitorEntry> = state.filtered_entries().collect();
    let row_count = filtered.len();

    let available_height = ui.available_height();

    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(80.0).at_least(60.0)) // Time
        .column(Column::initial(180.0).at_least(100.0)) // Device
        .column(Column::initial(140.0).at_least(80.0)) // Input
        .column(Column::initial(100.0).at_least(60.0)) // Value
        .column(Column::remainder().at_least(80.0)) // Mode
        .min_scrolled_height(0.0)
        .max_scroll_height(available_height);

    // Only auto-scroll when new rows have been appended, so the user
    // can freely scroll up without the table snapping back every frame.
    if state.auto_scroll && row_count > state.last_row_count {
        table = table.scroll_to_row(row_count.saturating_sub(1), Some(egui::Align::BOTTOM));
    }

    table
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Time");
            });
            header.col(|ui| {
                ui.strong("Device");
            });
            header.col(|ui| {
                ui.strong("Input");
            });
            header.col(|ui| {
                ui.strong("Value");
            });
            header.col(|ui| {
                ui.strong("Mode");
            });
        })
        .body(|body| {
            body.rows(18.0, row_count, |mut row| {
                let entry = filtered[row.index()];
                row.col(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{}", entry.timestamp_ms))
                            .color(colors.text_dim)
                            .monospace(),
                    );
                });
                row.col(|ui| {
                    ui.label(&entry.device_name);
                });
                row.col(|ui| {
                    ui.label(&entry.input_label);
                });
                row.col(|ui| {
                    ui.label(egui::RichText::new(&entry.value_text).monospace());
                });
                row.col(|ui| {
                    ui.label(egui::RichText::new(&entry.mode).color(colors.special));
                });
            });
        });

    row_count
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test entry with the given device name and label.
    fn make_entry(device: &str, label: &str) -> MonitorEntry {
        MonitorEntry {
            timestamp_ms: 0,
            device_name: device.to_owned(),
            input_label: label.to_owned(),
            value_text: "0".to_owned(),
            mode: "Default".to_owned(),
        }
    }

    #[test]
    fn ring_buffer_respects_max_500() {
        let mut state = InputMonitorState::new();
        for i in 0..600 {
            state.push(MonitorEntry {
                timestamp_ms: i,
                device_name: format!("dev-{i}"),
                input_label: "Axis 0".to_owned(),
                value_text: "0.5".to_owned(),
                mode: "Default".to_owned(),
            });
        }
        assert_eq!(state.entries.len(), 500);
        // The oldest entry should be index 100 (0..99 were evicted).
        assert_eq!(state.entries.front().unwrap().timestamp_ms, 100);
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut state = InputMonitorState::new();
        for _ in 0..10 {
            state.push(make_entry("dev", "btn"));
        }
        assert_eq!(state.entries.len(), 10);
        state.clear();
        assert!(state.entries.is_empty());
    }

    #[test]
    fn filter_narrows_results() {
        let mut state = InputMonitorState::new();
        state.push(make_entry("Joystick", "Axis 0"));
        state.push(make_entry("Throttle", "Slider 1"));
        state.push(make_entry("Joystick", "Button 3"));
        state.push(make_entry("Pedals", "Axis 0"));

        // No filter returns all.
        assert_eq!(state.filtered_entries().count(), 4);

        // Filter by device name.
        state.filter_text = "joystick".to_owned();
        assert_eq!(state.filtered_entries().count(), 2);

        // Filter by input label.
        state.filter_text = "axis".to_owned();
        assert_eq!(state.filtered_entries().count(), 2);

        // Filter with no match.
        state.filter_text = "nonexistent".to_owned();
        assert_eq!(state.filtered_entries().count(), 0);
    }

    #[test]
    fn pause_gates_push_and_resume_accepts() {
        let mut state = InputMonitorState::new();

        // Entry arrives while unpaused.
        state.push(make_entry("dev", "btn-1"));
        assert_eq!(state.entries.len(), 1);

        // Pause — entry is silently discarded.
        state.paused = true;
        state.push(make_entry("dev", "btn-2"));
        assert_eq!(state.entries.len(), 1);

        // Resume — entries flow again.
        state.paused = false;
        state.push(make_entry("dev", "btn-3"));
        assert_eq!(state.entries.len(), 2);
    }

    #[test]
    fn filter_includes_mode_field() {
        let mut state = InputMonitorState::new();
        state.push(MonitorEntry {
            timestamp_ms: 0,
            device_name: "Joystick".to_owned(),
            input_label: "Axis 0".to_owned(),
            value_text: "0".to_owned(),
            mode: "Combat".to_owned(),
        });
        state.filter_text = "combat".to_owned();
        assert_eq!(state.filtered_entries().count(), 1);
    }

    #[test]
    fn new_state_defaults() {
        let state = InputMonitorState::new();
        assert!(state.entries.is_empty());
        assert!(state.auto_scroll);
        assert!(!state.paused);
        assert!(state.filter_text.is_empty());
    }
}
