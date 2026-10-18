//! Batch group actions and row composition.
use super::super::labels::row_key;
use super::Row;
use crate::batch_mapping::{Control, Group as GroupModel, Intent, RowKind, RowState};
use dioxus::prelude::*;
#[component]
pub(in crate::frame::bulk_map) fn Group(
    model: GroupModel,
    rows: Vec<RowState>,
    conflicting: Vec<bool>,
    on_intent: Callback<Intent>,
) -> Element {
    let title = match model.kind {
        RowKind::Axis => "Axes",
        RowKind::Button => "Buttons",
        RowKind::Hat => "Hats",
    };
    let kind = model.kind;
    rsx! {
        div {
            role: "rowgroup",
            class: "if-bulk-map__group",
            div {
                role: "row",
                class: "if-bulk-map__group-header",
                span {
                    class: "if-bulk-map__group-title",
                    "{title} ({rows.len()})"
                }
                if model.controls.contains(&Control::Skip) {
                    Chip {
                        label: "Skip all conflicts",
                        onclick: move |_| {
                            on_intent
                                .call(Intent::ReplaceConflicts {
                                    kind,
                                    replace: false,
                                });
                        },
                    }
                }
                if model.controls.contains(&Control::Replace) {
                    Chip {
                        label: "Replace all conflicts",
                        onclick: move |_| {
                            on_intent
                                .call(Intent::ReplaceConflicts {
                                    kind,
                                    replace: true,
                                });
                        },
                    }
                }
                if model.controls.contains(&Control::Include) {
                    Chip {
                        label: "Include all",
                        onclick: move |_| on_intent.call(Intent::Include(kind)),
                    }
                }
                if model.controls.contains(&Control::Exclude) {
                    Chip {
                        label: "Exclude all",
                        onclick: move | _
                                | on_intent.call(Intent::Exclude(kind)),
                    }
                }
            }
            for (row, conflict) in rows.iter().zip(conflicting) {
                Row {
                    key: "{row_key(row)}",
                    row: row.clone(),
                    is_conflicting: conflict,
                    targets: model.targets.clone(),
                    on_intent,
                }
            }
        }
    }
}
#[component]
fn Chip(label: &'static str, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        crate::components::Button {
            size: crate::components::ButtonSize::Sm,
            variant: crate::components::ButtonVariant::Ghost,
            onclick,
            "{label}"
        }
    }
}
