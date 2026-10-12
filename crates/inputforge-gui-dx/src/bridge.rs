//! Signal publication for application-prepared rendering data.
use crate::{
    context::AppContext,
    frame::ViewState,
    routing::Context,
    snapshots::{Frame, MappingKey},
};
use dioxus::prelude::*;

/// GUI-owned publication handle for a single editor scope.
#[derive(Clone, Debug)]
pub struct Publisher {
    context: AppContext,
    selection: Signal<Option<MappingKey>>,
    routing: Context,
    controllers: crate::controllers::Context,
}
impl Publisher {
    pub(crate) fn new(
        context: AppContext,
        view: ViewState,
        routing: Context,
        controllers: crate::controllers::Context,
    ) -> Self {
        Self {
            context,
            selection: view.selected_mapping,
            routing,
            controllers,
        }
    }
    /// Read the selected mapping without subscribing the caller.
    #[must_use]
    pub fn selected_mapping(&self) -> Option<MappingKey> {
        self.selection.peek().clone()
    }
    /// Publish changed fields without waking unchanged subscribers.
    pub fn publish(&self, frame: Frame) {
        self.routing.publish(frame.routing);
        publish(self.controllers.view, frame.controllers);

        publish(self.context.meta, frame.meta);
        publish(self.context.config, frame.config);
        publish(self.context.live, frame.live);
        publish(self.context.settings, frame.settings);
    }
}
fn publish<T: PartialEq + 'static>(mut signal: Signal<T>, value: T) {
    if *signal.peek() != value {
        signal.set(value);
    }
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod tests;
