use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::view_state::{MainSurface, PanelSlot, ViewState};

#[component]
pub(crate) fn PrimaryNav() -> Element {
    tracing::trace!(target: "frame::render", region = "primary_nav");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());
    let surface = use_memo(move || *view.main_surface.read());
    let p = *has_profile.read();
    let s = *surface.read();

    let mut main_surface = view.main_surface;
    let mut panel_slot = view.panel_slot;

    rsx! {
        nav { class: "if-primary-nav", "aria-label": "Primary workspace",
            PrimaryNavButton {
                label: "Mappings",
                active: s == MainSurface::Mappings,
                disabled: false,
                disabled_reason: "",
                onclick: move |_| main_surface.set(MainSurface::Mappings),
            }
            PrimaryNavButton {
                label: "Batch map",
                active: s == MainSurface::BulkMap,
                disabled: !p,
                disabled_reason: "Load a profile to batch map a device.",
                onclick: move |_| {
                    main_surface.set(MainSurface::BulkMap);
                    panel_slot.set(PanelSlot::None);
                },
            }
        }
    }
}

#[component]
fn PrimaryNavButton(
    label: &'static str,
    active: bool,
    disabled: bool,
    disabled_reason: &'static str,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let class = if active {
        "if-primary-nav__button is-active"
    } else {
        "if-primary-nav__button"
    };
    let title = if disabled { disabled_reason } else { "" };

    rsx! {
        button {
            r#type: "button",
            class,
            disabled,
            "aria-current": if active { "page" } else { "false" },
            title,
            onclick,
            "{label}"
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

    #[component]
    fn Harness(surface: MainSurface) -> Element {
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
        view.main_surface.set(surface);
        use_context_provider(|| view);

        rsx! { PrimaryNav {} }
    }

    fn render_nav(surface: MainSurface) -> String {
        let mut vdom = VirtualDom::new_with_props(Harness, HarnessProps { surface });
        vdom.rebuild_in_place();
        render(&vdom)
    }

    #[test]
    fn renders_mappings_and_bulk_map_destinations() {
        let html = render_nav(MainSurface::Mappings);

        assert!(html.contains("Mappings"), "Mappings item missing: {html}");
        assert!(html.contains("Batch map"), "Batch map item missing: {html}");
        assert!(
            !html.contains("Bulk-map"),
            "old bulk-map copy must not render: {html}"
        );
        assert!(
            html.contains("if-primary-nav"),
            "primary nav class missing: {html}"
        );
    }

    #[test]
    fn bulk_map_active_state_derives_from_main_surface() {
        let html = render_nav(MainSurface::BulkMap);

        assert!(
            html.contains(r#"class="if-primary-nav__button is-active""#)
                && html.contains(r#"aria-current="page""#)
                && html.contains(">Batch map</button>"),
            "Batch map must be the active primary destination: {html}"
        );
    }
}
