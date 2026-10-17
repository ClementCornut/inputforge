//! App callbacks own current filter interpretation, independent of renderer gating.
use dioxus::prelude::*;
use inputforge_core::{
    action::Mapping,
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    state::AppState,
    types::{DeviceId, InputAddress, InputId},
};
use inputforge_gui_dx::{
    mapping_edits::ui,
    mapping_rail::{Group, Lookup, Scope},
    navigation,
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
#[derive(Clone)]
struct Fixture {
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    values: Rc<RefCell<Option<Values>>>,
}
#[derive(Clone)]
struct Values {
    rail: ui::Rail,
    drag: Signal<Option<inputforge_gui_dx::mapping_rail::Menu>>,
    bindings: inputforge_gui_dx::mapping_edits::Bindings,
}
fn root() -> Element {
    let f = use_context::<Fixture>();
    let c = use_hook(|| {
        inputforge_app::mapping_edits::Controller::new(Arc::clone(&f.state), f.commands.clone())
    });
    let rail = ui::Rail {
        filter: ui::Filter {
            query: use_signal(String::new),
            device: use_signal(|| None),
        },
        prepared: use_signal(|| None),
        menu: use_signal(|| None),
        rename: use_signal(|| None),
        duplicate: use_signal(|| None),
        delete: use_signal(|| None),
        error: use_signal(|| None),
        live: use_signal(String::new),
    };
    *f.values.borrow_mut() = Some(Values {
        rail,
        drag: use_signal(|| None),
        bindings: c.bindings(),
    });
    rsx! {span {}}
}
fn address(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("rail".into()),
        input: InputId::Button { index },
    }
}
fn mount() -> (VirtualDom, Fixture, Values, mpsc::Receiver<EngineCommand>) {
    let profile = Profile::new(
        "P".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        (0..3)
            .map(|i| Mapping {
                input: address(i),
                mode: "Default".into(),
                name: Some(format!("Row {i}")),
                actions: vec![],
            })
            .collect(),
        vec![],
        "Default".into(),
    );
    let (commands, rx) = mpsc::channel();
    let f = Fixture {
        state: Arc::new(RwLock::new(AppState::with_profile(profile))),
        commands,
        values: Rc::default(),
    };
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(f.clone());
    dom.rebuild_in_place();
    let values = f.values.borrow().clone().unwrap();
    (dom, f, values, rx)
}
fn lookup(f: &Fixture) -> Lookup {
    Lookup {
        scope: Scope {
            profile: f
                .state
                .read()
                .active_profile
                .as_ref()
                .map(|p| p.id().clone()),
            mode: "Default".into(),
        },
        input: address(1),
        name: Some("Row 1".into()),
        filtered: true,
    }
}
#[test]
fn projection_and_drag_preparation_interpret_whitespace_text_and_device_filters() {
    for (query, device, filtered) in [
        (" \t", None, false),
        ("Row", None, true),
        ("", Some(DeviceId("rail".into())), true),
    ] {
        let (dom, f, mut v, _) = mount();
        dom.in_runtime(|| {
            v.rail.filter.query.set(query.into());
            v.rail.filter.device.set(device.clone());
            let view = v
                .bindings
                .navigation
                .filter
                .call(navigation::FilterRequest {
                    config: inputforge_app::bridge::prepare(&f.state.read(), None).config,
                    mode: "Default".into(),
                    query: query.into(),
                    device,
                });
            assert_eq!(view.filtered, filtered);
            v.bindings.ui.drag.call(ui::DragRequest {
                publication: v.drag,
                lookup: lookup(&f),
                filter: v.rail.filter,
            });
            assert_eq!(v.drag.peek().as_ref().unwrap().can_up, !filtered);
        });
    }
}
#[test]
fn menu_and_drop_recheck_raw_filters_changed_since_preparation() {
    for drop in [false, true] {
        for (query, device, filtered) in [
            (" \t", None, false),
            ("Row", None, true),
            ("", Some(DeviceId("rail".into())), true),
        ] {
            let (dom, f, mut v, rx) = mount();
            dom.in_runtime(|| {
                v.bindings.ui.rail.call((
                    v.rail,
                    ui::RailEvent::Open {
                        lookup: lookup(&f),
                        x: 0.0,
                        y: 0.0,
                    },
                ));
                let menu = v.rail.prepared.peek().clone().unwrap();
                assert!(
                    menu.can_up,
                    "app must override the stale rendered lookup gate"
                );
                v.bindings.ui.drag.call(ui::DragRequest {
                    publication: v.drag,
                    lookup: lookup(&f),
                    filter: v.rail.filter,
                });
                v.rail.filter.query.set(query.into());
                v.rail.filter.device.set(device);
                let event = if drop {
                    ui::RailEvent::Drop {
                        drag: v.drag,
                        group: Group::Buttons,
                        gap: 0,
                    }
                } else {
                    ui::RailEvent::Step { menu, down: false }
                };
                assert_eq!(v.bindings.ui.rail.call((v.rail, event)), !filtered);
            });
            if filtered {
                assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
            } else {
                assert!(matches!(
                    rx.try_recv().unwrap(),
                    EngineCommand::ReorderMapping {
                        target_index_in_group: 0,
                        ..
                    }
                ));
            }
        }
    }
}
