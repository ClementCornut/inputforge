use dioxus::prelude::*;

use super::merge_class;

#[component]
pub fn Switch(
    checked: ReadSignal<bool>,
    onchange: Option<EventHandler<FormEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] id: Option<String>,
    #[props(default)] label: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-switch", "", class.as_deref());
    let change_handler = move |evt: FormEvent| {
        if let Some(h) = &onchange {
            h.call(evt);
        }
    };
    // HTML5 forbids id="", so render the attribute only when Some. Mirrors
    // the conditional-render pattern in `integer_input.rs`.
    rsx! {
        label { class: "{combined}",
            if let Some(ref id_val) = id {
                input {
                    r#type: "checkbox",
                    class: "if-switch__input",
                    id: "{id_val}",
                    checked: "{checked}",
                    disabled,
                    onchange: change_handler,
                }
            } else {
                input {
                    r#type: "checkbox",
                    class: "if-switch__input",
                    checked: "{checked}",
                    disabled,
                    onchange: change_handler,
                }
            }
            span { class: "if-switch__track", span { class: "if-switch__thumb" } }
            if let Some(l) = label.as_deref() {
                span { class: "if-switch__label", "{l}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::Switch;

    fn HarnessWithId() -> Element {
        rsx! {
            Switch {
                checked: false,
                id: Some("switch-under-test".to_owned()),
            }
        }
    }

    fn HarnessWithoutId() -> Element {
        rsx! {
            Switch { checked: false }
        }
    }

    #[test]
    fn renders_id_when_set() {
        let mut vdom = VirtualDom::new(HarnessWithId);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains(r#"id="switch-under-test""#),
            "expected id on input: {html}"
        );
    }

    #[test]
    fn omits_id_when_unset() {
        let mut vdom = VirtualDom::new(HarnessWithoutId);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains(" id=\""),
            "no input should carry an id when prop is None: {html}"
        );
    }
}
