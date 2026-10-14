//! Prepared row and group rendering with app callback wiring.
use super::{
    labels::{format_output_value, row_key, source_row_label, target_options},
    row_readout::RowReadout,
};
use crate::{
    batch_mapping::{Control, Group as GroupModel, Intent, RowKind, RowState},
    components::Select,
};
use dioxus::prelude::*;
use inputforge_core::types::OutputAddress;
#[component]
pub(super) fn Group(
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
        div { role: "rowgroup", class: "if-bulk-map__group",
            div { role: "row", class: "if-bulk-map__group-header",
                span { class: "if-bulk-map__group-title", "{title} ({rows.len()})" }
                    if model.controls.contains(&Control::Skip) { Chip { label: "Skip all conflicts", onclick: move |_| on_intent.call(Intent::ReplaceConflicts { kind, replace: false }) } }
                    if model.controls.contains(&Control::Replace) { Chip { label: "Replace all conflicts", onclick: move |_| on_intent.call(Intent::ReplaceConflicts { kind, replace: true }) } }
                    if model.controls.contains(&Control::Include) { Chip { label: "Include all", onclick: move |_| on_intent.call(Intent::Include(kind)) } }
                    if model.controls.contains(&Control::Exclude) { Chip { label: "Exclude all", onclick: move |_| on_intent.call(Intent::Exclude(kind)) } }
            }
            for (row, conflict) in rows.iter().zip(conflicting) {
                Row { key: "{row_key(row)}", row: row.clone(), is_conflicting: conflict, targets: model.targets.clone(), on_intent }
            }
        }
    }
}
#[component]
fn Chip(label: &'static str, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! { button { r#type: "button", class: "if-bulk-map__chip", onclick, "{label}" } }
}
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
        div { role: "row", class: "{row_class}",
            span { role: "gridcell", class: "if-bulk-map__kind", "{kind_letter}" }
            span { role: "gridcell", class: "{source_cell_class}",
                span { class: "if-bulk-map__source", "{source_label}" }
                span { class: "if-bulk-map__live-cell",
                    RowReadout { kind: row.kind, address: row.input.clone() }
                }
            }
            span { role: "gridcell", class: "if-bulk-map__target",
                Select {
                    id: Some(id_attr),
                    value: select_ro,
                    onchange: on_target_change,
                    options: target_options,
                }
            }
            if show_replace {
                span { role: "gridcell", class: "if-bulk-map__action",
                    button {
                        r#type: "button",
                        class: if row.replace {
                            "if-bulk-map__chip if-bulk-map__chip--active"
                        } else {
                            "if-bulk-map__chip"
                        },
                        "aria-pressed": "{row.replace}",
                        onclick,
                        if row.replace { "Replacing" } else { "Replace" }
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
