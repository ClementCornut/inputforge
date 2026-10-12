//! Routing button presentation; all eligibility and effects come from the app.
use crate::{
    components::{Button, ButtonSize, ButtonVariant},
    routing::{Action, Context},
};
use dioxus::prelude::*;

#[component]
pub(super) fn RoutingControls() -> Element {
    let routing = use_context::<Context>();
    let view = *routing.view.read();
    let label = view.start.map_or("Start", Action::label);
    rsx! {
        div { class: "if-controller-session__actions",
            Button { size: ButtonSize::Sm, disabled: view.start.is_none(),
                onclick: move |_| { if let Some(action) = view.start { routing.dispatch.call(action); } }, "{label}"
            }
            Button { size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: view.stop.is_none(),
                onclick: move |_| { if let Some(action) = view.stop { routing.dispatch.call(action); } }, "Stop"
            }
        }
    }
}

#[component]
pub(super) fn RefreshControl() -> Element {
    let routing = use_context::<Context>();
    let action = routing.view.read().refresh;
    rsx! {
        Button { size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: action.is_none(),
            onclick: move |_| { if let Some(action) = action { routing.dispatch.call(action); } }, "Refresh devices"
        }
    }
}
