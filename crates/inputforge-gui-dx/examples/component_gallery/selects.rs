use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Card, CardPadding, Cluster, InputSize, Select, SelectOption, Stack,
};

#[component]
pub(super) fn SelectsGallery() -> Element {
    rsx! {
        section {
            h2 { "Select" }
            Card { padding: CardPadding::Md,
                Stack { gap: "--space-3".to_owned(),
                    Cluster { gap: "--space-3".to_owned(),
                        Select {
                            id: "gallery-select-1",
                            aria_label: "Default selection",
                            value: "alpha".to_owned(),
                            options: vec![
                                SelectOption {
                                    value: "alpha".to_owned(),
                                    label: "Alpha".to_owned(),
                                    disabled: false,
                                    class: None,
                                },
                                SelectOption {
                                    value: "beta".to_owned(),
                                    label: "Beta".to_owned(),
                                    disabled: false,
                                    class: None,
                                },
                                SelectOption {
                                    value: "gamma".to_owned(),
                                    label: "Gamma".to_owned(),
                                    disabled: false,
                                    class: None,
                                },
                            ],
                        }
                        Select {
                            id: "gallery-select-2",
                            aria_label: "Disabled selection",
                            value: "alpha".to_owned(),
                            options: vec![
                                SelectOption {
                                    value: "alpha".to_owned(),
                                    label: "Alpha".to_owned(),
                                    disabled: false,
                                    class: None,
                                },
                            ],
                            disabled: true,
                        }
                    }
                    Cluster { gap: "--space-3".to_owned(),
                        Select {
                            id: "gallery-select-3",
                            aria_label: "Small selection",
                            value: "alpha".to_owned(),
                            options: vec![
                                SelectOption {
                                    value: "alpha".to_owned(),
                                    label: "Small".to_owned(),
                                    disabled: false,
                                    class: None,
                                },
                            ],
                            size: InputSize::Sm,
                        }
                        Select {
                            id: "gallery-select-4",
                            aria_label: "Medium selection",
                            value: "alpha".to_owned(),
                            options: vec![
                                SelectOption {
                                    value: "alpha".to_owned(),
                                    label: "Medium".to_owned(),
                                    disabled: false,
                                    class: None,
                                },
                            ],
                            size: InputSize::Md,
                        }
                        Select {
                            id: "gallery-select-5",
                            aria_label: "Large selection",
                            value: "alpha".to_owned(),
                            options: vec![
                                SelectOption {
                                    value: "alpha".to_owned(),
                                    label: "Large".to_owned(),
                                    disabled: false,
                                    class: None,
                                },
                            ],
                            size: InputSize::Lg,
                        }
                    }
                    // Per-option disabled + class (orphan pattern from F14
                    // ChangeMode body). Renders an italic, error-tinted
                    // option that cannot be picked.
                    Cluster { gap: "--space-3".to_owned(),
                        Select {
                            id: "gallery-select-6",
                            aria_label: "Missing option selection",
                            value: "ghost".to_owned(),
                            options: vec![
                                SelectOption {
                                    value: "ghost".to_owned(),
                                    label: "(removed) Ghost".to_owned(),
                                    disabled: true,
                                    class: Some("if-select__option--orphan".to_owned()),
                                },
                                SelectOption {
                                    value: "default".to_owned(),
                                    label: "Default".to_owned(),
                                    disabled: false,
                                    class: None,
                                },
                                SelectOption {
                                    value: "combat".to_owned(),
                                    label: "Combat".to_owned(),
                                    disabled: false,
                                    class: None,
                                },
                            ],
                        }
                    }
                }
            }
        }
    }
}
