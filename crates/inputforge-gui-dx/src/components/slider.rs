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
    /// HTML `id` for label↔input coupling when wrapped in `Field`.
    #[props(default)]
    id: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-slider", "", class.as_deref());
    let id_attr = id.clone().unwrap_or_default();
    let input_handler = move |evt: FormEvent| {
        if let Some(h) = &oninput {
            h.call(evt);
        }
    };
    rsx! {
        input {
            r#type: "range",
            class: "{combined}",
            id: "{id_attr}",
            value: "{value}",
            min: "{min}",
            max: "{max}",
            step: "{step}",
            disabled,
            oninput: input_handler,
        }
    }
}
