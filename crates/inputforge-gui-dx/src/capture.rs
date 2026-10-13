//! Prepared live-capture endpoints and application-provided callbacks.
use dioxus::prelude::*;
use inputforge_core::{action::Action, profile::ProfileId, types::InputAddress};

/// Input kinds accepted by a capture surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureFilter {
    #[default]
    Any,
    AxesOnly,
    ButtonsOnly,
}

/// The rendered destination of a live capture.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Target {
    pub profile: Option<ProfileId>,
    pub mode: String,
    pub mapping: Option<InputAddress>,
    pub name: Option<String>,
    pub actions: Option<Vec<Action>>,
    /// Distinguishes primary, secondary, and predicate fields within a mapping.
    pub field: String,
}

/// A consumer's publication signals and completion callbacks.
#[derive(Debug, Clone, Copy)]
pub struct Request {
    pub owner: ScopeId,
    pub filter: CaptureFilter,
    pub target: Signal<Target>,
    pub listening: Signal<bool>,
    pub on_capture: Callback<InputAddress>,
    pub on_cancel: Callback<()>,
}

/// Application-owned capture operations installed at the editor root.
#[derive(Debug, Clone, Copy)]
pub struct Bindings {
    pub install: Callback<Publication>,
    pub start: Callback<Request>,
    pub cancel: Callback<ScopeId>,
    pub dispose: Callback<()>,
}

/// Renderer-owned root signals used to revalidate capture navigation.
#[derive(Debug, Clone, Copy)]
pub struct Publication {
    pub active: Signal<bool>,
    pub mode: Signal<String>,
    pub selection: Signal<Option<crate::snapshots::MappingKey>>,
}

/// Prepared listening state and controls for one rendered capture surface.
#[derive(Debug, Clone, Copy)]
pub struct Capture {
    pub listening: Signal<bool>,
    pub start: Callback<()>,
    pub cancel: Callback<()>,
}
