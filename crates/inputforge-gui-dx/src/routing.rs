//! App-prepared routing controls and callbacks for the editor.
//!
//! Hosts supply the decision model; components only render it and forward actions.
use dioxus::prelude::*;
#[cfg(test)]
pub use inputforge_gui_dx::routing::{Action, Bindings, View};

#[cfg(not(test))]
/// Routing operation requested by a rendered control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Start routing.
    Start,
    /// Stop routing and retain neutral controllers.
    Stop,
    /// Retry failed routing.
    Retry,
    /// Refresh input devices.
    Refresh,
    /// Open Devices to review a pending layout.
    ReviewDevices,
}

#[cfg(not(test))]
/// Available actions prepared by the application; absent actions are disabled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// A retained output layout needs explicit review.
    pub pending: bool,
    /// Engine pill and tray toggle action.
    pub toggle: Option<Action>,
    /// Devices panel start action.
    pub start: Option<Action>,
    /// Devices panel stop action.
    pub stop: Option<Action>,
    /// Editor notice action.
    pub recovery: Option<Action>,
    /// Devices panel refresh action.
    pub refresh: Option<Action>,
}

#[cfg(not(test))]
/// App-owned dispatch for rendered routing actions.
#[derive(Clone, Copy, Debug)]
pub struct Bindings {
    /// Revalidate current engine state and execute a requested action.
    pub dispatch: Callback<Action>,
}

#[cfg(not(test))]
impl Action {
    /// Short display label for a routing action.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Stop => "Stop",
            Self::Retry => "Retry",
            Self::Refresh => "Refresh devices",
            Self::ReviewDevices => "Open Devices",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Context {
    pub view: Signal<View>,
    pub dispatch: Callback<Action>,
}

impl Context {
    pub(crate) fn publish(mut self, view: View) {
        if *self.view.peek() != view {
            self.view.set(view);
        }
    }
}

#[cfg(test)]
pub(crate) fn provide_fixture(view: View) {
    let view = use_signal(|| view);
    use_context_provider(|| Context {
        view,
        dispatch: Callback::new(|_| {}),
    });
}

#[cfg(test)]
#[path = "routing_tests.rs"]
mod tests;
