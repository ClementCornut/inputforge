//! Prepared drawing highlights for the four deadzone handles.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BodyState {
    pub dragging: Option<DragInProgress>,
    pub hovered_handle: Option<usize>,
    pub focused_handle: Option<usize>,
    pub pending_split: Option<(usize, usize)>,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DragInProgress {
    pub handle: usize,
}
