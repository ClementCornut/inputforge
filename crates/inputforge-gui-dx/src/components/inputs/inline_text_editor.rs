//! Accessible in-place text editing with app-owned draft and key/focus callbacks.
use super::text_input::{InputSize, TextInput, TextInputVariant};
use dioxus::prelude::*;

/// In-place text field with an explicit accessible name and selectable visual treatment.
#[component]
pub fn InlineTextEditor(
    value: ReadSignal<String>,
    aria_label: String,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default)] id: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] invalid: bool,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(default)] aria_errormessage: Option<String>,
    #[props(default)] data_editor_focus: Option<String>,
    #[props(default = InputSize::Sm)] size: InputSize,
    #[props(default = TextInputVariant::Inline)] variant: TextInputVariant,
    #[props(default)] class: Option<String>,
    #[props(default)] onmounted: Option<EventHandler<MountedEvent>>,
    #[props(default)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(default)] onblur: Option<EventHandler<FocusEvent>>,
    #[props(default)] onfocus: Option<EventHandler<FocusEvent>>,
) -> Element {
    rsx! {
        TextInput {
            value,
            aria_label,
            oninput,
            id,
            disabled,
            invalid,
            placeholder,
            aria_labelledby,
            aria_describedby,
            aria_errormessage,
            data_editor_focus,
            size,
            variant,
            class,
            onmounted,
            onkeydown,
            onblur,
            onfocus,
        }
    }
}
