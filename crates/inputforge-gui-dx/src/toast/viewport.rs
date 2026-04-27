use std::time::{Duration, Instant};

use dioxus::prelude::*;

use crate::components::Icon;
use crate::icons::{Icon as IconKind, IconSize};
use crate::toast::queue::ToastQueue;
use crate::toast::state::{Toast, ToastLevel};

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
/// Tick mechanism: a `use_signal(Instant::now)` Signal is updated every 250 ms
/// by a tokio interval; reading it in the body produces the per-tick re-render
/// that drives expiration GC. 250 ms is far coarser than per-frame and an
/// order of magnitude finer than the CSS fade-out duration, so cost is
/// negligible.
#[component]
pub fn ToastViewport() -> Element {
    let queue = use_context::<ToastQueue>();

    let mut now_signal = use_signal(Instant::now);
    use_future(move || async move {
        let mut t = tokio::time::interval(Duration::from_millis(250));
        loop {
            t.tick().await;
            now_signal.set(Instant::now());
        }
    });

    let now = *now_signal.read();
    let toasts = queue.visible(now);

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

    let onmouseenter = move |_| queue.pause(id);
    let onmouseleave = move |_| queue.resume(id);
    let onfocusin = move |_| queue.pause(id);
    let onfocusout = move |_| queue.resume(id);
    let onkeydown = move |e: KeyboardEvent| {
        if e.key() == Key::Escape {
            queue.dismiss(id);
        }
    };
    let onclick = move |_| queue.dismiss(id);

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
