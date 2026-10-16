//! Instrument key interpretation and nudge grouping policy.
use super::{deadzone, response_curve};
use inputforge_gui_dx::instruments::Event;
pub(super) fn is_nudge(event: &Event, action: &inputforge_core::action::Action) -> bool {
    let Event::Key { key, .. } = event else {
        return false;
    };
    matches!(key.as_str(), "ArrowLeft" | "ArrowRight")
        || (matches!(
            action,
            inputforge_core::action::Action::ResponseCurve { .. }
        ) && matches!(key.as_str(), "ArrowUp" | "ArrowDown"))
}
pub(super) fn curve(raw: &str, shift: bool) -> Option<response_curve::keyboard::KeyInput> {
    use response_curve::keyboard::KeyInput as K;
    Some(match raw {
        "Tab" if shift => K::ShiftTab,
        "Tab" => K::Tab,
        "ArrowLeft" => K::ArrowLeft { shift },
        "ArrowRight" => K::ArrowRight { shift },
        "ArrowUp" => K::ArrowUp { shift },
        "ArrowDown" => K::ArrowDown { shift },
        "Home" => K::Home,
        "End" => K::End,
        "Enter" => K::Enter,
        "Delete" | "Backspace" => K::Delete,
        "Escape" => K::Escape,
        _ => return None,
    })
}
pub(super) fn zone(raw: &str, shift: bool) -> Option<deadzone::keyboard::KeyInput> {
    use deadzone::keyboard::KeyInput as K;
    Some(match raw {
        "Tab" if shift => K::ShiftTab,
        "Tab" => K::Tab,
        "ArrowLeft" => K::ArrowLeft { shift },
        "ArrowRight" => K::ArrowRight { shift },
        "ArrowUp" => K::ArrowUp { shift },
        "ArrowDown" => K::ArrowDown { shift },
        "Home" => K::Home,
        "End" => K::End,
        "Enter" => K::Enter,
        "Delete" | "Backspace" => K::Delete,
        "Escape" => K::Escape,
        _ => return None,
    })
}
