use dioxus::prelude::*;

use crate::components::InputSize;
use crate::components::merge_class;

/// Reason a committed value was rejected. The consumer surfaces an inline
/// validation message and blocks the dispatch on the `Err` branch.
///
/// Locale-aware parsing is out of scope; "1,000" is not accepted.
#[cfg(not(test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegerInputError {
    Empty,
    NotANumber,
    OutOfRange { min: usize, max: usize },
}

#[cfg(test)]
pub use inputforge_gui_dx::components::IntegerInputError;

#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on per-element \
              event listeners; mirrors number_input.rs"
)]
pub fn IntegerInput(
    value: ReadSignal<usize>,
    min: usize,
    max: usize,
    /// Emits the parsed value when it lies in `[min, max]` after Enter or blur.
    /// In-flight typing fires `oninput` only.
    oncommit: Option<EventHandler<usize>>,
    /// Fires on Enter or blur when the buffer is empty, unparseable, or
    /// out of range. The consumer surfaces an inline validation message
    /// and blocks the dispatch.
    oninvalid: Option<EventHandler<IntegerInputError>>,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(default)] aria_errormessage: Option<String>,
    #[props(default)] invalid: bool,
    #[props(default)] id: Option<String>,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-integer-input--sm",
        InputSize::Md => "if-integer-input--md",
        InputSize::Lg => "if-integer-input--lg",
    };
    let combined = merge_class("if-integer-input", size_class, class.as_deref());
    let combined = merge_class(
        &combined,
        if invalid {
            "if-integer-input--invalid"
        } else {
            ""
        },
        None,
    );
    let display_value = format!("{}", value());

    let view = (crate::components::behavior().integer)(&crate::controls::IntegerProps {
        value,
        display: display_value,
        min,
        max,
        disabled,
        oninput,
        oncommit,
        oninvalid,
    });
    let display_value = view.text;
    let text = (crate::components::behavior().text)(crate::controls::TextProps {
        value: display_value,
        disabled,
        oninput: Some(view.input),
    });
    let input_handler = text.input;
    let on_input_keydown = view.key;
    let on_input_blur = view.blur;
    let on_input_focus = view.focus;
    // HTML5 forbids id="", so render the attribute only when Some.
    rsx! {
        div { class: "{combined}",
            if let Some(ref id_val) = id {
                input {
                    r#type: "number",
                    inputmode: "numeric",
                    class: "if-integer-input__field",
                    id: "{id_val}",
                    initial_value: "{display_value}",
                    "data-if-text-owner": "{text.owner}",
                    min: "{min}",
                    max: "{max}",
                    step: "1",
                    disabled,
                    "aria-label": aria_label.clone(),
                    "aria-labelledby": aria_labelledby.clone(),
                    "aria-describedby": aria_describedby.clone(),
                    "aria-invalid": invalid.then_some("true"),
                    "aria-errormessage": aria_errormessage.clone().filter(|_| invalid),
                    oninput: input_handler,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                    onfocus: on_input_focus,
                }
            } else {
                input {
                    r#type: "number",
                    inputmode: "numeric",
                    class: "if-integer-input__field",
                    initial_value: "{display_value}",
                    "data-if-text-owner": "{text.owner}",
                    min: "{min}",
                    max: "{max}",
                    step: "1",
                    disabled,
                    "aria-label": aria_label.clone(),
                    "aria-labelledby": aria_labelledby.clone(),
                    "aria-describedby": aria_describedby.clone(),
                    "aria-invalid": invalid.then_some("true"),
                    "aria-errormessage": aria_errormessage.clone().filter(|_| invalid),
                    oninput: input_handler,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                    onfocus: on_input_focus,
                }
            }
        }
    }
}
