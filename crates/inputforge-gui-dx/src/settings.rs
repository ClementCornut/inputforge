//! Prepared settings values and callbacks owned by the host application.
use crate::{components::IntegerInputError, snapshots::SettingsSnapshot};
use dioxus::prelude::*;
use inputforge_core::engine::PruneGuard;
/// Buffer reduction awaiting confirmation for this exact profile and settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prune {
    pub candidate_max: usize,
    pub will_remove: usize,
    pub profile_name: String,
    pub guard: PruneGuard,
}
/// Settings controls and inline feedback prepared by the app.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct View {
    pub workflow: u64,
    pub values: SettingsSnapshot,
    pub prune: Option<Prune>,
    pub error: Option<String>,
    pub buffer_error: Option<String>,
}
/// Raw settings input interpreted by the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Max(String),
    Invalid(IntegerInputError),
    InvalidGesture(IntegerInputError),
    Skip,
    Autostart,
    Minimized,
    DoubleTap(usize),
    LongPress(usize),
    Confirm,
    Cancel,
}
/// Settings callbacks scoped to a mounted panel.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub mount: Callback<Signal<View>, u64>,
    pub dispatch: Callback<(u64, u64, Intent), bool>,
    pub reconcile: Callback<()>,
    pub unmount: Callback<u64>,
}
/// GUI rendering signals paired with their app owner.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub view: Signal<View>,
    pub owner: u64,
    pub workflow: u64,
    pub bindings: Bindings,
}
impl Context {
    /// Forward input to the captured mounted panel.
    #[must_use]
    pub fn send(self, intent: Intent) -> bool {
        self.bindings
            .dispatch
            .call((self.owner, self.workflow, intent))
    }
}
