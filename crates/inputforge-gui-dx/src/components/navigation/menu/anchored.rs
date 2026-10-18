//! Cursor menu rendering, measurement and app-supplied handlers.
use super::placement::{MenuPlacement, MenuSize, Viewport, cursor_placement};
use super::{
    ClickAwayListener, CloseReason, MENU_ID_COUNTER, MenuAnchor, MenuState, Portal, merge_class,
};
use dioxus::prelude::*;
use std::sync::atomic::Ordering;

/// Render a measured cursor menu; the owning workflow acknowledges closure.
#[component]
pub fn AnchoredMenu(
    open: Option<MenuAnchor>,
    on_close: Callback<CloseReason, bool>,
    /// Optional menu ID for a trigger's `aria-controls` relationship.
    #[props(default)]
    id: Option<String>,
    #[props(default)] return_focus: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let mut menu_id = use_signal(|| {
        id.clone().unwrap_or_else(|| {
            format!(
                "if-menu-{}",
                MENU_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
            )
        })
    });
    if let Some(id) = id
        && *menu_id.peek() != id
    {
        menu_id.set(id);
    }
    let mut open_signal = use_signal(|| open.is_some());
    if *open_signal.peek() != open.is_some() {
        open_signal.set(open.is_some());
    }
    let trigger_id = use_signal(String::new);
    let behavior = (crate::components::behavior().overlays.menu)(crate::overlays::MenuProps {
        open: open_signal,
        trigger: return_focus.unwrap_or_default(),
        onclose: Some(on_close),
    });
    let state = MenuState {
        open: open_signal,
        menu_id,
        trigger_id,
        behavior,
    };
    use_context_provider(|| state);
    let mut placement: Signal<Option<(MenuAnchor, MenuPlacement)>> = use_signal(|| None);
    let mut mounted = use_signal(|| false);
    use_effect(use_reactive!(|open| {
        placement.set(None);
        if open.is_none() {
            if *mounted.peek() {
                mounted.set(false);
            }
            return;
        }
        if !mounted() {
            return;
        }
        if let Some(cursor) = open {
            let id = menu_id.read().clone();
            spawn(async move {
                let mut eval = document::eval(&format!(
                    "({})({id:?});",
                    include_str!("../../../../assets/components/navigation/measure_cursor_menu.js")
                ));
                if let Ok([width, height, vp_width, vp_height]) = eval.recv::<[f64; 4]>().await
                    && width > 0.0
                    && height > 0.0
                {
                    placement.set(Some((
                        cursor,
                        cursor_placement(
                            cursor,
                            MenuSize { width, height },
                            Viewport {
                                width: vp_width,
                                height: vp_height,
                            },
                        ),
                    )));
                }
            });
        }
    }));
    let measured = placement
        .read()
        .filter(|(cursor, _)| Some(*cursor) == open)
        .map(|(_, position)| position);
    (crate::components::behavior().overlays.focus)(
        open.is_some(),
        measured.is_some(),
        menu_id.read().clone(),
    );
    let Some(_) = open else {
        return rsx! {};
    };
    let id = menu_id.read().clone();
    let key_id = id.clone();
    let style = match measured {
        Some(position) => format!(
            "position: fixed; left: {}px; top: {}px; visibility: visible; z-index: var(--z-menu);",
            position.left, position.top
        ),
        None => "position: fixed; left: 0; top: 0; visibility: hidden; z-index: var(--z-menu);"
            .to_owned(),
    };
    let class = merge_class("if-menu__list", "", class.as_deref());
    let onkeydown = move |event| behavior.key.call((event, key_id.clone()));
    let onmounted = move |_| mounted.set(true);
    rsx! {
        Portal {
            ClickAwayListener {
                target_id: id.clone(),
                on_click_away: move |()| behavior.close.call(CloseReason::ClickOutside),
                div {
                    class,
                    id,
                    role: "menu",
                    tabindex: "-1",
                    "aria-labelledby": aria_labelledby,
                    style,
                    onkeydown,
                    onmounted,
                    {children}
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "anchored_tests.rs"]
mod tests;
