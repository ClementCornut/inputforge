use dioxus::prelude::*;

use super::merge_class;

#[component]
pub fn Slider(
    value: ReadSignal<f64>,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default = 0.0)] min: f64,
    #[props(default = 1.0)] max: f64,
    #[props(default = 0.01)] step: f64,
    #[props(default)] disabled: bool,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-slider", "", class.as_deref());
    let input_handler = move |evt: FormEvent| {
        if let Some(h) = &oninput {
            h.call(evt);
        }
    };
    rsx! {
        input {
            r#type: "range",
            class: "{combined}",
            value: "{value}",
            min: "{min}",
            max: "{max}",
            step: "{step}",
            disabled,
            oninput: input_handler,
        }
    }
}
