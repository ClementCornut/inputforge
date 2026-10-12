//! Engine status presentation with app-owned routing actions.
mod logic;
use crate::{
    context::AppContext,
    routing::{Action, Context},
};
use dioxus::prelude::*;
use logic::{Variant, engine_pill_state};

#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus event attributes produce redundant qualifications"
)]
pub(crate) fn EnginePill() -> Element {
    tracing::trace!(target: "frame::render", region = "engine_pill");
    let ctx = use_context::<AppContext>();
    let routing = use_context::<Context>();
    let status = use_memo(move || ctx.meta.read().engine_status);
    let (variant, label) = engine_pill_state(*status.read());
    let action = routing.view.read().toggle;
    let class = format!("if-engine-pill if-engine-pill--{}", variant.class_suffix());
    let aria_pressed = match variant {
        Variant::Live => "true",
        Variant::Warning | Variant::Error => "false",
    };
    let action_verb = match action {
        Some(Action::Start) => "Start routing",
        Some(Action::Stop) => "Stop routing",
        Some(Action::Retry) => "Retry routing",
        Some(Action::ReviewDevices) => "Review controller changes in Devices",
        Some(Action::Refresh) => "Refresh devices",
        None => "Routing unavailable",
    };
    let live_text = format!("Engine {}", label.to_lowercase());
    rsx! {
        button {
            r#type: "button",
            class: "{class}",
            disabled: action.is_none(),
            title: action_verb,
            "aria-label": action_verb,
            "aria-pressed": aria_pressed,
            onclick: move |_| { if let Some(action) = action { routing.dispatch.call(action); } },
            span { class: "if-engine-pill__dot", "aria-hidden": "true" }
            span { class: "if-engine-pill__label", "aria-hidden": "true", "{label}" }
        }
        span { class: "if-sr-only", role: "status", "aria-live": "polite", "{live_text}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot};
    use inputforge_core::{
        profile::controllers::ControllerConfig, state::AppState, types::VirtualDeviceConfig,
    };
    use parking_lot::RwLock;
    use std::sync::{Arc, mpsc};

    #[derive(Clone, Props, PartialEq)]
    struct HarnessProps {
        #[props(default)]
        unavailable: bool,
        #[props(default)]
        recorded: Option<std::rc::Rc<std::cell::RefCell<Vec<Action>>>>,
    }

    fn pending_harness(props: HarnessProps) -> Element {
        let mut state = AppState::new();
        state.session.output_active = true;
        state.session.output_layout = vec![VirtualDeviceConfig {
            device_id: 1,
            axes: vec![],
            button_count: 2,
            hat_count: 0,
        }];
        let mut meta = MetaSnapshot::from_state(&state);
        meta.profile_name = Some("test".into());
        let meta = use_signal(|| meta);
        let config = use_signal(|| ConfigSnapshot {
            controllers: Some(ControllerConfig {
                virtual_devices: vec![VirtualDeviceConfig {
                    device_id: 1,
                    axes: vec![],
                    button_count: 3,
                    hat_count: 0,
                }],
                ..Default::default()
            }),
            ..Default::default()
        });
        let live = use_signal(LiveSnapshot::default);
        let settings = use_signal(SettingsSnapshot::default);
        let (commands, _) = mpsc::channel();
        use_context_provider(|| AppContext {
            state: Arc::new(RwLock::new(state)),
            commands,
            meta,
            config,
            live,
            settings,
        });
        let view = crate::frame::use_view_state_provider(meta);
        use_context_provider(|| view);
        let routing = use_signal(|| crate::routing::View {
            pending: true,
            toggle: (!props.unavailable).then_some(Action::ReviewDevices),
            ..Default::default()
        });
        use_context_provider(|| Context {
            view: routing,
            dispatch: Callback::new(move |action| {
                if let Some(recorded) = &props.recorded {
                    recorded.borrow_mut().push(action);
                }
            }),
        });
        rsx! { EnginePill {} }
    }

    #[test]
    fn retained_layout_change_offers_review_instead_of_start() {
        let mut dom = VirtualDom::new_with_props(
            pending_harness,
            HarnessProps {
                unavailable: false,
                recorded: None,
            },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("aria-label=\"Review controller changes in Devices\""));
        assert!(!html.contains("aria-label=\"Start routing\""));
        assert!(!html.contains(" disabled"));
    }

    #[test]
    fn app_prepared_unavailable_action_disables_engine_pill() {
        let mut dom = VirtualDom::new_with_props(
            pending_harness,
            HarnessProps {
                unavailable: true,
                recorded: None,
            },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let document = scraper::Html::parse_fragment(&html);
        let selector = scraper::Selector::parse("button.if-engine-pill").unwrap();
        assert!(
            document
                .select(&selector)
                .next()
                .unwrap()
                .value()
                .attr("disabled")
                .is_some()
        );
    }

    #[test]
    fn engine_pill_delivers_the_supplied_review_callback() {
        use dioxus::html::{PlatformEventData, SerializedHtmlEventConverter, SerializedMouseData};
        use dioxus_core::Mutation;
        use std::{any::Any, cell::RefCell, rc::Rc};
        let recorded = Rc::new(RefCell::new(Vec::new()));
        let mut dom = VirtualDom::new_with_props(
            pending_harness,
            HarnessProps {
                unavailable: false,
                recorded: Some(Rc::clone(&recorded)),
            },
        );
        let mutations = dom.rebuild_to_vec();
        let button = mutations
            .edits
            .iter()
            .find_map(|mutation| match mutation {
                Mutation::NewEventListener { name, id } if name == "click" => Some(*id),
                _ => None,
            })
            .unwrap();
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let event: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(
            SerializedMouseData::default(),
        )));
        dom.runtime()
            .handle_event("click", Event::new(event, true), button);
        assert_eq!(*recorded.borrow(), vec![Action::ReviewDevices]);
    }
}
