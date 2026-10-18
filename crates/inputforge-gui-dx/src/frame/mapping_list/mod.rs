//! F8 mapping list (left rail).
//!
//! Composition (inside-out, in dependency order):
//!   - `source_label::format`, `InputAddress` -> "TFM Throttle . Z" formatter
//!   - `group::group_of`     , bucketing by `InputId` kind
//!   - `filter::matches_filter`, name + source-label substring match
//!   - `row::Row`            , single row component
//!   - `rename_inline::RenameInline`, inline rename
//!   - `add_inline::AddInline`, `+ Add mapping` capture state machine
//!   - `empty::EmptyZeroMappings` / `empty::EmptyZeroFilterResults`
//!   - `keyboard::handle_key`, Up/Down/Enter/Cmd-F/Esc
//!   - `MappingList` (this fn), orchestrates everything
#![allow(
    dead_code,
    reason = "Sub-modules expose APIs that orchestrator + Tasks 20-22 consume; \
              clippy's reachability check loses some pub(crate) items here."
)]
mod add_inline;
mod controls;
mod delete_dialog;
mod rows;
use controls::{DeviceFilterRow, FilterInput};
use delete_dialog::DeleteDialogMount;
use rows::Rows;
mod context_menu;
use context_menu::ContextMenuMount;
mod add_inline_rendering;
mod empty;
mod empty_rail;
use empty_rail::EmptyRail;
mod filter;
mod group;
mod rename_inline;
mod row;
pub(crate) use crate::labels as source_label;
#[cfg(test)]
mod rename_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod workflow_events_tests;
use crate::components::sortable::{SortableLiveRegion, use_sortable_state};
use crate::context::AppContext;
use crate::frame::mapping_list::add_inline::AddInline;
use crate::frame::view_state::{MappingKey, ViewState};
use crate::patterns::live_capture::LiveCapture;
use crate::{
    mapping_edits::Bindings,
    mapping_rail::{Menu, Target},
};
use dioxus::prelude::*;
use inputforge_core::types::{DeviceId, InputAddress};
const MAPPING_LIST_CSS: Asset = asset!("/assets/frame/mapping_list.css");
#[component]
pub(crate) fn MappingList() -> Element {
    tracing::trace!(target : "frame::render", region = "mapping_list");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();
    let bindings = use_context::<Bindings>();
    let feedback = use_signal(|| None::<String>);
    let drag_target: Signal<Option<Menu>> = use_signal(|| None);
    let editing = view.editing_mode;
    let filter_query: Signal<String> = use_signal(String::new);
    let filter_focused: Signal<bool> = use_signal(|| false);
    let renaming: Signal<Option<InputAddress>> = use_signal(|| None);
    let selected_device: Signal<Option<DeviceId>> = use_signal(|| None);
    let force_expand_add: Signal<bool> = use_signal(|| false);
    let menu_open: Signal<Option<(InputAddress, f64, f64)>> = use_signal(|| None);
    let delete_target: Signal<Option<Target>> = use_signal(|| None);
    let pending_duplicate: Signal<Option<Target>> = use_signal(|| None);
    let sortable = use_sortable_state::<u32>(crate::components::behavior().rail_sortable);
    let publication = crate::mapping_edits::ui::Rail {
        filter: crate::mapping_edits::ui::Filter {
            query: filter_query,
            device: selected_device,
        },
        prepared: use_signal(|| None),
        menu: menu_open,
        rename: renaming,
        duplicate: pending_duplicate,
        delete: delete_target,
        error: feedback,
        live: sortable.live_announcement,
    };
    use_context_provider(|| publication);
    let navigation = bindings.navigation;
    let projection = use_memo(move || {
        navigation.filter.call(crate::navigation::FilterRequest {
            config: ctx.config.read().clone(),
            mode: editing.read().clone(),
            query: filter_query.read().clone(),
            device: selected_device.read().clone(),
        })
    });
    let view_state_memo = use_memo(move || {
        let p = projection.read();
        (p.total, p.rows.clone())
    });
    let device_chips_memo = use_memo(move || projection.read().chips.clone());
    use_effect(move || {
        let p = projection.read();
        let mut device = selected_device;
        if *device.peek() != p.device {
            device.set(p.device.clone());
        }
    });
    let nav_rows_memo = use_memo(move || {
        view_state_memo
            .read()
            .1
            .iter()
            .map(|r| (r.mode.clone(), r.input.clone()))
            .collect::<Vec<MappingKey>>()
    });
    let capture = use_context::<LiveCapture>();
    let keyboard = use_hook(|| {
        bindings
            .navigation
            .rail
            .call(crate::navigation::RailPublication {
                query: filter_query,
                focused: filter_focused,
                device: selected_device,
                rows: nav_rows_memo,
                config: ctx.config,
                meta: ctx.meta,
                capture: capture.active,
                feedback,
                announcement: sortable.live_announcement,
                focus: Callback::new(|selector: String| crate::frame::focus(&selector)),
            })
    });
    use_drop(move || keyboard.close.call(()));
    let navigation_context = format!(
        "{:?}:{}",
        ctx.meta.read().profile_id,
        view.editing_mode.read(),
    );
    let (total, rows) = {
        let snapshot = view_state_memo.read();
        (snapshot.0, snapshot.1.clone())
    };
    let filter_active = projection.read().filtered;
    if total == 0 || (filter_active && rows.is_empty()) {
        return rsx! {
            EmptyRail {
                total,
                keyboard,
                focused: filter_focused,
                force_expanded: force_expand_add,
                pending_duplicate,
                chips: device_chips_memo.read()
                            .clone(),
                navigation_context,
            }
        };
    }
    rsx! {
        Stylesheet {
            href: MAPPING_LIST_CSS,
        }
        if let Some(error) = feedback.read().as_ref() {
            crate::components::InlineFeedback {
                tone: crate::components::FeedbackTone::Error,
                alert: true,
                "{error}"
            }
        }
        div {
            class: "if-rail",
            "data-navigation-context": navigation_context.clone(),
            FilterInput {
                value: filter_query,
                focused: filter_focused,
                events: keyboard,
            }
            DeviceFilterRow {
                chips: device_chips_memo.read().clone(),
                selected: selected_device,
                events: keyboard,
            }
            div {
                class: "if-rail__scroll",
                Rows {
                    rows,
                    renaming,
                    sortable,
                    drag_target,
                    filter_active,
                }
            }
            div {
                class: "if-rail__add-sticky",
                AddInline {
                    force_expanded: force_expand_add,
                    pending_duplicate,
                }
            }
            ContextMenuMount {
                filtered: projection.read().filtered,
            }
            DeleteDialogMount {
                delete_target,
            }
            SortableLiveRegion {
                state: sortable,
            }
        }
    }
}
