use dioxus::prelude::*;

use super::merge_class;

/// Groups mutually exclusive pill options.
#[component]
pub fn SegmentedControl(
    aria_label: String,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let combined = merge_class("if-stage__body-strategy", "", class.as_deref());
    rsx! {
        div {
            class: "{combined}",
            role: "group",
            "aria-label": "{aria_label}",
            {children}
        }
    }
}

/// Renders one option in a [`SegmentedControl`].
#[component]
pub fn SegmentedControlOption(
    value: String,
    selected: bool,
    #[props(default)] disabled: bool,
    #[props(default)] tabindex: Option<String>,
    #[props(default)] data_strategy: Option<String>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let aria_pressed = if selected { "true" } else { "false" };
    let aria_disabled = if disabled { "true" } else { "false" };
    let tabindex_value = tabindex.unwrap_or_else(|| {
        if disabled {
            "-1".to_owned()
        } else {
            "0".to_owned()
        }
    });
    let click_handler = move |evt: MouseEvent| {
        if !disabled && let Some(handler) = &onclick {
            handler.call(evt);
        }
    };

    if let Some(strategy) = data_strategy {
        rsx! {
            button {
                r#type: "button",
                class: "if-stage__body-strategy-pill",
                "data-value": "{value}",
                "data-strategy": "{strategy}",
                "aria-pressed": "{aria_pressed}",
                "aria-disabled": "{aria_disabled}",
                tabindex: "{tabindex_value}",
                onclick: click_handler,
                {children}
            }
        }
    } else {
        rsx! {
            button {
                r#type: "button",
                class: "if-stage__body-strategy-pill",
                "data-value": "{value}",
                "aria-pressed": "{aria_pressed}",
                "aria-disabled": "{aria_disabled}",
                tabindex: "{tabindex_value}",
                onclick: click_handler,
                {children}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_ssr::render;

    #[test]
    fn segmented_control_renders_group_label_selected_and_disabled_options() {
        #[expect(
            non_snake_case,
            reason = "Dioxus components are PascalCase by convention"
        )]
        fn Harness() -> Element {
            rsx! {
                SegmentedControl { aria_label: "Behavior".to_owned(),
                    SegmentedControlOption {
                        value: "hold".to_owned(),
                        selected: true,
                        onclick: move |_| {},
                        "Hold"
                    }
                    SegmentedControlOption {
                        value: "pulse".to_owned(),
                        selected: false,
                        disabled: true,
                        onclick: move |_| {},
                        "Pulse"
                    }
                }
            }
        }

        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        assert!(
            html.contains(r#"role="group""#),
            "group role missing: {html}"
        );
        assert!(
            html.contains(r#"aria-label="Behavior""#),
            "accessible group label missing: {html}"
        );
        let hold_idx = html
            .find(r#"data-value="hold""#)
            .expect("hold option must render");
        let hold_end = (hold_idx + 220).min(html.len());
        let hold_slice = &html[hold_idx..hold_end];
        assert!(
            hold_slice.contains(r#"aria-pressed="true""#),
            "selected option must be pressed: {hold_slice}"
        );

        let pulse_idx = html
            .find(r#"data-value="pulse""#)
            .expect("pulse option must render");
        let pulse_end = (pulse_idx + 220).min(html.len());
        let pulse_slice = &html[pulse_idx..pulse_end];
        assert!(
            pulse_slice.contains(r#"aria-disabled="true""#),
            "disabled option must carry aria-disabled: {pulse_slice}"
        );
        assert!(
            pulse_slice.contains(r#"tabindex="-1""#),
            "disabled option must leave tab order: {pulse_slice}"
        );
    }
}
