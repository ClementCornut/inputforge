mod add_inline;
mod context_menu;
mod logic;
mod rename_inline;

use std::rc::Rc;

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;
use crate::frame::view_state::ViewState;

use logic::{MarkerColor, runtime_marker};

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures (the macro suggests \
              shorthand-with-no-prop-name as a fix, which would erase the intent). \
              This is a macro-level artifact, not authored qualifications."
)]
pub(crate) fn ModeTabs() -> Element {
    tracing::trace!(target: "frame::render", region = "mode_tabs");
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
    // arrow-rolling onto it lands focus nowhere).
    let mut renaming: Signal<Option<String>> = use_signal(|| None);
    // Tail `+` inline editor open-state and F4 delete-confirm target.
    let mut adding: Signal<bool> = use_signal(|| false);
    let mut delete_target: Signal<Option<String>> = use_signal(|| None);

    // Which tab's context menu is open (if any), with anchor coords.
    // Hoisted so per-tab handlers can write and the post-loop render can
    // read; carried into `ModeTabContextMenu` as the open-state signal.
    let mut open_for_tab: Signal<Option<(String, context_menu::AnchorRect)>> = use_signal(|| None);

    let editing_now = editing.read().clone();

    // T31: F4 destructive-confirm dialog open-state mirrored from
    // `delete_target`. Two effects keep them in sync — one drives
    // `dialog_open` from `delete_target`, the other clears
    // `delete_target` if `dialog_open` flips back to false (ESC path).
    let mut dialog_open: Signal<bool> = use_signal(|| false);
    use_effect(move || {
        let want = delete_target.read().is_some();
        if *dialog_open.peek() != want {
            dialog_open.set(want);
        }
    });
    use_effect(move || {
        let is_open = *dialog_open.read();
        if !is_open && delete_target.peek().is_some() {
            delete_target.set(None);
        }
    });

    // T31 Step 3a: focus newly-created tab once it appears in `modes`.
    // Sentinel-guarded so it only fires on transitions, not every meta
    // tick — without `last_focused`, this would steal focus from any
    // in-flight inline editor on every tick.
    let editing_for_focus = view.editing_mode;
    let mut last_focused: Signal<Option<String>> = use_signal(|| None);
    use_effect(move || {
        let modes = mode_data.read().0.clone();
        let target = editing_for_focus.read().clone();
        if last_focused.peek().as_ref() == Some(&target) {
            return;
        }
        if renaming.peek().is_some() || *adding.peek() {
            return;
        }
        if let Some(idx) = modes.iter().position(|m| m == &target) {
            if let Some(node) = tab_refs.read().get(idx).and_then(Clone::clone) {
                last_focused.set(Some(target));
                spawn(async move {
                    let _ = node.set_focus(true).await;
                });
            }
        }
    });

    // Pre-compute the affected counts every render — cheap walk.
    let (display_name, modes_count, mappings_count) = match delete_target.read().as_ref() {
        Some(name) => {
            let s = ctx.state.read();
            let counts = s.active_profile.as_ref().map_or((1, 0), |p| {
                let descendants = p.modes().descendants_of(name).unwrap_or_default();
                let modes_count = 1 + descendants.len();
                let mut deleted: Vec<String> = descendants;
                deleted.push(name.clone());
                let mappings_count = p
                    .mappings()
                    .iter()
                    .filter(|m| deleted.iter().any(|d| d == &m.mode))
                    .count();
                (modes_count, mappings_count)
            });
            (name.clone(), counts.0, counts.1)
        }
        None => (String::new(), 0, 0),
    };

    let cmd_for_delete = ctx.commands.clone();
    let confirm_name = display_name.clone();
    let restore_idx_for_dialog = modes_now.iter().position(|m| m == &display_name);

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
                    // DOM ids are derived from the tab's index, not the
                    // mode name, so they're guaranteed HTML5-valid AND safe
                    // to interpolate into JS-eval strings (see kb_tab_id
                    // below + focus_walker.rs eval). Mode names land on
                    // `data-mode` for DevTools/CSS hooks.
                    let tab_id = format!("mode-tab-{idx}");
                    let menu_id = format!("mode-tab-menu-{idx}");
                    let menu_open = open_for_tab
                        .read()
                        .as_ref()
                        .is_some_and(|(n, _)| n == &name);
                    let mut editing_setter = editing;
                    let select_name = name.clone();
                    let key_modes = modes_now.clone();
                    let ctxmenu_name = name.clone();
                    // Carries the mode name into the open-state signal so
                    // the matching tab can be identified later. The DOM
                    // lookup uses the integer-derived `kb_tab_id` instead.
                    let kb_menu_name = name.clone();
                    let kb_tab_id = tab_id.clone();
                    // Plumbing for the Delete keybind: the closure resolves
                    // the disabled flag at event time (cheap — runs only on
                    // Delete keystroke) by reading meta + state, so render
                    // doesn't pay an O(N) descendants_of cost per tab.
                    let kb_delete_name = name.clone();
                    let kb_ctx = ctx.clone();
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

                        // Delete → opens F4 destructive-confirm (T31). Same
                        // disabled rules as the context-menu Delete item:
                        // root tabs and any tab whose subtree contains the
                        // startup mode are immune. The existing
                        // `dialog_open` effect picks up the `delete_target`
                        // change and mounts the dialog.
                        if evt.key() == Key::Delete {
                            evt.prevent_default();
                            let modes_snapshot = key_modes.clone();
                            let startup = kb_ctx.meta.read().startup_mode.clone();
                            let descendants = kb_ctx
                                .state
                                .read()
                                .active_profile
                                .as_ref()
                                .and_then(|p| p.modes().descendants_of(&kb_delete_name).ok())
                                .unwrap_or_default();
                            if !logic::delete_disabled_for_tab(
                                &kb_delete_name,
                                &modes_snapshot,
                                startup.as_deref(),
                                &descendants,
                            ) {
                                let mut delete_target = delete_target;
                                delete_target.set(Some(kb_delete_name.clone()));
                            }
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

                    if renaming.read().as_deref() == Some(name.as_str()) {
                        rsx! {
                            rename_inline::RenameInline {
                                key: "{name}",
                                from: name.clone(),
                                state: renaming,
                            }
                        }
                    } else {
                        rsx! {
                            button {
                                key: "{name}",
                                id: "{tab_id}",
                                "data-mode": "{name}",
                                r#type: "button",
                                class: if is_active { "if-mode-tab if-mode-tab--active" } else { "if-mode-tab" },
                                role: "tab",
                                title: "{name}",
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

                    // Resolve descendants once; both the menu's Delete-flag
                    // and the keydown Delete arm share the same helper for
                    // a single source of truth.
                    let descendants = {
                        let s = ctx.state.read();
                        s.active_profile
                            .as_ref()
                            .and_then(|p| p.modes().descendants_of(&open_name).ok())
                            .unwrap_or_default()
                    };

                    let is_startup = startup.as_ref().is_some_and(|s| s == &open_name);
                    let already_forced =
                        force_mode.is_some_and(|m| m == open_name);
                    // Numeric index of the open tab. Used by the context
                    // menu to derive its DOM id and aria-labelledby target
                    // (which point at the integer-derived tab id, never
                    // the raw mode name — see JS-injection note in mod.rs).
                    let open_tab_idx = modes_for_flags
                        .iter()
                        .position(|m| m == &open_name)
                        .unwrap_or(0);

                    let flags = context_menu::ContextMenuFlags {
                        activate_disabled: already_forced,
                        rename_disabled: !has_profile,
                        delete_disabled: logic::delete_disabled_for_tab(
                            &open_name,
                            &modes_for_flags,
                            startup.as_deref(),
                            &descendants,
                        ),
                        set_default_disabled: is_startup,
                    };

                    let modes_for_close = modes_for_flags.clone();

                    rsx! {
                        context_menu::ModeTabContextMenu {
                            tab_name: open_name.clone(),
                            tab_idx: open_tab_idx,
                            open: open_for_tab,
                            flags,
                            on_close: move |(n, reason): (String, context_menu::CloseReason)| {
                                // Tab key: the browser's natural traversal
                                // is moving focus to the next focusable
                                // element; re-focusing the tab here would
                                // fight that intent. For every other
                                // close path (Escape / click-outside /
                                // ItemActivated) the tab is the natural
                                // landing focus.
                                if matches!(reason, context_menu::CloseReason::Tab) {
                                    return;
                                }
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
                            on_rename: move |n: String| {
                                renaming.set(Some(n));
                                open_for_tab.set(None);
                            },
                            on_delete: move |n: String| {
                                delete_target.set(Some(n));
                                open_for_tab.set(None);
                            },
                        }
                    }
                } else {
                    rsx! {}
                }
            }
            // T31: tail `+` add tab — swaps to inline editor when open.
            if *adding.read() {
                add_inline::AddInline { open: adding }
            } else {
                button {
                    r#type: "button",
                    class: "if-mode-tab if-mode-tab--add",
                    onclick: move |_| adding.set(true),
                    "aria-label": "Add mode",
                    "+"
                }
            }
        }
        // T31: F4 destructive-confirm dialog for Delete. Lives outside the
        // tablist so the dialog backdrop doesn't disturb tab layout.
        crate::components::DialogRoot {
            open: dialog_open,
            onclose: move |()| {
                if let Some(idx) = restore_idx_for_dialog {
                    let target_idx = idx.min(tab_refs.read().len().saturating_sub(1));
                    if let Some(node) = tab_refs.read().get(target_idx).and_then(Clone::clone) {
                        spawn(async move {
                            let _ = node.set_focus(true).await;
                        });
                    }
                }
                delete_target.set(None);
            },
            crate::components::DialogTitle { "Delete mode" }
            // Body splits the prose question from the numeric blast-
            // radius readout. The lead carries the action; the mono
            // count strip below carries the consequence. Cockpit
            // vocabulary: a system-status caption ("MODES" / "MAPPINGS")
            // beside its tabular-nums value reads as a real instrument-
            // panel readout, which fits a destructive-confirm better
            // than a single prose sentence.
            crate::components::DialogBody {
                "Delete '{display_name}'?"
                div { class: "if-modetab-delete-confirm__counts",
                    span { strong { "{modes_count}" } " modes" }
                    span { strong { "{mappings_count}" } " mappings" }
                }
            }
            crate::components::DialogFooter {
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Ghost,
                    onmounted: move |evt: MountedEvent| {
                        spawn(async move {
                            let _ = evt.data().set_focus(true).await;
                        });
                    },
                    onclick: move |_| { delete_target.set(None); },
                    "Cancel"
                }
                crate::components::Button {
                    variant: crate::components::ButtonVariant::Secondary,
                    onclick: move |_| {
                        let _ = cmd_for_delete.send(EngineCommand::DeleteMode {
                            name: confirm_name.clone(),
                        });
                        delete_target.set(None);
                    },
                    "Confirm"
                }
            }
        }
    }
}
