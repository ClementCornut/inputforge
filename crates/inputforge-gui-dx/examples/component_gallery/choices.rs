use dioxus::prelude::*;
use inputforge_gui_dx::components::{Card, CardPadding, Checkbox, Cluster, Slider, Stack, Switch};

#[component]
pub(super) fn ChoicesGallery() -> Element {
    rsx! {
        section {
            h2 { "Slider" }
            Card { padding: CardPadding::Md,
                div { style: "max-width: 320px;",
                    Stack { gap: "--space-3".to_owned(),
                        Slider {
                            id: "gallery-slider-1",
                            aria_label: "Enabled slider",
                            value: 0.5,
                        }
                        Slider {
                            id: "gallery-slider-2",
                            aria_label: "Disabled slider",
                            value: 0.25,
                            disabled: true,
                        }
                    }
                }
            }
        }

        section {
            h2 { "Switch" }
            Card { padding: CardPadding::Md,
                Cluster { gap: "--space-4".to_owned(),
                    Switch {
                        id: "gallery-switch-1",
                        aria_label: "Off switch",
                        checked: false,
                        label: "Off".to_owned(),
                    }
                    Switch {
                        id: "gallery-switch-2",
                        aria_label: "On switch",
                        checked: true,
                        label: "On".to_owned(),
                    }
                    Switch {
                        id: "gallery-switch-3",
                        aria_label: "Disabled switch",
                        checked: false,
                        disabled: true,
                        label: "Disabled".to_owned(),
                    }
                }
            }
        }

        section {
            h2 { "Checkbox" }
            Card { padding: CardPadding::Md,
                Cluster { gap: "--space-4".to_owned(),
                    Checkbox {
                        id: "gallery-checkbox-1",
                        aria_label: "Unchecked checkbox",
                        checked: false,
                    }
                    Checkbox {
                        id: "gallery-checkbox-2",
                        aria_label: "Checked checkbox",
                        checked: true,
                    }
                    Checkbox {
                        id: "gallery-checkbox-3",
                        aria_label: "Mixed checkbox",
                        checked: false,
                        indeterminate: true,
                    }
                    Checkbox {
                        id: "gallery-checkbox-4",
                        aria_label: "Disabled checkbox",
                        checked: false,
                        disabled: true,
                    }
                }
            }
        }
    }
}
