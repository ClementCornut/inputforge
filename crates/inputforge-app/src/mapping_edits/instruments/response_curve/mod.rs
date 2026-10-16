//! Curve mutation and keyboard behavior.
pub(super) mod interaction;
pub(super) mod keyboard;
pub(super) mod mutation;
pub(super) mod state;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CurveType {
    PiecewiseLinear,
    CubicSpline,
    CubicBezier,
}

mod focus;
