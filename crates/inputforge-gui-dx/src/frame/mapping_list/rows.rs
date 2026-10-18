//! Mapping groups, sortable gaps and prepared row callback wiring.
use super::{
    group::{GroupKind, group_of},
    row::{Row, group_to_u32},
};
use crate::components::sortable::SortableGap;
use crate::components::sortable::SortableState;
use crate::context::{AppContext, MappingSummary};
use crate::frame::view_state::ViewState;
use crate::mapping_edits::Bindings;
use crate::mapping_rail::{Group, Lookup, Menu, Scope};
use dioxus::prelude::*;
use inputforge_core::types::InputAddress;
#[component]
pub(super) fn Rows(
    rows: Vec<MappingSummary>,
    renaming: Signal<Option<InputAddress>>,
    sortable: SortableState<u32>,
    drag_target: Signal<Option<Menu>>,
    filter_active: bool,
) -> Element {
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let bindings = use_context::<Bindings>();
    let publication = use_context::<crate::mapping_edits::ui::Rail>();
    let drag_context = format!(
        "profile:{:?};mode:{}",
        ctx.meta.read().profile_id,
        view.editing_mode.read().as_str(),
    );
    let ctx_drag = ctx.clone();
    let on_drag_start = Callback::new(move |summary: MappingSummary| {
        bindings
            .ui
            .drag
            .call(crate::mapping_edits::ui::DragRequest {
                publication: drag_target,
                filter: publication.filter,
                lookup: Lookup {
                    scope: Scope {
                        profile: ctx_drag.meta.peek().profile_id.clone(),
                        mode: summary.mode,
                    },
                    input: summary.input,
                    name: summary.name,
                    filtered: filter_active,
                },
            });
    });
    let group_iter = GroupKind::ordered().into_iter().filter_map(|group| {
        let group_rows: Vec<MappingSummary> = rows
            .iter()
            .filter(|r| group_of(&r.input) == group)
            .cloned()
            .collect();
        if group_rows.is_empty() {
            return None;
        }
        let group_id = group_to_u32(group);
        let group_len = group_rows.len();
        let drop_handler = EventHandler::new(move |gap_index: usize| {
            bindings.ui.rail.call((
                publication,
                crate::mapping_edits::ui::RailEvent::Drop {
                    drag: drag_target,
                    group: match group {
                        GroupKind::Axes => Group::Axes,
                        GroupKind::Buttons => Group::Buttons,
                        GroupKind::Hats => Group::Hats,
                    },
                    gap: gap_index,
                },
            ));
        });
        let row_items: Vec<(usize, MappingSummary, bool)> = group_rows
            .into_iter()
            .enumerate()
            .map(|(i, row)| {
                let is_active = view
                    .selected_mapping
                    .read()
                    .as_ref()
                    .is_some_and(|(m, x)| m == &row.mode && x == &row.input);
                (i, row, is_active)
            })
            .collect();
        Some(rsx! {
            div {
                class: "if-rail__group",
                "data-sortable-group": "{sortable.drag_from.id():?}",
                "data-sortable-context": "{drag_context}",
                div {
                    class: "if-rail__group-header",
                    span {
                        class: "if-rail__group-header__label",
                        {group.header()}
                    }
                    span {
                        class: "if-rail__group-header__count",
                        "{group_len}"
                    }
                }
                SortableGap {
                    key: "gap-{group_id}-0",
                    state: sortable,
                    gap_index: 0_usize,
                    group: group_id,
                    on_drop: drop_handler,
                }
                for (i, row, is_active) in row_items {
                    {
                        rsx! {
                            Row {
                                key: "{row.input:?}-{row.mode}",
                                summary: row.clone(),
                                is_active,
                                renaming,
                                sortable,
                                filter_active,
                                on_drag_start: Some(on_drag_start),
                                on_open_menu: {
                                    let lookup = Lookup {
                                        scope: Scope {
                                            profile: ctx.meta.peek().profile_id.clone(),
                                            mode: row.mode.clone(),
                                        },
                                        input: row.input.clone(),
                                        name: row.name.clone(),
                                        filtered: filter_active,
                                    };
                                    move |(_input, x, y): (InputAddress, f64, f64)| {
                                        bindings
                                            .ui
                                            .rail
                                            .call((
                                                publication,
                                                crate::mapping_edits::ui::RailEvent::Open {
                                                    lookup: lookup.clone(),
                                                    x,
                                                    y,
                                                },
                                            ));
                                    }
                                },
                            }
                            SortableGap {
                                state: sortable,
                                gap_index: i + 1,
                                group: group_id,
                                on_drop: drop_handler,
                            }
                        }
                    }
                }
            }
        })
    });
    rsx! {
        {group_iter}
    }
}
