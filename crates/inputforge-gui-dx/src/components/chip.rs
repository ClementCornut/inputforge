//! Chip primitive. Three variants (Outline, Output, Capture) cover the
//! rail's chip-like surfaces (device filter idle, qualifier, vJoy output,
//! capture chip). Status semantics live on Badge; classification chips
//! with mono fonts and per-kind hues live here. See DESIGN.md section 7
//! for the Badge vs Chip split.

use dioxus::prelude::*;

use super::merge_class;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChipVariant {
    /// Transparent fill, --color-border-strong border, --color-text-muted
    /// label. Used by device-chip idle and row qualifier chips.
    Outline,
    /// --color-output label, --font-mono, faint output-tinted surface.
    /// Used by the row's vJoy out chip.
    Output,
    /// kind-tinted via data-kind="axis|button|hat", mono. Used by the
    /// add-inline pad's input identifier chip.
    Capture,
}

#[component]
pub fn Chip(
    #[props(default = ChipVariant::Outline)] variant: ChipVariant,
    #[props(default)] class: Option<String>,
    #[props(default)] title: Option<String>,
    children: Element,
) -> Element {
    let v = match variant {
        ChipVariant::Outline => "if-chip--outline",
        ChipVariant::Output => "if-chip--output",
        ChipVariant::Capture => "if-chip--capture",
    };
    let combined = merge_class("if-chip", v, class.as_deref());
    rsx! {
        span {
            class: "{combined}",
            title: title.as_deref(),
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::{Chip, ChipVariant};

    fn render_with(variant: ChipVariant) -> String {
        fn make(variant: ChipVariant) -> Element {
            rsx! { Chip { variant: variant, "x" } }
        }
        let mut vdom = VirtualDom::new_with_props(make, variant);
        vdom.rebuild_in_place();
        render(&vdom)
    }

    #[test]
    fn chip_outline_variant_emits_outline_class() {
        let html = render_with(ChipVariant::Outline);
        assert!(html.contains("if-chip"), "base class missing: {html}");
        assert!(
            html.contains("if-chip--outline"),
            "outline variant class missing: {html}",
        );
    }

    #[test]
    fn chip_renders_title_when_provided() {
        fn TestComponent() -> Element {
            rsx! {
                Chip {
                    variant: ChipVariant::Outline,
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
            "Chip must forward title prop to span: {html}",
        );
    }

    #[test]
    fn chip_omits_title_attribute_when_absent() {
        fn TestComponent() -> Element {
            rsx! { Chip { variant: ChipVariant::Outline, "Label" } }
        }
        let mut vdom = VirtualDom::new(TestComponent);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            !html.contains("title="),
            "Chip must not emit a title attribute when prop is None: {html}",
        );
    }

    #[test]
    fn chip_output_variant_emits_output_class_and_css_rule() {
        let html = render_with(ChipVariant::Output);
        assert!(
            html.contains("if-chip--output"),
            "output variant class missing: {html}",
        );
        let css = include_str!("../../assets/components/chip.css");
        assert!(
            css.contains(".if-chip--output"),
            "Chip Output CSS rule must land in chip.css: {css}",
        );
    }
}
