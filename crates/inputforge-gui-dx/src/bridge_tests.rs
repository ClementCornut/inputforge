use super::*;
use crate::context::{ConfigSnapshot, LiveSnapshot, MetaSnapshot, SettingsSnapshot};
use crate::{frame, routing};
use inputforge_core::state::AppState;
use parking_lot::RwLock;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, mpsc},
};

#[derive(Clone, Props)]
struct Props {
    publisher: Rc<RefCell<Option<Publisher>>>,
    renders: Rc<Cell<usize>>,
}
impl PartialEq for Props {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.publisher, &other.publisher) && Rc::ptr_eq(&self.renders, &other.renders)
    }
}
fn harness(props: Props) -> Element {
    let Props { publisher, renders } = props;
    let (commands, _) = mpsc::channel();
    let context = AppContext {
        state: Arc::new(RwLock::new(AppState::new())),
        commands,
        meta: use_signal(MetaSnapshot::default),
        config: use_signal(ConfigSnapshot::default),
        live: use_signal(LiveSnapshot::default),
        settings: use_signal(SettingsSnapshot::default),
    };
    let view = frame::use_view_state_provider(context.meta);
    let routing = Context {
        view: use_signal(routing::View::default),
        dispatch: Callback::new(|_| {}),
    };
    let controllers = crate::controllers::Context {
        view: use_signal(crate::controllers::View::default),
        error: use_signal(|| None),
        dispatch: Callback::new(|_| {}),
    };
    *publisher.borrow_mut() = Some(Publisher::new(context.clone(), view, routing, controllers));
    renders.set(renders.get() + 1);
    let current = context.meta.read().current_mode.clone();
    let device_count = context.config.read().devices.len();
    let generation = context.live.read().capture_epoch.generation;
    let count = context.settings.read().unpinned_snapshot_count;
    let toggle = routing
        .view
        .read()
        .toggle
        .map_or("Unavailable", routing::Action::label);
    let editable = controllers.view.read().editable;
    rsx! {span {"{current}:{device_count}:{generation}:{count}:{toggle}:{editable}"}}
}
#[test]
fn unchanged_frames_do_not_wake_any_snapshot_subscriber() {
    let handle = Rc::new(RefCell::new(None));
    let renders = Rc::new(Cell::new(0));
    let mut dom = VirtualDom::new_with_props(
        harness,
        Props {
            publisher: Rc::clone(&handle),
            renders: Rc::clone(&renders),
        },
    );
    dom.rebuild_in_place();
    let publisher = handle.borrow().clone().unwrap();
    dom.in_runtime(|| publisher.publish(Frame::default()));
    dom.render_immediate_to_vec();
    assert_eq!(renders.get(), 1);
    let mut changed = Frame::default();
    changed.meta.current_mode = "Prepared".into();
    changed
        .config
        .virtual_devices
        .push(inputforge_core::types::VirtualDeviceConfig {
            device_id: 1,
            axes: vec![],
            button_count: 2,
            hat_count: 0,
        });
    changed.live.capture_epoch.generation = 7;
    changed.settings.unpinned_snapshot_count = 3;
    changed.routing.toggle = Some(routing::Action::Start);
    changed.controllers.editable = true;
    dom.in_runtime(|| publisher.publish(changed.clone()));
    dom.render_immediate_to_vec();
    assert!(dioxus_ssr::render(&dom).contains("Prepared:0:7:3:Start:true"));
    let after_changed = renders.get();
    dom.in_runtime(|| publisher.publish(changed.clone()));
    dom.render_immediate_to_vec();
    assert_eq!(renders.get(), after_changed);
    changed.controllers.editable = false;
    dom.in_runtime(|| publisher.publish(changed.clone()));
    dom.render_immediate_to_vec();
    assert_eq!(
        renders.get(),
        after_changed + 1,
        "controller-only changes must publish independently"
    );
    assert!(dioxus_ssr::render(&dom).contains("Prepared:0:7:3:Start:false"));
    let after_changed = renders.get();
    dom.in_runtime(|| publisher.publish(changed));
    dom.render_immediate_to_vec();
    assert_eq!(renders.get(), after_changed);
    let selected = (
        "Default".into(),
        inputforge_core::types::InputAddress::Bound {
            device: inputforge_core::types::DeviceId("fixture".into()),
            input: inputforge_core::types::InputId::Button { index: 0 },
        },
    );
    dom.in_runtime(|| {
        let mut selection = publisher.selection;
        selection.set(Some(selected.clone()));
        assert_eq!(publisher.selected_mapping(), Some(selected));
    });
    dom.render_immediate_to_vec();
    assert_eq!(
        renders.get(),
        after_changed,
        "peek must not subscribe the rendering root"
    );
    let mut axis_frame = Frame::default();
    axis_frame
        .controllers
        .axes
        .push(crate::controllers::AxisRow {
            device: inputforge_core::types::DeviceId("publication-axis".into()),
            index: 0,
            code: 24,
            override_polarity: None,
            detected: None,
            can_override: true,
            can_detect: false,
        });
    dom.in_runtime(|| publisher.publish(axis_frame.clone()));
    dom.render_immediate_to_vec();
    let after_axis = renders.get();
    axis_frame.controllers.axes[0].detected = Some(inputforge_core::types::AxisPolarity::Unipolar);
    dom.in_runtime(|| publisher.publish(axis_frame.clone()));
    dom.render_immediate_to_vec();
    assert_eq!(
        renders.get(),
        after_axis + 1,
        "axis-only changes must publish"
    );
    let after_axis = renders.get();
    dom.in_runtime(|| publisher.publish(axis_frame));
    dom.render_immediate_to_vec();
    assert_eq!(
        renders.get(),
        after_axis,
        "unchanged axis rows must not wake subscribers"
    );
}
#[test]
fn deadline_reset_clears_the_published_readout_once_and_does_not_revive_on_reselection() {
    use inputforge_core::{action::Mapping, mode::Modes, profile::Profile, types::InputAddress};
    let handle = Rc::new(RefCell::new(None));
    let renders = Rc::new(Cell::new(0));
    let mut dom = VirtualDom::new_with_props(
        harness,
        Props {
            publisher: Rc::clone(&handle),
            renders: Rc::clone(&renders),
        },
    );
    dom.rebuild_in_place();
    let publisher = handle.borrow().clone().unwrap();
    let key = ("Default".to_owned(), InputAddress::Unbound);
    let state = AppState::with_profile(Profile::new(
        "P".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![Mapping {
            input: key.1.clone(),
            mode: key.0.clone(),
            name: None,
            actions: vec![],
        }],
        vec![],
        "Default".into(),
    ));
    let frame = inputforge_app::bridge::prepare(&state, Some(&key));
    dom.in_runtime(|| publisher.publish(frame));
    dom.render_immediate_to_vec();
    assert!(publisher.context.live.peek().readout.is_some());
    dom.in_runtime(|| publisher.publish_readout(None));
    dom.render_immediate_to_vec();
    assert!(publisher.context.live.peek().readout.is_none());
    let after = renders.get();
    dom.in_runtime(|| {
        publisher.publish_readout(None);
        let mut selection = publisher.selection;
        selection.set(Some(key));
    });
    dom.render_immediate_to_vec();
    assert!(publisher.context.live.peek().readout.is_none());
    assert_eq!(renders.get(), after);
}
