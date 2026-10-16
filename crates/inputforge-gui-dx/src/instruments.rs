//! Prepared instrument views, normalized observations and app callbacks.
use crate::{mapping_edits::StageId, mapping_rail::Target};
use dioxus::prelude::*;
use inputforge_core::action::Action;
use serde::Deserialize;

/// Captured stage identity and first-render value.
#[derive(Debug, Clone, PartialEq)]
pub struct Open {
    pub target: Option<Target>,
    pub path: StageId,
    pub seed: Action,
}
/// Read-only drawing state for one mounted curve or deadzone editor.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub id: u128,
    pub value: Action,
    pub hovered: Option<usize>,
    pub focused: Option<usize>,
    pub dragging: Option<usize>,
    pub split: bool,
    pub error: Option<String>,
    pub prevent_default: bool,
    pub bounds: [(f64, f64); 4],
}
/// Geometry measured by the renderer in engine-native input/output coordinates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Observation {
    pub position: Option<(f64, f64)>,
    pub hit: Option<usize>,
    pub centers: bool,
    pub split_dx: f64,
}
/// Native input forwarded without interpreting editing policy in the renderer.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Refresh,
    Pointer {
        kind: String,
        observation: Observation,
    },
    Key {
        key: String,
        shift: bool,
    },
    Blur,
    Cancel,
    Type(String),
    Symmetric(bool),
    Reset,
    Number {
        field: usize,
        value: f64,
    },
}
/// Per-event bounding rectangle from the existing desktop pointer transport.
#[derive(Debug, Clone, Deserialize)]
pub struct Pointer {
    pub kind: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub rl: f64,
    #[serde(default)]
    pub rt: f64,
    #[serde(default)]
    pub rs: f64,
}
/// A mounted plot and the renderer's geometry/publication callbacks.
#[derive(Debug, Clone)]
pub struct Mount {
    pub id: u128,
    pub plot: String,
    pub observe: Callback<Pointer, Observation>,
    pub publish: Callback<View>,
}
/// App-owned editor behavior and document-listener lifetime.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub open: Callback<Open, View>,
    pub event: Callback<(u128, Event), Option<View>>,
    pub close: Callback<u128>,
    pub mount: Callback<Mount>,
}
