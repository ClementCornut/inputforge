//! Prepared device inspector state and mounted app callbacks.
use crate::snapshots::DevicePanelRow;
use dioxus::prelude::*;
use inputforge_core::types::DeviceId;
/// App-prepared rows, draft, availability and feedback.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct View {
    pub workflow: u64,
    pub rows: Vec<DevicePanelRow>,
    pub selected: Option<DevicePanelRow>,
    pub draft: String,
    pub can_save: bool,
    pub removal_reason: Option<String>,
    pub error: Option<String>,
}
/// Raw observations from the rendered inspector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Select(DeviceId),
    Input(String),
    Save,
    Cancel,
    Remove,
    Copy(String),
}
/// One mounted inspector uses captured ownership and selection generations.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub mount: Callback<Signal<View>, u64>,
    pub dispatch: Callback<(u64, u64, Intent), bool>,
    /// Returns whether the app consumed this key, independently of command success.
    pub key: Callback<(u64, u64, String), bool>,
    pub reconcile: Callback<()>,
    pub unmount: Callback<u64>,
}
/// Captured app callbacks paired with GUI rendering signals.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub view: Signal<View>,
    pub owner: u64,
    pub workflow: u64,
    pub bindings: Bindings,
}
impl Context {
    /// Forward an observation to its original selection and mounted owner.
    #[must_use]
    pub fn send(self, intent: Intent) -> bool {
        self.bindings
            .dispatch
            .call((self.owner, self.workflow, intent))
    }
}

impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.view == other.view && self.owner == other.owner && self.workflow == other.workflow
    }
}
