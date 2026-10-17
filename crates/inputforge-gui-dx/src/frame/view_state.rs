//! Renderer-owned navigation signals; app callbacks own all transitions.
use crate::context::MetaSnapshot;
pub(crate) use crate::snapshots::MappingKey;
use dioxus::prelude::*;
#[cfg(test)]
pub use inputforge_gui_dx::PanelSlot;
#[cfg(not(test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code, reason = "Used by regions in Task 18+")]
pub enum PanelSlot {
    #[default]
    None,
    Devices,
    Profiles,
    Settings,
}

pub(crate) use crate::navigation::{Publication as ViewState, Surface as MainSurface};
pub(crate) fn use_view_state_provider(meta: Signal<MetaSnapshot>) -> ViewState {
    let main_surface = use_signal(MainSurface::default);
    let editing_mode = use_signal(|| "Default".to_owned());
    let local_panel = use_signal(PanelSlot::default);
    let local_calibration = use_signal(|| false);
    let navigation = try_consume_context::<crate::EditorNavigation>();
    let panel_slot = navigation.map_or(local_panel, |n| n.panel);
    let via_calibration = navigation.map_or(local_calibration, |n| n.calibration);
    let selected_mapping = use_signal(|| None);
    let view = ViewState {
        main_surface,
        editing_mode,
        panel_slot,
        via_calibration,
        selected_mapping,
    };
    #[cfg(test)]
    {
        let bindings = use_hook(|| {
            try_consume_context::<crate::mapping_edits::Bindings>().or_else(|| {
                let fixture = try_consume_context::<crate::test_support::Fixture>()?;
                Some(
                    inputforge_app::mapping_edits::Controller::new(
                        std::sync::Arc::clone(&fixture.state),
                        fixture.commands.clone(),
                    )
                    .bindings(),
                )
            })
        });
        if let Some(b) = bindings {
            use_context_provider(|| b);
        }
    }
    let bindings = try_consume_context::<crate::mapping_edits::Bindings>();
    let inherited = try_consume_context::<ViewState>();
    use_hook(|| {
        if inherited.is_none()
            && let Some(b) = bindings
        {
            b.navigation.install.call(view);
        }
    });
    use_effect(move || {
        let _ = meta.read();
        if let Some(b) = bindings {
            b.navigation.reconcile.call(());
        }
    });
    view
}
