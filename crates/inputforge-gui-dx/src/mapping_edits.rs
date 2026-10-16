//! Prepared mapping edit intents, history labels and app callbacks.
use crate::snapshots::MappingKey;
use dioxus::prelude::*;
use inputforge_core::{
    action::{Action, ActionBranch, Mapping},
    profile::ProfileId,
};
use std::collections::HashMap;

/// Root-relative stage path used by rendering and app edits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StageId(pub Vec<StageIdSegment>);
/// One positional stage path segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageIdSegment {
    Index(usize),
    Branch(ActionBranch),
}
/// Existing edit categories used by history labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoKind {
    StageEdit,
    StageAdd,
    StageRemove,
    StageReorder,
    Rename,
    Rebind,
}
/// A rendered edit requested against a saved mapping.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    Replace { path: StageId, action: Action },
    Insert { path: StageId, action: Action },
    Remove(StageId),
    Move { from: StageId, to: StageId },
    Rename(String),
    Rebind(inputforge_core::types::InputAddress),
}
/// One keyboard burst identity, including the selected instrument handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub path: StageId,
    pub key: String,
}
/// Target snapshot and display label captured by a rendered control.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub profile: Option<ProfileId>,
    pub before: Mapping,
    pub edit: Edit,
    pub label: String,
    pub group: Option<Group>,
    /// Editor operations require selection. Rail rename validates captured name metadata
    /// and obtains actions in the app; its `before.actions` may be empty.
    pub editor: bool,
}
/// Prepared history label, without restore snapshots or mutable stack operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: UndoKind,
    pub label: String,
}
/// Labels available in a mapping's undo and redo histories.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub undo: Vec<Entry>,
    pub redo: Vec<Entry>,
}
/// Read-only history projection for rendering.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct View {
    pub stacks: HashMap<MappingKey, Summary>,
}
/// Renderer-owned signals used by the app to publish and validate navigation.
#[derive(Debug, Clone, Copy)]
pub struct Publication {
    pub history: Signal<View>,
    pub mode: Signal<String>,
    pub selection: Signal<Option<MappingKey>>,
}
/// App-owned mapping and history operations.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub batch: crate::batch_mapping::Bindings,
    pub modes: crate::mode_management::Bindings,
    /// Unique mounted-controller identity for isolated listener wiring.
    pub instance: u128,
    pub menu: Callback<crate::mapping_rail::Lookup, Option<crate::mapping_rail::Menu>>,
    pub rail: Callback<crate::mapping_rail::Request, crate::mapping_rail::Outcome>,
    pub add: Callback<crate::mapping_rail::AddRequest, crate::mapping_rail::AddView>,
    pub install: Callback<Publication>,
    pub dispatch: Callback<Request, bool>,
    pub undo: Callback<MappingKey, bool>,
    pub redo: Callback<MappingKey, bool>,
    pub forget: Callback<MappingKey>,
    pub end_group: Callback<()>,
    pub reconcile: Callback<()>,
    pub dispose: Callback<()>,
}
