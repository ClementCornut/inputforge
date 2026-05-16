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

    let main_surface = view.main_surface;
    let panel_slot = view.panel_slot;
    let destinations = primary_nav_destinations(s, p, main_surface, panel_slot);

    rsx! {
        nav { class: "if-primary-nav", "aria-label": "Primary workspace",
            for item in destinations {
            PrimaryNavButton {
                label: item.label,
                active: item.active,
                disabled: item.disabled,
                disabled_reason: item.disabled_reason,
                onclick: item.onclick,
                }
            }
        }
    }
}

struct PrimaryNavDestination {
    label: &'static str,
    active: bool,
    disabled: bool,
    disabled_reason: &'static str,
    onclick: EventHandler<()>,
}

fn primary_nav_destinations(
    current_surface: MainSurface,
    has_profile: bool,
    main_surface: Signal<MainSurface>,
    panel_slot: Signal<PanelSlot>,
) -> [PrimaryNavDestination; 3] {
    let mut mappings_surface = main_surface;
    let mut mappings_panel_slot = panel_slot;
    let mut bulk_map_surface = main_surface;
    let mut bulk_map_panel_slot = panel_slot;
    let mut sheets_surface = main_surface;
    let mut sheets_panel_slot = panel_slot;

    [
        PrimaryNavDestination {
            label: "Mappings",
            active: current_surface == MainSurface::Mappings,
            disabled: false,
            disabled_reason: "",
            onclick: EventHandler::new(move |()| {
                select_primary_surface(
                    MainSurface::Mappings,
                    &mut mappings_surface,
                    &mut mappings_panel_slot,
                );
            }),
        },
        PrimaryNavDestination {
            label: "Batch map",
            active: current_surface == MainSurface::BulkMap,
            disabled: !has_profile,
            disabled_reason: "Load a profile to batch map a device.",
            onclick: EventHandler::new(move |()| {
                select_primary_surface(
                    MainSurface::BulkMap,
                    &mut bulk_map_surface,
                    &mut bulk_map_panel_slot,
                );
            }),
        },
        PrimaryNavDestination {
            label: "Sheets",
            active: current_surface == MainSurface::Sheets,
            disabled: false,
            disabled_reason: "",
            onclick: EventHandler::new(move |()| {
                select_primary_surface(
                    MainSurface::Sheets,
                    &mut sheets_surface,
                    &mut sheets_panel_slot,
                );
            }),
        },
    ]
}

fn select_primary_surface(
    surface: MainSurface,
    main_surface: &mut Signal<MainSurface>,
    panel_slot: &mut Signal<PanelSlot>,
) {
    main_surface.set(surface);
    if surface != MainSurface::Mappings {
        panel_slot.set(PanelSlot::None);
    }
}

#[component]
fn PrimaryNavButton(
    label: &'static str,
    active: bool,
    disabled: bool,
    disabled_reason: &'static str,
    onclick: EventHandler<()>,
) -> Element {
    let class = if active {
        "if-primary-nav__button is-active"
    } else {
        "if-primary-nav__button"
    };
    let title = if disabled { disabled_reason } else { "" };
    let on_button_click = move |_| onclick.call(());

    rsx! {
        button {
            r#type: "button",
            class,
            disabled,
            "aria-current": if active { "page" } else { "false" },
            title,
            onclick: on_button_click,
            "{label}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, mpsc};

    use dioxus::dioxus_core::{ElementId, Mutation, Mutations};
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
        let ctx = AppContext {
            state: Arc::new(RwLock::new(AppState::with_profile(profile))),
            commands: tx,
            settings: use_signal(SettingsSnapshot::default),
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

    fn synthetic_mouse_event() -> Event<dyn std::any::Any> {
        Event::new(
            std::rc::Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default()))
                as std::rc::Rc<dyn std::any::Any>,
            true,
        )
    }

    #[component]
    fn EventHarness() -> Element {
        let (tx, _) = mpsc::channel();
        let profile = Profile::new(
            "T".to_owned(),
            Vec::new(),
            Modes::new(vec!["Default".to_owned()]).expect("single Default mode is valid"),
            Vec::new(),
            Vec::new(),
            "Default".to_owned(),
        );
        let ctx = AppContext {
            state: Arc::new(RwLock::new(AppState::with_profile(profile))),
            commands: tx,
            settings: use_signal(SettingsSnapshot::default),
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
        let view = crate::frame::use_view_state_provider(ctx.meta);
        use_hook({
            let mut view = view;
            move || {
                view.main_surface.set(MainSurface::Mappings);
                view.panel_slot.set(PanelSlot::Devices);
            }
        });
        use_context_provider(|| view);

        rsx! {
            PrimaryNav {}
            div { "surface={view.main_surface.read():?};panel={view.panel_slot.read():?}" }
        }
    }

    fn click_listener_ids(mutations: &Mutations) -> (String, Vec<ElementId>) {
        let mut event_name = String::new();
        let ids: Vec<_> = mutations
            .edits
            .iter()
            .filter_map(|edit| {
                if let Mutation::NewEventListener { name, id } = edit
                    && matches!(name.as_str(), "click" | "onclick")
                {
                    if event_name.is_empty() {
                        event_name.clone_from(name);
                    }
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();

        assert!(
            ids.len() >= 3,
            "expected at least three primary-nav click listeners, found {ids:?} in {mutations:#?}"
        );
        (event_name, ids)
    }

    fn render_after_click(button_index: usize) -> String {
        set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
        let mut vdom = VirtualDom::new(EventHarness);
        let mutations = vdom.rebuild_to_vec();
        let (event_name, ids) = click_listener_ids(&mutations);
        vdom.runtime()
            .handle_event(&event_name, synthetic_mouse_event(), ids[button_index]);
        vdom.render_immediate_to_vec();
        render(&vdom)
    }

    #[test]
    fn renders_mappings_bulk_map_and_sheets_destinations() {
        let html = render_nav(MainSurface::Mappings);

        assert!(html.contains("Mappings"), "Mappings item missing: {html}");
        assert!(html.contains("Batch map"), "Batch map item missing: {html}");
        assert!(html.contains("Sheets"), "Sheets item missing: {html}");
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
    fn sheets_active_state_derives_from_main_surface() {
        let html = render_nav(MainSurface::Sheets);

        assert!(
            html.contains(
                r#"class="if-primary-nav__button is-active" aria-current="page" title="">Sheets</button>"#
            ),
            "Sheets must be the active primary destination: {html}"
        );
    }

    #[test]
    fn sheets_button_click_selects_surface_and_clears_panel_slot() {
        let html = render_after_click(2);

        assert!(
            html.contains("surface=Sheets;panel=None"),
            "Sheets click must select Sheets and clear the panel slot: {html}"
        );
    }

    #[test]
    fn bulk_map_button_click_selects_surface_and_clears_panel_slot() {
        let html = render_after_click(1);

        assert!(
            html.contains("surface=BulkMap;panel=None"),
            "BulkMap click must select BulkMap and clear the panel slot: {html}"
        );
    }

    #[test]
    fn mappings_button_click_selects_surface_and_preserves_panel_slot() {
        let html = render_after_click(0);

        assert!(
            html.contains("surface=Mappings;panel=Devices"),
            "Mappings click must select Mappings and preserve the panel slot: {html}"
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
