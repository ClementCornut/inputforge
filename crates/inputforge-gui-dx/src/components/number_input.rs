use dioxus::prelude::*;

use super::merge_class;
use crate::components::button::{ButtonSize, ButtonVariant};
use crate::components::icon_button::IconButton;
use crate::components::text_input::InputSize;
use crate::icons::Icon as IconKind;

#[component]
pub fn NumberInput(
    value: ReadSignal<f64>,
    oninput: Option<EventHandler<FormEvent>>,
    /// Emits the post-clamp value when the +/- stepper buttons are clicked.
    /// The native `<input type="number">` arrow keys still fire `oninput` instead.
    onstep: Option<EventHandler<f64>>,
    #[props(default = f64::NEG_INFINITY)] min: f64,
    #[props(default = f64::INFINITY)] max: f64,
    #[props(default = 1.0)] step: f64,
    /// Decimal places used to format `value` for display. `None` = native default.
    #[props(default)]
    precision: Option<usize>,
    #[props(default)] disabled: bool,
    /// HTML `id` for label↔input coupling when wrapped in `Field`.
    #[props(default)]
    id: Option<String>,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-number-input--sm",
        InputSize::Md => "if-number-input--md",
        InputSize::Lg => "if-number-input--lg",
    };
    let combined = merge_class("if-number-input", size_class, class.as_deref());
    let input_handler = move |evt: FormEvent| {
        if let Some(h) = &oninput {
            h.call(evt);
        }
    };
    let display_value = match precision {
        Some(p) => format!("{:.*}", p, value()),
        None => format!("{}", value()),
    };
    let onstep_inc = onstep;
    let onstep_dec = onstep;
    let on_inc = move |_| {
        if let Some(h) = &onstep_inc {
            let next = (value() + step).min(max).max(min);
            h.call(next);
        }
    };
    let on_dec = move |_| {
        if let Some(h) = &onstep_dec {
            let next = (value() - step).min(max).max(min);
            h.call(next);
        }
    };
    // HTML5 forbids id="" — so render the attribute only when Some.
    rsx! {
        div { class: "{combined}",
            if let Some(ref id_val) = id {
                input {
                    r#type: "number",
                    class: "if-number-input__field",
                    id: "{id_val}",
                    value: "{display_value}",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    disabled,
                    oninput: input_handler,
                }
            } else {
                input {
                    r#type: "number",
                    class: "if-number-input__field",
                    value: "{display_value}",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    disabled,
                    oninput: input_handler,
                }
            }
            div {
                class: "if-number-input__steppers",
                IconButton { icon: IconKind::Plus,  label: "Increment", size: ButtonSize::Sm, variant: ButtonVariant::Ghost, disabled, onclick: on_inc }
                IconButton { icon: IconKind::Minus, label: "Decrement", size: ButtonSize::Sm, variant: ButtonVariant::Ghost, disabled, onclick: on_dec }
            }
        }
    }
}
