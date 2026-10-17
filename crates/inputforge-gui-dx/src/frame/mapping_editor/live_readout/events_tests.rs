//! Real chevron events and live prepared publication across target and mount changes.
use crate::{
    context::AppContext,
    readouts::ExpandState,
    snapshots::{Frame, LiveSnapshot},
};
use dioxus::prelude::*;
use inputforge_core::{
    action::{Action, Mapping},
    mode::Modes,
    profile::Profile,
    state::{AppState, EngineStatus},
    types::{
        AxisPolarity, AxisValue, DeviceId, InputAddress, InputId, InputValue, MergeOp,
        OutputAddress, OutputId, VJoyAxis,
    },
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
type Signals = (Signal<LiveSnapshot>, Signal<bool>, Signal<ExpandState>);
#[derive(Default)]
struct Fixture {
    production: bool,
    mode: RefCell<Option<Signal<String>>>,
    signals: RefCell<Option<Signals>>,
    state: Arc<RwLock<AppState>>,
}
fn primary() -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("stick".into()),
        input: InputId::Axis { index: 0 },
    }
}
fn fixture() -> Rc<Fixture> {
    let profile = Profile::new(
        "P".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![Mapping {
            input: primary(),
            mode: "Default".into(),
            name: None,
            actions: vec![
                Action::MergeAxis {
                    second_input: primary(),
                    operation: MergeOp::Average,
                },
                Action::MapToVJoy {
                    output: OutputAddress {
                        device: 1,
                        output: OutputId::Axis { id: VJoyAxis::X },
                    },
                },
            ],
        }],
        vec![],
        "Default".into(),
    );
    let mut state = AppState::with_profile(profile);
    state.engine_status = EngineStatus::Running;
    Rc::new(Fixture {
        state: Arc::new(RwLock::new(state)),
        ..Default::default()
    })
}
fn frame(f: &Fixture) -> Frame {
    inputforge_app::bridge::prepare(&f.state.read(), Some(&("Default".into(), primary())))
}
fn root() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let frame = use_hook(|| frame(&f));
    let (tx, _) = mpsc::channel();
    let bindings = use_hook(|| {
        inputforge_app::mapping_edits::Controller::new(Arc::clone(&f.state), tx.clone()).bindings()
    });
    use_context_provider(|| bindings);
    let live = use_signal(|| frame.live);
    let visible = use_signal(|| true);
    let mode = use_signal(|| "Default".to_owned());
    *f.mode.borrow_mut() = Some(mode);
    let expand = use_signal(ExpandState::default);
    *f.signals.borrow_mut() = Some((live, visible, expand));
    let settings = use_signal(Default::default);
    let meta = use_signal(|| frame.meta);
    let config = use_signal(|| frame.config);
    use_context_provider(|| {
        crate::test_support::provide_engine(Arc::clone(&f.state), tx);
        AppContext {
            settings,
            meta,
            config,
            live,
        }
    });
    let Some(prepared) = live.read().readout.clone() else {
        return rsx! {};
    };
    if *visible.read() && f.production {
        rsx! {super::LiveReadout {mapping_key:(mode.read().clone(),primary()),actions:prepared.key.actions}}
    } else if *visible.read() {
        rsx! {super::LiveReadoutTest{mapping_key:("Default".into(),primary()),actions:prepared.key.actions,expand_state:expand}}
    } else {
        rsx! {}
    }
}
fn listeners(dom: &VirtualDom, node: &VNode, ids: &mut Vec<dioxus::core::ElementId>) {
    use dioxus::core::DynamicNode;
    for (i, attrs) in node.dynamic_attrs.iter().enumerate() {
        if attrs
            .iter()
            .any(|a| a.name == "click" || a.name == "onclick")
            && let Some(id) = node.mounted_dynamic_attribute(i, dom)
        {
            ids.push(id);
        }
    }
    for (i, child) in node.dynamic_nodes.iter().enumerate() {
        match child {
            DynamicNode::Component(c) => {
                if let Some(s) = c.mounted_scope(i, node, dom) {
                    listeners(dom, s.root_node(), ids);
                }
            }
            DynamicNode::Fragment(nodes) => {
                for node in nodes {
                    listeners(dom, node, ids);
                }
            }
            _ => {}
        }
    }
}
#[test]
fn chevron_uses_app_transitions_and_reactive_model_while_engine_guard_is_unavailable() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let f = fixture();
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    let mut ids = vec![];
    listeners(&dom, dom.base_scope().root_node(), &mut ids);
    assert_eq!(ids.len(), 1);
    dom.runtime().handle_event(
        "click",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(
                SerializedMouseData::default(),
            ))) as Rc<dyn std::any::Any>,
            false,
        ),
        ids[0],
    );
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("if-editor__readout-chain\""));
    f.state.write().input_cache.update(
        &primary(),
        &InputValue::Axis {
            value: AxisValue::new(0.8),
            polarity: AxisPolarity::Bipolar,
        },
    );
    let prepared = frame(&f);
    let _guard = f.state.write();
    let (mut live, _, expand) = f.signals.borrow().unwrap();
    dom.in_runtime(|| live.set(prepared.live));
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("+0.80"));
    assert!(expand.peek().is_expanded(0));
}
#[test]
fn selection_replacement_resets_expansion_without_changing_output_count() {
    let f = fixture();
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    let (mut live, _, mut expand) = f.signals.borrow().unwrap();
    dom.in_runtime(|| expand.with_mut(|state| state.per_output[0] = true));
    dom.render_immediate_to_vec();
    let old = live.peek().readout.as_ref().unwrap().key.clone();
    f.state.write().active_profile = fixture().state.read().active_profile.clone();
    let changed = frame(&f);
    assert_ne!(old, changed.live.readout.as_ref().unwrap().key);
    dom.in_runtime(|| live.set(changed.live));
    dom.render_immediate_to_vec();
    dom.render_immediate_to_vec();
    assert!(!expand.peek().is_expanded(0));
    assert!(!dioxus_ssr::render(&dom).contains("if-editor__readout-chain\""));
}

