//! Actual capture controls exercise app bindings and existing edit handlers.
mod fixture;
mod rail;
use dioxus::{
    core::{DynamicNode, ElementId, TemplateAttribute, TemplateNode},
    html::SerializedHtmlEventConverter,
    prelude::*,
};
use fixture::{Surface, actions, address, fixture, fixture_with_name, primary};
use inputforge_core::{
    action::{Action, Condition},
    engine::EngineCommand,
    types::{AxisPolarity, AxisValue, DeviceId, InputId, InputValue},
};
use std::{
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};
fn element(dom: &VirtualDom, node: &VNode, label: &str) -> Option<ElementId> {
    for (index, path) in node.template.attr_paths.iter().enumerate() {
        let mut element = &node.template.roots[usize::from(path[0])];
        for offset in &path[1..] {
            if let TemplateNode::Element { children, .. } = element {
                element = &children[usize::from(*offset)];
            }
        }
        if let TemplateNode::Element { attrs, .. } = element
            && attrs.iter().any(|attr| matches!(attr, TemplateAttribute::Static { name: "aria-label", value, .. } if *value == label)) {
            return node.mounted_dynamic_attribute(index, dom);
        }
    }
    for (index, child) in node.dynamic_nodes.iter().enumerate() {
        let found = match child {
            DynamicNode::Component(component) => component
                .mounted_scope(index, node, dom)
                .and_then(|scope| element(dom, scope.root_node(), label)),
            DynamicNode::Fragment(children) => {
                children.iter().find_map(|child| element(dom, child, label))
            }
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}
fn click(dom: &VirtualDom, label: &str) {
    let id = element(dom, dom.base_scope().root_node(), label)
        .unwrap_or_else(|| panic!("missing capture control {label}"));
    let data = PlatformEventData::new(Box::new(SerializedMouseData::default()));
    dom.runtime().handle_event(
        "click",
        Event::new(Rc::new(data) as Rc<dyn std::any::Any>, false),
        id,
    );
}

#[test]
fn four_capture_surfaces_preserve_native_addresses_and_existing_mapping_and_undo_handlers() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for (surface, label) in [
        (Surface::Add, "Add mapping"),
        (Surface::Header, "Rebind mapping input"),
        (Surface::Merge, "Rebind secondary input"),
        (Surface::Predicate, "Rebind predicate input"),
    ] {
        let (mut dom, fixture, received) = fixture(surface);
        click(&dom, label);
        dom.render_immediate_to_vec();
        assert!(dioxus_ssr::render(&dom).contains("Press an input"));
        assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        dom.in_scope(ScopeId::APP, || {
            let controller = fixture.controller.borrow().as_ref().unwrap().clone();
            let now = Instant::now();
            controller.sample(&fixture.state.read(), now);
            let (input, value) = if matches!(surface, Surface::Merge) {
                (InputId::Axis { index: 4 }, InputValue::Axis { value: AxisValue::new(0.9), polarity: AxisPolarity::Bipolar })
            } else { (InputId::Button { index: 3 }, InputValue::Button { pressed: true }) };
            let captured = address(input);
            fixture.state.write().input_cache.update(&captured, &value);
            controller.sample(&fixture.state.read(), now + Duration::from_millis(16));
            controller.deliver();
            controller.sample(&fixture.state.read(), now + Duration::from_millis(80));
            controller.deliver();
            if matches!(surface, Surface::Add) {
                assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty, "Add must still require its explicit commit");
            } else {
                let EngineCommand::SetMapping { input, mode, name, actions: changed } = received.try_recv().unwrap() else { panic!("capture must deliver to the existing SetMapping handler"); };
                assert_eq!(mode, "Default");
                assert_eq!(name.as_deref(), Some("Fixture"));
                match surface {
                    Surface::Header => { assert_eq!(input, captured); assert_eq!(changed, actions()); }
                    Surface::Merge => { assert_eq!(input, primary()); assert!(matches!(&changed[0], Action::MergeAxis { second_input, .. } if second_input == &captured)); }
                    Surface::Predicate => { assert_eq!(input, primary()); assert!(matches!(&changed[1], Action::Conditional { condition: Condition::ButtonPressed { input }, .. } if input == &captured)); }
                    Surface::Add => unreachable!(),
                }
                assert_eq!(fixture.editor.borrow().unwrap().undo_log.peek().stacks[&("Default".into(), primary())].undo.len(), 1);
                assert!(matches!(received.try_recv().unwrap(), EngineCommand::MappingEditBarrier { .. }));
                assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
            }
            assert_eq!(fixture.state.read().active_profile.as_ref().unwrap().find_mapping(&primary(), "Default").unwrap().actions, actions(), "delivery must not optimistically mutate engine mappings");
        });
        dom.render_immediate_to_vec();
        assert!(!dioxus_ssr::render(&dom).contains("Press an input"));
    }
}

