//! Prepared mapping edit intents, history labels and app callbacks.
use crate::snapshots::MappingKey;
use dioxus::prelude::*;
use inputforge_core::{
    action::{Action, ActionBranch, Mapping, OutputBehavior},
    profile::ProfileId,
    types::{HatDirection, InputAddress, KeyCombo},
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
    Field { path: StageId, field: Field },
    Add { path: StageId, kind: StageKind },
    Step { path: StageId, down: bool },
    Insert { path: StageId, action: Action },
    Remove(StageId),
    Move { from: StageId, to: StageId },
    Rename(String),
    Rebind(InputAddress),
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
    /// Editor operations require selection. Legacy rail dispatch may omit actions;
    /// inline drafts always retain the full target prepared when editing starts.
    pub editor: bool,
}
/// Inline intent with a target captured when the draft or capture starts.
#[derive(Debug, Clone, PartialEq)]
pub struct Inline {
    /// Missing preparation stays unavailable for the lifetime of the draft.
    pub target: Option<crate::mapping_rail::Target>,
    pub edit: Edit,
    pub label: String,
    pub editor: bool,
    pub cancel: bool,
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
    pub instruments: crate::instruments::Bindings,
    pub readouts: crate::readouts::Bindings,
    pub batch: crate::batch_mapping::Bindings,
    pub modes: crate::mode_management::Bindings,
    /// Unique mounted-controller identity for isolated listener wiring.
    pub instance: u128,
    pub menu: Callback<crate::mapping_rail::Lookup, Option<crate::mapping_rail::Menu>>,
    pub rail: Callback<crate::mapping_rail::Request, crate::mapping_rail::Outcome>,
    pub add: Callback<crate::mapping_rail::AddRequest, crate::mapping_rail::AddView>,
    pub install: Callback<Publication>,
    pub dispatch: Callback<Request, bool>,
    /// Full captured target: cancel/no-op/success closes; rejection retains the draft.
    pub inline: Callback<Inline, Result<bool, String>>,
    /// Raw editor intent outcome; unchanged fields deliver no command.
    pub edit: Callback<Request, Result<bool, String>>,
    pub palette: Callback<(MappingKey, Vec<StageIdSegment>), Palette>,
    pub number: Callback<String, Option<f64>>,
    pub stage_menu: Callback<(Vec<Action>, StageId), Option<StageMenu>>,
    pub undo: Callback<MappingKey, bool>,
    pub redo: Callback<MappingKey, bool>,
    pub forget: Callback<MappingKey>,
    pub end_group: Callback<()>,
    pub reconcile: Callback<()>,
    pub dispose: Callback<()>,
}

/// One field change; unrelated queued fields remain intact.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    ModeHold(bool),
    ModeTarget(String),
    Keyboard(KeyCombo),
    Behavior(OutputBehavior),
    MouseTarget(String),
    OutputDevice(String),
    OutputControl(String),
    MergeInput(InputAddress),
    MergeOperation(String),
    ConditionKind(String),
    ConditionInput(InputAddress),
    ConditionMin(String),
    ConditionMax(String),
    ConditionDirection(HatDirection),
    GestureThreshold(String),
    GestureToggle(bool),
}
/// Palette selection; defaults are supplied by the app.
#[derive(Debug, Clone, PartialEq)]
pub enum StageKind {
    Invert,
    Deadzone,
    ResponseCurve,
    VirtualDevice,
    Keyboard(KeyCombo),
    Mouse,
    MergeAxis,
    Conditional,
    ChangeMode,
    Tap,
    Press,
}

/// Prepared palette availability, independent of rendered labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub gestures: bool,
    pub keyboard: bool,
    pub mouse: bool,
}

/// Prepared sibling boundaries for the stage menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageMenu {
    pub parent: Vec<StageIdSegment>,
    pub index: usize,
    pub up: bool,
    pub down: bool,
}
