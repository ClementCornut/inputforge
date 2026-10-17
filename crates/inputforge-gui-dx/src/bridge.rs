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
    mappings: Option<crate::mapping_edits::Bindings>,
    profiles: Option<crate::profiles::Bindings>,
    settings: Option<crate::settings::Bindings>,
    inspector: Option<crate::controllers::inspector::Bindings>,
    notifications: Option<crate::toast::ToastQueue>,
}
impl Publisher {
    pub(crate) fn new(
        context: AppContext,
        view: ViewState,
        routing: Context,
        controllers: crate::controllers::Context,
    ) -> Self {
        Self {
            notifications: try_consume_context::<crate::toast::ToastQueue>(),
            context,
            selection: view.selected_mapping,
            routing,
            controllers,
            mappings: try_consume_context::<crate::mapping_edits::Bindings>(),
            profiles: try_consume_context::<crate::profiles::Bindings>(),
            settings: try_consume_context::<crate::settings::Bindings>(),
            inspector: try_consume_context::<crate::controllers::Bindings>().map(|b| b.inspector),
        }
    }
    /// Read the selected mapping without subscribing the caller.
    #[must_use]
    pub fn selected_mapping(&self) -> Option<MappingKey> {
        self.selection.peek().clone()
    }
    /// Forward the app reconciliation callback on the existing polling cadence.
    pub fn reconcile_history(&self) {
        if let Some(bindings) = self.mappings {
            bindings.reconcile.call(());
        }
    }
    /// Advance app notification deadlines on the existing tick, including contention.
    pub fn advance_notifications(&self, warnings: Option<Vec<String>>, now: std::time::Instant) {
        if let Some(notifications) = self.notifications {
            notifications.tick.call((warnings, now));
        }
    }
    /// Publish deadline-only changes when the engine read is contended.
    pub fn publish_readout(&self, readout: Option<crate::readouts::Readout>) {
        let mut live = self.context.live;
        if live.peek().readout != readout {
            live.with_mut(|live| live.readout = readout);
        }
    }
    /// Publish changed fields without waking unchanged subscribers.
    pub fn publish(&self, frame: Frame) {
        if let Some(bindings) = self.profiles {
            bindings.reconcile.call(frame.meta.clone());
        }
        if let Some(bindings) = self.settings {
            bindings.reconcile.call(());
        }
        if let Some(bindings) = self.inspector {
            bindings.reconcile.call(());
        }
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