#[test]
fn rendered_cancel_buttons_stop_capture_without_mapping_commands_or_undo() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for (surface, start, cancel) in [
        (
            Surface::Header,
            "Rebind mapping input",
            "Cancel mapping input capture",
        ),
        (
            Surface::Merge,
            "Rebind secondary input",
            "Cancel secondary input capture",
        ),
        (
            Surface::Predicate,
            "Rebind predicate input",
            "Cancel predicate input capture",
        ),
    ] {
        let (mut dom, fixture, received) = fixture(surface);
        click(&dom, start);
        dom.render_immediate_to_vec();
        click(&dom, cancel);
        dom.render_immediate_to_vec();
        assert!(!dioxus_ssr::render(&dom).contains("Press an input"));
        assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        assert_eq!(
            fixture
                .editor
                .borrow()
                .unwrap()
                .undo_log
                .peek()
                .stacks
                .len(),
            0
        );
    }
}

#[test]
fn capture_targets_use_stored_optional_name_in_the_selected_mode() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for (surface, label, name, other_mode) in [
        (Surface::Header, "Rebind mapping input", None, false),
        (
            Surface::Merge,
            "Rebind secondary input",
            Some("Fixture".into()),
            true,
        ),
        (Surface::Predicate, "Rebind predicate input", None, true),
    ] {
        let (mut dom, fixture, received) = fixture_with_name(surface, name, other_mode, false);
        click(&dom, label);
        dom.render_immediate_to_vec();
        dom.in_scope(ScopeId::APP, || {
            let controller = fixture.controller.borrow().as_ref().unwrap().clone();
            controller.sample(&fixture.state.read(), Instant::now());
            controller.deliver();
        });
        dom.render_immediate_to_vec();
        assert!(
            dioxus_ssr::render(&dom).contains("Press an input"),
            "valid capture target must remain armed"
        );
        assert_eq!(received.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    }
}

#[test]
fn invalid_merge_path_cannot_confirm_an_ambiguous_control() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let (mut dom, fixture, received) =
        fixture_with_name(Surface::Merge, Some("Fixture".into()), false, true);
    fixture.state.write().session.bindings.push(
        inputforge_core::profile::controllers::DeviceBinding {
            device: DeviceId("capture-stick".into()),
            observed_layout: None,
            unavailable: vec![InputId::Axis { index: 4 }],
            axes: vec![],
            buttons: vec![],
            hats: vec![],
        },
    );
    click(&dom, "Rebind secondary input");
    dom.render_immediate_to_vec();
    dom.in_scope(ScopeId::APP, || {
        let controller = fixture.controller.borrow().as_ref().unwrap().clone();
        let now = Instant::now();
        controller.sample(&fixture.state.read(), now);
        fixture.state.write().input_cache.update(
            &address(InputId::Axis { index: 4 }),
            &InputValue::Axis {
                value: AxisValue::new(0.9),
                polarity: AxisPolarity::Bipolar,
            },
        );
        controller.sample(&fixture.state.read(), now + Duration::from_millis(16));
        controller.deliver();
        controller.sample(&fixture.state.read(), now + Duration::from_millis(80));
        controller.deliver();
        assert_eq!(
            received.try_recv().unwrap_err(),
            mpsc::TryRecvError::Empty,
            "invalid stage paths must reject before native confirmation"
        );
        assert_eq!(
            fixture
                .editor
                .borrow()
                .unwrap()
                .undo_log
                .peek()
                .stacks
                .len(),
            0
        );
    });
}
