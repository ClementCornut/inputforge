mod empty_state;

use dioxus::prelude::*;

#[allow(
    dead_code,
    reason = "rsx! macro is opaque to rustc; constant is consumed by Stylesheet { href: LAYOUT_CSS }"
)]
const LAYOUT_CSS: Asset = asset!("/assets/frame/layout.css");

use crate::context::AppContext;
use crate::frame::panel_slot::PanelSlot as PanelSlotComponent;
use crate::frame::top_bar::{ModeDeleteDialog, ModeTabs, TopBar};
use crate::frame::view_state::{MainSurface, ViewState};

pub(crate) use empty_state::EmptyState;

/// F7 layout shell: top bar, primary workspace plus optional right panel,
/// or empty-state.
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
    let profile_scope = format!("{:?}", ctx.meta.read().profile_id);
    let p = *has_profile.read();
    let s = *main_surface.read();
    let delete_target = use_signal(|| None);
    let mode_focus = use_signal(|| Option::<String>::None);
    crate::frame::top_bar::mode_tabs::provide_publication(delete_target, mode_focus);

    rsx! {
        div { class: "if-layout",
            Stylesheet { href: LAYOUT_CSS }
            TopBar {}
            ModeDeleteDialog {}
            div { class: "if-layout__main",
                div { class: "if-layout__surface",
                    if p {
                        match s {
                            MainSurface::Mappings => rsx! { MappingWorkspace {key:"{profile_scope}"} },
                            MainSurface::BulkMap => rsx! { BulkMapWorkspace {} },
                        }
                    } else {
                        EmptyState {}
                    }
                }
                PanelSlotComponent {}
            }
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
mod acceptance_tests;
#[cfg(test)]
mod library_tests;

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, mpsc};

    use dioxus_ssr::render;
    use inputforge_core::mode::Modes;
    use inputforge_core::profile::Profile;
    use inputforge_core::state::AppState;
    use parking_lot::RwLock;

    use crate::context::{ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot};

    #[allow(
        non_snake_case,
        reason = "Dioxus components are PascalCase by convention"
    )]
    fn BulkMapHarness() -> Element {
        let (tx, _) = mpsc::channel();
        let profile = Profile::new(
            "T".to_owned(),
            Vec::new(),
            Modes::new(vec!["Default".to_owned()]).expect("single Default mode is valid"),
            Vec::new(),
            Vec::new(),
            "Default".to_owned(),
        );
        let ctx = {
            crate::test_support::provide_engine(
                Arc::new(RwLock::new(AppState::with_profile(profile))),
                tx,
            );
            AppContext {
                settings: use_signal(SettingsSnapshot::default),
                meta: use_signal(|| MetaSnapshot {
                    profile_name: Some("T".to_owned()),
                    startup_mode: Some("Default".to_owned()),
                    modes: vec!["Default".to_owned()],
                    ..MetaSnapshot::default()
                }),
                config: use_signal(ConfigSnapshot::default),
                live: use_signal(LiveSnapshot::default),
            }
        };
        use_context_provider(|| ctx.clone());
        let controller = use_hook(|| {
            inputforge_app::mapping_edits::Controller::new(
                Arc::clone(&crate::test_support::engine().state),
                crate::test_support::engine().commands.clone(),
            )
        });
        let bindings = use_hook(|| controller.bindings());
        use_context_provider(|| bindings);
        crate::routing::provide_fixture(crate::routing::View::default());
        crate::controllers::provide_fixture(crate::controllers::View::default());
        inputforge_app::notifications::use_provider();
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
