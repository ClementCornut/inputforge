use dioxus::prelude::*;

use super::merge_class;
use crate::components::button::{ButtonSize, ButtonVariant};
use crate::components::icon_button::IconButton;
use crate::components::text_input::InputSize;
use crate::icons::Icon as IconKind;

/// Parse `raw` as `f64` and clamp to `[min, max]`. Returns `None` when the
/// text fails to parse (locale-aware parsing is out of scope; `0,5` is not
/// accepted). Shared by the production commit handlers and the unit tests.
fn parse_and_clamp(raw: &str, min: f64, max: f64) -> Option<f64> {
    let v: f64 = raw.parse().ok()?;
    Some(v.min(max).max(min))
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures. Mirrors the suppression \
              used in mapping_editor/header.rs."
)]
pub fn NumberInput(
    value: ReadSignal<f64>,
    oninput: Option<EventHandler<FormEvent>>,
    /// Emits the post-clamp value when the +/- stepper buttons are clicked.
    /// The native `<input type="number">` arrow keys still fire `oninput` instead.
    onstep: Option<EventHandler<f64>>,
    /// Emits the post-parse, post-clamp value when the user finishes editing
    /// (Enter pressed or input loses focus). Free-typing fires `oninput` only.
    oncommit: Option<EventHandler<f64>>,
    #[props(default = f64::NEG_INFINITY)] min: f64,
    #[props(default = f64::INFINITY)] max: f64,
    #[props(default = 1.0)] step: f64,
    /// Decimal places used to format `value` for display. `None` = native default.
    #[props(default)]
    precision: Option<usize>,
    #[props(default)] disabled: bool,
    /// HTML `id` for label↔input coupling when wrapped in `Field`.
    #[props(default)]
    id: Option<String>,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-number-input--sm",
        InputSize::Md => "if-number-input--md",
        InputSize::Lg => "if-number-input--lg",
    };
    let combined = merge_class("if-number-input", size_class, class.as_deref());
    let display_value = match precision {
        Some(p) => format!("{:.*}", p, value()),
        None => format!("{}", value()),
    };
    // Mirror the live input text into a Signal so `on_input_blur` can read it.
    // Dioxus 0.7's `FocusEvent` does not expose `.value()`, so we follow
    // the canonical Signal-mirror pattern from `header.rs:230-311`
    // (rename-inline). `use_signal` runs once per component instance; the
    // initial seed is the formatted `value` at first mount.
    let mut local_text = use_signal(|| display_value.clone());
    // Resync `local_text` whenever the formatted external value changes (drag,
    // keyboard nudge, sibling commit). Without this, blurring without typing
    // would re-commit the stale mount-time value and clobber the live state.
    // While the user is actively typing, `display_value` does not change (the
    // external signal is unchanged until commit), so this effect does not
    // overwrite typed text.
    let display_for_sync = display_value.clone();
    use_effect(use_reactive!(|display_for_sync| {
        local_text.set(display_for_sync);
    }));
    let input_handler = move |evt: FormEvent| {
        local_text.set(evt.value());
        if let Some(h) = &oninput {
            h.call(evt);
        }
    };
    let onstep_inc = onstep;
    let onstep_dec = onstep;
    let on_inc = move |_| {
        if let Some(h) = &onstep_inc {
            let next = (value() + step).min(max).max(min);
            h.call(next);
        }
    };
    let on_dec = move |_| {
        if let Some(h) = &onstep_dec {
            let next = (value() - step).min(max).max(min);
            h.call(next);
        }
    };
    // Enter commits via the canonical blur-via-JS path (mirrors
    // `header.rs:288-299` rename-inline). `KeyboardEvent` does not expose
    // the live `<input>` text in Dioxus 0.7, so we trigger `blur()` and let
    // `on_input_blur` do the parse, clamp, and dispatch.
    let on_input_keydown = move |evt: KeyboardEvent| {
        if evt.key() == Key::Enter {
            evt.prevent_default();
            let _ = document::eval(
                r"
                const el = document.activeElement;
                if (el && el instanceof HTMLInputElement) { el.blur(); }
                ",
            );
        }
    };
    // `EventHandler<f64>` is `Copy`; no clone needed when reused per branch.
    // Read the live text from `local_text` (written by `input_handler`) since
    // `FocusEvent` does not expose `.value()` in Dioxus 0.7.
    let on_input_blur = move |_evt: FocusEvent| {
        let Some(handler) = oncommit.as_ref() else {
            return;
        };
        let raw = local_text.peek().clone();
        if let Some(v) = parse_and_clamp(&raw, min, max) {
            handler.call(v);
        }
    };
    // HTML5 forbids id="", so render the attribute only when Some.
    rsx! {
        div { class: "{combined}",
            if let Some(ref id_val) = id {
                input {
                    r#type: "number",
                    class: "if-number-input__field",
                    id: "{id_val}",
                    value: "{display_value}",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    disabled,
                    oninput: input_handler,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                }
            } else {
                input {
                    r#type: "number",
                    class: "if-number-input__field",
                    value: "{display_value}",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    disabled,
                    oninput: input_handler,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                }
            }
            div {
                class: "if-number-input__steppers",
                IconButton { icon: IconKind::Plus,  label: "Increment", size: ButtonSize::Sm, variant: ButtonVariant::Ghost, disabled, onclick: on_inc }
                IconButton { icon: IconKind::Minus, label: "Decrement", size: ButtonSize::Sm, variant: ButtonVariant::Ghost, disabled, onclick: on_dec }
            }
        }
    }
}

#[cfg(test)]
mod oncommit_tests {
    use super::parse_and_clamp;

    #[test]
    fn enter_clamps_above_max() {
        assert_eq!(parse_and_clamp("1.5", -1.0, 1.0), Some(1.0));
    }

    #[test]
    fn blur_clamps_below_min() {
        assert_eq!(parse_and_clamp("-2.5", -1.0, 1.0), Some(-1.0));
    }

    #[test]
    fn invalid_text_returns_none() {
        assert_eq!(parse_and_clamp("abc", -1.0, 1.0), None);
    }

    #[test]
    fn comma_decimal_returns_none() {
        // Locale-aware parsing is out of F11 scope; comma decimals fail to parse.
        assert_eq!(parse_and_clamp("0,5", -1.0, 1.0), None);
    }
}
