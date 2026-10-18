//! Mapping source qualifiers retain their domain glyphs and titles.
use crate::components::{Chip, ChipVariant};
use dioxus::prelude::*;
#[component]
pub(super) fn Qualifiers(merge: Option<String>, conditional: Option<String>) -> Element {
    rsx! {
                if merge.is_some() || conditional.is_some() {
                    div {
                        class: "if-row__source-qualifiers",
                        if let Some(secondary_label) = merge {
                            Chip {
                                variant: ChipVariant::Outline,
                                class: "if-row__qualifier glyph-merge"
                                        .to_owned(),
                                title: format!("Merge: {secondary_label}"),
                                span {
                                    class: "if-row__qualifier-glyph",
                                    "+"
                                }
                                span {
                                    class: "if-row__qualifier-text",
                                    "{secondary_label}"
                                }
                            }
                        }
                        if let Some(predicate_label) = conditional {
                            Chip {
                                variant: ChipVariant::Outline,
                                class: "if-row__qualifier glyph-cond"
                                        .to_owned(),
                                title: format!("Condition: {predicate_label}"),
                                span {
                                    class: "if-row__qualifier-glyph",
                                    "\u{2295}"
                                }
                                span {
                                    class: "if-row__qualifier-text",
                                    "{predicate_label}"
                                }
                            }
                        }
                    }
                }
    }
}
