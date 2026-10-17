//! Keyboard-capture publication signals and application callback wiring.
use crate::keyboard_capture::{Bindings, Capture, Destination, Publication, Request, Target};
use dioxus::prelude::*;
use inputforge_core::types::KeyCombo;

/// Install editor navigation and dispose the app controller with this scope.
pub fn use_keyboard_capture_provider() {
    #[cfg(not(test))]
    let bindings = use_context::<Bindings>();
    #[cfg(test)]
    let bindings = try_use_context::<Bindings>().unwrap_or_else(|| {
        inputforge_app::keyboard_capture::Controller::new(crate::test_support::engine().state)
            .bindings()
    });
    use_context_provider(|| bindings);
    #[cfg(not(test))]
    let view = use_context::<crate::frame::ViewState>();
    #[cfg(test)]
    let view = try_use_context::<crate::frame::ViewState>().unwrap_or_else(|| {
        let ctx = use_context::<crate::context::AppContext>();
        crate::frame::use_view_state_provider(ctx.meta)
    });
    let publication = Publication {
        mode: view.editing_mode,
        selection: view.selected_mapping,
    };
    use_context_provider(|| publication);
    use_hook(|| bindings.install.call(publication));
    use_drop(move || bindings.dispose.call(()));
}

/// Wire one rendered destination to app capture and its existing commit handler.
#[must_use]
pub fn use_keyboard_capture(target: Target, on_commit: Callback<KeyCombo>) -> Capture {
    let bindings = use_context::<Bindings>();
    let publication = use_context::<Publication>();
    let mut target_signal = use_signal(|| target.clone());
    if *target_signal.peek() != target {
        target_signal.set(target);
    }
    let request = Request {
        owner: dioxus::core::current_scope_id(),
        target: target_signal,
        active: use_signal(|| false),
        pending: use_signal(|| false),
        hint: use_signal(|| None),
        on_commit,
    };
    let start = use_callback(move |()| bindings.start.call(request));
    let cancel = use_callback(move |()| bindings.cancel.call(request.owner));
    use_effect(move || {
        let _ = (
            request.pending.read(),
            request.target.read(),
            publication.mode.read(),
            publication.selection.read(),
        );
        bindings.deliver.call(request.owner);
    });
    use_drop(move || bindings.cancel.call(request.owner));
    Capture {
        active: request.active,
        hint: request.hint,
        start,
        cancel,
    }
}

pub(crate) fn target(
    ctx: &crate::context::AppContext,
    mapping_key: &crate::snapshots::MappingKey,
    actions: &[inputforge_core::action::Action],
    destination: Destination,
) -> Target {
    Target {
        mapping: crate::capture::Target {
            profile: ctx.meta.read().profile_id.clone(),
            mode: mapping_key.0.clone(),
            mapping: Some(mapping_key.1.clone()),
            name: ctx
                .config
                .read()
                .mappings
                .iter()
                .find(|mapping| mapping.input == mapping_key.1 && mapping.mode == mapping_key.0)
                .and_then(|mapping| mapping.name.clone()),
            actions: Some(actions.to_vec()),
            field: "keyboard".into(),
        },
        destination,
    }
}
#[cfg(test)]
mod display_tests;
