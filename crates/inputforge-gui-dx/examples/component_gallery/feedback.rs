use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Badge, BadgeVariant, Button, Card, CardPadding, Chip, ChipVariant, Cluster, Separator,
    SeparatorOrientation, Spinner, SpinnerSize, Tooltip, TooltipPlacement,
};

#[component]
pub(super) fn FeedbackGallery() -> Element {
    rsx! {
        section {
            h2 { "Card" }
            Cluster { gap: "--space-3".to_owned(),
                Card { padding: CardPadding::Sm, "Small padding" }
                Card { padding: CardPadding::Md, "Medium padding" }
                Card { padding: CardPadding::Lg, "Large padding" }
            }
        }

        section {
            h2 { "Badge" }
            Cluster { gap: "--space-2".to_owned(),
                Badge { variant: BadgeVariant::Neutral, "Neutral" }
                Badge { variant: BadgeVariant::Info, "Info" }
                Badge { variant: BadgeVariant::Success, "Success" }
                Badge { variant: BadgeVariant::Warning, "Warning" }
                Badge { variant: BadgeVariant::Error, "Error" }
            }
        }

        section {
            h2 { "Chip" }
            Cluster { gap: "--space-2".to_owned(),
                Chip { variant: ChipVariant::Outline, "Outline" }
                Chip { variant: ChipVariant::Output, "Device 2 · X" }
                // Hover this chip to confirm the `title` prop
                // forwards to the underlying readout (parity with
                // Badge `title`, used in the rail's qualifier
                // chips and elsewhere).
                Chip {
                    variant: ChipVariant::Outline,
                    title: "Hover for tooltip".to_owned(),
                    "Hover me"
                }
                Chip { variant: ChipVariant::Capture, kind: "axis", "AXIS 0" }
                Chip { variant: ChipVariant::Capture, kind: "button", "BTN 5" }
                Chip { variant: ChipVariant::Capture, kind: "hat", "HAT 0" }
                Chip { variant: ChipVariant::Capture, listening: true, "Listening…" }
            }
        }

        section {
            h2 { "Separator" }
            Separator {}
            // Vertical separator demo: explicit height keeps the separator
            // visible without a surrounding flex stretch, not a Cluster fit.
            div { style: "display: flex; gap: var(--space-3); align-items: center; height: 30px;",
                span { "Left" }
                Separator { orientation: SeparatorOrientation::Vertical }
                span { "Right" }
            }
        }

        section {
            h2 { "Spinner" }
            Cluster { gap: "--space-3".to_owned(),
                Spinner { size: SpinnerSize::Sm }
                Spinner { size: SpinnerSize::Md }
                Spinner { size: SpinnerSize::Lg }
            }
        }

        section {
            h2 { "Tooltip" }
            Card { padding: CardPadding::Md,
                Cluster { gap: "--space-6".to_owned(),
                    Tooltip {
                        id: "gallery-tooltip-1",
                        content: "Hovers up".to_owned(),
                        Button { aria_describedby: "gallery-tooltip-1", "Top" }
                    }
                    Tooltip {
                        id: "gallery-tooltip-2",
                        content: "Hovers down".to_owned(),
                        placement: TooltipPlacement::Bottom,
                        Button { aria_describedby: "gallery-tooltip-2", "Bottom" }
                    }
                    Tooltip {
                        id: "gallery-tooltip-3",
                        content: "Hovers left".to_owned(),
                        placement: TooltipPlacement::Left,
                        Button { aria_describedby: "gallery-tooltip-3", "Left" }
                    }
                    Tooltip {
                        id: "gallery-tooltip-4",
                        content: "Hovers right".to_owned(),
                        placement: TooltipPlacement::Right,
                        Button { aria_describedby: "gallery-tooltip-4", "Right" }
                    }
                }
            }
        }
    }
}
