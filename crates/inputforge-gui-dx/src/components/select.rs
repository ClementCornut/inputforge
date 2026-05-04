use dioxus::prelude::*;

use super::merge_class;
use crate::components::Icon;
use crate::components::text_input::InputSize;
use crate::icons::{Icon as IconKind, IconSize};

#[component]
pub fn Select(
    value: ReadSignal<String>,
    onchange: Option<EventHandler<FormEvent>>,
    options: Vec<(String, String)>, // (value, label)
    #[props(default)] disabled: bool,
    /// HTML `id` for label↔input coupling when wrapped in `Field`.
    #[props(default)]
    id: Option<String>,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-select--sm",
        InputSize::Md => "if-select--md",
        InputSize::Lg => "if-select--lg",
    };
    let combined = merge_class("if-select", size_class, class.as_deref());
    let selected_value = value.read().clone();
    let change_handler = move |evt: FormEvent| {
        if let Some(h) = &onchange {
            h.call(evt);
        }
    };
    // Wrapper provides the positioning context for the chevron overlay.
    // The native UA chevron is suppressed via `appearance: none` in CSS;
    // we draw our own with the shared Phosphor ChevronDown icon so it
    // tracks --color-text-muted via currentColor like every other icon
    // in the app.
    rsx! {
        span { class: "if-select-wrapper",
            if let Some(ref id_val) = id {
                select {
                    class: "{combined}",
                    id: "{id_val}",
                    value: "{selected_value}",
                    disabled,
                    onchange: change_handler,
                    for (val, label) in options.iter() {
                        option { value: "{val}", selected: *val == selected_value, "{label}" }
                    }
                }
            } else {
                select {
                    class: "{combined}",
                    value: "{selected_value}",
                    disabled,
                    onchange: change_handler,
                    for (val, label) in options.iter() {
                        option { value: "{val}", selected: *val == selected_value, "{label}" }
                    }
                }
            }
            Icon {
                name: IconKind::ChevronDown,
                size: IconSize::Sm,
                class: "if-select-wrapper__chevron".to_owned(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    #[test]
    fn select_marks_matching_option_selected() {
        #[expect(
            non_snake_case,
            reason = "Dioxus components are PascalCase by convention"
        )]
        fn Harness() -> Element {
            let value = use_signal(|| "b".to_owned());
            let value_ro: ReadSignal<String> = value.into();
            rsx! {
                Select {
                    value: value_ro,
                    onchange: move |_| {},
                    options: vec![
                        ("a".to_owned(), "A".to_owned()),
                        ("b".to_owned(), "B".to_owned()),
                    ],
                }
            }
        }

        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        assert!(
            html.contains(r#"<option value="b" selected=true>B</option>"#),
            "matching option should be selected: {html}"
        );
    }
}
