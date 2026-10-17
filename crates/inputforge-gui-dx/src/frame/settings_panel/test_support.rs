//! Standalone renderer fixtures use real app settings behavior.
use crate::{context::AppContext, settings::Bindings};
use dioxus::prelude::*;
pub(super) fn bindings() -> Bindings {
    let ctx = use_context::<AppContext>();
    let controller = use_hook(|| {
        let values = ctx.settings.peek().clone();
        {
            let mut s = ctx.state.write();
            s.snapshot_config = values.snapshot;
            s.startup = values.startup;
            s.default_double_tap_threshold_ms = values.default_double_tap_threshold_ms;
            s.default_long_press_threshold_ms = values.default_long_press_threshold_ms;
        };
        inputforge_app::settings::Controller::new(ctx.state, ctx.commands)
    });
    controller.bindings()
}
