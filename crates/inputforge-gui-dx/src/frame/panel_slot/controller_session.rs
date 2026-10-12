//! Controller session presentation; inspecting a device never selects it for routing.
#![expect(
    unused_qualifications,
    reason = "Dioxus event attributes produce false-positive qualification spans"
)]
use super::{
    axis_settings::AxisSettings,
    routing_controls::{RefreshControl, RoutingControls},
    virtual_config::VirtualConfig,
};
use crate::{
    context::AppContext,
    controllers::{Action, Context},
};
use dioxus::prelude::*;

#[component]
pub(super) fn ControllerSession() -> Element {
    let ctx = use_context::<AppContext>();
    let controllers = use_context::<Context>();
    let view = controllers.view.read().clone();
    let error = controllers.error.read().clone();
    let meta = ctx.meta.read().clone();
    let ready = if meta.session.ready {
        "Inputs ready"
    } else {
        "Waiting for inputs"
    };
    let monitored = meta.session.monitored.len();
    let captured = meta.session.captured.len();
    rsx! {
        section { class: "if-controller-session", "aria-label": "Controller session",
            h3 { "Controller session" }
            p { role: "status", "{ready} · {monitored} monitored · {captured} captured" }
            RoutingControls {}
            if view.retained { p { "Routing stopped. Virtual controllers stay connected and neutral. Inputs remain available." } }
            if let Some(error) = meta.session.error { p { role: "alert", "{error}" } }
            if let Some(error) = error { p { role: "alert", "{error}" } }
            if !meta.session.keyboard_supported { p { "Keyboard output is unavailable." } }
            if !meta.session.mouse_supported { p { "Mouse output is unavailable." } }
            fieldset { disabled: !view.editable,
                legend { "Route these controllers" }
                for row in view.selections {
                    { let dispatch = controllers.dispatch;
                        rsx! { label {
                            input { r#type: "checkbox", aria_label: row.label.clone(), checked: row.selected, disabled: !row.enabled,
                                onchange: move |event: FormEvent| dispatch.call(Action::Select { device: row.device.clone(), selected: event.checked() })
                            }
                            "{row.label}"
                            if !row.connected { " (disconnected)" }
                        } }
                    }
                }
                p { "Stop routing to change the controller selection or virtual output." }
            }
            RefreshControl {}
            AxisSettings {}
            VirtualConfig {}
        }
    }
}
