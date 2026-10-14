//! Prepared mapping rail states, targets and app workflow intents.
use inputforge_core::{action::Mapping, profile::ProfileId, types::InputAddress};

/// Stable profile and editing mode captured by a rail control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub profile: Option<ProfileId>,
    pub mode: String,
}
/// Rendered metadata used to resolve a full target in the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lookup {
    pub scope: Scope,
    pub input: InputAddress,
    pub name: Option<String>,
    pub filtered: bool,
}
/// Full target prepared when a menu, dialog or drag is opened.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub scope: Scope,
    pub before: Mapping,
}
/// Prepared menu availability and group order.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    pub target: Target,
    pub order: Vec<InputAddress>,
    pub group: Group,
    pub can_up: bool,
    pub can_down: bool,
    pub other_modes: Vec<String>,
}
/// Rendering group shared by menu, keyboard and drag requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    Axes,
    Buttons,
    Hats,
}
/// Raw movement intent; gap indices precede removal of the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Up,
    Down,
    Gap(usize),
}
/// One forward rail operation, retaining the existing history policy.
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Duplicate(String),
    Delete,
    Reorder {
        movement: Move,
        order: Vec<InputAddress>,
        group: Group,
        filtered: bool,
    },
}
/// Request against a prepared full mapping target.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub target: Target,
    pub operation: Operation,
}
/// Prepared result for rendering feedback and success announcements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Delivered,
    Opened,
    Moved {
        index: usize,
        len: usize,
        group: Group,
    },
    NoOp,
    Rejected(String),
}
/// Add pad phase, including the app-resolved collision target.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Phase {
    #[default]
    Resting,
    Capturing,
    Captured(InputAddress),
    Collision(Mapping),
}
/// Renderer-owned projection of one Add/duplicate workflow.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AddView {
    pub phase: Phase,
    pub scope: Option<Scope>,
    pub source: Option<Mapping>,
    pub error: Option<String>,
}
/// Add pad intent interpreted by the app.
#[derive(Debug, Clone, PartialEq)]
pub enum AddIntent {
    Start {
        scope: Scope,
        source: Option<Target>,
    },
    Captured(InputAddress),
    Refresh,
    Reconcile,
    Commit(String),
    Open,
    Cancel,
    CaptureCancelled,
}
/// Prepared pad state and the event requesting its next state.
#[derive(Debug, Clone, PartialEq)]
pub struct AddRequest {
    pub view: AddView,
    pub intent: AddIntent,
}
