// Rust guideline compliant 2026-05-03

//! Pure handle-mutation helpers for F11. Each function takes a current
//! `DeadzoneConfig` and returns either a candidate `DeadzoneConfig` or the
//! geometry needed to render / hit-test handles.

use inputforge_core::error::Result;
use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::frame::mapping_editor::pipeline::stage_body::deadzone::state::HandleId;

/// Minimum strict-inequality gap enforced between adjacent thresholds.
///
/// `DeadzoneConfig::new` rejects `low >= center_low` and
/// `center_high >= high`. To keep `adjacent_bounds` returning a value that
/// stays strictly less than the neighbouring threshold (so a clamp followed
/// by `DeadzoneConfig::new` does not trip the strict-less-than check), the
/// upper bound for `Low` (and lower bound for `High`) is offset by this
/// epsilon. The value matches the egui editor that this module replaces.
const EPSILON: f64 = 0.001;

/// Inclusive (min, max) viewBox-x bounds the given handle is allowed to
/// occupy without violating the engine's `low < center_low <= center_high
/// < high` invariant. The `EPSILON` offset reserves the strict-inequality
/// gap between `low`/`center_low` and `center_high`/`high`.
pub(crate) fn adjacent_bounds(handle: HandleId, config: &DeadzoneConfig) -> (f64, f64) {
    match handle {
        HandleId::Low => (-1.0, config.center_low() - EPSILON),
        HandleId::CenterLow => (config.low() + EPSILON, config.center_high()),
        HandleId::CenterHigh => (config.center_low(), config.high() - EPSILON),
        HandleId::High => (config.center_high() + EPSILON, 1.0),
    }
}

/// Build a candidate `DeadzoneConfig` with the named handle's X coordinate
/// replaced by `new_x`, clamped to the handle's adjacent bounds. Validation
/// runs through `DeadzoneConfig::new`.
///
/// # Errors
///
/// Returns `EngineError::InvalidConfig` if `new_x` is non-finite (NaN or
/// infinity), or if `DeadzoneConfig::new` rejects the resulting four-tuple.
pub(crate) fn with_handle(
    config: &DeadzoneConfig,
    handle: HandleId,
    new_x: f64,
) -> Result<DeadzoneConfig> {
    // NaN/Inf would propagate through `min`/`max` and slip past
    // `DeadzoneConfig::new`, whose ordering checks (`low < center_low`,
    // etc.) are all false for non-finite values, allowing an invalid
    // config through. Reject at the API boundary instead.
    if !new_x.is_finite() {
        return Err(inputforge_core::error::EngineError::InvalidConfig {
            reason: "deadzone handle position must be finite".into(),
        });
    }
    let (lo, hi) = adjacent_bounds(handle, config);
    let clamped = new_x.min(hi).max(lo);
    let (low, cl, ch, high) = match handle {
        HandleId::Low => (
            clamped,
            config.center_low(),
            config.center_high(),
            config.high(),
        ),
        HandleId::CenterLow => (config.low(), clamped, config.center_high(), config.high()),
        HandleId::CenterHigh => (config.low(), config.center_low(), clamped, config.high()),
        HandleId::High => (
            config.low(),
            config.center_low(),
            config.center_high(),
            clamped,
        ),
    };
    DeadzoneConfig::new(low, cl, ch, high)
}

/// Return the four `(x, y)` viewBox coordinates of the four handles in
/// `HandleId::ALL` order. Y is fixed per handle: Low/High at +/- 1.0,
/// CenterLow/CenterHigh at 0.0.
pub(crate) fn handle_positions(config: &DeadzoneConfig) -> [(f64, f64); 4] {
    [
        (config.low(), -1.0),
        (config.center_low(), 0.0),
        (config.center_high(), 0.0),
        (config.high(), 1.0),
    ]
}

/// Convenience alias for `DeadzoneConfig::default()`.
pub(crate) fn default_config() -> DeadzoneConfig {
    DeadzoneConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(low: f64, cl: f64, ch: f64, high: f64) -> DeadzoneConfig {
        DeadzoneConfig::new(low, cl, ch, high).expect("valid")
    }

    #[test]
    fn handle_positions_y_locks() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let p = handle_positions(&c);
        assert_eq!(p[0], (-0.85, -1.0));
        assert_eq!(p[1], (-0.15, 0.0));
        assert_eq!(p[2], (0.15, 0.0));
        assert_eq!(p[3], (0.85, 1.0));
    }

    #[test]
    fn adjacent_bounds_low_runs_to_center_low_minus_epsilon() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let (lo, hi) = adjacent_bounds(HandleId::Low, &c);
        assert!((lo - -1.0).abs() < 1e-9);
        assert!((hi - -0.151).abs() < 1e-9);
    }

    #[test]
    fn adjacent_bounds_center_low_runs_from_low_to_center_high() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let (lo, hi) = adjacent_bounds(HandleId::CenterLow, &c);
        assert!((lo - -0.849).abs() < 1e-9);
        assert!((hi - 0.15).abs() < 1e-9);
    }

    #[test]
    fn adjacent_bounds_center_high_runs_from_center_low_to_high() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let (lo, hi) = adjacent_bounds(HandleId::CenterHigh, &c);
        assert!((lo - -0.15).abs() < 1e-9);
        assert!((hi - 0.849).abs() < 1e-9);
    }

    #[test]
    fn adjacent_bounds_high_runs_from_center_high_plus_epsilon_to_one() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let (lo, hi) = adjacent_bounds(HandleId::High, &c);
        assert!((lo - 0.151).abs() < 1e-9);
        assert!((hi - 1.0).abs() < 1e-9);
    }

    #[test]
    fn with_handle_clamps_to_adjacent_bounds() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let result = with_handle(&c, HandleId::CenterLow, 0.5).expect("valid");
        assert!(result.center_low() <= result.center_high());
    }

    #[test]
    fn with_handle_low_replaces_only_low() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let result = with_handle(&c, HandleId::Low, -0.6).expect("valid");
        assert!((result.low() - -0.6).abs() < 1e-9);
        assert!((result.center_low() - -0.15).abs() < 1e-9);
    }

    #[test]
    fn with_handle_rejects_nan_input() {
        let c = cfg(-0.85, -0.15, 0.15, 0.85);
        let result = with_handle(&c, HandleId::CenterLow, f64::NAN);
        assert!(
            result.is_err(),
            "NaN must not slip through to DeadzoneConfig::new"
        );
        let result = with_handle(&c, HandleId::CenterLow, f64::INFINITY);
        assert!(result.is_err(), "Inf must not slip through either");
    }

    #[test]
    fn default_config_round_trips_engine_default() {
        let c = default_config();
        assert!((c.low() - -1.0).abs() < 1e-9);
        assert!((c.center_low()).abs() < 1e-9);
        assert!((c.center_high()).abs() < 1e-9);
        assert!((c.high() - 1.0).abs() < 1e-9);
    }
}