#[test]
fn remount_resets_production_expansion_and_has_one_chevron_handler() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let mut f = fixture();
    Rc::get_mut(&mut f).unwrap().production = true;
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    for remount in [false, true] {
        if remount {
            let (_, mut visible, _) = f.signals.borrow().unwrap();
            dom.in_runtime(|| visible.set(false));
            dom.render_immediate_to_vec();
            dom.in_runtime(|| visible.set(true));
            dom.render_immediate_to_vec();
            dom.render_immediate_to_vec();
        }
        assert!(!dioxus_ssr::render(&dom).contains("if-editor__readout-chain\""));
        let mut ids = vec![];
        listeners(&dom, dom.base_scope().root_node(), &mut ids);
        assert_eq!(ids.len(), 1);
        dom.runtime().handle_event(
            "click",
            Event::new(
                Rc::new(PlatformEventData::new(Box::new(
                    SerializedMouseData::default(),
                ))) as Rc<dyn std::any::Any>,
                false,
            ),
            ids[0],
        );
        dom.render_immediate_to_vec();
        assert!(dioxus_ssr::render(&dom).contains("if-editor__readout-chain\""));
    }
}

#[test]
fn renderer_rejects_a_different_mode_even_with_identical_input_and_actions() {
    let mut f = fixture();
    Rc::get_mut(&mut f).unwrap().production = true;
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("if-editor__readout\""));
    let _locked = f.state.write();
    let mut mode = f.mode.borrow().unwrap();
    dom.in_runtime(|| mode.set("Combat".into()));
    dom.render_immediate_to_vec();
    assert!(
        !dioxus_ssr::render(&dom).contains("if-editor__readout\""),
        "the prior mode's prepared pulse must never render as the newly selected mapping"
    );
}
