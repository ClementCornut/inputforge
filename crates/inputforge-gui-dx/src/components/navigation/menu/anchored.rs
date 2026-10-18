//! Menu rendering and app-supplied handlers.
use super::{
    ClickAwayListener, CloseReason, MENU_ID_COUNTER, MenuAnchor, MenuState, Portal, merge_class,
};
use dioxus::prelude::*;
use std::sync::atomic::Ordering;
/// Render a cursor-anchored menu; the owning workflow acknowledges closure.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro may emit redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures; defensive allow."
)]
pub fn AnchoredMenu(
    open: Option<MenuAnchor>,
    on_close: EventHandler<CloseReason>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let menu_id = use_signal(|| {
        format!(
            "if-menu-{}",
            MENU_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });

    let open_signal = use_signal(|| true);
    let trigger_id = use_signal(String::new);
    let behavior = (crate::components::behavior().overlays.menu)(crate::overlays::MenuProps {
        open: open_signal,
        trigger: String::new(),
        onclose: Some(on_close),
    });
    let state = MenuState {
        open: open_signal,
        menu_id,
        trigger_id,
        behavior,
    };
    use_context_provider(|| state);
    (crate::components::behavior().overlays.focus)(open.is_some(), true, menu_id.read().clone());

    let Some(coords) = open else {
        return rsx! {};
    };

    let menu_id_str = menu_id.read().clone();
    let menu_id_for_keys = menu_id_str.clone();

    let close = behavior.close;
    let key = behavior.key;
    let onkeydown = move |event| key.call((event, menu_id_for_keys.clone()));

    let combined = merge_class("if-menu__list", "", class.as_deref());
    let style = format!(
        "position: fixed; left: {}px; top: {}px; z-index: 1001;",
        coords.x, coords.y
    );
    let menu_id_for_caw = menu_id_str.clone();
    let close_for_caw = close;

    rsx! {
        Portal {
            ClickAwayListener {
                target_id: menu_id_for_caw,
                on_click_away: move |()| close_for_caw.call(CloseReason::ClickOutside),
                div {
                    class: "{combined}",
                    id: "{menu_id_str}",
                    role: "menu",
                    tabindex: "-1",
                    "aria-labelledby": aria_labelledby,
                    style: "{style}",
                    onkeydown,
                    {children}
                }
            }
        }
    }
}
