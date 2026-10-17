//! Standalone handler fixtures use the real app inspector.
use dioxus::prelude::*;
pub(super) fn bindings() -> crate::controllers::inspector::Bindings {
    let controller = use_hook(|| {
        inputforge_app::controllers::inspector::Controller::new(
            crate::test_support::engine().state,
            crate::test_support::engine().commands,
        )
    });
    controller.bindings()
}
