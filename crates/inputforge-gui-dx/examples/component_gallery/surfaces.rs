use dioxus::prelude::*;
use inputforge_gui_dx::components::{
    Badge, BadgeVariant, Button, ButtonPresentation, Cluster, Disclosure, EmptyState, FeedbackTone,
    InlineFeedback, PlotFrame, ReadoutCaption, Separator, Stack, StatusDot,
};

#[component]
pub(super) fn SurfacesGallery() -> Element {
    rsx! {
        section {
            h2 { "Feedback and status" }
            Stack { gap: "--space-3",
                InlineFeedback { "Helper text stays in normal flow." }
                InlineFeedback { tone: FeedbackTone::Warning, "The output driver provides this layout." }
                InlineFeedback { tone: FeedbackTone::Error, alert: true,
                    "The preview save was rejected; the draft remains."
                }
                Cluster { inline: true, nowrap: true,
                    Badge { variant: BadgeVariant::Warning, count: true, "×3" }
                    StatusDot { variant: BadgeVariant::Success, label: "Running" }
                    StatusDot { hollow: true, label: "Stopped" }
                    ReadoutCaption { "0.500" }
                }
            }
        }
        section {
            h2 { "Disclosure, empty state and plot frame" }
            Disclosure { summary: rsx! { "Advanced preview settings" },
                p { "Native disclosure owns its collapsed state and keyboard behavior." }
            }
            EmptyState {
                title: "No preview mappings",
                description: "Create a mapping to start this preview.",
                eyebrow: rsx! {
                    ReadoutCaption { "Mappings" }
                },
                actions: rsx! {
                    Button { presentation: ButtonPresentation::Create, "New mapping" }
                },
            }
            PlotFrame {
                id: "gallery-plot",
                label: "Preview response curve",
                description: "A straight response from minimum to maximum.",
                instructions: "Focus to inspect this static preview.",
                value_text: "Linear response",
                div { style: "height: 120px;", "Static plot preview" }
            }
            Separator { decorative: true, strong: true }
        }
    }
}
