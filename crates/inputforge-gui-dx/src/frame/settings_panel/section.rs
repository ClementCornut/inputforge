//! `SettingsSection`: heading + body. Panel-scoped primitive for F15.

use dioxus::prelude::*;

#[component]
pub(crate) fn SettingsSection(heading: String, children: Element) -> Element {
    rsx! {
        section { class: "if-settings-section",
            h3 { class: "if-settings-section__heading", "{heading}" }
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
                heading: "Snapshots".to_owned(),
                children: rsx! { p { "body content" } },
            }
        }
    }

    #[test]
    fn renders_h3_heading_and_body() {
        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(html.contains("<h3"), "expected h3, got: {html}");
        assert!(
            html.contains("Snapshots"),
            "expected heading text, got: {html}"
        );
        assert!(html.contains("body content"), "expected body, got: {html}");
        assert!(!html.contains("<h2"), "must not promote to h2");
    }
}
