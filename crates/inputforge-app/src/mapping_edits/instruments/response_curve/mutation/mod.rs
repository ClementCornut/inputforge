//! Validated curve edits and topology changes.
mod bezier;
mod conversion;
mod point;
mod symmetry;
mod topology;
#[cfg(test)]
use super::CurveType;
pub(crate) use conversion::{convert_curve_type, default_identity_curve, reconstruct_curve};
#[cfg(test)]
use inputforge_core::processing::curves::{BezierSegment, ResponseCurve};
pub(crate) use point::{adjacent_x_bounds, update_point_in_curve};
pub(crate) use symmetry::apply_symmetry;
pub(crate) use topology::{add_control_point, remove_control_point};
#[cfg(test)]
mod tests {
    use super::*;

    fn identity_piecewise() -> ResponseCurve {
        ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false).unwrap()
    }

    fn identity_bezier() -> ResponseCurve {
        let seg = BezierSegment {
            start: (-1.0, -1.0),
            control1: (-1.0 / 3.0, -1.0 / 3.0),
            control2: (1.0 / 3.0, 1.0 / 3.0),
            end: (1.0, 1.0),
        };
        ResponseCurve::cubic_bezier(vec![seg], false).unwrap()
    }

    #[test]
    fn adjacent_x_bounds_locks_first_and_last() {
        let curve = identity_piecewise();
        let (lo, hi) = adjacent_x_bounds(&curve, 0);
        assert!((lo - (-1.0)).abs() < f64::EPSILON);
        assert!((hi - (-1.0)).abs() < f64::EPSILON);
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 1.0).abs() < f64::EPSILON);
        assert!((hi - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adjacent_x_bounds_locks_symmetric_center() {
        let curve = ResponseCurve::piecewise_linear(
            vec![
                (-1.0, -1.0),
                (-0.5, -0.3),
                (0.0, 0.0),
                (0.5, 0.3),
                (1.0, 1.0),
            ],
            true,
        )
        .unwrap();
        let (lo, hi) = adjacent_x_bounds(&curve, 2);
        assert!((lo - 0.0).abs() < f64::EPSILON);
        assert!((hi - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn update_point_freezes_symmetric_center() {
        let mut curve = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
            symmetric: true,
        };
        let bounds = adjacent_x_bounds(&curve, 1);
        update_point_in_curve(&mut curve, 1, (0.3, 0.5), bounds);
        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert!(points[1].0.abs() < f64::EPSILON, "center x stays at 0");
            assert!(points[1].1.abs() < f64::EPSILON, "center y stays at 0");
        } else {
            panic!("expected PiecewiseLinear");
        }
    }

    #[test]
    fn update_point_mirrors_in_symmetric() {
        let mut curve = ResponseCurve::PiecewiseLinear {
            points: vec![
                (-1.0, -1.0),
                (-0.5, -0.5),
                (0.0, 0.0),
                (0.5, 0.5),
                (1.0, 1.0),
            ],
            symmetric: true,
        };
        let bounds = adjacent_x_bounds(&curve, 3);
        update_point_in_curve(&mut curve, 3, (0.4, 0.7), bounds);
        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert!((points[3].0 - 0.4).abs() < 1e-9);
            assert!((points[3].1 - 0.7).abs() < 1e-9);
            // Mirror at index 1.
            assert!((points[1].0 - (-0.4)).abs() < 1e-9);
            assert!((points[1].1 - (-0.7)).abs() < 1e-9);
        }
    }

    #[test]
    fn convert_curve_type_preserves_symmetric_flag() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], true)
                .unwrap();
        let bezier = convert_curve_type(&curve, CurveType::CubicBezier).unwrap();
        match bezier {
            ResponseCurve::CubicBezier {
                symmetric,
                segments,
            } => {
                assert!(symmetric);
                assert_eq!(segments.len(), 2, "symmetric bezier has 2 segments");
            }
            _ => panic!("expected CubicBezier"),
        }
    }

    #[test]
    fn add_control_point_inserts_between_neighbors() {
        let mut curve = identity_piecewise();
        let added = add_control_point(&mut curve, (0.5, 0.7));
        assert!(added);
        if let ResponseCurve::PiecewiseLinear { points, .. } = &curve {
            assert_eq!(points.len(), 4);
            assert!(points.windows(2).all(|w| w[0].0 < w[1].0));
        }
    }

    #[test]
    fn remove_control_point_refuses_edges_and_handles() {
        let mut curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (0.5, 0.5), (1.0, 1.0)],
            false,
        )
        .unwrap();
        assert!(
            !remove_control_point(&mut curve, 0),
            "first edge cannot be removed"
        );
        assert!(
            !remove_control_point(&mut curve, 3),
            "last edge cannot be removed"
        );
        // Bezier handle (local 1 or 2) cannot be removed.
        let mut bz = identity_bezier();
        assert!(
            !remove_control_point(&mut bz, 1),
            "bezier handle cannot be removed"
        );
        assert!(
            !remove_control_point(&mut bz, 2),
            "bezier handle cannot be removed"
        );
    }

    #[test]
    fn reconstruct_curve_returns_validated() {
        let curve = identity_piecewise();
        reconstruct_curve(&curve).unwrap();
    }

    #[test]
    fn reconstruct_curve_returns_error_for_duplicate_x() {
        let invalid = ResponseCurve::PiecewiseLinear {
            points: vec![(-1.0, -1.0), (0.0, 0.0), (0.0, 0.5), (1.0, 1.0)],
            symmetric: false,
        };
        let result = reconstruct_curve(&invalid);
        assert!(result.is_err(), "duplicate x must reject");
        let err = result.unwrap_err();
        assert!(!err.is_empty(), "error string must not be empty");
    }

    #[test]
    fn default_identity_curve_preserves_type_and_symmetric() {
        let curve = ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-0.5, -0.5),
                control2: (0.5, 0.5),
                end: (1.0, 1.0),
            }],
            true,
        )
        .unwrap();
        let reset = default_identity_curve(&curve);
        match reset {
            ResponseCurve::CubicBezier {
                symmetric: true,
                segments,
            } => {
                assert_eq!(segments.len(), 2, "symmetric reset is 2 segments");
            }
            _ => panic!("expected symmetric CubicBezier"),
        }
    }

    #[test]
    fn apply_symmetry_enabling_enforces_antisymmetric_points() {
        let curve =
            ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)], false)
                .unwrap();
        let result = apply_symmetry(&curve, true).expect("enable symmetry on identity");
        if let ResponseCurve::PiecewiseLinear { points, symmetric } = result {
            assert!(symmetric);
            assert!(points.len() >= 3);
            let center = points.iter().find(|(x, _)| x.abs() < f64::EPSILON);
            assert!(center.is_some(), "origin must be present");
            assert!(center.unwrap().1.abs() < f64::EPSILON);
        } else {
            panic!("expected PiecewiseLinear");
        }
    }

    #[test]
    fn apply_symmetry_two_point_default_curve() {
        let curve = ResponseCurve::piecewise_linear(vec![(-1.0, -1.0), (1.0, 1.0)], false).unwrap();
        let result = apply_symmetry(&curve, true).expect("enable symmetry on 2-point");
        if let ResponseCurve::PiecewiseLinear { points, symmetric } = result {
            assert!(symmetric);
            assert!(points.len() >= 3);
            assert!(points[0].0 < 0.0);
            assert!(points[points.len() - 1].0 > 0.0);
        }
    }

    #[test]
    fn apply_symmetry_disabling_keeps_all_points() {
        let curve = ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (0.0, 0.0), (0.5, 0.2), (1.0, 1.0)],
            true,
        )
        .unwrap();
        let result = apply_symmetry(&curve, false).expect("disable symmetry");
        if let ResponseCurve::PiecewiseLinear { points, symmetric } = result {
            assert!(!symmetric);
            assert_eq!(points.len(), 4);
        }
    }

    #[test]
    fn apply_symmetry_bezier_round_trip() {
        let curve = ResponseCurve::cubic_bezier(
            vec![BezierSegment {
                start: (-1.0, -1.0),
                control1: (-0.5, -0.5),
                control2: (0.5, 0.5),
                end: (1.0, 1.0),
            }],
            false,
        )
        .unwrap();
        let sym = apply_symmetry(&curve, true).expect("enable bezier symmetry");
        if let ResponseCurve::CubicBezier {
            segments,
            symmetric: true,
        } = sym
        {
            assert!(segments.len() >= 2);
        } else {
            panic!("expected symmetric CubicBezier");
        }
    }
}
