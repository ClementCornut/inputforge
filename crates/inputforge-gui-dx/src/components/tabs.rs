use dioxus::prelude::*;

use super::merge_class;

/// WAI-ARIA Tabs primitive with focus-roving and automatic activation.
///
/// - `role="tablist"` on the wrapper, `role="tab"` per item.
/// - Arrow Left / Right cycles focus AND activates (automatic activation —
///   panel swaps are synchronous and cheap, see spec rationale).
/// - Home / End jumps to first / last (and activates).
/// - `tabindex` is `0` for the active tab and `-1` for the rest (focus-roving).
/// - `disabled` short-circuits keyboard and click; visible state via
///   `.if-tabs--disabled`.
///
/// The component is stateless: the caller owns `value` and renders panel
/// content based on it. F11 (Modes) reuses this — keeping it stateless avoids
/// over-coupling.
#[component]
pub fn Tabs(
    /// Stable id of the active tab.
    value: String,
    onchange: EventHandler<String>,
    /// (id, label) pairs in display order.
    items: Vec<(String, String)>,
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
) -> Element {
    let combined = merge_class(
        "if-tabs",
        if disabled { "if-tabs--disabled" } else { "" },
        class.as_deref(),
    );

    rsx! {
        div {
            class: "{combined}",
            role: "tablist",
            "aria-orientation": "horizontal",
            for (idx, (id, label)) in items.iter().cloned().enumerate() {
                {
                    let is_active = id == value;
                    let id_for_click = id.clone();
                    let items_for_key = items.clone();
                    let onclick = move |_| {
                        if !disabled {
                            onchange.call(id_for_click.clone());
                        }
                    };
                    let onkeydown = move |evt: KeyboardEvent| {
                        if disabled { return; }
                        let key = evt.key();
                        let len = items_for_key.len();
                        if len == 0 { return; }
                        let next_idx: Option<usize> = match key {
                            Key::ArrowRight => Some((idx + 1) % len),
                            Key::ArrowLeft  => Some((idx + len - 1) % len),
                            Key::Home       => Some(0),
                            Key::End        => Some(len - 1),
                            Key::Character(ref s) if s == " " => {
                                evt.prevent_default();
                                None
                            }
                            Key::Enter => {
                                evt.prevent_default();
                                None
                            }
                            _ => None,
                        };
                        if let Some(i) = next_idx {
                            evt.prevent_default();
                            if let Some((next_id, _)) = items_for_key.get(i) {
                                onchange.call(next_id.clone());
                            }
                        }
                    };
                    rsx! {
                        button {
                            key: "{id}",
                            r#type: "button",
                            class: if is_active { "if-tab if-tab--active" } else { "if-tab" },
                            role: "tab",
                            "aria-selected": "{is_active}",
                            tabindex: if is_active { "0" } else { "-1" },
                            disabled,
                            onclick,
                            onkeydown,
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
