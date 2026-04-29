//! Right-click / Shift+F10 menu for a mode tab. Hand-rolled floating list;
//! does not consume F2 `MenuRoot`.
//!
//! The F2 `MenuRoot` is internally state-managed (no external `open` prop)
//! and trigger-attached. Retrofitting it for a context menu would force two
//! mental models into one component, so this menu is a hand-rolled floating
//! list that reuses only the shared `focus_walker` helper.

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::menu::{FocusAction, focus_menu_item};
use crate::context::AppContext;

/// Anchor coordinates for the menu (mouse position for right-click,
/// originating tab's bounding rect for keyboard).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AnchorRect {
    pub left: f64,
    pub bottom: f64,
}

/// Disabled-state inputs (computed by the parent from meta + tab name +
/// `has_profile` + `startup_mode` + subtree).
///
/// Four bool fields by design: each maps to one of the four menu items
/// (Activate / Rename / Delete / Set as default). A state-machine or
/// two-variant enum would obscure the parallel structure with the rendered
/// list, so we override `clippy::struct_excessive_bools`.
#[allow(
    clippy::struct_excessive_bools,
    reason = "Each bool maps 1:1 to a menu item — parallel structure is the point."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextMenuFlags {
    pub activate_disabled: bool,
    pub rename_disabled: bool,
    pub delete_disabled: bool,
    pub set_default_disabled: bool,
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures (the macro suggests \
              shorthand-with-no-prop-name as a fix, which would erase the intent). \
              This is a macro-level artifact, not authored qualifications."
)]
pub(crate) fn ModeTabContextMenu(
    tab_name: String,
    /// Source of truth for whether this menu is open. Carries the anchor
    /// coordinates so the menu can position itself.
    open: Signal<Option<(String, AnchorRect)>>,
    flags: ContextMenuFlags,
    /// Called on close (any path) — gives the parent the originating tab
    /// name so it can restore focus.
    on_close: EventHandler<String>,
    /// Called when the user picks Rename — the parent enters inline-rename mode.
    on_rename: EventHandler<String>,
    /// Called when the user picks Delete — the parent opens the F4 destructive
    /// dialog with the affected count.
    on_delete: EventHandler<String>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let cmd_activate = ctx.commands.clone();
    let cmd_default = ctx.commands.clone();

    let menu_id = format!("mode-tab-menu-{tab_name}");
    let menu_id_for_keys = menu_id.clone();

    let anchor = match open.read().as_ref() {
        Some((n, rect)) if n == &tab_name => *rect,
        _ => return rsx! {},
    };

    let style = format!(
        "position: fixed; top: {bot}px; left: {left}px;",
        bot = anchor.bottom,
        left = anchor.left
    );

    let close_name_for_backdrop = tab_name.clone();
    let close_name_for_kb = tab_name.clone();
    let close_name_tab_kb = tab_name.clone();
    let close_name_for_activate = tab_name.clone();
    let close_name_for_rename = tab_name.clone();
    let close_name_for_delete = tab_name.clone();
    let close_name_for_default = tab_name.clone();
    let activate_name = tab_name.clone();
    let rename_name = tab_name.clone();
    let delete_name = tab_name.clone();
    let default_name = tab_name.clone();

    let backdrop_onclick = {
        let mut open = open;
        let on_close = on_close;
        move |_| {
            open.set(None);
            on_close.call(close_name_for_backdrop.clone());
        }
    };

    let menu_onkeydown = {
        let mut open = open;
        let on_close = on_close;
        move |evt: KeyboardEvent| match evt.key() {
            Key::Escape => {
                evt.prevent_default();
                open.set(None);
                on_close.call(close_name_for_kb.clone());
            }
            Key::ArrowDown => {
                evt.prevent_default();
                focus_menu_item(&menu_id_for_keys, FocusAction::Next);
            }
            Key::ArrowUp => {
                evt.prevent_default();
                focus_menu_item(&menu_id_for_keys, FocusAction::Prev);
            }
            Key::Home => {
                evt.prevent_default();
                focus_menu_item(&menu_id_for_keys, FocusAction::First);
            }
            Key::End => {
                evt.prevent_default();
                focus_menu_item(&menu_id_for_keys, FocusAction::Last);
            }
            Key::Tab => {
                // Let focus leave the tablist entirely; close first so
                // focus restoration target is not the menu.
                open.set(None);
                on_close.call(close_name_tab_kb.clone());
            }
            _ => {}
        }
    };

    let activate_onmounted = move |evt: MountedEvent| {
        if !flags.activate_disabled {
            spawn(async move {
                let _ = evt.data().set_focus(true).await;
            });
        }
    };
    let activate_onclick = {
        let mut open = open;
        let on_close = on_close;
        move |_| {
            if flags.activate_disabled {
                return;
            }
            let _ = cmd_activate.send(EngineCommand::ForceMode {
                mode: activate_name.clone(),
            });
            open.set(None);
            on_close.call(close_name_for_activate.clone());
        }
    };

    let rename_onclick = {
        let mut open = open;
        let on_close = on_close;
        let on_rename = on_rename;
        move |_| {
            if flags.rename_disabled {
                return;
            }
            on_rename.call(rename_name.clone());
            open.set(None);
            on_close.call(close_name_for_rename.clone());
        }
    };

    let delete_onclick = {
        let mut open = open;
        let on_close = on_close;
        let on_delete = on_delete;
        move |_| {
            if flags.delete_disabled {
                return;
            }
            on_delete.call(delete_name.clone());
            open.set(None);
            on_close.call(close_name_for_delete.clone());
        }
    };

    let default_onclick = {
        let mut open = open;
        let on_close = on_close;
        move |_| {
            if flags.set_default_disabled {
                return;
            }
            let _ = cmd_default.send(EngineCommand::SetDefaultMode {
                name: default_name.clone(),
            });
            open.set(None);
            on_close.call(close_name_for_default.clone());
        }
    };

    rsx! {
        // Backdrop catches clicks outside the menu and dismisses it.
        // aria-hidden so AT only sees the menu itself, not the overlay.
        div {
            class: "if-menu__backdrop",
            "aria-hidden": "true",
            onclick: backdrop_onclick,
        }
        ul {
            class: "if-modetab-context-menu",
            id: "{menu_id}",
            role: "menu",
            "aria-labelledby": "mode-tab-{tab_name}",
            tabindex: "-1",
            style: "{style}",
            onkeydown: menu_onkeydown,

            // Activate. onmounted on the first non-disabled item moves
            // focus into the menu when it opens.
            li { role: "none",
                button {
                    r#type: "button",
                    role: "menuitem",
                    tabindex: "-1",
                    "aria-disabled": "{flags.activate_disabled}",
                    onmounted: activate_onmounted,
                    onclick: activate_onclick,
                    "Activate"
                }
            }

            li { role: "none",
                button {
                    r#type: "button",
                    role: "menuitem",
                    tabindex: "-1",
                    "aria-disabled": "{flags.rename_disabled}",
                    onclick: rename_onclick,
                    "Rename"
                }
            }

            li { role: "none",
                button {
                    r#type: "button",
                    role: "menuitem",
                    tabindex: "-1",
                    "aria-disabled": "{flags.delete_disabled}",
                    onclick: delete_onclick,
                    "Delete"
                }
            }

            li { role: "none",
                button {
                    r#type: "button",
                    role: "menuitem",
                    tabindex: "-1",
                    "aria-disabled": "{flags.set_default_disabled}",
                    onclick: default_onclick,
                    "Set as default"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags_for(
        name: &str,
        modes: &[String],
        startup: Option<&str>,
        force: Option<&str>,
        has_profile: bool,
        descendants_of_name_contains_startup: bool,
    ) -> ContextMenuFlags {
        let is_root = modes.first().is_some_and(|first| first == name);
        let is_startup = startup.is_some_and(|s| s == name);
        let already_forced = force.is_some_and(|m| m == name);
        ContextMenuFlags {
            activate_disabled: already_forced,
            rename_disabled: !has_profile,
            delete_disabled: is_root || is_startup || descendants_of_name_contains_startup,
            set_default_disabled: is_startup,
        }
    }

    #[test]
    fn delete_disabled_when_subtree_contains_startup() {
        let modes = vec!["Combat".to_owned(), "Default".to_owned()];
        let f = flags_for("Combat", &modes, Some("Default"), None, true, true);
        assert!(
            f.delete_disabled,
            "must reject delete when subtree holds startup"
        );
    }

    #[test]
    fn delete_enabled_for_leaf_unrelated_to_startup() {
        let modes = vec!["Default".to_owned(), "Landing".to_owned()];
        let f = flags_for("Landing", &modes, Some("Default"), None, true, false);
        assert!(!f.delete_disabled);
    }

    #[test]
    fn rename_disabled_when_no_profile() {
        let f = flags_for(
            "Default",
            &["Default".to_owned()],
            Some("Default"),
            None,
            false,
            false,
        );
        assert!(f.rename_disabled);
    }

    #[test]
    fn activate_disabled_when_already_forced() {
        let modes = vec!["Default".to_owned(), "Combat".to_owned()];
        let f = flags_for(
            "Combat",
            &modes,
            Some("Default"),
            Some("Combat"),
            true,
            false,
        );
        assert!(f.activate_disabled);
    }

    #[test]
    fn set_default_disabled_when_already_startup() {
        let modes = vec!["Default".to_owned()];
        let f = flags_for("Default", &modes, Some("Default"), None, true, false);
        assert!(f.set_default_disabled);
    }
}
