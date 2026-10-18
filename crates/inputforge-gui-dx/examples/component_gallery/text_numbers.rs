use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Card, CardPadding, Cluster, Field, InputSize, NumberInput, Stack, TextInput,
};

#[component]
pub(super) fn TextNumbersGallery() -> Element {
    let mut number_demo = use_signal(|| 50.0_f64);
    let mut number_demo_b = use_signal(|| 50.0_f64);
    let mut number_demo_precision = use_signal(|| 0.123_456_f64);
    let mut number_demo_size = use_signal(|| 0.0_f64);
    rsx! {
        section {
            h2 { "TextInput" }
            Card { padding: CardPadding::Md,
                // max-width belongs to the demo, not to Stack, keep on a wrapper div.
                div { style: "max-width: 320px;",
                    Stack { gap: "--space-3".to_owned(),
                        TextInput {
                            id: "gallery-textinput-1",
                            aria_label: "Default text",
                            value: "hello".to_owned(),
                            placeholder: "Type here…".to_owned(),
                        }
                        TextInput {
                            id: "gallery-textinput-2",
                            aria_label: "Disabled text",
                            value: String::new(),
                            placeholder: "Disabled".to_owned(),
                            disabled: true,
                        }
                        TextInput {
                            id: "gallery-textinput-3",
                            aria_label: "Invalid text",
                            value: "wrong".to_owned(),
                            invalid: true,
                        }
                        TextInput {
                            id: "gallery-textinput-4",
                            aria_label: "Small text",
                            value: "Small".to_owned(),
                            size: InputSize::Sm,
                        }
                        TextInput {
                            id: "gallery-textinput-5",
                            aria_label: "Medium text",
                            value: "Medium".to_owned(),
                            size: InputSize::Md,
                        }
                        TextInput {
                            id: "gallery-textinput-6",
                            aria_label: "Large text",
                            value: "Large".to_owned(),
                            size: InputSize::Lg,
                        }
                    }
                }
            }
        }

        section {
            h2 { "NumberInput" }
            Card { padding: CardPadding::Md,
                Stack { gap: "--space-3".to_owned(),
                    Cluster { gap: "--space-3".to_owned(),
                        NumberInput {
                            id: "gallery-numberinput-1",
                            aria_label: "Stepped number",
                            value: number_demo(),
                            min: 0.0,
                            max: 100.0,
                            step: 1.0,
                            onstep: move |v| number_demo.set(v),
                        }
                        NumberInput {
                            id: "gallery-numberinput-2",
                            aria_label: "Independent stepped number",
                            value: number_demo_b(),
                            min: 0.0,
                            max: 100.0,
                            step: 1.0,
                            onstep: move |v| number_demo_b.set(v),
                        }
                        NumberInput {
                            id: "gallery-numberinput-3",
                            aria_label: "Disabled number",
                            value: 0.0,
                            disabled: true,
                        }
                        NumberInput {
                            id: "gallery-numberinput-4",
                            aria_label: "Decimal precision",
                            value: number_demo_precision(),
                            step: 0.01,
                            precision: 2,
                            onstep: move |v| number_demo_precision.set(v),
                        }
                    }
                    Cluster { gap: "--space-3".to_owned(),
                        NumberInput {
                            id: "gallery-numberinput-5",
                            aria_label: "Small number",
                            value: number_demo_size(),
                            step: 1.0,
                            size: InputSize::Sm,
                            onstep: move |v| number_demo_size.set(v),
                        }
                        NumberInput {
                            id: "gallery-numberinput-6",
                            aria_label: "Medium number",
                            value: number_demo_size(),
                            step: 1.0,
                            size: InputSize::Md,
                            onstep: move |v| number_demo_size.set(v),
                        }
                        NumberInput {
                            id: "gallery-numberinput-7",
                            aria_label: "Large number",
                            value: number_demo_size(),
                            step: 1.0,
                            size: InputSize::Lg,
                            onstep: move |v| number_demo_size.set(v),
                        }
                    }
                }
            }
        }

        section {
            h2 { "Field + Label" }
            Card { padding: CardPadding::Md,
                div { style: "max-width: 320px;",
                    Stack { gap: "--space-4".to_owned(),
                        // Field couples label↔input by passing the same string
                        // to `for_id` (Field) and `id` (the wrapped input).
                        Field {
                            label: "Profile name".to_owned(),
                            for_id: "profile-name".to_owned(),
                            helper: "Used in dropdowns.".to_owned(),
                            helper_id: "gallery-profile-name-help",
                            required: true,
                            TextInput {
                                id: "profile-name".to_owned(),
                                aria_describedby: "gallery-profile-name-help",
                                value: String::new(),
                                placeholder: "My profile".to_owned(),
                            }
                        }
                        Field {
                            label: "Sensitivity".to_owned(),
                            for_id: "sensitivity".to_owned(),
                            error: "Must be between 0 and 1.".to_owned(),
                            error_id: "gallery-sensitivity-error",
                            NumberInput {
                                id: "sensitivity".to_owned(),
                                invalid: true,
                                aria_describedby: "gallery-sensitivity-error",
                                aria_errormessage: "gallery-sensitivity-error",
                                value: 1.5,
                                min: 0.0,
                                max: 1.0,
                                step: 0.01,
                                precision: 2,
                            }
                        }
                    }
                }
            }
        }
    }
}
