// Rust guideline compliant 2026-03-06

//! Toast notification system for transient warnings and errors.
//!
//! Renders floating notifications anchored to the top-right corner of the
//! window. Toasts auto-dismiss after [`TOAST_DURATION`] but pause their
//! timer while hovered. Each toast has a colored left accent bar matching
//! its severity level.

use std::time::{Duration, Instant};

use egui::{Area, Frame, Id, Margin, Order, Stroke, Vec2};

use crate::theme;

/// How long a toast stays visible before auto-dismissing.
///
/// Chosen to give the user enough time to read a two-line message
/// without being so long that stale toasts pile up.
const TOAST_DURATION: Duration = Duration::from_secs(8);

/// Duration of the fade-out animation before a toast disappears.
const FADE_DURATION: Duration = Duration::from_secs(1);

/// Maximum width of a toast notification in logical pixels.
const TOAST_MAX_WIDTH: f32 = 380.0;

/// Vertical gap between stacked toasts in logical pixels.
const TOAST_GAP: f32 = 8.0;

/// Margin from the top-right corner of the screen in logical pixels.
const TOAST_MARGIN: f32 = 12.0;

/// Width of the colored accent bar on the left edge of a toast.
const ACCENT_WIDTH: f32 = 3.0;

/// Severity level of a toast notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToastLevel {
    /// Amber accent — non-fatal issue the user should be aware of.
    Warning,
    /// Red accent — something went wrong.
    #[expect(dead_code, reason = "will be used for engine error toasts")]
    Error,
}

/// A single toast notification with message, level, and lifetime.
#[derive(Debug)]
struct Toast {
    message: String,
    level: ToastLevel,
    created: Instant,
    dismissed: bool,
    /// Accumulated pause time from hovering.
    paused_duration: Duration,
    /// When the hover pause started (if currently hovered).
    pause_start: Option<Instant>,
}

impl Toast {
    fn new(message: String, level: ToastLevel) -> Self {
        Self {
            message,
            level,
            created: Instant::now(),
            dismissed: false,
            paused_duration: Duration::ZERO,
            pause_start: None,
        }
    }

    /// Effective elapsed time, excluding paused intervals.
    fn effective_elapsed(&self) -> Duration {
        let total = self.created.elapsed();
        let current_pause = self
            .pause_start
            .map_or(Duration::ZERO, |start| start.elapsed());
        total.saturating_sub(self.paused_duration + current_pause)
    }

    /// Whether this toast has exceeded its display duration.
    fn is_expired(&self) -> bool {
        self.dismissed || self.effective_elapsed() >= TOAST_DURATION
    }

    /// Opacity multiplier in `[0.0, 1.0]` for the fade-out animation.
    fn opacity(&self) -> f32 {
        let elapsed = self.effective_elapsed();
        let fade_start = TOAST_DURATION.saturating_sub(FADE_DURATION);
        if elapsed >= TOAST_DURATION {
            return 0.0;
        }
        if elapsed <= fade_start {
            return 1.0;
        }
        let fade_elapsed = elapsed.saturating_sub(fade_start);
        1.0 - (fade_elapsed.as_secs_f32() / FADE_DURATION.as_secs_f32())
    }
}

/// Manages a collection of toast notifications.
///
/// Owned by [`InputForgeApp`](crate::app::InputForgeApp) and rendered
/// each frame via [`show`](Self::show).
#[derive(Debug, Default)]
pub(crate) struct ToastManager {
    toasts: Vec<Toast>,
    next_id: usize,
}

impl ToastManager {
    /// Add a new toast notification.
    pub(crate) fn push(&mut self, message: String, level: ToastLevel) {
        self.toasts.push(Toast::new(message, level));
    }

