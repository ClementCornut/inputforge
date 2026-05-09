//! `SettingsSection`: panel-scoped wrapper that renders a body container.
//!
//! Matches the Devices/Profiles convention of no in-panel title.

use dioxus::prelude::*;

#[component]
pub(crate) fn SettingsSection(children: Element) -> Element {
    rsx! {
        section { class: "if-settings-section",
            div { class: "if-settings-section__body", {children} }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::SettingsSection;

    fn Harness() -> Element {
        rsx! {
            SettingsSection {
                children: rsx! { p { "body content" } },
            }
        }
    }

    #[test]
    fn renders_body_inside_section_wrapper() {
        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("body content"), "expected body, got: {html}");
        assert!(
            html.contains("if-settings-section__body"),
            "expected body class, got: {html}"
        );
        assert!(!html.contains("<h3"), "must not render any heading");
    }
}
