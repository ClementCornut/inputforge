//! Prepared mode rendering values and app workflow callbacks.
use dioxus::prelude::Callback;
use inputforge_core::profile::ProfileId;
/// Rendered profile and editing navigation captured when a workflow opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub revision: u64,
    pub profile: Option<ProfileId>,
    pub editing: String,
}
/// Prepared deletion confirmation, retained until confirmation or cancellation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub scope: Scope,
    pub name: String,
    pub mappings: usize,
}
/// Prepared row availability and current direct deletion count.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag supplies one rendered mode affordance"
)]
pub struct Row {
    pub name: String,
    pub running: bool,
    pub can_activate: bool,
    pub can_rename: bool,
    pub can_delete: bool,
    pub can_default: bool,
    pub mappings: usize,
}
/// Mode rows prepared under the existing coherent engine read guard.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct View {
    pub revision: u64,
    pub rows: Vec<Row>,
    pub can_add: bool,
}
/// Mode workflow intent; text remains raw until app validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    BeginAdd,
    Add {
        raw: String,
        blur: bool,
    },
    BeginRename(String),
    Rename {
        from: String,
        raw: String,
        blur: bool,
    },
    BeginDelete(String),
    Delete(Target),
    Select(String),
    Activate(String),
    Default(String),
    Cancel(Option<String>),
    CloseMenu {
        name: String,
        tab: bool,
    },
}
/// Captured context paired with the current user intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub scope: Scope,
    pub operation: Operation,
}
/// App-decided presentation transition; errors produce no transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    Close(Option<String>),
    Add,
    Rename(String),
    Delete(Target),
}
/// Existing editor controller's mode workflow and navigation callbacks.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub dispatch: Callback<Request, Result<Transition, String>>,
    pub reconcile: Callback<()>,
}