    /// Render all active toasts as floating overlays.
    ///
    /// Expired toasts are removed after rendering. Call this at the end
    /// of `update()` so toasts appear on top of all panels.
    pub(crate) fn show(&mut self, ctx: &egui::Context) {
        let colors = theme::colors(ctx);

        let mut y_offset = TOAST_MARGIN;

        for (i, toast) in self.toasts.iter_mut().enumerate() {
            if toast.is_expired() {
                continue;
            }

            let opacity = toast.opacity();
            let accent_color = match toast.level {
                ToastLevel::Warning => colors.warning,
                ToastLevel::Error => colors.error,
            };

            let toast_id = Id::new("inputforge_toast").with(self.next_id + i);

            let response = Area::new(toast_id)
                .order(Order::Foreground)
                .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-TOAST_MARGIN, y_offset))
                .show(ctx, |ui| {
                    ui.set_max_width(TOAST_MAX_WIDTH);
                    ui.set_opacity(opacity);

                    Frame::new()
                        .fill(colors.surface0)
                        .stroke(Stroke::new(1.0, colors.surface1.gamma_multiply(opacity)))
                        .corner_radius(4.0)
                        .inner_margin(Margin::ZERO)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Left accent bar.
                                let bar_rect = ui.allocate_space(Vec2::new(ACCENT_WIDTH, 0.0)).1;
                                // The bar height will be painted after layout.
                                let _ = bar_rect;

                                ui.add_space(8.0);

                                ui.vertical(|ui| {
                                    ui.add_space(8.0);

                                    ui.horizontal(|ui| {
                                        // Icon.
                                        let icon = match toast.level {
                                            ToastLevel::Warning => "\u{26A0}",
                                            ToastLevel::Error => "\u{2716}",
                                        };
                                        ui.colored_label(
                                            accent_color.gamma_multiply(opacity),
                                            icon,
                                        );

                                        ui.label(
                                            egui::RichText::new(&toast.message)
                                                .color(colors.text.gamma_multiply(opacity)),
                                        );
                                    });

                                    ui.add_space(8.0);
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::TOP),
                                    |ui| {
                                        ui.add_space(4.0);
                                        ui.add_space(4.0);
                                        let close = ui.small_button(
                                            egui::RichText::new("\u{2715}")
                                                .color(colors.text_dim.gamma_multiply(opacity)),
                                        );
                                        if close.clicked() {
                                            toast.dismissed = true;
                                        }
                                    },
                                );
                            });

                            // Paint the accent bar over the full height of the toast.
                            let full_rect = ui.min_rect();
                            let bar_rect = egui::Rect::from_min_size(
                                full_rect.left_top(),
                                Vec2::new(ACCENT_WIDTH, full_rect.height()),
                            );
                            ui.painter().rect_filled(
                                bar_rect,
                                egui::CornerRadius {
                                    nw: 4,
                                    sw: 4,
                                    ne: 0,
                                    se: 0,
                                },
                                accent_color.gamma_multiply(opacity),
                            );
                        });
                });

            // Pause timer while hovered.
            let hovered = response
                .response
                .rect
                .contains(ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO)));
            if hovered && toast.pause_start.is_none() {
                toast.pause_start = Some(Instant::now());
            } else if !hovered {
                if let Some(start) = toast.pause_start.take() {
                    toast.paused_duration += start.elapsed();
                }
            }

            y_offset += response.response.rect.height() + TOAST_GAP;
        }

        // Remove expired toasts and advance the ID counter.
        let before = self.toasts.len();
        self.toasts.retain(|t| !t.is_expired());
        self.next_id += before;

        // Request repaint while toasts are active (for fade animation).
        if !self.toasts.is_empty() {
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toast_starts_at_full_opacity() {
        let toast = Toast::new("test".into(), ToastLevel::Warning);
        assert!((toast.opacity() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn toast_not_expired_initially() {
        let toast = Toast::new("test".into(), ToastLevel::Warning);
        assert!(!toast.is_expired());
    }

    #[test]
    fn dismissed_toast_is_expired() {
        let mut toast = Toast::new("test".into(), ToastLevel::Warning);
        toast.dismissed = true;
        assert!(toast.is_expired());
    }

    #[test]
    fn toast_manager_push_adds_toast() {
        let mut manager = ToastManager::default();
        manager.push("warning".into(), ToastLevel::Warning);
        manager.push("error".into(), ToastLevel::Error);
        assert_eq!(manager.toasts.len(), 2);
    }

    #[test]
    fn toast_level_equality() {
        assert_eq!(ToastLevel::Warning, ToastLevel::Warning);
        assert_ne!(ToastLevel::Warning, ToastLevel::Error);
    }
}
