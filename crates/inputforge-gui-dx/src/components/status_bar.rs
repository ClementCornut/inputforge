use dioxus::prelude::*;

use super::merge_class;

/// Three-slot horizontal bar used as a window-level status surface.
///
/// `start` flows left, `end` is right-anchored, `middle` fills the gap.
/// Fixed 28px height (matches today's egui status bar; reviewable by
/// frontend-design).
///
/// **ARIA shape.** The wrapper is intentionally neutral — no `role`, no
/// `aria-label`. `role="status"` is a live region; applying it at the
/// primitive level would make every badge change announce. Consumers add
/// `role="status"` (or `aria-live`) on the *specific* element they want
/// announced (typically a single Badge), or `aria-label` on the wrapper if a
/// labeled landmark is desired.
#[component]
pub fn StatusBar(
    start: Element,
    middle: Element,
    end: Element,
    #[props(default)] class: Option<String>,
) -> Element {
    let combined = merge_class("if-status-bar", "", class.as_deref());
    rsx! {
        div { class: "{combined}",
            div { class: "if-status-bar__start",  {start}  }
            div { class: "if-status-bar__middle", {middle} }
            div { class: "if-status-bar__end",    {end}    }
        }
    }
}
