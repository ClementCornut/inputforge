use dioxus::prelude::*;

use super::merge_class;
use crate::components::Icon;
use crate::components::text_input::InputSize;
use crate::icons::{Icon as IconKind, IconSize};

/// One option in a [`Select`]. `disabled` and `class` are per-option so a
/// surface (e.g. F14's stage-mode dropdown) can render an orphaned reference
/// as a disabled, error-tinted option without forking the primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
    pub class: Option<String>,
}

#[component]
pub fn Select(
    value: ReadSignal<String>,
    onchange: Option<EventHandler<FormEvent>>,
    options: Vec<SelectOption>,
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
                    for opt in options.iter() {
                        {render_option(opt, &selected_value)}
                    }
                }
            } else {
                select {
                    class: "{combined}",
                    value: "{selected_value}",
                    disabled,
                    onchange: change_handler,
                    for opt in options.iter() {
                        {render_option(opt, &selected_value)}
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

/// Render one `<option>`. Skips the `class` attribute entirely when the
/// option has no class so the rendered HTML stays free of an empty
/// `class=""`. The conditional branches also keep `disabled=true` from
/// leaking onto non-orphan options when SSR concatenates adjacent slices
/// for substring assertions.
fn render_option(opt: &SelectOption, selected_value: &str) -> Element {
    let value = opt.value.clone();
    let label = opt.label.clone();
    let is_selected = opt.value == selected_value;
    let is_disabled = opt.disabled;
    if let Some(ref class) = opt.class {
        rsx! {
            option {
                value: "{value}",
                selected: is_selected,
                disabled: is_disabled,
                class: "{class}",
                "{label}"
            }
        }
    } else {
        rsx! {
            option {
                value: "{value}",
                selected: is_selected,
                disabled: is_disabled,
                "{label}"
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
                        SelectOption { value: "a".into(), label: "A".into(), disabled: false, class: None },
                        SelectOption { value: "b".into(), label: "B".into(), disabled: false, class: None },
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

    #[test]
    fn select_renders_per_option_disabled_and_class() {
        #[expect(
            non_snake_case,
            reason = "Dioxus components are PascalCase by convention"
        )]
        fn Harness() -> Element {
            let value = use_signal(|| "live".to_owned());
            let value_ro: ReadSignal<String> = value.into();
            rsx! {
                Select {
                    value: value_ro,
                    onchange: move |_| {},
                    options: vec![
                        SelectOption {
                            value: "live".into(),
                            label: "Combat".into(),
                            disabled: false,
                            class: None,
                        },
                        SelectOption {
                            value: "ghost".into(),
                            label: "ghostly mode".into(),
                            disabled: true,
                            class: Some("if-select__option--orphan".into()),
                        },
                    ],
                }
            }
        }

        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        let ghost_idx = html
            .find(r#"<option value="ghost""#)
            .expect("orphan option must render");
        let ghost_slice = &html[ghost_idx..ghost_idx + 200];
        assert!(
            ghost_slice.contains("disabled=true"),
            "orphan option must carry disabled=true: {ghost_slice}"
        );
        assert!(
            ghost_slice.contains("if-select__option--orphan"),
            "orphan option must carry its class: {ghost_slice}"
        );

        let live_idx = html
            .find(r#"<option value="live""#)
            .expect("non-orphan option must render");
        let live_close_offset = html[live_idx..]
            .find("</option>")
            .expect("live option must close");
        let live_slice = &html[live_idx..live_idx + live_close_offset];
        assert!(
            !live_slice.contains("disabled=true"),
            "non-orphan option must not carry disabled=true: {live_slice}"
        );
    }
}
