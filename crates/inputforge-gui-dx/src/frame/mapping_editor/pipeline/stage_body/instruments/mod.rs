// Rust guideline compliant 2026-05-03

//! Cross-instrument shared infrastructure for `StageBody` editors (F10, F11,
//! and future signature instruments). Each helper here has at least two
//! consumers; helpers with only one consumer stay inside their owning editor.

// TODO(Task 3-4): re-enable after sibling modules land.
// pub(crate) mod bridge;
// pub(crate) mod stage_dispatch;
pub(crate) mod live_axis;
pub(crate) mod nudge_coalesce;

/// SVG `feGaussianBlur` standard deviation used by every instrument's curve
/// glow filter. Pinned in Rust (rather than CSS) because SVG attributes do
/// not resolve CSS custom properties.
#[allow(dead_code, reason = "real consumer lands in Task 5")]
pub(crate) const INSTR_GLOW_STDDEV: f64 = 0.012;
