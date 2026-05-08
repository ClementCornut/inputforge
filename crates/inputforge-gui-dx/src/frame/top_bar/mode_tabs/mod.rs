mod add_inline;
mod context_menu;
mod delete_dialog;
mod logic;
mod rename_inline;

use dioxus::prelude::*;

use crate::components::{TabButton, TabsList, TabsRoot};
use crate::context::AppContext;
use crate::frame::view_state::ViewState;

pub(crate) use delete_dialog::{ModeDeleteDialog, ModeDeleteSignal, ModeFocusSignal};
use logic::runtime_marker;

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

    // Combine the two meta reads into one memo so a single read-lock
    // acquisition serves both values per tick. PartialEq on the tuple
    // gates re-runs to actual changes.
    let mode_data = use_memo(move || {
        let m = ctx.meta.read();
        (m.modes.clone(), m.current_mode.clone())
    });

    let mut editing = view.editing_mode;
    let (modes_now, cur) = mode_data.read().clone();
    let marker = runtime_marker(&modes_now, &cur);

    // Hoisted above the per-tab loop so the `RenameInline` swap can
    // read it. The Tabs primitive's keyboard coordinator walks the
    // registry of mounted `TabButton`s; `RenameInline` does not
    // register, so the renaming index is auto-skipped during arrow
    // navigation without an explicit "skip" flag.
    let mut renaming: Signal<Option<String>> = use_signal(|| None);
    // Tail `+` inline editor open-state.
    let mut adding: Signal<bool> = use_signal(|| false);
    // F4 delete-confirm target. Owned by `Layout` and provided through
    // `ModeDeleteSignal` so the mode strip can live inside the mappings
    // workspace while `ModeDeleteDialog` remains a shell-level sibling.
    let mut delete_target: Signal<Option<String>> = use_context::<ModeDeleteSignal>().0;

    // Which tab's context menu is open (if any), with anchor coords.
    // Hoisted so per-tab handlers can write and the post-loop render can
    // read; carried into `ModeTabContextMenu` as the open-state signal.
    let mut open_for_tab: Signal<Option<(String, context_menu::AnchorRect)>> = use_signal(|| None);

    let editing_now = editing.read().clone();

    // T31 Step 3a: focus newly-created tab once it appears in `modes`.
    // `pending_focus` is set by:
    //   1. `add_inline::run_commit` on successful AddMode dispatch.
    //   2. The context-menu `on_close` handler to refocus the tab when
    //      the menu closes via Escape / click-outside / item activation.
    //   3. `ModeDeleteDialog` to land focus on the surviving tab after
    //      Confirm or on the originating tab after Cancel / Escape.
    //
    // The signal lives in shell-scope (`ModeFocusSignal`) so the
    // sibling dialog reaches it through context, replacing the prior
    // `document.querySelectorAll('[role="tab"]').focus()` JS-eval path
    // with a single canonical channel through `TabsList`.
    //
    // `TabsList` watches this signal: when it is `Some(name)` and a
    // matching `TabButton` has registered its `MountedData` ref, the
    // primitive calls `set_focus(true)` and clears the signal. Pending
    // requests for not-yet-mounted tabs stay set until the registry
    // grows to include the target, which is exactly the behavior the
    // hand-rolled use_effect used to implement.
    let pending_focus: Signal<Option<String>> = use_context::<ModeFocusSignal>().0;

    rsx! {
        div { class: "if-mode-tabs-outer",
            TabsRoot {
                value: editing_now.clone(),
                onchange: move |id: String| editing.set(id),
                focus_request: pending_focus,
                // aria-label is required because the tablist has no visible
                // heading. "Editing mode" matches the F5 spec's chrome name.
                // aria-controls (panel relationship) is intentionally
                // omitted at the list level: until F11/F13 mounts a real
                // tabpanel, half-implementing the relationship would
                // confuse AT.
                TabsList {
                    class: "if-mode-tabs-wrap".to_owned(),
                    aria_label: "Editing mode".to_owned(),
                    for (idx, name) in modes_now.iter().cloned().enumerate() {
                        {
                            let show_marker = marker.tab_index == Some(idx);
                            // DOM ids are derived from the tab's index, not
                            // the mode name, so they are guaranteed
                            // HTML5-valid AND safe to interpolate into
                            // JS-eval strings (see kb_tab_id below + the
                            // focus_walker eval inside the context menu
                            // module). Mode names land on `data-mode` for
                            // DevTools/CSS hooks.
                            let tab_id = format!("mode-tab-{idx}");
                            let menu_id = format!("mode-tab-menu-{idx}");
                            let menu_open = open_for_tab
                                .read()
                                .as_ref()
                                .is_some_and(|(n, _)| n == &name);
                            let key_modes = modes_now.clone();
                            let ctxmenu_name = name.clone();
                            // Carries the mode name into the open-state
                            // signal so the matching tab can be identified
                            // later. The DOM lookup uses the
                            // integer-derived `kb_tab_id` instead.
                            let kb_menu_name = name.clone();
                            let kb_tab_id = tab_id.clone();
                            // Plumbing for the Delete keybind: the closure
                            // resolves the disabled flag at event time
                            // (cheap, runs only on Delete keystroke) by
                            // reading meta + state, so render does not pay
                            // an O(N) descendants_of cost per tab.
                            let kb_delete_name = name.clone();
                            let kb_ctx = ctx.clone();
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
                                // Per-tab keydown for Shift+F10 (open the
                                // context menu) and Delete (open the F4
                                // confirm). Arrow / Home / End navigation
                                // is handled by `TabsList`'s coordinator;
                                // we stop_propagation on the keys we
                                // handle here so the coordinator does not
                                // also see them.

                                // Shift+F10 → open the context menu anchored
                                // to this tab's bounding-rect. Dioxus 0.7
                                // does not expose `get_client_rect` on
                                // `MountedData`, so we ride the DOM via
                                // `document::eval` and parse the JSON
                                // [left, bottom] result back into the
                                // open-state signal.
                                if evt.key() == Key::F10 && evt.modifiers().shift() {
                                    evt.prevent_default();
                                    evt.stop_propagation();
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

                                // Delete → opens F4 destructive-confirm.
                                // Same disabled rules as the context-menu
                                // Delete item: root tabs and any tab whose
                                // subtree contains the startup mode are
                                // immune.
                                if evt.key() == Key::Delete {
                                    evt.prevent_default();
                                    evt.stop_propagation();
                                    let modes_snapshot = key_modes.clone();
                                    let startup = kb_ctx.meta.read().startup_mode.clone();
                                    let descendants = kb_ctx
                                        .state
                                        .read()
                                        .active_profile
                                        .as_ref()
                                        .and_then(|p| {
                                            p.modes().descendants_of(&kb_delete_name).ok()
                                        })
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
                                }
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
                                    TabButton {
                                        key: "{name}",
                                        id: name.clone(),
                                        label: name.clone(),
                                        dom_id: tab_id,
                                        title: name.clone(),
                                        data_mode: name.clone(),
                                        running: show_marker,
                                        running_sr_label: "Engine running".to_owned(),
                                        aria_haspopup: "menu".to_owned(),
                                        aria_expanded: menu_open,
                                        // Only emit aria-controls while the
                                        // menu is mounted, pointing at a
                                        // missing id confuses AT.
                                        aria_controls: menu_open.then_some(menu_id),
                                        oncontextmenu,
                                        onkeydown,
                                    }
                                }
                            }
                        }
                    }
                }
                // T31: tail `+` add tab, sibling of the tablist (under
                // TabsRoot but outside TabsList) so AT tab counts stay
                // honest, the `+` is not a real tab.
                if *adding.read() {
                    add_inline::AddInline { open: adding, pending_focus }
                } else {
                    button {
                        r#type: "button",
                        class: "if-mode-tab--add",
                        onclick: move |_| adding.set(true),
                        "aria-label": "Add mode",
                        "+"
                    }
                }
                // The context menu lives outside the tablist so it does
                // not disrupt the flex layout. Rendered once for whichever
                // tab is currently open; flag-derivation walks the active
                // profile's mode tree to compute "subtree contains
                // startup" precisely.
                {
                    // Bind the read result into an owned Option so the
                    // signal's read guard drops before any inner branch
                    // can call `open_for_tab.set(None)` (the stale-name
                    // recovery path), avoiding a borrow conflict.
                    let open_snapshot = open_for_tab.read().as_ref().cloned();
                    if let Some((open_name, _)) = open_snapshot {
                        let modes_for_flags = modes_now.clone();
                        let m = ctx.meta.read();
                        let startup = m.startup_mode.clone();
                        let current_mode = m.current_mode.clone();
                        let has_profile = m.profile_name.is_some();
                        drop(m);

                        // Resolve descendants once; both the menu's
                        // Delete-flag and the keydown Delete arm share the
                        // same helper for a single source of truth.
                        let descendants = {
                            let s = ctx.state.read();
                            s.active_profile
                                .as_ref()
                                .and_then(|p| p.modes().descendants_of(&open_name).ok())
                                .unwrap_or_default()
                        };

                        let is_startup = startup.as_ref().is_some_and(|s| s == &open_name);
                        let already_current = current_mode == open_name;
                        // Numeric index of the open tab. Used by the
                        // context menu to derive its DOM id and
                        // aria-labelledby target (which point at the
                        // integer-derived tab id, never the raw mode
                        // name, see JS-injection note above). If the
                        // open name is no longer in the modes list (a
                        // benign rename/delete race) we clear the signal
                        // and render nothing rather than fall back to
                        // index 0, which would point AT at the wrong
                        // tab.
                        if let Some(open_tab_idx) =
                            modes_for_flags.iter().position(|m| m == &open_name)
                        {
                            let flags = context_menu::ContextMenuFlags {
                                activate_disabled: already_current,
                                rename_disabled: !has_profile,
                                delete_disabled: logic::delete_disabled_for_tab(
                                    &open_name,
                                    &modes_for_flags,
                                    startup.as_deref(),
                                    &descendants,
                                ),
                                set_default_disabled: is_startup,
                            };

                            let mut focus_writer = pending_focus;

                            rsx! {
                                context_menu::ModeTabContextMenu {
                                    tab_name: open_name.clone(),
                                    tab_idx: open_tab_idx,
                                    open: open_for_tab,
                                    flags,
                                    on_close: move |(n, reason): (String, context_menu::CloseReason)| {
                                        // Tab key: the browser's natural
                                        // traversal is moving focus to the
                                        // next focusable element; re-focusing
                                        // the tab here would fight that
                                        // intent. For every other close path
                                        // (Escape / click-outside /
                                        // ItemActivated) the tab is the
                                        // natural landing focus, route the
                                        // request through TabsRoot's
                                        // focus_request signal so TabsList
                                        // hits the matching `TabButton`'s
                                        // ref.
                                        if matches!(reason, context_menu::CloseReason::Tab) {
                                            return;
                                        }
                                        focus_writer.set(Some(n));
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
                            open_for_tab.set(None);
                            rsx! {}
                        }
                    } else {
                        rsx! {}
                    }
                }
            }
        }
    }
}
