mod logic;

use std::rc::Rc;

use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::view_state::ViewState;

use logic::{MarkerColor, runtime_marker};

#[component]
pub(crate) fn ModeTabs() -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    // Combine the three meta reads into one memo so a single read-lock
    // acquisition serves all three values per tick. PartialEq on the
    // tuple gates re-runs to actual changes.
    let mode_data = use_memo(move || {
        let m = ctx.meta.read();
        (
            m.modes.clone(),
            m.current_mode.clone(),
            m.mode_force.clone(),
        )
    });

    let editing = view.editing_mode;
    let (modes_now, cur, force) = mode_data.read().clone();
    let marker = runtime_marker(&modes_now, &cur, force.as_ref());

    // Per-tab MountedData refs for keyboard focus movement.
    // Resized via use_effect on length change — never in render.
    let mut tab_refs: Signal<Vec<Option<Rc<MountedData>>>> =
        use_signal(|| vec![None; modes_now.len()]);
    // Resize on `modes` length change. Done in a use_effect rather than
    // inline in render to avoid a signal write during render (Dioxus 0.7
    // warns / errors on this).
    let modes_len = modes_now.len();
    use_effect(move || {
        if tab_refs.read().len() != modes_len {
            tab_refs.write().resize(modes_len, None);
        }
    });

    // Hoisted above the per-tab loop so the keydown closure can read
    // `renaming.peek()` and skip the index whose tab is currently
    // swapped to a `RenameInline` editor (the button isn't mounted, so
    // arrow-rolling onto it lands focus nowhere). Set by T31 below.
    let renaming: Signal<Option<String>> = use_signal(|| None);
    // Same forward-declaration for the add-inline tail editor and the
    // F4 delete-confirm target — T31 step 3 reads these.
    let _adding: Signal<bool> = use_signal(|| false);
    let _delete_target: Signal<Option<String>> = use_signal(|| None);

    let editing_now = editing.read().clone();

    rsx! {
        // aria-label is required because the tablist has no visible
        // heading. "Editing mode" matches the F5 spec's chrome name.
        // aria-controls is intentionally omitted: until F11/F13 mounts a
        // real tabpanel for the editing surface, half-implementing the
        // tabpanel relationship would confuse AT.
        div { class: "if-mode-tabs", role: "tablist",
            "aria-orientation": "horizontal", "aria-label": "Editing mode",
            for (idx, name) in modes_now.iter().cloned().enumerate() {
                {
                    let is_active = name == editing_now;
                    let marker_for_tab = (marker.tab_index == Some(idx)).then_some(marker.color);
                    let tab_id = format!("mode-tab-{name}");
                    let mut editing_setter = editing;
                    let select_name = name.clone();
                    let key_modes = modes_now.clone();
                    let onclick = move |_| {
                        editing_setter.set(select_name.clone());
                    };
                    let onkeydown = move |evt: KeyboardEvent| {
                        // Roving-tabindex navigation. Shift+F10 (open
                        // context menu) and Delete (open F4 confirm) are
                        // wired by Tasks 30 and 31 respectively; this
                        // arm-set is the minimal navigation contract.
                        // Skips any index whose name matches `renaming`
                        // (its button isn't mounted while the inline
                        // editor occupies that slot).
                        let len = key_modes.len();
                        if len == 0 {
                            return;
                        }
                        let renaming_now = renaming.peek().clone();
                        let is_skippable = |i: usize| -> bool {
                            renaming_now
                                .as_ref()
                                .is_some_and(|r| key_modes.get(i).is_some_and(|n| n == r))
                        };
                        // Step direction: +1 / -1 / jump-to-bound. For
                        // jumps, walk forward (or backward) past any
                        // renaming index.
                        let raw_next = match evt.key() {
                            Key::ArrowRight => Some(((idx + 1) % len, 1isize)),
                            Key::ArrowLeft  => Some(((idx + len - 1) % len, -1isize)),
                            Key::Home       => Some((0, 1isize)),
                            Key::End        => Some((len - 1, -1isize)),
                            _ => None,
                        };
                        let Some((mut target, step)) = raw_next else { return };
                        // Walk past renaming indexes — bounded by `len`
                        // iterations so we never infinite-loop even if
                        // every tab is renaming (impossible in practice).
                        for _ in 0..len {
                            if !is_skippable(target) {
                                break;
                            }
                            target = if step > 0 {
                                (target + 1) % len
                            } else {
                                (target + len - 1) % len
                            };
                        }
                        if is_skippable(target) {
                            return; // Every tab is renaming — no-op.
                        }
                        evt.prevent_default();
                        if let Some(target_name) = key_modes.get(target) {
                            editing_setter.set(target_name.clone());
                            let node = tab_refs.read().get(target).and_then(Clone::clone);
                            if let Some(node) = node {
                                spawn(async move {
                                    let _ = node.set_focus(true).await;
                                });
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
                            key: "{name}",
                            id: "{tab_id}",
                            r#type: "button",
                            class: if is_active { "if-mode-tab if-mode-tab--active" } else { "if-mode-tab" },
                            role: "tab",
                            "aria-selected": "{is_active}",
                            tabindex: if is_active { "0" } else { "-1" },
                            onclick,
                            onkeydown,
                            onmounted,
                            "{name}"
                            if let Some(color) = marker_for_tab {
                                // Visual marker dot.
                                span {
                                    class: match color {
                                        MarkerColor::Natural => "if-mode-tab__marker if-mode-tab__marker--natural",
                                        MarkerColor::Forced  => "if-mode-tab__marker if-mode-tab__marker--forced",
                                    },
                                    "aria-hidden": "true",
                                }
                                // sr-only sibling so AT users get the
                                // semantic ("Engine running" / "forced")
                                // that color alone cannot convey.
                                span {
                                    class: "if-sr-only",
                                    {match color {
                                        MarkerColor::Natural => "Engine running",
                                        MarkerColor::Forced  => "Engine running (forced)",
                                    }}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
