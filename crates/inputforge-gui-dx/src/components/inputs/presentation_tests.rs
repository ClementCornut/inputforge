//! Capture and radio presentation preserves native semantics without owning workflows.
use super::{
    audit_tests::html,
    capture::{KeyCaptureButton, RebindField},
    inline_text_editor::InlineTextEditor,
    radio::{RadioGroup, RadioOption},
    text_input::TextInputVariant,
};
use dioxus::prelude::*;

#[test]
fn radio_details_are_outside_the_native_option_label() {
    fn root() -> Element {
        rsx! {
            RadioGroup { label: "Source",
                RadioOption { name: "source", value: "file", label: "Open file", checked: true,
                    description: "Pick a profile", details: rsx! { button { "Choose file" } }
                }
            }
        }
    }
    let rendered = html(root);
    assert!(rendered.contains("<fieldset"), "{rendered}");
    assert!(rendered.contains("<legend"), "{rendered}");
    assert!(rendered.contains("type=\"radio\""), "{rendered}");
    assert!(
        rendered.find("</label>").unwrap() < rendered.find("<button").unwrap(),
        "{rendered}"
    );
}

#[test]
fn capture_presentations_announce_listening_and_preserve_dom_hooks() {
    fn root() -> Element {
        rsx! {
            RebindField { source_label: "Unbound", unbound: true, listening: true,
                action_label: "Rebind mapping input", cancel_label: "Cancel mapping input capture", prompt: "Move input", onclick: |_| {}, oncancel: |_| {} }
            KeyCaptureButton { value: "Press keys", aria_label: "Capture physical key", listening: true, hint: "Escape cancels", autofocus: true, data_key_capture: "active", onclick: |_| {} }
        }
    }
    let rendered = html(root);
    for expected in [
        "if-rebind-field--listening",
        "if-rebind-field--unbound",
        "role=\"status\"",
        "aria-live=\"polite\"",
        "Cancel mapping input capture",
        "data-key-capture=\"active\"",
        "autofocus",
        "Escape cancels",
    ] {
        assert!(
            rendered.contains(expected),
            "missing {expected}: {rendered}"
        );
    }
}

#[test]
fn inline_text_editor_forwards_name_and_title_variant() {
    fn root() -> Element {
        rsx! { InlineTextEditor { value: "Title".to_owned(), aria_label: "Edit mapping name", data_editor_focus: "true", variant: TextInputVariant::Title } }
    }
    let rendered = html(root);
    for expected in [
        "aria-label=\"Edit mapping name\"",
        "data-editor-focus=\"true\"",
        "if-text-input--title",
        "data-if-text-owner",
    ] {
        assert!(rendered.contains(expected), "{rendered}");
    }
}
