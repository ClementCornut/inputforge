use dioxus::prelude::*;

use super::merge_class;
use crate::components::text_input::InputSize;

/// Reason a committed value was rejected. The consumer surfaces an inline
/// validation message and blocks the dispatch on the `Err` branch.
///
/// Locale-aware parsing is out of scope; "1,000" is not accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegerInputError {
    Empty,
    NotANumber,
    OutOfRange { min: usize, max: usize },
}

/// Parse `raw` as `usize` and confirm it lies in `[min, max]`.
///
/// Returns `Ok(value)` on success. On `Err`, the caller surfaces an inline
/// validation message and blocks the dispatch; on `Ok`, the value is
/// forwarded to `oncommit`.
fn parse_and_validate(raw: &str, min: usize, max: usize) -> Result<usize, IntegerInputError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(IntegerInputError::Empty);
    }
    // `usize::from_str` rejects negative strings and floats, so "-5" and
    // "3.14" both map to `NotANumber` without any special-casing.
    let v: usize = trimmed.parse().map_err(|_| IntegerInputError::NotANumber)?;
    if !(min..=max).contains(&v) {
        return Err(IntegerInputError::OutOfRange { min, max });
    }
    Ok(v)
}

#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on per-element \
              event listeners; mirrors number_input.rs"
)]
pub fn IntegerInput(
    value: ReadSignal<usize>,
    min: usize,
    max: usize,
    /// Emits the parsed value when it lies in `[min, max]` after Enter or blur.
    /// In-flight typing fires `oninput` only.
    oncommit: Option<EventHandler<usize>>,
    /// Fires on Enter or blur when the buffer is empty, unparseable, or
    /// out of range. The consumer surfaces an inline validation message
    /// and blocks the dispatch.
    oninvalid: Option<EventHandler<IntegerInputError>>,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default)] disabled: bool,
    #[props(default)] id: Option<String>,
    #[props(default = InputSize::Md)] size: InputSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let size_class = match size {
        InputSize::Sm => "if-integer-input--sm",
        InputSize::Md => "if-integer-input--md",
        InputSize::Lg => "if-integer-input--lg",
    };
    let combined = merge_class("if-integer-input", size_class, class.as_deref());
    let display_value = format!("{}", value());

    // Mirror the live input text into a Signal so `on_input_blur` can read it.
    // `FocusEvent` does not expose `.value()` in Dioxus 0.7, so the
    // Signal-mirror pattern from `number_input.rs` applies here unchanged.
    let mut local_text = use_signal(|| display_value.clone());

    // Resync `local_text` whenever the external value changes (sibling commit,
    // undo, programmatic reset). While the user is actively typing, the
    // external signal is unchanged so this effect does not overwrite typed text.
    let display_for_sync = display_value.clone();
    use_effect(use_reactive!(|display_for_sync| {
        local_text.set(display_for_sync);
    }));

    // Escape rewrites local_text to the polled value, then blurs. Without
    // suppression, the subsequent blur would parse the polled value and fire
    // a redundant `oncommit`. The flag is consumed by the next blur.
    let mut suppress_next_commit = use_signal(|| false);

    let input_handler = move |evt: FormEvent| {
        local_text.set(evt.value());
        if let Some(h) = &oninput {
            h.call(evt);
        }
    };

    // Enter commits via the canonical blur-via-JS path. `KeyboardEvent` does
    // not expose the live `<input>` text in Dioxus 0.7, so we trigger `blur()`
    // and let `on_input_blur` do the parse, validate, and dispatch.
    // Escape reverts the text to the last committed value, then blurs without
    // firing `oncommit` (suppress flag consumed by the blur handler).
    let on_input_keydown = move |evt: KeyboardEvent| {
        if evt.key() == Key::Enter {
            evt.prevent_default();
            let _ = document::eval(
                r"
                const el = document.activeElement;
                if (el && el instanceof HTMLInputElement) { el.blur(); }
                ",
            );
        } else if evt.key() == Key::Escape {
            local_text.set(format!("{}", value()));
            suppress_next_commit.set(true);
            evt.prevent_default();
            let _ = document::eval(
                r"
                const el = document.activeElement;
                if (el && el instanceof HTMLInputElement) { el.blur(); }
                ",
            );
        }
    };

    let on_input_blur = move |_evt: FocusEvent| {
        if suppress_next_commit() {
            suppress_next_commit.set(false);
            return;
        }
        let raw = local_text.peek().clone();
        match parse_and_validate(&raw, min, max) {
            Ok(v) => {
                if let Some(handler) = oncommit.as_ref() {
                    handler.call(v);
                }
            }
            Err(e) => {
                if let Some(handler) = oninvalid.as_ref() {
                    handler.call(e);
                }
            }
        }
    };

    // HTML5 forbids id="", so render the attribute only when Some.
    rsx! {
        div { class: "{combined}",
            if let Some(ref id_val) = id {
                input {
                    r#type: "number",
                    inputmode: "numeric",
                    class: "if-integer-input__field",
                    id: "{id_val}",
                    value: "{display_value}",
                    min: "{min}",
                    max: "{max}",
                    step: "1",
                    disabled,
                    oninput: input_handler,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                }
            } else {
                input {
                    r#type: "number",
                    inputmode: "numeric",
                    class: "if-integer-input__field",
                    value: "{display_value}",
                    min: "{min}",
                    max: "{max}",
                    step: "1",
                    disabled,
                    oninput: input_handler,
                    onkeydown: on_input_keydown,
                    onblur: on_input_blur,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IntegerInputError, parse_and_validate};

    #[test]
    fn in_range_returns_value() {
        assert_eq!(parse_and_validate("42", 1, 100), Ok(42));
        assert_eq!(parse_and_validate("1", 1, 100), Ok(1));
        assert_eq!(parse_and_validate("100", 1, 100), Ok(100));
    }

    #[test]
    fn above_max_is_out_of_range() {
        assert_eq!(
            parse_and_validate("200", 1, 100),
            Err(IntegerInputError::OutOfRange { min: 1, max: 100 })
        );
    }

    #[test]
    fn below_min_is_out_of_range() {
        assert_eq!(
            parse_and_validate("0", 1, 100),
            Err(IntegerInputError::OutOfRange { min: 1, max: 100 })
        );
    }

    #[test]
    fn non_numeric_is_not_a_number() {
        assert_eq!(
            parse_and_validate("abc", 1, 100),
            Err(IntegerInputError::NotANumber)
        );
    }

    #[test]
    fn empty_is_empty_error() {
        assert_eq!(
            parse_and_validate("", 1, 100),
            Err(IntegerInputError::Empty)
        );
        assert_eq!(
            parse_and_validate("   ", 1, 100),
            Err(IntegerInputError::Empty)
        );
    }

    #[test]
    fn negative_is_not_a_number() {
        // usize cannot represent negatives; parse returns Err -> NotANumber.
        assert_eq!(
            parse_and_validate("-5", 1, 100),
            Err(IntegerInputError::NotANumber)
        );
    }
}
// Rust guideline compliant 2026-05-09
