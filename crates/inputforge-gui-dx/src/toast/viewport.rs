use dioxus::prelude::*;

use crate::components::Icon;
use crate::icons::{Icon as IconKind, IconSize};
use crate::toast::state::{Toast, ToastLevel};
use crate::toast::{Interaction, ToastQueue};

/// Renders the toast queue. Single fixed-positioned `.if-toast-stack`
/// container anchored at top-right; toasts render in chronological
/// (insertion) order regardless of level so a Success pushed after a
/// Warning lands BELOW the warning, not above it.
///
/// ARIA: the stack itself is `role="status"` / `aria-live="polite"`;
/// individual Warning/Error items override with `role="alert"` so AT
/// still announces them assertively. This keeps the visual order
/// chronological and the ARIA delivery per-level without splitting
/// the visible stack.
///
#[component]
pub fn ToastViewport() -> Element {
    let queue = use_context::<ToastQueue>();

    let toasts = queue.visible.read().clone();

    rsx! {
        div {
            class: "if-toast-stack",
            role: "status",
            "aria-live": "polite",
            for t in toasts {
                ToastItem { key: "{t.id}", toast: t }
            }
        }
    }
}

#[component]
fn ToastItem(toast: Toast) -> Element {
    let queue = use_context::<ToastQueue>();
    let id = toast.id;
    let (level_class, icon_kind) = match toast.level {
        ToastLevel::Info => ("if-toast--info", IconKind::Info),
        ToastLevel::Success => ("if-toast--success", IconKind::Check),
        ToastLevel::Warning => ("if-toast--warning", IconKind::Warning),
        ToastLevel::Error => ("if-toast--error", IconKind::Error),
    };
    // Per-item role override: Warning/Error get `role="alert"` so AT
    // announces them assertively even though the stack container is
    // `aria-live="polite"`. Info/Success inherit polite from the stack.
    let item_role = match toast.level {
        ToastLevel::Warning | ToastLevel::Error => Some("alert"),
        ToastLevel::Info | ToastLevel::Success => None,
    };
    let count = toast.count;
    let message = toast.message.clone();

    let onmouseenter = move |_| queue.event.call((id, Interaction::Hover(true)));
    let onmouseleave = move |_| queue.event.call((id, Interaction::Hover(false)));
    let onfocusin = move |_| queue.event.call((id, Interaction::Focus(true)));
    let onfocusout = move |_| queue.event.call((id, Interaction::Focus(false)));
    let onkeydown = move |e: KeyboardEvent| {
        queue.event.call((id, Interaction::Key(e.key())));
    };
    let onclick = move |_| queue.event.call((id, Interaction::Dismiss));

    rsx! {
        div {
            class: "if-toast {level_class}",
            role: item_role,
            tabindex: "0",
            onmouseenter,
            onmouseleave,
            onfocusin,
            onfocusout,
            onkeydown,
            Icon { name: icon_kind, size: IconSize::Sm }
            span { class: "if-toast__message", "{message}" }
            if count > 1 {
                span { class: "if-toast__count", "×{count}" }
            }
            button {
                class: "if-toast__close",
                "aria-label": "Dismiss",
                onclick,
                "×"
            }
        }
    }
}
