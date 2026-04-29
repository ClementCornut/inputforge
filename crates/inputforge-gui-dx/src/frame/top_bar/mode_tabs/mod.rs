mod context_menu;
mod logic;

use std::rc::Rc;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

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

    // Which tab's context menu is open (if any), with anchor coords.
    // Hoisted so per-tab handlers can write and the post-loop render can
    // read; carried into `ModeTabContextMenu` as the open-state signal.
    let mut open_for_tab: Signal<Option<(String, context_menu::AnchorRect)>> = use_signal(|| None);

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
                    let menu_id = format!("mode-tab-menu-{name}");
                    let menu_open = open_for_tab
                        .read()
                        .as_ref()
                        .is_some_and(|(n, _)| n == &name);
                    let mut editing_setter = editing;
                    let select_name = name.clone();
                    let key_modes = modes_now.clone();
                    let ctxmenu_name = name.clone();
                    let kb_menu_name = name.clone();
                    let kb_tab_id = tab_id.clone();
                    let onclick = move |_| {
                        editing_setter.set(select_name.clone());
                    };
                    let oncontextmenu = move |evt: MouseEvent| {
                        // Suppress the platform browser menu so our
                        // hand-rolled list takes over.
                        evt.prevent_default();
                        evt.stop_propagation();
                        let coords = evt.client_coordinates();
                        open_for_tab.set(Some((
                            ctxmenu_name.clone(),
                            context_menu::AnchorRect {
                                left: coords.x,
                                bottom: coords.y,
                            },
                        )));
                    };
                    let onkeydown = move |evt: KeyboardEvent| {
                        // Roving-tabindex navigation. Shift+F10 opens the
                        // context menu (this task); Delete opens the F4
                        // confirm (T31). The remaining arms are the
                        // minimal navigation contract.
                        // Skips any index whose name matches `renaming`
                        // (its button isn't mounted while the inline
                        // editor occupies that slot).

                        // Shift+F10 → open the context menu anchored to
                        // this tab's bounding-rect. Dioxus 0.7 doesn't
                        // expose `get_client_rect` on `MountedData`, so
                        // we ride the DOM via `document::eval` and parse
                        // the JSON [left, bottom] result back into the
                        // open-state signal.
                        if evt.key() == Key::F10 && evt.modifiers().shift() {
                            evt.prevent_default();
                            let target_id = kb_tab_id.clone();
                            let menu_for_tab = kb_menu_name.clone();
                            let mut open_for_tab_inner = open_for_tab;
                            spawn(async move {
                                let mut handle = document::eval(&format!(
                                    "var el = document.getElementById('{target_id}');\n\
                                     if (!el) {{ dioxus.send([0, 0]); return; }}\n\
                                     var r = el.getBoundingClientRect();\n\
                                     dioxus.send([r.left, r.bottom]);"
                                ));
                                if let Ok(value) = handle.recv::<[f64; 2]>().await {
                                    let [left, bottom] = value;
                                    open_for_tab_inner.set(Some((
                                        menu_for_tab,
                                        context_menu::AnchorRect { left, bottom },
                                    )));
                                }
                            });
                            return;
                        }

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
                            "aria-haspopup": "menu",
                            "aria-expanded": "{menu_open}",
                            // Only emit aria-controls while the menu is
                            // mounted — pointing at a missing id confuses
                            // AT.
                            "aria-controls": menu_open.then(|| menu_id.clone()),
                            tabindex: if is_active { "0" } else { "-1" },
                            onclick,
                            oncontextmenu,
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
            // The context menu lives outside the tablist so it doesn't
            // disrupt the flex layout. Rendered once for whichever tab is
            // currently open; flag-derivation walks the active profile's
            // mode tree to compute "subtree contains startup" precisely.
            {
                if let Some((open_name, _)) = open_for_tab.read().as_ref().cloned() {
                    let modes_for_flags = modes_now.clone();
                    let m = ctx.meta.read();
                    let startup = m.startup_mode.clone();
                    let force_mode = m.mode_force.as_ref().map(|f| f.mode.clone());
                    let has_profile = m.profile_name.is_some();
                    drop(m);

                    let subtree_contains_startup = {
                        let s = ctx.state.read();
                        s.active_profile
                            .as_ref()
                            .and_then(|p| {
                                let descendants = p.modes().descendants_of(&open_name).ok()?;
                                startup.as_ref().map(|sm| {
                                    sm == &open_name || descendants.iter().any(|d| d == sm)
                                })
                            })
                            .unwrap_or(false)
                    };

                    let is_root = modes_for_flags
                        .first()
                        .is_some_and(|first| first == &open_name);
                    let is_startup = startup.as_ref().is_some_and(|s| s == &open_name);
                    let already_forced =
                        force_mode.is_some_and(|m| m == open_name);

                    let flags = context_menu::ContextMenuFlags {
                        activate_disabled: already_forced,
                        rename_disabled: !has_profile,
                        // Spec: delete is disabled when the tab is the
                        // root, when the tab IS the startup mode, or when
                        // the subtree of the tab CONTAINS the startup
                        // mode.
                        delete_disabled: is_root || subtree_contains_startup,
                        set_default_disabled: is_startup,
                    };

                    let modes_for_close = modes_for_flags.clone();
                    let cmd_delete_for_now = ctx.commands.clone();

                    rsx! {
                        context_menu::ModeTabContextMenu {
                            tab_name: open_name.clone(),
                            open: open_for_tab,
                            flags,
                            on_close: move |n: String| {
                                if let Some(idx) = modes_for_close.iter().position(|m| m == &n) {
                                    if let Some(node) =
                                        tab_refs.read().get(idx).and_then(Clone::clone)
                                    {
                                        spawn(async move {
                                            let _ = node.set_focus(true).await;
                                        });
                                    }
                                }
                            },
                            on_rename: move |_n: String| {
                                // T31 wires this to set
                                // `renaming.set(Some(n))`. For now, just
                                // close.
                                open_for_tab.set(None);
                            },
                            on_delete: move |n: String| {
                                // TODO(F7-Task-31): swap this to open the
                                // F4 destructive-confirm dialog. T30
                                // ships the direct dispatch behind the
                                // merge-contract; T31 closes that gap.
                                let _ = cmd_delete_for_now.send(EngineCommand::DeleteMode {
                                    name: n,
                                });
                                open_for_tab.set(None);
                            },
                        }
                    }
                } else {
                    rsx! {}
                }
            }
        }
    }
}
