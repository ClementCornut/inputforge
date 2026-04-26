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
    #[props(default = f64::NEG_INFINITY)] min: f64,
    #[props(default = f64::INFINITY)] max: f64,
    #[props(default = 1.0)] step: f64,
    #[props(default)] disabled: bool,
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
    rsx! {
        div { class: "{combined}",
            input {
                r#type: "number",
                class: "if-number-input__field",
                value: "{value}",
                min: "{min}",
                max: "{max}",
                step: "{step}",
                disabled,
                oninput: input_handler,
            }
            div {
                class: "if-number-input__steppers",
                IconButton { icon: IconKind::Plus,  label: "Increment", size: ButtonSize::Sm, variant: ButtonVariant::Ghost, disabled }
                IconButton { icon: IconKind::Minus, label: "Decrement", size: ButtonSize::Sm, variant: ButtonVariant::Ghost, disabled }
            }
        }
    }
}
