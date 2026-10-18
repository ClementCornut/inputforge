use dioxus::prelude::*;

use crate::components::IconButton;
use crate::components::InputSize;
use crate::components::merge_class;
use crate::components::{ButtonSize, ButtonVariant};
use crate::icons::Icon as IconKind;

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures. Mirrors the suppression \
              used in mapping_editor/header.rs."
)]
pub fn NumberInput(
    value: ReadSignal<f64>,
    oninput: Option<EventHandler<FormEvent>>,
    /// Emits the post-clamp value when the +/- stepper buttons are clicked.
    /// The native `<input type="number">` arrow keys still fire `oninput` instead.
    onstep: Option<EventHandler<f64>>,
    /// Emits the post-parse, post-clamp value when the user finishes editing
    /// (Enter pressed or input loses focus). Free-typing fires `oninput` only.
    oncommit: Option<EventHandler<f64>>,
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
    let display_value = match precision {
        Some(p) => format!("{:.*}", p, value()),
        None => format!("{}", value()),
    };
    let view = (crate::components::behavior().number)(&crate::controls::NumberProps {
        value,
        display: display_value,
        min,
        max,
        step,
        disabled,
        oninput,
        oncommit,
        onstep,
    });
    let display_value = view.input.text;
    let text = (crate::components::behavior().text)(crate::controls::TextProps {
        value: display_value,
        disabled,
        oninput: Some(view.input.input),
    });
    let input_handler = text.input;
    let on_input_keydown = view.input.key;
    let on_input_blur = view.input.blur;
    let on_input_focus = view.input.focus;
    let on_inc = move |_| view.step.call(true);
    let on_dec = move |_| view.step.call(false);
    // HTML5 forbids id="", so render the attribute only when Some.
    rsx! {
        div { class: "{combined}",
            if let Some(ref id_val) = id {
                input {
                    r#type: "number",
                    class: "if-number-input__field",
                    id: "{id_val}",
                    initial_value: "{display_value}",
                    "data-if-text-owner": "{text.owner}",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    disabled,
                    oninput: input_handler,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                    onfocus: on_input_focus,
                }
            } else {
                input {
                    r#type: "number",
                    class: "if-number-input__field",
                    initial_value: "{display_value}",
                    "data-if-text-owner": "{text.owner}",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    disabled,
                    oninput: input_handler,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                    onfocus: on_input_focus,
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
