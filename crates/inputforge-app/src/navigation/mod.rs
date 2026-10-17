//! Editor transitions share mapping mutation guards and mounted ownership.
mod editor;
mod editor_keys;
mod expansion;
mod filter;
mod listener;
mod rail;
mod rail_keys;
mod reconcile;
use super::Controller;
use dioxus::prelude::*;
pub(crate) use filter::active as filter_active;
use inputforge_core::profile::ProfileId;
use inputforge_gui_dx::{
    PanelSlot,
    navigation::{Bindings, Intent, Publication, Surface},
};
#[derive(Debug, Default)]
pub(super) struct State {
    publication: Option<Publication>,
    profile: Option<ProfileId>,
    mode: String,
    editor: Option<inputforge_gui_dx::navigation::EditorPublication>,
    selection: Option<inputforge_gui_dx::snapshots::MappingKey>,
}
impl Controller {
    pub(super) fn navigation_bindings(&self) -> Bindings {
        let install_editor = self.clone();
        let editor = self.clone();
        let rail = self.clone();
        let interaction = self.clone();
        let install = self.clone();
        let dispatch = self.clone();
        let reconcile = self.clone();
        Bindings {
            install_editor: Callback::new(move |p| {
                install_editor.inner.borrow_mut().chrome.editor = Some(p);
            }),
            editor: Callback::new(move |p| editor::mount(editor.clone(), p)),
            rail: Callback::new(move |p| rail::mount(rail.clone(), p)),
            stage: Callback::new(move |r| interaction.stage_event(r)),
            filter: Callback::new(filter_rows),
            install: Callback::new(move |p: Publication| {
                if install.inner.borrow().chrome.publication.is_none()
                    && p.selected_mapping.peek().is_some()
                {
                    let state = install.state.read();
                    let mut inner = install.inner.borrow_mut();
                    inner.chrome.profile = state.active_profile.as_ref().map(|p| p.id().clone());
                    inner.chrome.mode.clone_from(&p.editing_mode.peek());
                    inner
                        .chrome
                        .selection
                        .clone_from(&p.selected_mapping.peek());
                }
                install.inner.borrow_mut().chrome.publication = Some(p);
                install.reconcile_navigation();
            }),
            dispatch: Callback::new(move |i| dispatch.navigate_editor(i)),
            reconcile: Callback::new(move |()| reconcile.reconcile()),
        }
    }
    fn navigate_editor(&self, intent: Intent) {
        let Some(mut p) = self
            .inner
            .borrow()
            .chrome
            .publication
            .filter(|_| !self.inner.borrow().disposed)
        else {
            return;
        };
        let Some(state) = self.state.try_read() else {
            return;
        };
        let has_profile = state.active_profile.is_some();
        if let Intent::Select {
            ref profile,
            ref key,
        } = intent
        {
            if state
                .active_profile
                .as_ref()
                .map(inputforge_core::profile::Profile::id)
                != profile.as_ref()
            {
                return;
            }
            if key.0 != *p.editing_mode.peek() {
                return;
            }
            let mut inner = self.inner.borrow_mut();
            inner.history.reconcile(&state, self.owner);
            if inner.history.current(&state, key).is_none() {
                return;
            }
        }
        drop(state);
        match intent {
            Intent::Surface(surface) => {
                if surface == Surface::BulkMap && !has_profile {
                    return;
                }
                p.main_surface.set(surface);
                if surface == Surface::BulkMap {
                    p.panel_slot.set(PanelSlot::None);
                }
            }
            Intent::TogglePanel(panel) => {
                if panel == PanelSlot::Devices && !has_profile {
                    return;
                }
                let active = *p.panel_slot.peek() == panel
                    && !(panel == PanelSlot::Devices && *p.via_calibration.peek());
                p.panel_slot
                    .set(if active { PanelSlot::None } else { panel });
                if !active {
                    p.via_calibration.set(false);
                }
            }
            Intent::OpenPanel(panel) => {
                p.panel_slot.set(panel);
                p.via_calibration.set(false);
            }
            Intent::Select { key, .. } => p.selected_mapping.set(Some(key)),
        }
    }
}

fn filter_rows(
    r: inputforge_gui_dx::navigation::FilterRequest,
) -> inputforge_gui_dx::navigation::FilterView {
    let chips = filter::device_chips_for_mode(&r.config.mappings, &r.mode, &r.config);
    let device = r.device.filter(|d| chips.iter().any(|c| c.id == *d));
    let mode_rows = r.config.mappings.iter().filter(|m| m.mode == r.mode);
    let total = mode_rows.clone().count();
    let mut rows: Vec<_> = mode_rows
        .filter(|m| {
            filter::matches_filter(m, &r.query, &r.config)
                && filter::matches_device_filter(m, device.as_ref())
        })
        .cloned()
        .collect();
    rows.sort_by_key(|row| match super::rail::reorder::group(&row.input) {
        Some(inputforge_gui_dx::mapping_rail::Group::Axes) => 0,
        Some(inputforge_gui_dx::mapping_rail::Group::Buttons) => 1,
        Some(inputforge_gui_dx::mapping_rail::Group::Hats) => 2,
        None => 3,
    });
    inputforge_gui_dx::navigation::FilterView {
        filtered: filter::active(&r.query, device.as_ref()),
        total,
        rows,
        chips,
        device,
    }
}
