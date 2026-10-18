//! Shared form regressions exercise native semantics and app-owned draft transport.
use super::{
    number_input::NumberInput, segmented_control::SegmentedControl, text_input::TextInput,
};
use dioxus::prelude::*;

pub(super) fn html(root: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn invalid_text_input_announces_native_invalid_state() {
    fn root() -> Element {
        rsx! { TextInput { value: "draft".to_owned(), invalid: true } }
    }
    let rendered = html(root);
    assert!(rendered.contains("aria-invalid=\"true\""), "{rendered}");
}

#[test]
fn segmented_control_owns_its_visual_namespace() {
    fn root() -> Element {
        rsx! { SegmentedControl { aria_label: "Behavior", "Hold" } }
    }
    let rendered = html(root);
    assert!(rendered.contains("if-segmented-control"), "{rendered}");
    assert!(!rendered.contains("if-stage__body-strategy"), "{rendered}");
}

#[test]
fn numeric_size_has_matching_styles() {
    fn root() -> Element {
        rsx! { NumberInput { value: 1.0 } }
    }
    let styles = include_str!("../../../assets/components/inputs/number-input.css");
    for size in ["sm", "md", "lg"] {
        assert!(
            styles.contains(&format!(".if-number-input--{size}")),
            "missing {size}"
        );
    }
    assert!(html(root).contains("if-number-input--md"));
}
