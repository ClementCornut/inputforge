use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Button, ButtonVariant, Card, CardPadding, Cluster, DialogBody, DialogDescription, DialogFooter,
    DialogRoot, DialogTitle, Stack,
};
use inputforge_gui_dx::patterns::DirtyConfirmDialog;

#[component]
pub(super) fn DialogsGallery() -> Element {
    let mut basic_open = use_signal(|| false);
    let mut non_dismiss_open = use_signal(|| false);
    let mut backdrop_open = use_signal(|| false);
    let mut scroll_open = use_signal(|| false);
    let mut dirty_a_open = use_signal(|| false);
    let mut dirty_b_open = use_signal(|| false);
    let mut dirty_a_outcome = use_signal(String::new);
    let mut dirty_b_outcome = use_signal(String::new);
    rsx! {
        section {
            h2 { "Dialog primitives" }
            Card { padding: CardPadding::Md,
                Cluster { gap: "--space-3".to_owned(),
                    Button { onclick: move |_| basic_open.set(true), "Basic" }
                    Button { onclick: move |_| non_dismiss_open.set(true), "Non-dismissible" }
                    Button { onclick: move |_| backdrop_open.set(true), "Close-on-backdrop" }
                    Button { onclick: move |_| scroll_open.set(true), "Scrollable body" }
                }
            }

            DialogRoot { open: basic_open, onclose: move |()| true,
                DialogTitle { "Basic dialog" }
                DialogDescription { "ESC dismisses; backdrop click does not." }
                DialogBody {}
                DialogFooter {
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| basic_open.set(false),
                        "Close"
                    }
                }
            }

            DialogRoot {
                open: non_dismiss_open,
                onclose: move |()| true,
                dismissible: false,
                DialogTitle { "Non-dismissible dialog" }
                DialogDescription { "ESC and backdrop click do nothing. Only the explicit Close button resolves this." }
                DialogBody {}
                DialogFooter {
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| non_dismiss_open.set(false),
                        "Close"
                    }
                }
            }

            DialogRoot {
                open: backdrop_open,
                onclose: move |()| true,
                close_on_backdrop_click: true,
                DialogTitle { "Close-on-backdrop dialog" }
                DialogDescription { "Click outside the panel to close." }
                DialogBody {}
                DialogFooter {
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| backdrop_open.set(false),
                        "Close"
                    }
                }
            }

            DialogRoot { open: scroll_open, onclose: move |()| true,
                DialogTitle { "Scrollable body" }
                DialogDescription { "Long content scrolls inside the body region." }
                DialogBody {
                    {
                        let lines = (1..=80).map(|i| rsx! {
                            p { "Line {i}" }
                        });
                        rsx! {
                            for el in lines {
                                {el}
                            }
                        }
                    }
                }
                DialogFooter {
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| scroll_open.set(false),
                        "Close"
                    }
                }
            }
        }

        section {
            h2 { "DirtyConfirmDialog" }
            Card { padding: CardPadding::Md,
                Stack { gap: "--space-3".to_owned(),
                    Cluster { gap: "--space-3".to_owned(),
                        Button {
                            onclick: move |_| {
                                dirty_a_outcome.set(String::new());
                                dirty_a_open.set(true);
                            },
                            "Switch input (dirty)"
                        }
                        span { style: "color: var(--color-text-muted);",
                            "Last outcome: "
                            code { "{dirty_a_outcome.read()}" }
                        }
                    }
                    Cluster { gap: "--space-3".to_owned(),
                        Button {
                            onclick: move |_| {
                                dirty_b_outcome.set(String::new());
                                dirty_b_open.set(true);
                            },
                            "With custom copy"
                        }
                        span { style: "color: var(--color-text-muted);",
                            "Last outcome: "
                            code { "{dirty_b_outcome.read()}" }
                        }
                    }
                }
            }

            DirtyConfirmDialog {
                open: dirty_a_open,
                oncancel: move |()| {
                    dirty_a_outcome.set("cancel".to_owned());
                    true
                },
                ondiscard: move |()| {
                    dirty_a_outcome.set("discard".to_owned());
                    true
                },
                onsave: move |()| {
                    dirty_a_outcome.set("save".to_owned());
                    true
                },
            }

            DirtyConfirmDialog {
                open: dirty_b_open,
                title: Some("Switch profile?".to_owned()),
                message: Some("Profile X has unsaved mappings. Saving applies them now.".to_owned()),
                save_label: Some("Save & Switch".to_owned()),
                oncancel: move |()| {
                    dirty_b_outcome.set("cancel".to_owned());
                    true
                },
                ondiscard: move |()| {
                    dirty_b_outcome.set("discard".to_owned());
                    true
                },
                onsave: move |()| {
                    dirty_b_outcome.set("save".to_owned());
                    true
                },
            }
        }
    }
}
