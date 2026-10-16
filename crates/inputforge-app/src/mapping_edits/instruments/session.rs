//! Private working drafts and prepared drawing projections.
use super::{deadzone, response_curve};
use inputforge_core::action::{Action, Mapping};
use inputforge_gui_dx::instruments::{Open, View};
#[derive(Debug)]
pub(in crate::mapping_edits) struct Session {
    pub open: Open,
    pub expected: Option<Mapping>,
    pub value: Action,
    pub curve: response_curve::state::BodyState,
    pub zone: deadzone::state::BodyState,
    pub revision: u64,
    pub draft: bool,
    pub available: bool,
    pub error: Option<String>,
    pub prevent_default: bool,
    pub(super) listener: Option<super::lifecycle::Listener>,
}
impl Session {
    pub(super) fn new(open: Open, revision: u64) -> Self {
        let mut session = Self {
            expected: open.target.as_ref().map(|t| t.before.clone()),
            value: open.seed.clone(),
            open,
            curve: response_curve::state::BodyState::default(),
            zone: deadzone::state::BodyState::default(),
            revision,
            draft: false,
            available: true,
            error: None,
            prevent_default: false,
            listener: None,
        };
        session.anchors();
        session
    }
    pub(super) fn dragging(&self) -> bool {
        self.curve.dragging.is_some()
            || self.zone.dragging.is_some()
            || self.zone.pending_split.is_some()
    }
    pub(super) fn anchors(&mut self) {
        if let Action::ResponseCurve { curve } = &self.value {
            self.curve.cached_anchors = response_curve::state::extract_anchors(curve);
            self.curve = response_curve::state::clamp_focus(
                self.curve.clone(),
                self.curve.cached_anchors.len(),
            );
        }
    }
    pub(super) fn adopt(&mut self, mapping: Mapping) {
        self.cancel();
        if let Some(action) = super::paths::at(&mapping.actions, &self.open.path.0) {
            if std::mem::discriminant(action) == std::mem::discriminant(&self.open.seed) {
                self.value = action.clone();
            } else {
                self.available = false;
            }
        } else {
            self.available = false;
        }
        self.expected = Some(mapping);
        self.anchors();
    }
    pub(super) fn cancel(&mut self) {
        self.curve.dragging = None;
        self.curve.pre_drag_curve = None;
        self.zone.dragging = None;
        self.zone.pre_drag_config = None;
        self.zone.pending_split = None;
        if let Some(action) = self
            .expected
            .as_ref()
            .and_then(|m| super::paths::at(&m.actions, &self.open.path.0))
        {
            self.value = action.clone();
        }
        self.draft = false;
        self.error = None;
        self.anchors();
    }
    pub(super) fn view(&self, id: u128) -> View {
        let mut view = View {
            id,
            value: self.value.clone(),
            hovered: None,
            focused: None,
            dragging: None,
            split: false,
            error: self.error.clone(),
            prevent_default: self.prevent_default,
            bounds: [(-1.0, 1.0); 4],
        };
        match &self.value {
            Action::ResponseCurve { .. } => {
                view.hovered = self.curve.hovered_point;
                view.focused = self.curve.focused_point;
                view.dragging = self.curve.dragging.as_ref().map(|d| d.point_index);
            }
            Action::Deadzone { config } => {
                let index = |h| {
                    deadzone::state::HandleId::ALL
                        .iter()
                        .position(|&value| value == h)
                };
                view.hovered = self.zone.hovered_handle.and_then(index);
                view.focused = self.zone.focused_handle.and_then(index);
                view.dragging = self.zone.dragging.as_ref().and_then(|d| index(d.handle));
                view.split = self.zone.pending_split.is_some();
                view.bounds = deadzone::state::HandleId::ALL
                    .map(|h| deadzone::mutation::adjacent_bounds(h, config));
            }
            _ => {}
        }
        view
    }
}
