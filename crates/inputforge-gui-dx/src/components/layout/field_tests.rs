//! Field metadata is explicit and associated with the mounted native control.
use super::*;
use crate::components::TextInput;

fn rendered(root: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(root);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn field_mounts_only_active_helper_or_error_id() {
    fn root() -> Element {
        rsx! {
            Field { label: "Name", for_id: "name", helper: "Hint", helper_id: "name-help", error: "Rejected", error_id: "name-error", horizontal: true,
                TextInput { id: "name", value: "draft".to_owned(), invalid: true, aria_describedby: "name-error", aria_errormessage: "name-error" }
            }
        }
    }
    let html = rendered(root);
    assert!(html.contains("if-field--horizontal"), "{html}");
    assert!(html.contains("for=\"name\""), "{html}");
    assert!(html.contains("id=\"name-error\""), "{html}");
    assert!(!html.contains("id=\"name-help\""), "{html}");
    assert!(html.contains("aria-errormessage=\"name-error\""), "{html}");
}

#[test]
fn field_omits_ids_without_mounted_messages() {
    fn root() -> Element {
        rsx! { Field { label: "Name", helper_id: "unused-help", error_id: "unused-error", TextInput { value: String::new() } } }
    }
    let html = rendered(root);
    assert!(!html.contains("unused-help"), "{html}");
    assert!(!html.contains("unused-error"), "{html}");
    assert!(!html.contains("aria-describedby"), "{html}");
    assert!(!html.contains("aria-errormessage"), "{html}");
}
