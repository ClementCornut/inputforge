//! `SettingsSection`: optional heading + body. Panel-scoped primitive for F15.
//!
//! When `heading` is `None` the section renders only its body, matching the
//! Devices/Profiles convention of no in-panel title. Future sections may opt
//! in to a heading once the panel hosts more than one.

use dioxus::prelude::*;

#[component]
pub(crate) fn SettingsSection(
    #[props(default)] heading: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        section { class: "if-settings-section",
            if let Some(heading_text) = heading {
                h3 { class: "if-settings-section__heading", "{heading_text}" }
            }
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

    fn HarnessWithHeading() -> Element {
        rsx! {
            SettingsSection {
                heading: "Snapshots".to_owned(),
                children: rsx! { p { "body content" } },
            }
        }
    }

    fn HarnessNoHeading() -> Element {
        rsx! {
            SettingsSection {
                children: rsx! { p { "body content" } },
            }
        }
    }

    #[test]
    fn renders_h3_heading_and_body_when_heading_provided() {
        let mut vdom = VirtualDom::new(HarnessWithHeading);
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

    #[test]
    fn omits_heading_when_none() {
        let mut vdom = VirtualDom::new(HarnessNoHeading);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains("<h3"),
            "must not render h3 when heading is None: {html}"
        );
        assert!(html.contains("body content"), "expected body, got: {html}");
    }
}
