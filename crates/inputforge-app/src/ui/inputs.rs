//! Numeric parsing, bounded steppers and commit/cancel coordination.
use super::draft::{self, Draft};
use dioxus::prelude::*;
use inputforge_gui_dx::controls::{InputView, NumberProps, NumberView};
fn parse(raw: &str, min: f64, max: f64) -> Option<f64> {
    bounded(raw.trim().parse().ok()?, min, max)
}
fn bounded(value: f64, min: f64, max: f64) -> Option<f64> {
    if !value.is_finite() || min.is_nan() || max.is_nan() || min > max {
        return None;
    }
    let value = value.clamp(min, max);
    value.is_finite().then_some(value)
}
pub(super) fn use_number(props: &NumberProps) -> NumberView {
    let mut state = use_signal(|| Draft::new(props.display.clone()));
    let mut stepped = use_signal(|| None::<f64>);
    let text = use_memo(move || state.read().raw.clone());
    let external = (props.value)();
    let display = props.display.clone();
    let min = props.min;
    let max = props.max;
    use_effect(use_reactive!(|display, external, min, max| {
        let acknowledged = {
            let draft = state.peek();
            (draft.acknowledged() && parse(&draft.raw, min, max) == Some(external))
                || *stepped.peek() == Some(external)
        };
        state.write().sync(&display, acknowledged);
        if acknowledged {
            stepped.set(None);
        }
    }));
    let p = props.clone();
    let input = use_callback(move |event: FormEvent| {
        if p.disabled {
            return;
        }
        stepped.set(None);
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
        let raw = state.write().take();
        if let Some(raw) = raw
            && let Some(value) = parse(&raw, p.min, p.max)
        {
            if (p.value)().partial_cmp(&value) == Some(std::cmp::Ordering::Equal) {
                state.write().accept();
            }
            if let Some(handler) = p.oncommit {
                stepped.set(None);
                handler.call(value);
            }
        }
    });
    let p = props.clone();
    let key = use_callback(move |event: KeyboardEvent| {
        if p.disabled {
            return;
        }
        match event.key() {
            Key::Enter => {
                event.prevent_default();
                stepped.set(None);
                state.write().enter();
                commit.call(());
                draft::blur();
            }
            Key::Escape => {
                event.prevent_default();
                stepped.set(None);
                state.write().cancel(p.display.clone());
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
    let step = use_step(props, stepped);
    NumberView {
        input: InputView {
            text: text.into(),
            input,
            key,
            blur,
            focus,
        },
        step,
    }
}

fn use_step(props: &NumberProps, mut stepped: Signal<Option<f64>>) -> EventHandler<bool> {
    let props = props.clone();
    use_callback(move |increment: bool| {
        if props.disabled || !props.step.is_finite() || props.step <= 0.0 {
            return;
        }
        let next = (props.value)() + if increment { props.step } else { -props.step };
        if let Some(value) = bounded(next, props.min, props.max)
            && let Some(handler) = props.onstep
        {
            let changed = (props.value)().partial_cmp(&value) != Some(std::cmp::Ordering::Equal);
            stepped.set(changed.then_some(value));
            handler.call(value);
        }
    })
}

#[cfg(test)]
mod oncommit_tests {
    use super::parse;

    #[test]
    fn nonfinite_text_and_impossible_finite_bounds_do_not_commit() {
        for raw in ["NaN", "inf", "-inf"] {
            assert_eq!(parse(raw, f64::NEG_INFINITY, f64::INFINITY), None);
        }
        assert_eq!(parse("0", f64::INFINITY, f64::INFINITY), None);
        assert_eq!(parse("0", f64::NEG_INFINITY, f64::NEG_INFINITY), None);
        assert_eq!(parse("0", f64::NAN, 1.0), None);
        assert_eq!(parse("0", 2.0, 1.0), None);
    }

    #[test]
    fn enter_clamps_above_max() {
        assert_eq!(parse("1.5", -1.0, 1.0), Some(1.0));
    }

    #[test]
    fn blur_clamps_below_min() {
        assert_eq!(parse("-2.5", -1.0, 1.0), Some(-1.0));
    }

    #[test]
    fn invalid_text_returns_none() {
        assert_eq!(parse("abc", -1.0, 1.0), None);
    }

    #[test]
    fn comma_decimal_returns_none() {
        // Locale-aware parsing is out of F11 scope; comma decimals fail to parse.
        assert_eq!(parse("0,5", -1.0, 1.0), None);
    }
}
