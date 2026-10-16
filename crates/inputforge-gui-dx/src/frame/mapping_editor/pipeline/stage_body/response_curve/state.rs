//! Drawing caches and highlight coordinates for the response curve.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BodyState {
    pub dragging: Option<DragInProgress>,
    pub hovered_point: Option<usize>,
    pub focused_point: Option<usize>,
    pub cached_path: Vec<(f64, f64)>,
    pub cached_anchors: Vec<(f64, f64)>,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DragInProgress {
    pub point_index: usize,
}
pub(crate) use super::super::instruments::geometry::extract_anchors;
