//! Menu rendering and app-supplied handlers.
use super::{
    Anchor, ClickAwayListener, CloseReason, MenuPlacement, MenuSize, MenuState, Portal,
    TriggerRect, Viewport, compute_placement, merge_class,
};
use dioxus::prelude::*;
/// Render a measured floating list and forward its native key observations.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro may emit redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures; defensive allow."
)]
pub fn MenuItems(
    #[props(default)] class: Option<String>,
    #[props(default)] anchor: Anchor,
    children: Element,
) -> Element {
    let state = use_context::<MenuState>();
    let open_signal = state.open;
    let menu_id = state.menu_id.read().clone();
    let trigger_id = state.trigger_id.read().clone();

    let mut placement: Signal<Option<MenuPlacement>> = use_signal(|| None);

    let menu_id_for_eval = menu_id.clone();
    let trigger_id_for_eval = trigger_id.clone();
    let anchor_for_eval = anchor;
    use_effect(move || {
        let is_open = *open_signal.read();
        if !is_open {
            placement.set(None);
            return;
        }
        let menu_id_owned = menu_id_for_eval.clone();
        let trigger_id_owned = trigger_id_for_eval.clone();
        spawn(async move {
            let mut handle = document::eval(&format!(
                "var t = document.getElementById('{trigger_id_owned}');\n\
                 var m = document.getElementById('{menu_id_owned}');\n\
                 if (!t || !m) {{ dioxus.send([0, 0, 0, 0, 0, 0, 0, 0]); return; }}\n\
                 var tr = t.getBoundingClientRect();\n\
                 var mr = m.getBoundingClientRect();\n\
                 dioxus.send([tr.left, tr.top, tr.right, tr.bottom, \
                              mr.width, mr.height, \
                              window.innerWidth, window.innerHeight]);"
            ));
            if let Ok(value) = handle.recv::<[f64; 8]>().await {
                let [t_left, t_top, t_right, t_bottom, m_w, m_h, vp_w, vp_h] = value;
                if m_w > 0.0 && m_h > 0.0 {
                    let p = compute_placement(
                        TriggerRect {
                            left: t_left,
                            top: t_top,
                            right: t_right,
                            bottom: t_bottom,
                        },
                        MenuSize {
                            width: m_w,
                            height: m_h,
                        },
                        Viewport {
                            width: vp_w,
                            height: vp_h,
                        },
                        anchor_for_eval,
                    );
                    placement.set(Some(p));
                }
            }
        });
    });

    let close = state.behavior.close;
    let key = state.behavior.key;
    let key_id = menu_id.clone();
    let onkeydown = move |event| key.call((event, key_id.clone()));
    (super::super::behavior().overlays.focus)(
        *open_signal.read(),
        placement.read().is_some(),
        menu_id.clone(),
    );

    let is_open = *open_signal.read();
    if !is_open {
        let combined = merge_class("if-menu__list", "", class.as_deref());
        return rsx! {
            div {
                class: "{combined}",
                id: "{menu_id}",
                role: "menu",
                tabindex: "-1",
                "aria-hidden": "true",
                style: "display: none;",
                {children}
            }
        };
    }

    let style = match *placement.read() {
        None => "position: fixed; left: 0; top: 0; visibility: hidden; z-index: 1001;".to_owned(),
        Some(p) => format!(
            "position: fixed; left: {left}px; top: {top}px; visibility: visible; z-index: 1001;",
            left = p.left,
            top = p.top,
        ),
    };

    let combined = merge_class("if-menu__list", "", class.as_deref());
    let menu_id_for_caw = menu_id.clone();
    let trigger_id_for_caw = trigger_id.clone();
    rsx! {
        Portal {
            ClickAwayListener {
                target_id: menu_id_for_caw,
                exclude_id: Some(trigger_id_for_caw),
                on_click_away: move |()| close.call(CloseReason::ClickOutside),
                div {
                    class: "{combined}",
                    id: "{menu_id}",
                    role: "menu",
                    tabindex: "-1",
                    style: "{style}",
                    onkeydown,
                    {children}
                }
            }
        }
    }
}
