use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    ActionTone, BadgeVariant, Button, ButtonPresentation, ButtonSize, ButtonVariant, Card,
    CardPadding, Cluster, Icon, IconButton, Stack, StatusDot, ToggleChip,
};
use inputforge_gui_dx::icons::{Icon as IconKind, IconSize};

#[component]
pub(super) fn ActionsGallery() -> Element {
    let mut selected = use_signal(|| false);
    let mut expanded = use_signal(|| false);
    rsx! {
        section {
            h2 { "Icon" }
            Card { padding: CardPadding::Md,
                Cluster { gap: "--space-4".to_owned(),
                    Icon { name: IconKind::Joystick, size: IconSize::Sm }
                    Icon { name: IconKind::Joystick, size: IconSize::Md }
                    Icon { name: IconKind::Joystick, size: IconSize::Lg }
                    Icon { name: IconKind::Settings }
                    Icon { name: IconKind::Save }
                    Icon { name: IconKind::Trash }
                }
            }
        }

        section {
            h2 { "Button" }
            Card { padding: CardPadding::Md,
                Stack { gap: "--space-3".to_owned(),
                    Cluster { gap: "--space-3".to_owned(),
                        Button { variant: ButtonVariant::Primary, "Primary" }
                        Button { variant: ButtonVariant::Secondary, "Secondary" }
                        Button { variant: ButtonVariant::Ghost, "Ghost" }
                        Button { variant: ButtonVariant::Danger, "Danger" }
                        Button { disabled: true, "Disabled" }
                    }
                    Cluster { gap: "--space-3".to_owned(),
                        Button { size: ButtonSize::Sm, "Small" }
                        Button { size: ButtonSize::Md, "Medium" }
                        Button { size: ButtonSize::Lg, "Large" }
                    }
                }
            }
        }

        section {
            h2 { "IconButton" }
            Card { padding: CardPadding::Md,
                Cluster { gap: "--space-3".to_owned(),
                    IconButton { icon: IconKind::Settings, label: "Settings" }
                    IconButton {
                        icon: IconKind::Save,
                        label: "Save",
                        variant: ButtonVariant::Primary,
                    }
                    IconButton {
                        icon: IconKind::Trash,
                        label: "Delete",
                        variant: ButtonVariant::Danger,
                    }
                    IconButton { icon: IconKind::Eye, label: "Show", disabled: true }
                }
            }
        }
        section {
            h2 { "Action presentations" }
            Stack { gap: "--space-3",
                Button {
                    presentation: ButtonPresentation::Navigation,
                    variant: ButtonVariant::Ghost,
                    selected: true,
                    aria_current: "page",
                    "Mappings"
                }
                Button {
                    presentation: ButtonPresentation::Disclosure,
                    variant: ButtonVariant::Ghost,
                    aria_expanded: expanded(),
                    aria_controls: "gallery-disclosure-panel",
                    onclick: move |_| expanded.toggle(),
                    "Preview details"
                }
                div { id: "gallery-disclosure-panel", hidden: !expanded(), "Disclosure content" }
                Button {
                    presentation: ButtonPresentation::Status,
                    tone: ActionTone::Live,
                    aria_pressed: true,
                    StatusDot { variant: BadgeVariant::Success }
                    "Running"
                }
                Button {
                    presentation: ButtonPresentation::Row,
                    selected: selected(),
                    aria_pressed: selected(),
                    onclick: move |_| selected.toggle(),
                    "Selectable preview row"
                }
                ToggleChip {
                    pressed: selected(),
                    compact: true,
                    onclick: move |_| selected.toggle(),
                    "Selected filter"
                }
                Button { presentation: ButtonPresentation::Compact, "Compact action" }
                Button {
                    presentation: ButtonPresentation::Create,
                    variant: ButtonVariant::GhostPrimary,
                    Icon { name: IconKind::Plus }
                    "New preview"
                }
                Button { variant: ButtonVariant::GhostDanger, "Delete preview" }
                Button { variant: ButtonVariant::GhostPrimary, "Primary ghost" }
                IconButton {
                    icon: IconKind::Trash,
                    label: "Delete preview item",
                    variant: ButtonVariant::GhostDanger,
                }
                Button { variant: ButtonVariant::Ghost, tone: ActionTone::Warning, "Warning" }
                Button { variant: ButtonVariant::Ghost, tone: ActionTone::Error, "Error" }
            }
        }
    }
}
