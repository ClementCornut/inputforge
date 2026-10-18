use dioxus::prelude::*;

use crate::components::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadgeVariant {
    Neutral,
    Info,
    Success,
    Warning,
    Error,
}

#[component]
pub fn Badge(
    #[props(default = BadgeVariant::Neutral)] variant: BadgeVariant,
    #[props(default)] class: Option<String>,
    #[props(default)] title: Option<String>,
    /// Monospaced count presentation; announcement remains explicit.
    #[props(default)]
    count: bool,
    #[props(default)] aria_live: Option<String>,
    #[props(default)] aria_label: Option<String>,
    children: Element,
) -> Element {
    let v = match variant {
        BadgeVariant::Neutral => "if-badge--neutral",
        BadgeVariant::Info => "if-badge--info",
        BadgeVariant::Success => "if-badge--success",
        BadgeVariant::Warning => "if-badge--warning",
        BadgeVariant::Error => "if-badge--error",
    };
    let variants = if count {
        format!("{v} if-badge--count")
    } else {
        v.to_owned()
    };
    let combined = merge_class("if-badge", &variants, class.as_deref());
    rsx! {
        span {
            class: "{combined}",
            title: title.as_deref(),
            "aria-live": aria_live,
            "aria-label": aria_label,
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::{Badge, BadgeVariant};

    #[test]
    fn badge_renders_title_when_provided() {
        fn TestComponent() -> Element {
            rsx! {
                Badge {
                    variant: BadgeVariant::Warning,
                    title: "tooltip text".to_owned(),
                    "Label"
                }
            }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("title=\"tooltip text\""),
            "Badge must forward title prop to span: {html}",
        );
    }

    #[test]
    fn badge_omits_title_attribute_when_absent() {
        fn TestComponent() -> Element {
            rsx! { Badge { variant: BadgeVariant::Warning, "Label" } }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains("title="),
            "Badge must not emit a title attribute when prop is None: {html}",
        );
    }
}
