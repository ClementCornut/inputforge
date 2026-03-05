// Rust guideline compliant 2026-03-03

//! Bottom status bar showing engine status, mode badge, and device count.
//!
//! Rendered as an `egui::TopBottomPanel::bottom` with a compact
//! horizontal layout. Color-coded indicators provide at-a-glance
//! awareness of engine state.

use inputforge_core::state::EngineStatus;

use crate::app::CachedState;
use crate::theme;
use crate::widgets::status_dot;

/// Render the bottom status bar.
pub(crate) fn show(ctx: &egui::Context, cache: &CachedState) {
    let colors = theme::colors(ctx);
    egui::TopBottomPanel::bottom("status_bar")
        // 28px: fits one text row (14px) + 7px vertical padding on each side.
        .exact_height(28.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Engine status indicator.
                show_engine_status(ui, cache.engine_status);

                ui.separator();

                // Current mode badge.
                show_mode_badge(ui, &cache.current_mode);

                ui.separator();

                // Device count.
                let connected = cache.devices.iter().filter(|d| d.connected).count();
                let total = cache.devices.len();
                ui.label(
                    egui::RichText::new(format!("{connected}/{total} devices"))
                        .color(colors.text_dim)
                        .size(theme::SMALL_FONT_SIZE),
                );

                // Profile name (right-aligned).
                if let Some(ref name) = cache.profile_name {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(name)
                                .color(colors.text_dim)
                                .size(theme::SMALL_FONT_SIZE),
                        );
                    });
                }
            });
        });
}

/// Render the colored engine status dot and label.
fn show_engine_status(ui: &mut egui::Ui, status: EngineStatus) {
    let colors = theme::colors(ui.ctx());
    let (color, label) = match status {
        EngineStatus::Running => (colors.live, "Running"),
        EngineStatus::Paused => (colors.warning, "Paused"),
        EngineStatus::Stopped => (colors.error, "Stopped"),
    };

    // Status dot: filled when running, ring otherwise.
    let is_running = matches!(status, EngineStatus::Running);
    status_dot::status_dot(ui, color, is_running);

    ui.label(
        egui::RichText::new(label)
            .color(color)
            .size(theme::SMALL_FONT_SIZE),
    );
}

/// Render the current mode name with a "Mode:" prefix and purple accent.
fn show_mode_badge(ui: &mut egui::Ui, mode: &str) {
    let colors = theme::colors(ui.ctx());
    ui.label(
        egui::RichText::new("Mode:")
            .color(colors.text_dim)
            .size(theme::SMALL_FONT_SIZE),
    );
    ui.label(
        egui::RichText::new(mode)
            .color(colors.special)
            .size(theme::SMALL_FONT_SIZE),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_status_colors_differ() {
        let statuses = [
            EngineStatus::Running,
            EngineStatus::Paused,
            EngineStatus::Stopped,
        ];
        let colors: Vec<_> = statuses
            .iter()
            .map(|s| match s {
                EngineStatus::Running => theme::DARK.live,
                EngineStatus::Paused => theme::DARK.warning,
                EngineStatus::Stopped => theme::DARK.error,
            })
            .collect();

        for (i, a) in colors.iter().enumerate() {
            for (j, b) in colors.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "status colors at {i} and {j} must differ");
                }
            }
        }
    }
}
