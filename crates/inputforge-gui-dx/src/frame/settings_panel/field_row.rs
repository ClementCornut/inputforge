//! `SettingsFieldRow`: label + helper + control + ARIA wiring.
//!
//! Owns `<label for="...">`, the helper-text id, `aria-describedby`, and
//! (when `error` is set) `aria-invalid` + `aria-errormessage`. Wrapped
//! controls do not need their own a11y props; the row threads ids through
//! the slotted `control` element via the `control_id` prop.

use dioxus::prelude::*;

#[component]
pub(crate) fn SettingsFieldRow(
    /// Visible label.
    label: String,
    /// Helper text rendered below the control. May be replaced by the
    /// validation error when `error` is `Some`.
    helper: String,
    /// HTML id used as `<label for="...">` and as the control's `id`. The
    /// caller must set the same id on the control inside `control`.
    control_id: String,
    /// Inline validation error replacing the helper when set.
    #[props(default)]
    error: Option<String>,
    control: Element,
) -> Element {
    let helper_id = format!("{control_id}__helper");
    let error_id = format!("{control_id}__error");

    let helper_text = error.clone().unwrap_or_else(|| helper.clone());
    let is_invalid = error.is_some();
    let aria_describedby = if is_invalid {
        format!("{helper_id} {error_id}")
    } else {
        helper_id.clone()
    };

    rsx! {
        div { class: "if-settings-field-row",
            "data-invalid": "{is_invalid}",
            label {
                class: "if-settings-field-row__label",
                r#for: "{control_id}",
                "{label}"
            }
            div {
                class: "if-settings-field-row__control",
                "aria-describedby": "{aria_describedby}",
                "aria-invalid": if is_invalid { "true" } else { "false" },
                "aria-errormessage": if is_invalid { error_id.clone() } else { String::new() },
                {control}
            }
            p {
                id: "{helper_id}",
                class: "if-settings-field-row__helper",
                "data-error": "{is_invalid}",
                "{helper_text}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "Dioxus components are PascalCase")]

    use dioxus::prelude::*;
    use dioxus_ssr::render;

    use super::SettingsFieldRow;

    fn Harness() -> Element {
        rsx! {
            SettingsFieldRow {
                label: "Label".to_owned(),
                helper: "Helper text".to_owned(),
                control_id: "test-control".to_owned(),
                control: rsx! { input { id: "test-control" } },
            }
        }
    }

    fn HarnessWithError() -> Element {
        rsx! {
            SettingsFieldRow {
                label: "Label".to_owned(),
                helper: "Helper text".to_owned(),
                control_id: "test-control".to_owned(),
                error: Some("Must be between 1 and 100".to_owned()),
                control: rsx! { input { id: "test-control" } },
            }
        }
    }

    #[test]
    fn renders_label_helper_and_links_control_id() {
        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains(r#"for="test-control""#),
            "expected label.for: {html}"
        );
        assert!(html.contains("Helper text"), "expected helper text: {html}");
        assert!(
            html.contains(r#"aria-describedby="test-control__helper""#),
            "expected aria-describedby: {html}"
        );
        assert!(
            html.contains(r#"aria-invalid="false""#),
            "default invalid=false: {html}"
        );
    }

    #[test]
    fn error_replaces_helper_and_sets_invalid_attrs() {
        let mut vdom = VirtualDom::new(HarnessWithError);
        vdom.rebuild_in_place();
        let html = render(&vdom);
        assert!(
            html.contains("Must be between 1 and 100"),
            "expected error message: {html}"
        );
        assert!(
            !html.contains("Helper text"),
            "helper must be replaced: {html}"
        );
        assert!(
            html.contains(r#"aria-invalid="true""#),
            "expected invalid=true: {html}"
        );
        assert!(
            html.contains(r#"aria-errormessage="test-control__error""#),
            "expected errormessage: {html}"
        );
    }
}
