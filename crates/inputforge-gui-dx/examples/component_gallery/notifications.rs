use dioxus::prelude::*;
use inputforge_gui_dx::ToastLevel;
use inputforge_gui_dx::components::{Button, Card, CardPadding, Cluster, Stack};

#[component]
pub(super) fn NotificationsGallery() -> Element {
    let toast_queue = use_context::<inputforge_gui_dx::ToastQueue>();
    rsx! {
        section {
            h2 { "Toasts" }
            Card { padding: CardPadding::Md,
                Stack { gap: "--space-3".to_owned(),
                    Cluster { gap: "--space-3".to_owned(),
                        Button { onclick: move |_| toast_queue.push(ToastLevel::Info, "Info toast"),
                            "Push Info"
                        }
                        Button { onclick: move |_| toast_queue.push(ToastLevel::Success, "Saved successfully"),
                            "Push Success"
                        }
                        Button { onclick: move |_| toast_queue.push(ToastLevel::Warning, "HidHide unavailable"),
                            "Push Warning"
                        }
                        Button { onclick: move |_| toast_queue.push(ToastLevel::Error, "Engine failed to start"),
                            "Push Error"
                        }
                    }
                    Cluster { gap: "--space-3".to_owned(),
                        Button {
                            onclick: move |_| {
                                for _ in 0..10 {
                                    toast_queue.push(ToastLevel::Warning, "Spammy");
                                }
                            },
                            "Push spam (×10)"
                        }
                        Button {
                            onclick: move |_| {
                                for i in 0..7 {
                                    toast_queue.push(ToastLevel::Info, format!("Distinct {i}"));
                                }
                            },
                            "Push 7 distinct"
                        }
                    }
                    p { style: "color: var(--color-text-muted); font-size: var(--text-sm);",
                        "Hover or focus a toast to pause its timer; press ESC while focused to dismiss; click × to dismiss."
                    }
                }
            }
        }
    }
}
