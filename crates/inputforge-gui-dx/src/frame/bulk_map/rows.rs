//! Prepared row and group rendering with app callback wiring.
use super::{
    labels::{format_output_value, source_row_label, target_options},
    row_readout::RowReadout,
};
use crate::{
    batch_mapping::{Intent, RowKind, RowState},
    components::Select,
};
use dioxus::prelude::*;
#[path = "groups.rs"]
mod groups;
pub(super) use groups::Group;
use inputforge_core::types::OutputAddress;
#[component]
fn Row(
    row: RowState,
    is_conflicting: bool,
    targets: Vec<OutputAddress>,
    on_intent: Callback<Intent>,
) -> Element {
    let kind_letter = match row.kind {
        RowKind::Axis => "A",
        RowKind::Button => "B",
        RowKind::Hat => "H",
    };
    let row_class = match row.kind {
        RowKind::Axis => "if-bulk-map__row if-bulk-map__row--axis",
        RowKind::Button => "if-bulk-map__row if-bulk-map__row--button",
        RowKind::Hat => "if-bulk-map__row if-bulk-map__row--hat",
    };
    let source_cell_class = match row.kind {
        RowKind::Axis => "if-bulk-map__source-cell if-bulk-map__source-cell--axis",
        RowKind::Button => "if-bulk-map__source-cell if-bulk-map__source-cell--button",
        RowKind::Hat => "if-bulk-map__source-cell if-bulk-map__source-cell--hat",
    };
    let source_label = source_row_label(row.kind, row.source_index);
    let target_options = target_options(&targets);
    let current = row
        .target
        .as_ref()
        .map_or_else(|| "(do not map)".to_owned(), format_output_value);
    let mut select_value = use_signal(|| current.clone());
    if select_value.peek().as_str() != current.as_str() {
        select_value.set(current.clone());
    }
    let id_attr = format!("bulk-map-row-{kind_letter}-{}", row.source_index);
    let on_target_change = {
        let kind = row.kind;
        let source_index = row.source_index;
        move |evt: FormEvent| {
            let value = evt.value();
            on_intent.call(Intent::RowTarget {
                kind,
                index: source_index,
                value,
            });
        }
    };
    let select_ro: ReadSignal<String> = select_value.into();
    let onclick = move |_| on_intent.call(Intent::Replace(row.kind, row.source_index));
    let show_replace = is_conflicting || row.replace;
    rsx! {
        div {
            role: "row",
            class: "{row_class}",
            span {
                role: "gridcell",
                class: "if-bulk-map__kind",
                "{kind_letter}"
            }
            span {
                role: "gridcell",
                class: "{source_cell_class}",
                span {
                    class: "if-bulk-map__source",
                    "{source_label}"
                }
                span {
                    class: "if-bulk-map__live-cell",
                    RowReadout {
                        kind: row.kind,
                        address: row.input.clone(),
                    }
                }
            }
            span {
                role: "gridcell",
                class: "if-bulk-map__target",
                Select {
                    id: Some(id_attr),
                    aria_label: format!("Output for {source_label}"),
                    value: select_ro,
                    onchange: on_target_change,
                    options: target_options,
                }
            }
            if show_replace {
                span {
                    role: "gridcell",
                    class: "if-bulk-map__action",
                    crate::components::ToggleChip {
                        compact: true,
                        pressed: row.replace,
                        aria_label: format!("Replace conflict for {source_label}"),
                        onclick,
                        if row.replace {
                            "Replacing"
                        } else {
                            "Replace"
                        }
                    }
                }
            } else {
                span {
                    role: "gridcell",
                    class: "if-bulk-map__action if-bulk-map__action--empty",
                    "aria-hidden": "true",
                }
            }
        }
    }
}
