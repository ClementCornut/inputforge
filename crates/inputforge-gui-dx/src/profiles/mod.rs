//! Prepared profile presentation and app-owned workflow callbacks.
pub mod snapshots;
use crate::{
    PanelSlot,
    snapshots::{MetaSnapshot, ProfileRowView},
};
use dioxus::prelude::*;
use inputforge_core::profile::ProfileId;
use std::path::PathBuf;

/// Profile surface selected by the app.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Library,
    New,
    Open {
        path: PathBuf,
    },
}
/// Stable identity captured by a row or source selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub path: PathBuf,
    /// Lossless native-path token for DOM keys and source values.
    pub key: String,
    pub name: String,
    /// Loaded active identity, when this path is active.
    pub profile: Option<ProfileId>,
    /// On-disk identity projected independently by the core library scan.
    pub library_profile: Option<ProfileId>,
}
/// Source selected for a new profile.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Source {
    #[default]
    Blank,
    Active,
    Library(Option<Target>),
    File,
}
/// A prepared row and its captured action target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub row: ProfileRowView,
    pub target: Target,
}
/// Profile form and list prepared by the app.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent prepared affordances and confirmation labels"
)]
pub struct View {
    pub workflow: u64,
    pub snapshots: snapshots::View,
    pub name_error: Option<String>,
    pub mode: Mode,
    pub source: Source,
    pub name: String,
    pub filter: String,
    pub rows: Vec<Row>,
    pub library: Vec<Target>,
    pub rename: Option<Target>,
    pub delete: Option<Target>,
    pub deleting_active: bool,
    pub can_create: bool,
    pub can_add: bool,
    pub filtered_empty: bool,
    pub error: Option<String>,
}
/// Raw profile user input; decisions are made by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Snapshot(snapshots::Intent),
    New,
    Back,
    Name(String),
    Source(Source),
    Filter(String),
    Create,
    LoadOnce,
    Add,
    Open(Target),
    Reveal(Target),
    Duplicate(Target),
    AddRow(Target),
    BeginRename(Target),
    Rename,
    CancelRename,
    BeginDelete(Target),
    Delete,
    CancelDelete,
    Pick,
    LibrarySource { key: String, choices: Vec<Target> },
}
/// App callbacks scoped to a mounted profile panel.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub shortcut_mount: Callback<Callback<String>, u64>,
    pub shortcut_unmount: Callback<u64>,
    pub mount: Callback<Signal<View>, u64>,
    pub dispatch: Callback<(u64, u64, Intent), bool>,
    pub reconcile: Callback<MetaSnapshot>,
    pub unmount: Callback<u64>,
    pub rename_key: Callback<(u64, u64, KeyboardEvent)>,
    pub focus: Callback<(u64, u64, MountedEvent)>,
    pub open_panel: Callback<Signal<PanelSlot>>,
    pub start: Callback<(Signal<PanelSlot>, Intent)>,
}
/// Profile rendering signals and their app callback owner.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub view: Signal<View>,
    pub owner: u64,
    pub workflow: u64,
    pub bindings: Bindings,
}
impl Context {
    /// Capture the currently rendered form for delayed event rejection.
    #[must_use]
    pub fn capture(mut self) -> Self {
        self.workflow = self.view.read().workflow;
        self
    }
    /// Forward an event to its captured mounted owner; rejection retains menus.
    #[must_use]
    pub fn send(self, intent: Intent) -> bool {
        self.bindings
            .dispatch
            .call((self.owner, self.workflow, intent))
    }
}
