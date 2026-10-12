//! Prepared virtual layouts with app-owned edit and Apply callbacks.
#![expect(
    unused_qualifications,
    reason = "Dioxus event attributes produce false-positive qualification spans"
)]
use crate::{
    components::{Button, ButtonSize, ButtonVariant},
    controllers::{Action, Context},
};
use dioxus::prelude::*;
use inputforge_core::{
    output::traits::ControllerCapabilities,
    types::{VJoyAxis, VirtualDeviceConfig},
};
const AXES: [VJoyAxis; 8] = [
    VJoyAxis::X,
    VJoyAxis::Y,
    VJoyAxis::Z,
    VJoyAxis::Rx,
    VJoyAxis::Ry,
    VJoyAxis::Rz,
    VJoyAxis::Slider0,
    VJoyAxis::Slider1,
];

#[component]
pub(super) fn VirtualConfig() -> Element {
    let controllers = use_context::<Context>();
    let view = controllers.view.read().clone();
    let dispatch = controllers.dispatch;
    rsx! {
        fieldset { disabled: !view.editable,
            legend { "Virtual controllers" }
            if !view.capabilities.configurable { p { "Virtual controller layouts are provided by the output driver." } }
            if view.pending {
                p { role: "status", "Apply controller changes before starting. This reconnects the virtual controllers." }
                Button { size: ButtonSize::Sm, disabled: !view.can_apply, aria_label: "Apply controller changes",
                    onclick: move |_| dispatch.call(Action::Apply), "Apply controller changes" }
            }
            for config in view.slots {
                VirtualSlot { config, editable: view.layout_editable, capabilities: view.capabilities }
            }
            if view.capabilities.configurable {
                Button { size: ButtonSize::Sm, variant: ButtonVariant::Secondary, disabled: !view.can_add, aria_label: "Add virtual controller",
                    onclick: move |_| dispatch.call(Action::Add), "Add virtual controller" }
            }
        }
    }
}
#[component]
fn VirtualSlot(
    config: VirtualDeviceConfig,
    editable: bool,
    capabilities: ControllerCapabilities,
) -> Element {
    let dispatch = use_context::<Context>().dispatch;
    let slot = config.device_id;
    rsx! {
        div { class: "if-controller-session__slot",
            h4 { "Virtual controller {slot}" }
            label { "Buttons"
                input { r#type: "number", aria_label: format!("Buttons for virtual controller {slot}"), disabled: !editable, min: "{capabilities.min_buttons}", max: "{capabilities.max_buttons}", value: "{config.button_count}",
                    onchange: move |event: FormEvent| dispatch.call(Action::Buttons { slot, value: event.value() }) }
            }
            label { "Hats"
                input { r#type: "number", aria_label: format!("Hats for virtual controller {slot}"), disabled: !editable, min: "0", max: "{capabilities.max_hats}", value: "{config.hat_count}",
                    onchange: move |event: FormEvent| dispatch.call(Action::Hats { slot, value: event.value() }) }
            }
            div { class: "if-controller-session__axes",
                for axis in AXES {
                    label { input { r#type: "checkbox", aria_label: format!("{axis:?} for virtual controller {slot}"), disabled: !editable, checked: config.axes.contains(&axis),
                        onchange: move |event: FormEvent| dispatch.call(Action::Axis { slot, axis, enabled: event.checked() })
                    } "{axis:?}" }
                }
            }
            if capabilities.configurable {
                Button { size: ButtonSize::Sm, variant: ButtonVariant::Danger, disabled: !editable, aria_label: format!("Remove controller {slot}"),
                    onclick: move |_| dispatch.call(Action::Remove { slot }), "Remove controller {slot}" }
            }
        }
    }
}
