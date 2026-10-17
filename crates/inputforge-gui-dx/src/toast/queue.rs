//! App-supplied notification callbacks and prepared visible values.
use super::{Toast, ToastLevel};
use dioxus::prelude::*;
use std::time::Instant;

/// Raw notification observations interpreted by the application.
#[derive(Clone, Debug, PartialEq)]
pub enum Interaction {
    Hover(bool),
    Focus(bool),
    Key(Key),
    Dismiss,
}

/// Rendering context with application-owned production and interaction callbacks.
#[derive(Clone, Copy, Debug)]
pub struct ToastQueue {
    pub visible: Signal<Vec<Toast>>,
    pub push: Callback<(ToastLevel, String)>,
    pub event: Callback<(u64, Interaction)>,
    pub tick: Callback<(Option<Vec<String>>, Instant)>,
    pub dispose: Callback<()>,
}
impl ToastQueue {
    /// Forward an existing workflow notification to the application.
    pub fn push(&self, level: ToastLevel, message: impl Into<String>) {
        self.push.call((level, message.into()));
    }
}

/// Install one editor-owned notification context into prepared rendering signals.
#[derive(Clone, Copy, Debug)]
pub struct Bindings {
    pub mount: Callback<Signal<Vec<Toast>>, ToastQueue>,
}
