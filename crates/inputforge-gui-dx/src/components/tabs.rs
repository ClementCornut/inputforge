use std::rc::Rc;

use dioxus::prelude::*;

use super::merge_class;

/// One entry in a `Tabs` tablist.
///
/// `id` is the stable identifier the caller passes back via `onchange` and
/// matches against `value`. `label` is the visible button text. `controls`,
/// when set, is the DOM `id` of the tab's panel — it wires `aria-controls` on
/// the tab to a `role="tabpanel"` element the caller renders elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub controls: Option<String>,
}

/// WAI-ARIA Tabs primitive with focus-roving and automatic activation.
///
/// - `role="tablist"` on the wrapper, `role="tab"` per item.
/// - Each tab button gets `id="tab-{id}"`; when `TabItem::controls` is set,
///   `aria-controls` points at the caller's tabpanel id.
/// - Arrow Left / Right cycles focus AND activates (automatic activation —
///   panel swaps are synchronous and cheap).
/// - Home / End jumps to first / last (and activates).
/// - `tabindex` is `0` for the active tab and `-1` for the rest (focus-roving).
///   Focus is moved imperatively via `MountedData::set_focus` after each
///   keyboard activation so the focus ring follows the active tab.
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
    /// Tabs in display order. Each item carries its own id, label, and
    /// optional `controls` panel id.
    items: Vec<TabItem>,
    #[props(default)] class: Option<String>,
    #[props(default)] disabled: bool,
) -> Element {
    let combined = merge_class(
        "if-tabs",
        if disabled { "if-tabs--disabled" } else { "" },
        class.as_deref(),
    );

    // Per-tab mounted-element refs, indexed by position. Populated by the
    // `onmounted` callback on each button; consumed by `onkeydown` to call
    // `set_focus(true)` on the newly-active tab so the browser's focus
    // follows the selection (WAI-ARIA APG: automatic-activation tablists
    // require focus and selection to move together).
    let mut tab_refs: Signal<Vec<Option<Rc<MountedData>>>> = use_signal(|| vec![None; items.len()]);

    rsx! {
        div {
            class: "{combined}",
            role: "tablist",
            "aria-orientation": "horizontal",
            for (idx, item) in items.iter().cloned().enumerate() {
                {
                    let TabItem { id, label, controls } = item;
                    let is_active = id == value;
                    let id_for_click = id.clone();
                    let items_for_key = items.clone();
                    let tab_id = format!("tab-{id}");
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
                            if let Some(next) = items_for_key.get(i) {
                                onchange.call(next.id.clone());
                                // Move focus to the new tab so the focus
                                // ring tracks selection. Reads/clones the
                                // Rc<MountedData> before awaiting so the
                                // signal's read borrow is dropped first.
                                let target = tab_refs
                                    .read()
                                    .get(i)
                                    .and_then(Clone::clone);
                                if let Some(node) = target {
                                    spawn(async move {
                                        let _ = node.set_focus(true).await;
                                    });
                                }
                            }
                        }
                    };
                    let onmounted = move |evt: MountedEvent| {
                        let mut refs = tab_refs.write();
                        if refs.len() <= idx {
                            refs.resize(idx + 1, None);
                        }
                        refs[idx] = Some(evt.data());
                    };
                    rsx! {
                        button {
                            key: "{id}",
                            id: "{tab_id}",
                            r#type: "button",
                            class: if is_active { "if-tab if-tab--active" } else { "if-tab" },
                            role: "tab",
                            "aria-selected": "{is_active}",
                            "aria-controls": controls,
                            tabindex: if is_active { "0" } else { "-1" },
                            disabled,
                            onclick,
                            onkeydown,
                            onmounted,
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
