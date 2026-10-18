//! Deadzone numeric toolbar rendering and supplied intent callbacks.
use crate::{
    components::{Button, ButtonSize, ButtonVariant, Field, NumberInput},
    instruments::{Event, View},
};
use dioxus::prelude::*;
use inputforge_core::action::Action;
#[component]
pub(crate) fn Toolbar(view: View, send: EventHandler<Event>, id_prefix: String) -> Element {
    let Action::Deadzone { config } = &view.value else {
        return rsx! {};
    };
    let values = [
        config.low(),
        config.center_low(),
        config.center_high(),
        config.high(),
    ];
    let labels = ["Low", "CL", "CH", "High"];
    let ids = ["dz-low", "dz-cl", "dz-ch", "dz-high"].map(|suffix| format!("{id_prefix}-{suffix}"));
    rsx! {
        div {
            class: "if-deadzone__toolbar",
            for field in 0..4 {
                Field {
                    label: labels[field].to_owned(),
                    for_id: ids[field].clone(),
                    error: view.error.clone(),
                    error_id: format!("{}-error", ids[field]),
                    NumberInput {
                        id: ids[field].clone(),
                        aria_describedby: view.error.as_ref().map(|_| format!("{}-error", ids[field])),
                        value: values[field],
                        min: view.bounds[field].0,
                        max: view.bounds[field].1,
                        step: 0.01,
                        precision: 2_usize,
                        oncommit: move |value| send.call(Event::Number { field, value }),
                        onstep: move |value| send.call(Event::Number { field, value }),
                    }
                }
            }
            Button {
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                onclick: move |_| send.call(Event::Reset),
                "Reset"
            }
        }
    }
}
