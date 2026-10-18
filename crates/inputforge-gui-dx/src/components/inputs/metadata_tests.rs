//! Form metadata belongs to the focusable native control.
use super::audit_tests::html;
use super::{
    checkbox::Checkbox, integer_input::IntegerInput, number_input::NumberInput, select::Select,
    slider::Slider, switch::Switch, text_input::TextInput,
};
use super::{native_number::NativeNumberInput, radio::RadioOption};
use dioxus::prelude::*;

#[test]
fn every_control_forwards_metadata_to_its_native_leaf() {
    fn root() -> Element {
        rsx! {
            NativeNumberInput { value: "1", aria_label: "Native number", aria_labelledby: "label", aria_describedby: "help", aria_errormessage: "error", invalid: true }
            RadioOption { name: "source", value: "file", label: "File", checked: true, aria_label: "Native radio", aria_labelledby: "label", aria_describedby: "help", aria_errormessage: "error", invalid: true }
            TextInput { value: "draft".to_owned(), aria_label: "Text", aria_labelledby: "label", aria_describedby: "help", aria_errormessage: "error", invalid: true }
            IntegerInput { value: 1usize, min: 0, max: 2, aria_label: "Integer", aria_labelledby: "label", aria_describedby: "help", aria_errormessage: "error", invalid: true }
            NumberInput { value: 1.0, aria_label: "Number", aria_labelledby: "label", aria_describedby: "help", aria_errormessage: "error", invalid: true }
            Select { value: "a".to_owned(), options: vec![], aria_label: "Select", aria_labelledby: "label", aria_describedby: "help", aria_errormessage: "error", invalid: true }
            Slider { value: 0.5, aria_label: "Slider", aria_labelledby: "label", aria_describedby: "help", aria_errormessage: "error", invalid: true }
            Checkbox { checked: false, aria_label: "Checkbox", aria_labelledby: "label", aria_describedby: "help", aria_errormessage: "error", invalid: true }
            Switch { checked: false, aria_label: "Switch", aria_labelledby: "label", aria_describedby: "help", aria_errormessage: "error", invalid: true }
        }
    }
    let rendered = html(root);
    let leaves: Vec<_> = rendered
        .split('<')
        .filter(|part| part.starts_with("input ") || part.starts_with("select "))
        .collect();
    assert_eq!(leaves.len(), 9, "{rendered}");
    for leaf in leaves {
        let attributes = leaf.split('>').next().unwrap();
        for expected in [
            "aria-label=",
            "aria-labelledby=\"label\"",
            "aria-describedby=\"help\"",
            "aria-errormessage=\"error\"",
            "aria-invalid=\"true\"",
        ] {
            assert!(
                attributes.contains(expected),
                "missing {expected}: {attributes}"
            );
        }
    }
}

#[test]
fn optional_metadata_omits_idrefs_and_error_relation_when_valid() {
    fn root() -> Element {
        rsx! { TextInput { value: String::new(), aria_errormessage: "not-mounted" } }
    }
    let rendered = html(root);
    for absent in [
        "aria-labelledby",
        "aria-describedby",
        "aria-errormessage",
        " id=",
    ] {
        assert!(!rendered.contains(absent), "{rendered}");
    }
}

#[test]
fn checkbox_label_content_and_external_label_are_not_nested() {
    fn root() -> Element {
        rsx! {
            Checkbox { checked: false, "Flight stick (disconnected)" }
            div {
                crate::components::Label { for_id: "external", "Axis X" }
                Checkbox { id: "external", checked: true }
            }
        }
    }
    let rendered = html(root);
    assert!(
        rendered.contains("Flight stick (disconnected)"),
        "{rendered}"
    );
    assert!(rendered.contains("for=\"external\""), "{rendered}");
    let mut depth = 0;
    for tag in rendered.split('<') {
        if tag.starts_with("label ") {
            depth += 1;
            assert_eq!(depth, 1, "{rendered}");
        }
        if tag.starts_with("/label>") {
            depth -= 1;
        }
    }
    assert_eq!(depth, 0);
}
