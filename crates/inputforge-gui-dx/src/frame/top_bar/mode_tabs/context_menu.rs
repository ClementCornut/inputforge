//! Right-click / Shift+F10 menu for a mode tab. Adopts the shared
//! `AnchoredMenu` primitive for surface, click-outside dismissal
//! (via `ClickAwayListener`), keyboard handling, and auto-focus on open.

use dioxus::prelude::*;

use inputforge_core::engine::EngineCommand;

use crate::components::{AnchoredMenu, MenuAnchor, MenuItem};
use crate::context::AppContext;

pub(crate) use crate::components::CloseReason;

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
    reason = "Each bool maps 1:1 to a menu item, parallel structure is the point."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextMenuFlags {
    pub activate_disabled: bool,
    pub rename_disabled: bool,
    pub delete_disabled: bool,
    pub set_default_disabled: bool,
}

/// Free-function dispatch path for the "Set as default" menu item. Keeps the
/// per-arm closure lean and gives tests a target they can drive without
/// mounting Dioxus state. Mirrors the closure-extraction pattern used by
/// `dispatch_add_helper` in `mapping_list::add_inline`.
fn dispatch_set_default(commands: &std::sync::mpsc::Sender<EngineCommand>, name: &str) {
    let _ = commands.send(EngineCommand::SetDefaultMode {
        name: name.to_owned(),
    });
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
    /// Index of the open tab in the modes list. The menu's
    /// `aria-labelledby` target is derived from this integer so the
    /// referenced DOM id is JS-string-safe even when the mode name
    /// contains quotes, backslashes, or `'); alert(1); //`. The mode name
    /// itself never reaches the DOM/JS layer.
    tab_idx: usize,
    /// Source of truth for whether this menu is open. Carries the anchor
    /// coordinates so the menu can position itself.
    open: Signal<Option<(String, AnchorRect)>>,
    flags: ContextMenuFlags,
    /// Called on close. The parent uses the `CloseReason` to decide
    /// whether to re-focus the originating tab, see [`CloseReason`].
    on_close: EventHandler<(String, CloseReason)>,
    /// Called when the user picks Rename, the parent enters inline-rename mode.
    on_rename: EventHandler<String>,
    /// Called when the user picks Delete, the parent opens the F4 destructive
    /// dialog with the affected count.
    on_delete: EventHandler<String>,
) -> Element {
    tracing::trace!(target: "frame::render", region = "mode_tabs::context_menu");
    let ctx = use_context::<AppContext>();
    let cmd_activate = ctx.commands.clone();
    let cmd_default = ctx.commands.clone();

    let labelled_by_owned = format!("mode-tab-{tab_idx}");

    let anchor = match open.read().as_ref() {
        Some((n, rect)) if n == &tab_name => *rect,
        _ => return rsx! {},
    };

    let activate_name = tab_name.clone();
    let rename_name = tab_name.clone();
    let delete_name = tab_name.clone();
    let default_name = tab_name.clone();

    let activate_onclick = move |_| {
        let _ = cmd_activate.send(EngineCommand::SwitchMode {
            mode: activate_name.clone(),
        });
    };

    let rename_onclick = {
        let on_rename = on_rename;
        move |_| {
            on_rename.call(rename_name.clone());
        }
    };

    let delete_onclick = {
        let on_delete = on_delete;
        move |_| {
            on_delete.call(delete_name.clone());
        }
    };

    let default_onclick = move |_| {
        dispatch_set_default(&cmd_default, &default_name);
    };

    let anchor = MenuAnchor {
        x: anchor.left,
        y: anchor.bottom,
    };

    rsx! {
        AnchoredMenu {
            open: Some(anchor),
            on_close: move |reason: CloseReason| {
                let mut open = open;
                open.set(None);
                on_close.call((tab_name.clone(), reason));
            },
            aria_labelledby: labelled_by_owned,
            class: "if-modetab-context-menu".to_owned(),
            MenuItem {
                disabled: flags.activate_disabled,
                onclick: activate_onclick,
                "Activate"
            }
            MenuItem {
                disabled: flags.rename_disabled,
                onclick: rename_onclick,
                "Rename"
            }
            MenuItem {
                disabled: flags.delete_disabled,
                onclick: delete_onclick,
                "Delete"
            }
            MenuItem {
                disabled: flags.set_default_disabled,
                onclick: default_onclick,
                "Set as default"
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
        current: &str,
        has_profile: bool,
        descendants_of_name_contains_startup: bool,
    ) -> ContextMenuFlags {
        let is_root = modes.first().is_some_and(|first| first == name);
        let is_startup = startup.is_some_and(|s| s == name);
        let already_current = current == name;
        ContextMenuFlags {
            activate_disabled: already_current,
            rename_disabled: !has_profile,
            delete_disabled: is_root || is_startup || descendants_of_name_contains_startup,
            set_default_disabled: is_startup,
        }
    }

    #[test]
    fn delete_disabled_when_subtree_contains_startup() {
        let modes = vec!["Combat".to_owned(), "Default".to_owned()];
        let f = flags_for("Combat", &modes, Some("Default"), "Default", true, true);
        assert!(
            f.delete_disabled,
            "must reject delete when subtree holds startup"
        );
    }

    #[test]
    fn delete_enabled_for_leaf_unrelated_to_startup() {
        let modes = vec!["Default".to_owned(), "Landing".to_owned()];
        let f = flags_for("Landing", &modes, Some("Default"), "Default", true, false);
        assert!(!f.delete_disabled);
    }

    #[test]
    fn rename_disabled_when_no_profile() {
        let f = flags_for(
            "Default",
            &["Default".to_owned()],
            Some("Default"),
            "Default",
            false,
            false,
        );
        assert!(f.rename_disabled);
    }

    #[test]
    fn activate_disabled_when_already_current() {
        let modes = vec!["Default".to_owned(), "Combat".to_owned()];
        let f = flags_for("Combat", &modes, Some("Default"), "Combat", true, false);
        assert!(f.activate_disabled);
    }

    #[test]
    fn set_default_disabled_when_already_startup() {
        let modes = vec!["Default".to_owned()];
        let f = flags_for("Default", &modes, Some("Default"), "Default", true, false);
        assert!(f.set_default_disabled);
    }

    #[test]
    fn dispatch_set_default_sends_set_default_mode_command() {
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel::<EngineCommand>();
        dispatch_set_default(&tx, "Combat");
        match rx.try_recv() {
            Ok(EngineCommand::SetDefaultMode { name }) => assert_eq!(name, "Combat"),
            other => panic!("expected SetDefaultMode {{ name: \"Combat\" }}, got {other:?}"),
        }
    }

    #[test]
    fn set_as_default_item_renders_when_flag_is_enabled() {
        use std::sync::{Arc, mpsc};

        use dioxus_ssr::render;
        use inputforge_core::settings::AppSettings;
        use inputforge_core::state::AppState;
        use parking_lot::RwLock;

        use crate::context::{ConfigSnapshot, LiveSnapshot, MetaSnapshot};

        #[expect(
            non_snake_case,
            reason = "Dioxus components are PascalCase by convention"
        )]
        fn Harness() -> Element {
            let (tx, _rx) = mpsc::channel();
            let ctx = AppContext {
                state: Arc::new(RwLock::new(AppState::new())),
                commands: tx,
                settings: Arc::new(AppSettings::default()),
                meta: use_signal(MetaSnapshot::default),
                config: use_signal(ConfigSnapshot::default),
                live: use_signal(LiveSnapshot::default),
            };
            use_context_provider(|| ctx);

            let open = use_signal(|| {
                Some((
                    "Combat".to_owned(),
                    AnchorRect {
                        left: 0.0,
                        bottom: 0.0,
                    },
                ))
            });

            rsx! {
                ModeTabContextMenu {
                    tab_name: "Combat".to_owned(),
                    tab_idx: 0_usize,
                    open,
                    flags: ContextMenuFlags {
                        activate_disabled: false,
                        rename_disabled: false,
                        delete_disabled: false,
                        set_default_disabled: false,
                    },
                    on_close: |_| {},
                    on_rename: |_| {},
                    on_delete: |_| {},
                }
            }
        }

        let mut vdom = VirtualDom::new(Harness);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        assert!(
            html.contains("Set as default"),
            "menu must surface the 'Set as default' item: {html}"
        );

        let item_idx = html
            .find("Set as default")
            .expect("Set as default item must be present in rendered html");
        let preceding = &html[..item_idx];
        let item_open = preceding
            .rfind("<button")
            .expect("Set as default item must be wrapped in a <button> menuitem");
        let item_slice = &html[item_open..item_idx];
        assert!(
            !item_slice.contains(r#"aria-disabled="true""#),
            "the Set as default item must not be aria-disabled when flag is false: {item_slice}"
        );
    }
}
