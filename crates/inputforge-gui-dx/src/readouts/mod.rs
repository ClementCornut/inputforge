//! Prepared live readout values and application-owned expansion transitions.
pub mod labels;
mod model;
use dioxus::prelude::Callback;
use inputforge_core::{
    action::Action,
    profile::ProfileId,
    types::{AxisPolarity, HatDirection},
};
#[doc(inline)]
pub use model::{
    Branch, ChainStep, LiveReadoutModel, OutputDescriptor, OutputDestination, PredicateDescriptor,
    PredicateKind,
};
use std::path::PathBuf;
/// Axis value in the polarity's natural display domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisDisplay {
    pub value: f64,
    pub polarity: AxisPolarity,
}
/// Completed input or destination value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadoutDisplay {
    Axis(AxisDisplay),
    Button { pressed: bool },
    Hat { direction: HatDirection },
}
impl Default for ReadoutDisplay {
    fn default() -> Self {
        Self::Axis(AxisDisplay {
            value: 0.0,
            polarity: AxisPolarity::Bipolar,
        })
    }
}
/// Prepared input row and rail highlight magnitude.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InputReadout {
    pub display: ReadoutDisplay,
    pub intensity: f64,
}
/// Captured selected action-tree identity for expansion and activity resets.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub profile: Option<ProfileId>,
    pub path: Option<PathBuf>,
    pub mapping: crate::snapshots::MappingKey,
    pub actions: Vec<Action>,
    pub generation: u64,
}
/// One coherent selected mapping readout, including instrument tracking dots.
#[derive(Debug, Clone, PartialEq)]
pub struct Readout {
    pub key: Key,
    pub model: LiveReadoutModel,
    /// Completed values aligned with `model.outputs`.
    pub outputs: Vec<ReadoutDisplay>,
    /// Root-stage input values; nested and ineligible stages have no dot.
    pub instruments: Vec<Option<f64>>,
    pub running: bool,
}
/// Per-mount rendering state; the application interprets its transitions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExpandState {
    pub key: Option<Key>,
    pub per_output: Vec<bool>,
}
impl ExpandState {
    /// Return the prepared expansion state for an output row.
    #[must_use]
    pub fn is_expanded(&self, index: usize) -> bool {
        self.per_output.get(index).copied().unwrap_or(false)
    }
}
/// Expansion input captured against the currently rendered model.
#[derive(Debug, Clone, PartialEq)]
pub struct Expansion {
    pub state: ExpandState,
    pub key: Key,
    pub outputs: usize,
    pub toggle: Option<usize>,
}
/// Application-owned reset and output-toggle decisions.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub expand: Callback<Expansion, ExpandState>,
}
