//! Prepared sortable publications and application-owned gesture hooks.
use dioxus::prelude::*;
/// Current gap indicator, unique within its group.
#[derive(Clone, Debug, PartialEq)]
pub struct DropTarget<G> {
    pub gap_index: usize,
    pub group: G,
    pub invalid: bool,
}
/// Typed hooks for one shared sortable consumer.
#[derive(Clone)]
pub struct Bindings<G: 'static + Clone + PartialEq> {
    pub state: fn() -> State<G>,
    pub handle: fn(HandleProps<G>) -> HandleView,
    pub gap: fn(GapProps<G>) -> GapView,
}
impl<G: 'static + Clone + PartialEq> Copy for Bindings<G> {}
impl<G: 'static + Clone + PartialEq> std::fmt::Debug for Bindings<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bindings").finish_non_exhaustive()
    }
}
/// Signals published by the app and observed by sortable rendering.
#[derive(Clone)]
pub struct State<G: 'static + Clone + PartialEq> {
    pub drag_from: Signal<Option<usize>>,
    pub drag_group: Signal<Option<G>>,
    pub drop_target: Signal<Option<DropTarget<G>>>,
    pub live_announcement: Signal<String>,
    pub handle_id: Signal<Option<String>>,
    pub closed: Signal<bool>,
    pub close: Callback<()>,
    pub bindings: Bindings<G>,
}
impl<G: 'static + Clone + PartialEq> Copy for State<G> {}
impl<G: 'static + Clone + PartialEq> std::fmt::Debug for State<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("owner", &self.drag_from.id())
            .finish_non_exhaustive()
    }
}
impl<G: 'static + Clone + PartialEq> PartialEq for State<G> {
    fn eq(&self, other: &Self) -> bool {
        self.drag_from == other.drag_from
    }
}
/// Current handle identity and prepared source observations.
#[derive(Clone, Debug)]
pub struct HandleProps<G: 'static + Clone + PartialEq> {
    pub state: State<G>,
    pub index: usize,
    pub group: G,
    pub context: String,
    pub draggable: bool,
}
/// App handlers and browser transport wiring for a handle.
#[derive(Clone, Debug)]
pub struct HandleView {
    pub id: String,
    pub start: EventHandler<DragEvent>,
    pub end: EventHandler<DragEvent>,
    pub pointer: &'static str,
    pub click: &'static str,
}
/// Current gap and consumer callback; the app interprets eligibility and no-ops.
#[derive(Clone, Debug)]
pub struct GapProps<G: 'static + Clone + PartialEq> {
    pub state: State<G>,
    pub gap_index: usize,
    pub group: G,
    pub on_drop: EventHandler<usize>,
}
/// Prepared gap appearance and native event callbacks.
#[derive(Clone, Debug)]
pub struct GapView {
    pub class: String,
    pub over: EventHandler<DragEvent>,
    pub leave: EventHandler<DragEvent>,
    pub end: EventHandler<DragEvent>,
    pub drop: EventHandler<DragEvent>,
}
/// Raw pipeline drop observations, using the existing final mapping dispatcher.
#[derive(Clone, Debug)]
pub struct PipelineDrop {
    pub state: State<crate::mapping_edits::StageId>,
    pub parent: crate::mapping_edits::StageId,
    pub gap: usize,
    pub title: Callback<crate::mapping_edits::StageId, String>,
    pub submit: Callback<(crate::mapping_edits::Edit, String), bool>,
}
