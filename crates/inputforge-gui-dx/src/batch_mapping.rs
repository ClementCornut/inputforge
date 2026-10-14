//! Prepared batch mapping values, signal publication and app callbacks.
use dioxus::prelude::{Callback, Signal};
use inputforge_core::types::{DeviceId, InputAddress, OutputAddress, VirtualDeviceConfig};
/// Input group rendered by the wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Axis,
    Button,
    Hat,
}
/// Prepared input identity and its current draft choices.
#[derive(Debug, Clone, PartialEq)]
pub struct RowState {
    pub kind: RowKind,
    pub source_index: u8,
    pub input: InputAddress,
    pub target: Option<OutputAddress>,
    pub replace: bool,
}
/// Prepared batch summary; excluded counts rows rather than row-mode pairs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub create: usize,
    pub replace: usize,
    pub skip: usize,
    pub excluded: usize,
}
/// Available targets and contextual group controls.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub kind: RowKind,
    pub targets: Vec<OutputAddress>,
    pub controls: Vec<Control>,
}
/// Contextual actions currently available in one row group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Skip,
    Replace,
    Include,
    Exclude,
}
/// Current wizard rendering data, without engine handles or mutable history.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct View {
    pub owner: u128,
    pub generation: u64,
    pub profile: Option<inputforge_core::profile::ProfileId>,
    pub sources: Vec<(DeviceId, String)>,
    pub targets: Vec<VirtualDeviceConfig>,
    pub modes: Vec<String>,
    pub source: Option<DeviceId>,
    pub target: Option<u8>,
    pub mode: String,
    pub all_modes: bool,
    pub rows: Vec<RowState>,
    pub conflicting: Vec<bool>,
    pub groups: Vec<Group>,
    pub counts: Counts,
    pub dirty: bool,
    pub can_apply: bool,
    pub error: Option<String>,
}
/// User intent; raw form values are interpreted and validated by the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Source(String),
    Target(String),
    Mode(String),
    AllModes(bool),
    RowTarget {
        kind: RowKind,
        index: u8,
        value: String,
    },
    Replace(RowKind, u8),
    ReplaceConflicts {
        kind: RowKind,
        replace: bool,
    },
    Include(RowKind),
    Exclude(RowKind),
    Reset,
    Apply,
}
/// Mounted wizard and rendered context identity accompanying an intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub owner: u128,
    pub generation: u64,
    pub intent: Intent,
}
/// Renderer-owned signals and presentation callback supplied at mount.
#[derive(Debug, Clone, Copy)]
pub struct Publication {
    pub view: Signal<View>,
    pub mode: Signal<String>,
    /// Called only after batch and barrier delivery; count is requested, not persisted.
    pub applied: Callback<usize>,
}
/// App-owned wizard installation, actions and teardown.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub install: Callback<Publication, u128>,
    pub dispatch: Callback<Request>,
    pub dispose: Callback<u128>,
}
