//! Menu rendering and app-supplied handlers.
use super::{MENU_ID_COUNTER, MenuState, merge_class};
use dioxus::prelude::*;
use std::sync::atomic::Ordering;
/// Render a trigger-attached menu with app-controlled open state.
#[component]
pub fn MenuRoot(#[props(default)] class: Option<String>, children: Element) -> Element {
    let open = use_signal(|| false);
    let menu_id = use_signal(|| {
        format!(
            "if-menu-{}",
            MENU_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });
    let trigger_id = use_signal(|| format!("{}-trigger", menu_id.read()));
    let behavior = (crate::components::behavior().overlays.menu)(crate::overlays::MenuProps {
        open,
        trigger: trigger_id.read().clone(),
        onclose: None,
    });
    let state = MenuState {
        open,
        menu_id,
        trigger_id,
        behavior,
    };
    use_context_provider(|| state);

    let combined = merge_class("if-menu", "", class.as_deref());
    rsx! { div { class: "{combined}", {children} } }
}
