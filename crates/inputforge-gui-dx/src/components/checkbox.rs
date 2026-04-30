use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use super::merge_class;

/// Per-component-instance ID counter. Backs the auto-generated `id` so the
/// indeterminate-IDL eval can address a stable element when no caller `id`
/// is supplied. Process-local; restart-stable IDs aren't required.
static CHECKBOX_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[component]
pub fn Checkbox(
    checked: ReadSignal<bool>,
    onchange: Option<EventHandler<FormEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] indeterminate: bool,
    /// HTML `id` for label↔input coupling when wrapped in `Field`.
    /// If omitted, an internal stable ID is generated so the indeterminate
    /// IDL property can still be applied to the underlying `<input>`.
    #[props(default)]
    id: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let variant_class = if indeterminate {
        "if-checkbox--indeterminate"
    } else {
        ""
    };
    let combined = merge_class("if-checkbox", variant_class, class.as_deref());

    let internal_id: String = use_hook(|| {
        format!(
            "if-checkbox-{}",
            CHECKBOX_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });
    let effective_id = id.clone().unwrap_or(internal_id);

    // Mirror the non-Signal `indeterminate` prop into a Signal so use_effect
    // can react to changes. Without this, the CSS class would update but the
    // DOM .indeterminate IDL property would never sync, :indeterminate would
    // never match and assistive tech would announce "checkbox, not checked"
    // instead of "mixed".
    let mut indet_signal = use_signal(|| indeterminate);
    if *indet_signal.peek() != indeterminate {
        indet_signal.set(indeterminate);
    }

    let target_id = effective_id.clone();
    use_effect(move || {
        let val = *indet_signal.read();
        let _ = document::eval(&format!(
            "var el = document.getElementById('{target_id}'); if (el) el.indeterminate = {val};"
        ));
    });

    let aria_checked: &str = if indeterminate {
        "mixed"
    } else if *checked.read() {
        "true"
    } else {
        "false"
    };

    let change_handler = move |evt: FormEvent| {
        if let Some(h) = &onchange {
            h.call(evt);
        }
    };
    rsx! {
        label { class: "{combined}",
            input {
                r#type: "checkbox",
                class: "if-checkbox__input",
                id: "{effective_id}",
                "aria-checked": "{aria_checked}",
                checked: "{checked}",
                disabled,
                onchange: change_handler,
            }
            span { class: "if-checkbox__box" }
        }
    }
}
