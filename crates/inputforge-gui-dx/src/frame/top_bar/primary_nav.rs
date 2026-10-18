use dioxus::prelude::*;

use crate::components::{Button, ButtonPresentation, ButtonVariant};
use crate::context::AppContext;
use crate::frame::view_state::{MainSurface, ViewState};

#[component]
pub(crate) fn PrimaryNav() -> Element {
    tracing::trace!(target: "frame::render", region = "primary_nav");
    let ctx = use_context::<AppContext>();
    let view = use_context::<ViewState>();

    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());
    let surface = use_memo(move || *view.main_surface.read());
    let p = *has_profile.read();
    let s = *surface.read();

    let navigation = use_context::<crate::mapping_edits::Bindings>().navigation;

    rsx! {
        nav { class: "if-primary-nav", "aria-label": "Primary workspace",
            PrimaryNavButton {
                label: "Mappings",
                active: s == MainSurface::Mappings,
                disabled: false,
                disabled_reason: "",
                onclick: move |_| navigation.dispatch.call(crate::navigation::Intent::Surface(MainSurface::Mappings)),
            }
            PrimaryNavButton {
                label: "Batch map",
                active: s == MainSurface::BulkMap,
                disabled: !p,
                disabled_reason: "Load a profile to batch map a device.",
                onclick: move |_| {
                    navigation.dispatch.call(crate::navigation::Intent::Surface(MainSurface::BulkMap));
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
    rsx! {
        Button {
            presentation: ButtonPresentation::Navigation,
            variant: ButtonVariant::Ghost,
            selected: active,
            disabled,
            aria_current: active.then(|| "page".to_owned()),
            title: disabled.then(|| disabled_reason.to_owned()),
            onclick,
            "{label}"
        }
    }
}

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

    #[component]
    fn Harness(surface: MainSurface) -> Element {
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
            html.contains(r#"data-selected="true""#)
                && html.contains(r#"aria-current="page""#)
                && html.contains(">Batch map</button>"),
            "Batch map must be the active primary destination: {html}"
        );
    }
}
