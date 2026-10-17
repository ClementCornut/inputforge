use super::state::State;
use inputforge_gui_dx::toast::{Interaction, ToastLevel};
use std::time::{Duration, Instant};
fn at(start: Instant, seconds: u64) -> Instant {
    start + Duration::from_secs(seconds)
}
#[test]
fn interleaved_hover_and_focus_keep_one_continuous_pause() {
    let mut state = State::default();
    let start = Instant::now();
    state.push(ToastLevel::Info, "paused".into(), start);
    state.event(0, &Interaction::Hover(true), at(start, 3));
    state.event(0, &Interaction::Focus(true), at(start, 4));
    state.event(0, &Interaction::Hover(false), at(start, 5));
    state.event(0, &Interaction::Hover(false), at(start, 6));
    state.expire(at(start, 20));
    assert_eq!(state.visible().len(), 1);
    state.event(0, &Interaction::Focus(false), at(start, 20));
    state.event(0, &Interaction::Focus(false), at(start, 21));
    state.expire(at(start, 24));
    assert_eq!(state.visible().len(), 1);
    state.expire(at(start, 25));
    assert_eq!(state.visible(), []);
}
#[test]
fn coalescing_while_focused_preserves_pause_and_restarts_eight_effective_seconds() {
    let mut state = State::default();
    let start = Instant::now();
    state.push(ToastLevel::Warning, "repeat".into(), start);
    state.event(0, &Interaction::Focus(true), at(start, 2));
    state.push(ToastLevel::Warning, "repeat".into(), at(start, 10));
    assert_eq!(state.visible()[0].count, 2);
    assert_eq!(state.deadline(), None);
    state.expire(at(start, 40));
    assert_eq!(state.visible().len(), 1);
    state.event(0, &Interaction::Focus(false), at(start, 40));
    state.expire(at(start, 47));
    assert_eq!(state.visible().len(), 1);
    state.expire(at(start, 48));
    assert_eq!(state.visible(), []);
}
#[test]
fn coalescing_restarts_lifetime_and_different_level_stays_separate() {
    let mut state = State::default();
    let start = Instant::now();
    state.push(ToastLevel::Info, "same".into(), start);
    state.push(ToastLevel::Info, "same".into(), at(start, 7));
    state.push(ToastLevel::Warning, "same".into(), at(start, 8));
    assert_eq!(
        state.visible().iter().map(|t| t.count).collect::<Vec<_>>(),
        vec![2, 1]
    );
    state.expire(at(start, 14));
    assert_eq!(state.visible().len(), 2);
    state.expire(at(start, 15));
    assert_eq!(state.visible().len(), 1);
    assert_eq!(state.visible()[0].level, ToastLevel::Warning);
    state.expire(at(start, 16));
    assert_eq!(state.visible(), []);
}
