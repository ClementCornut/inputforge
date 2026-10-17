//! Prepared stage validation and its rendered publication destinations.
use super::StageId;
use dioxus::prelude::*;
use inputforge_core::{
    action::{Condition, ModeChangeStrategy},
    types::{InputAddress, OutputAddress, VirtualDeviceConfig},
};
use std::collections::HashMap;
/// Existing incomplete-stage guidance.
pub const MERGE_UNBOUND: &str = "Bind a secondary input to complete this merge";
/// Existing incomplete-condition guidance.
pub const PREDICATE_UNBOUND: &str = "Bind an input to complete this condition";
/// Existing empty target-mode guidance.
pub const TARGET_EMPTY: &str = "Choose a mode to switch to";
/// Existing unsupported hold-input guidance.
pub const HOLD_NOT_BUTTON: &str =
    "Hold requires a button input. Pick a button on your device, or switch to Set.";
/// Raw prepared stage values whose validation belongs to the app.
#[derive(Clone, Debug)]
pub enum Input {
    Vjoy(OutputAddress, Vec<VirtualDeviceConfig>),
    Merge(InputAddress, InputAddress),
    Predicate(Condition, (f64, f64)),
    Mode(ModeChangeStrategy, InputAddress, Vec<String>),
}
/// Rendering flags prepared alongside validation text.
#[derive(Clone, Copy, Debug, Default)]
pub struct View {
    pub hold_disabled: bool,
    pub target_orphaned: bool,
}
/// Current rendered stage and its diagnostic publication destinations.
#[derive(Clone, Debug)]
pub struct Request {
    pub path: StageId,
    pub input: Input,
    pub hints: Signal<HashMap<StageId, String>>,
    pub tags: Option<Signal<HashMap<StageId, String>>>,
}
