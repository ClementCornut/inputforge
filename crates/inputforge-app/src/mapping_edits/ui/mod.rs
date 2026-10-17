//! Existing mapping presentation behavior shares the editor controller.
mod add;
mod header;
mod hints;
mod rail;
use super::Controller;
use dioxus::prelude::*;
use inputforge_gui_dx::mapping_edits::ui;
impl Controller {
    pub(super) fn ui_bindings(&self) -> ui::Bindings {
        let header = self.clone();
        let rename = self.clone();
        let rail = self.clone();
        let delete = self.clone();
        let add = self.clone();
        let add_state = self.clone();
        let drag = self.clone();
        ui::Bindings {
            row_menu: Callback::new(move |request: ui::RowMenuRequest| {
                request.event.prevent_default();
                request.event.stop_propagation();
                let point = request.event.client_coordinates();
                request.send.call((request.input, point.x, point.y));
            }),
            drag: Callback::new(move |mut request: ui::DragRequest| {
                if request.publication.try_peek().is_err() {
                    return;
                }
                let Some(filtered) = filtered(request.filter) else {
                    return;
                };
                request.lookup.filtered = filtered;
                request.publication.set(drag.rail_menu(request.lookup));
            }),
            header: Callback::new(move |request| header.header_ui(request)),
            rename: Callback::new(move |(publication, event)| rename.rename_ui(publication, event)),
            delete: Callback::new(move |request: ui::DeleteRequest| {
                delete.rail_delete(request.target, request.error, request.confirm)
            }),
            rail: Callback::new(move |(publication, event)| rail.rail_ui(publication, event)),
            add_state: Callback::new(move |(state, name, intent)| {
                add_state.add_submit(state, name, intent)
            }),
            add: Callback::new(move |(publication, event)| add.add_ui(publication, event)),
            combo_sync: Callback::new(
                move |(combo, capturing, mut previous, mut local): (
                    inputforge_core::types::KeyCombo,
                    bool,
                    Signal<inputforge_core::types::KeyCombo>,
                    Signal<inputforge_core::types::KeyCombo>,
                )| {
                    if !capturing && *previous.peek() != combo {
                        previous.set(combo.clone());
                        local.set(combo);
                    }
                },
            ),
            stage_hint: Callback::new(hints::publish),
            stage_close: Callback::new(
                move |(mut menu, activation, reason): (
                    Signal<Option<inputforge_gui_dx::navigation::StageMenu>>,
                    Signal<bool>,
                    inputforge_gui_dx::overlays::CloseReason,
                )| {
                    if reason != inputforge_gui_dx::overlays::CloseReason::ItemActivated
                        || *activation.peek()
                    {
                        menu.set(None);
                    }
                },
            ),
        }
    }
}

fn filtered(p: ui::Filter) -> Option<bool> {
    Some(crate::mapping_edits::navigation::filter_active(
        &p.query.try_peek().ok()?,
        p.device.try_peek().ok()?.as_ref(),
    ))
}
