//! Prepared keyboard-capture destinations, signals and application callbacks.
use dioxus::prelude::{Callback, ScopeId, Signal};
use inputforge_core::{pipeline::ActionPathSegment, types::KeyCombo};

/// An existing keyboard field or an insertion position in an action branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    Edit(Vec<ActionPathSegment>),
    Insert(Vec<ActionPathSegment>),
}
/// The rendered mapping and exact destination of a keyboard capture.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub mapping: crate::capture::Target,
    pub destination: Destination,
}
/// Renderer-owned navigation used to reject obsolete destinations.
#[derive(Debug, Clone, Copy)]
pub struct Publication {
    pub mode: Signal<String>,
    pub selection: Signal<Option<crate::snapshots::MappingKey>>,
}
/// One consumer's target, display signals and existing completion handler.
#[derive(Debug, Clone, Copy)]
pub struct Request {
    pub owner: ScopeId,
    pub target: Signal<Target>,
    pub active: Signal<bool>,
    pub pending: Signal<bool>,
    pub hint: Signal<Option<&'static str>>,
    pub on_commit: Callback<KeyCombo>,
}
/// Application operations wired by the renderer and its capture consumers.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub install: Callback<Publication>,
    pub start: Callback<Request>,
    pub cancel: Callback<ScopeId>,
    pub deliver: Callback<ScopeId>,
    pub dispose: Callback<()>,
}
/// Prepared state and controls for a keyboard-capture surface.
#[derive(Debug, Clone, Copy)]
pub struct Capture {
    pub active: Signal<bool>,
    pub hint: Signal<Option<&'static str>>,
    pub start: Callback<()>,
    pub cancel: Callback<()>,
}
