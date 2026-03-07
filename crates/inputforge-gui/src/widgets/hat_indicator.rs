// Rust guideline compliant 2026-03-03

//! Custom-painted 8-way hat switch (compass) indicator.
//!
//! Draws a 48x48 pixel compass rose with 8 directional triangles.
//! The active direction is highlighted in the live accent; inactive
//! directions use the idle indicator color. The center dot also uses
//! the idle indicator color for visibility against the base.

use std::f32::consts::PI;

use egui::{Pos2, Vec2};

use inputforge_core::types::HatDirection;

use crate::theme;

/// Size of the hat indicator widget in logical pixels.
const INDICATOR_SIZE: f32 = 48.0;

/// Radius of the center dot.
const CENTER_RADIUS: f32 = 4.0;

/// Inner radius where directional triangles begin.
const INNER_RADIUS: f32 = 8.0;

/// Outer radius where directional triangles end.
const OUTER_RADIUS: f32 = 20.0;

/// Half-angular width of each directional triangle in radians.
const TRIANGLE_HALF_ANGLE: f32 = PI / 12.0;

/// Return a human-readable label for the given hat direction.
pub(crate) fn direction_label(dir: HatDirection) -> &'static str {
    match dir {
        HatDirection::Center => "Center",
        HatDirection::N => "North",
        HatDirection::NE => "NE",
        HatDirection::E => "East",
        HatDirection::SE => "SE",
        HatDirection::S => "South",
        HatDirection::SW => "SW",
        HatDirection::W => "West",
        HatDirection::NW => "NW",
    }
}

/// Paint an 8-way hat direction compass indicator.
///
/// `direction` determines which of the 8 cardinal/ordinal triangles
/// is highlighted. [`HatDirection::Center`] means no triangle is active.
///
/// Returns the egui [`Response`](egui::Response) so callers can attach
/// tooltips or handle interaction.
pub(crate) fn hat_indicator(ui: &mut egui::Ui, direction: HatDirection) -> egui::Response {
    let colors = theme::colors(ui.ctx());
    let desired_size = Vec2::splat(INDICATOR_SIZE);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();

    // Draw 8 directional triangles.
    for &(dir, angle) in &DIRECTION_ANGLES {
        let is_active = direction == dir;
        let color = if is_active {
            colors.live
        } else {
            colors.indicator_idle
        };

        let tip = offset_point(center, angle, OUTER_RADIUS);
        let left = offset_point(center, angle - TRIANGLE_HALF_ANGLE, INNER_RADIUS);
        let right = offset_point(center, angle + TRIANGLE_HALF_ANGLE, INNER_RADIUS);

        painter.add(egui::Shape::convex_polygon(
            vec![tip, left, right],
            color,
            egui::Stroke::NONE,
        ));
    }

    // Center dot — uses idle color for visibility against the dark base.
    painter.circle_filled(center, CENTER_RADIUS, colors.indicator_idle);

    response
}

/// Compute a point offset from `center` at the given `angle` and `radius`.
fn offset_point(center: Pos2, angle: f32, radius: f32) -> Pos2 {
    Pos2::new(
        center.x + angle.cos() * radius,
        center.y - angle.sin() * radius,
    )
}

/// Map each `HatDirection` to its angle in radians (math convention:
/// 0 = East, counter-clockwise positive). N = 90 deg = PI/2.
const DIRECTION_ANGLES: [(HatDirection, f32); 8] = [
    (HatDirection::N, PI / 2.0),
    (HatDirection::NE, PI / 4.0),
    (HatDirection::E, 0.0),
    (HatDirection::SE, -PI / 4.0),
    (HatDirection::S, -PI / 2.0),
    (HatDirection::SW, -3.0 * PI / 4.0),
    (HatDirection::W, PI),
    (HatDirection::NW, 3.0 * PI / 4.0),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Return the angle in radians for a given `HatDirection`.
    ///
    /// Returns `None` for [`HatDirection::Center`].
    fn direction_angle(dir: HatDirection) -> Option<f32> {
        DIRECTION_ANGLES
            .iter()
            .find(|(d, _)| *d == dir)
            .map(|(_, angle)| *angle)
    }

    const _: () = assert!(INNER_RADIUS < OUTER_RADIUS);
    const _: () = assert!(CENTER_RADIUS < INNER_RADIUS);

    #[test]
    fn direction_angles_has_8_entries() {
        assert_eq!(DIRECTION_ANGLES.len(), 8);
    }

    #[test]
    fn direction_angles_cover_all_non_center() {
        let dirs = [
            HatDirection::N,
            HatDirection::NE,
            HatDirection::E,
            HatDirection::SE,
            HatDirection::S,
            HatDirection::SW,
            HatDirection::W,
            HatDirection::NW,
        ];
        for dir in &dirs {
            assert!(direction_angle(*dir).is_some(), "missing angle for {dir:?}");
        }
    }

    #[test]
    fn center_has_no_angle() {
        assert!(direction_angle(HatDirection::Center).is_none());
    }

    #[test]
    fn north_angle_is_pi_over_2() {
        let angle = direction_angle(HatDirection::N).unwrap();
        assert!((angle - PI / 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn east_angle_is_zero() {
        let angle = direction_angle(HatDirection::E).unwrap();
        assert!(angle.abs() < f32::EPSILON);
    }

    #[test]
    fn offset_point_at_zero_angle() {
        let center = Pos2::new(24.0, 24.0);
        let pt = offset_point(center, 0.0, 10.0);
        assert!((pt.x - 34.0).abs() < 0.01);
        assert!((pt.y - 24.0).abs() < 0.01);
    }

    #[test]
    fn offset_point_at_pi_over_2() {
        let center = Pos2::new(24.0, 24.0);
        let pt = offset_point(center, PI / 2.0, 10.0);
        assert!((pt.x - 24.0).abs() < 0.01);
        assert!((pt.y - 14.0).abs() < 0.01); // y decreases (up)
    }

    #[test]
    fn direction_label_returns_non_empty_for_all() {
        let all_dirs = [
            HatDirection::Center,
            HatDirection::N,
            HatDirection::NE,
            HatDirection::E,
            HatDirection::SE,
            HatDirection::S,
            HatDirection::SW,
            HatDirection::W,
            HatDirection::NW,
        ];
        for dir in all_dirs {
            assert!(!direction_label(dir).is_empty(), "empty label for {dir:?}");
        }
    }

    #[test]
    fn direction_label_center_is_center() {
        assert_eq!(direction_label(HatDirection::Center), "Center");
    }
}
