//! Mode presentation signals and native observations handled by the app.
use super::{Operation, Scope, Target, Transition};
use crate::overlays::CloseReason;
use dioxus::prelude::*;
/// Floating menu geometry paired with its captured mode identity.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuTarget {
    pub scope: Scope,
    pub name: String,
    pub left: f64,
    pub bottom: f64,
}
/// Shell-owned mode publications shared by tabs and the confirmation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Publication {
    pub adding: Signal<Option<Scope>>,
    pub renaming: Signal<Option<(Scope, String)>>,
    pub menu: Signal<Option<MenuTarget>>,
    pub error: Signal<Option<String>>,
    pub delete: Signal<Option<Target>>,
    pub focus: Signal<Option<String>>,
}
/// Mounted inline draft publications.
#[derive(Clone, Debug)]
pub struct Draft {
    pub value: Signal<String>,
    pub error: Signal<Option<String>>,
    pub from: Option<String>,
}
/// Native observation or existing mode operation.
#[derive(Clone, Debug)]
pub enum Event {
    BeginAdd,
    Select(String),
    Context(String, MouseEvent),
    Key {
        name: String,
        dom_id: String,
        event: KeyboardEvent,
    },
    Input {
        draft: Draft,
        value: String,
    },
    DraftKey {
        draft: Draft,
        event: KeyboardEvent,
    },
    Commit {
        draft: Draft,
        blur: bool,
    },
    CancelDraft(Draft),
    MenuAction {
        operation: Operation,
        activation: Signal<Option<Transition>>,
        error: Signal<Option<String>>,
    },
    MenuClose {
        target: MenuTarget,
        reason: CloseReason,
        activation: Signal<Option<Transition>>,
        error: Signal<Option<String>>,
    },
}
/// Captured scope and current presentation destinations for one observation.
#[derive(Clone, Debug)]
pub struct Request {
    pub publication: Publication,
    pub scope: Scope,
    pub event: Event,
}

/// Shared shell destinations used by the independently mounted confirmation.
#[derive(Clone, Copy, Debug)]
pub struct Shell {
    pub delete: Signal<Option<Target>>,
    pub focus: Signal<Option<String>>,
}
/// Existing confirmation observation and visible error destination.
#[derive(Clone, Copy, Debug)]
pub struct DeleteRequest {
    pub shell: Shell,
    pub error: Signal<Option<String>>,
    pub confirm: bool,
}
