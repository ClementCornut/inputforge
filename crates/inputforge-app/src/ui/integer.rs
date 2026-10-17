//! Integer validation and commit/cancel behavior shared by all numeric settings.
use super::draft::{self, Draft};
use dioxus::prelude::*;
use inputforge_gui_dx::{
    components::IntegerInputError,
    controls::{InputView, IntegerProps},
};
fn parse(raw: &str, min: usize, max: usize) -> Result<usize, IntegerInputError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(IntegerInputError::Empty);
    }
    let value = trimmed
        .parse()
        .map_err(|_error| IntegerInputError::NotANumber)?;
    if !(min..=max).contains(&value) {
        return Err(IntegerInputError::OutOfRange { min, max });
    }
    Ok(value)
}
pub(super) fn use_integer(props: &IntegerProps) -> InputView {
    let mut state = use_signal(|| Draft::new(props.display.clone()));
    let text = use_memo(move || state.read().raw.clone());
    let external = (props.value)();
    let display = props.display.clone();
    let min = props.min;
    let max = props.max;
    use_effect(use_reactive!(|display, external, min, max| {
        let acknowledged = {
            let draft = state.peek();
            draft.acknowledged() && parse(&draft.raw, min, max) == Ok(external)
        };
        state.write().sync(&display, acknowledged);
    }));
    let p = props.clone();
    let input = use_callback(move |event: FormEvent| {
        if p.disabled {
            return;
        }
        state.write().edit(event.value());
        if let Some(handler) = p.oninput {
            handler.call(event);
        }
    });
    let p = props.clone();
    let commit = use_callback(move |()| {
        if p.disabled {
            return;
        }
        let Some(raw) = state.write().take() else {
            return;
        };
        match parse(&raw, p.min, p.max) {
            Ok(value) => {
                if (p.value)() == value {
                    state.write().accept();
                }
                if let Some(handler) = p.oncommit {
                    handler.call(value);
                }
            }
            Err(error) => {
                state.write().retry();
                if let Some(handler) = p.oninvalid {
                    handler.call(error);
                }
            }
        }
    });
    let props = props.clone();
    let key = use_callback(move |event: KeyboardEvent| {
        if props.disabled {
            return;
        }
        match event.key() {
            Key::Enter => {
                event.prevent_default();
                state.write().enter();
                commit.call(());
                draft::blur();
            }
            Key::Escape => {
                event.prevent_default();
                state.write().cancel(props.display.clone());
                draft::blur();
            }
            _ => {}
        }
    });
    let blur = use_callback(move |_: FocusEvent| {
        if !state.write().blur() {
            commit.call(());
        }
    });
    let focus = use_callback(move |_: FocusEvent| state.write().retry());
    InputView {
        text: text.into(),
        input,
        key,
        blur,
        focus,
    }
}

#[cfg(test)]
mod tests {
    use super::{IntegerInputError, parse};

    #[test]
    fn in_range_returns_value() {
        assert_eq!(parse("42", 1, 100), Ok(42));
        assert_eq!(parse("1", 1, 100), Ok(1));
        assert_eq!(parse("100", 1, 100), Ok(100));
    }

    #[test]
    fn above_max_is_out_of_range() {
        assert_eq!(
            parse("200", 1, 100),
            Err(IntegerInputError::OutOfRange { min: 1, max: 100 })
        );
    }

    #[test]
    fn below_min_is_out_of_range() {
        assert_eq!(
            parse("0", 1, 100),
            Err(IntegerInputError::OutOfRange { min: 1, max: 100 })
        );
    }

    #[test]
    fn non_numeric_is_not_a_number() {
        assert_eq!(parse("abc", 1, 100), Err(IntegerInputError::NotANumber));
    }

    #[test]
    fn empty_is_empty_error() {
        assert_eq!(parse("", 1, 100), Err(IntegerInputError::Empty));
        assert_eq!(parse("   ", 1, 100), Err(IntegerInputError::Empty));
    }

    #[test]
    fn negative_is_not_a_number() {
        // usize cannot represent negatives; parse returns Err -> NotANumber.
        assert_eq!(parse("-5", 1, 100), Err(IntegerInputError::NotANumber));
    }

    #[test]
    fn float_is_not_a_number() {
        assert_eq!(parse("3.14", 1, 100), Err(IntegerInputError::NotANumber));
    }
}
// Rust guideline compliant 2026-05-09
