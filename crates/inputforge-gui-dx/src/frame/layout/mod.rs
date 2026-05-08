mod empty_state;

use dioxus::prelude::*;

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: LAYOUT_CSS }"
)]
const LAYOUT_CSS: Asset = asset!("/assets/frame/layout.css");

use crate::context::AppContext;
use crate::frame::panel_slot::PanelSlot as PanelSlotComponent;
use crate::frame::status_bar::StatusBar;
use crate::frame::top_bar::{
    ModeDeleteDialog, ModeDeleteSignal, ModeFocusSignal, ModeTabs, TopBar,
};
use crate::frame::view_state::{MainSurface, ViewState};

pub(crate) use empty_state::EmptyState;

/// F7 layout shell: top bar, primary workspace plus optional right panel,
/// or empty-state, status bar.
#[component]
pub(crate) fn Layout() -> Element {
    tracing::trace!(target: "frame::render", region = "layout");
    let ctx = use_context::<AppContext>();
    // Calling `use_context::<ViewState>()` here is a structural panic guard:
    // every region component (ModeTabs, ToolsCluster, etc.) reads ViewState
    // via `use_context`, and Dioxus panics with an opaque error if no
    // provider is in scope. Failing here keeps the panic readable
    // ("ViewState provider missing in app_root") and centralized.
    let view = use_context::<ViewState>();
    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());
    let main_surface = use_memo(move || *view.main_surface.read());
    let p = *has_profile.read();
    let s = *main_surface.read();
    let delete_target = use_signal(|| Option::<String>::None);
    use_context_provider(|| ModeDeleteSignal(delete_target));
    let mode_focus = use_signal(|| Option::<String>::None);
    use_context_provider(|| ModeFocusSignal(mode_focus));

    rsx! {
        div { class: "if-layout",
            Stylesheet { href: LAYOUT_CSS }
            TopBar {}
            ModeDeleteDialog {}
            if p {
                div { class: "if-layout__main",
                    div { class: "if-layout__surface",
                        match s {
                            MainSurface::Mappings => rsx! { MappingWorkspace {} },
                            MainSurface::BulkMap => rsx! { BulkMapWorkspace {} },
                        }
                    }
                    PanelSlotComponent {}
                }
            } else {
                EmptyState {}
            }
            StatusBar {}
        }
    }
}

#[component]
fn MappingWorkspace() -> Element {
    rsx! {
        div { class: "if-layout__mapping-workspace",
            div { class: "if-layout__mode-strip",
                ModeTabs {}
            }
            div { class: "if-layout__mapping-body",
                div { class: "if-layout__rail",
                    crate::frame::MappingList {}
                }
                div { class: "if-layout__center", crate::frame::MappingEditor {} }
            }
        }
    }
}

#[component]
fn BulkMapWorkspace() -> Element {
    rsx! {
        div { class: "if-layout__bulk-map",
            crate::frame::bulk_map::BulkMapPanel {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::sync::{Arc, mpsc};

    use dioxus_ssr::render;
    use inputforge_core::mode::ModeTree;
    use inputforge_core::profile::Profile;
    use inputforge_core::settings::AppSettings;
    use inputforge_core::state::AppState;
    use parking_lot::RwLock;

    use crate::context::{ConfigSnapshot, LiveSnapshot, MetaSnapshot};

    #[allow(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn BulkMapHarness() -> Element {
        let (tx, _) = mpsc::channel();
        let profile = Profile::new(
            "T".to_owned(),
            Vec::new(),
            ModeTree::from_adjacency(&HashMap::from([("Default".to_owned(), vec![])]))
                .expect("single Default mode tree is valid"),
            Vec::new(),
            Vec::new(),
            "Default".to_owned(),
        );
        let ctx = AppContext {
            state: Arc::new(RwLock::new(AppState::with_profile(profile))),
            commands: tx,
            settings: Arc::new(AppSettings::default()),
            meta: use_signal(|| MetaSnapshot {
                profile_name: Some("T".to_owned()),
                startup_mode: Some("Default".to_owned()),
                modes: vec!["Default".to_owned()],
                ..MetaSnapshot::default()
            }),
            config: use_signal(ConfigSnapshot::default),
            live: use_signal(LiveSnapshot::default),
        };
        use_context_provider(|| ctx.clone());
        let mut view = crate::frame::use_view_state_provider(ctx.meta);
        view.main_surface.set(MainSurface::BulkMap);
        use_context_provider(|| view);

        rsx! { Layout {} }
    }

    #[test]
    fn bulk_map_surface_hides_mapping_rail_and_editor() {
        let mut vdom = VirtualDom::new(BulkMapHarness);
        vdom.rebuild_in_place();
        let html = render(&vdom);

        assert!(
            html.contains("if-layout__bulk-map"),
            "bulk surface missing: {html}"
        );
        assert!(html.contains("if-bulk-map"), "bulk panel missing: {html}");
        assert!(
            !html.contains("if-layout__rail"),
            "mapping rail must not render in Bulk-map mode: {html}"
        );
        assert!(
            !html.contains("if-layout__center"),
            "mapping editor must not render in Bulk-map mode: {html}"
        );
    }
}
