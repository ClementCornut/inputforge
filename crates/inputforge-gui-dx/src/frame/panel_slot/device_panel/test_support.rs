//! Standalone handler fixtures use the real app inspector.
use dioxus::prelude::*;
pub(super) fn bindings() -> crate::controllers::inspector::Bindings {
    let ctx = use_context::<crate::context::AppContext>();
    let controller = use_hook(|| {
        inputforge_app::controllers::inspector::Controller::new(ctx.state, ctx.commands)
    });
    controller.bindings()
}
