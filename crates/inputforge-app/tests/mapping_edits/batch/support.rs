use dioxus::prelude::*;
use inputforge_app::mapping_edits::Controller;
use inputforge_core::{
    engine::EngineCommand,
    state::{AppState, DeviceState},
    types::{
        DeviceDiagnostics, DeviceId, DeviceInfo, InputAddress, InputId, VJoyAxis,
        VirtualDeviceConfig,
    },
};
use inputforge_gui_dx::batch_mapping::{Bindings, Intent, Publication, Request, View};
use parking_lot::RwLock;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, mpsc},
};
pub(super) struct Fixture {
    pub c: Controller,
    pub state: Arc<RwLock<AppState>>,
    pub batch: RefCell<Option<Bindings>>,
    pub view: RefCell<Option<Signal<View>>>,
    pub mode: RefCell<Option<Signal<String>>>,
    pub completed: Cell<usize>,
    pub mapping: RefCell<Option<inputforge_gui_dx::mapping_edits::Bindings>>,
    pub applied: RefCell<Option<Callback<usize>>>,
}
fn root() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let view = use_signal(View::default);
    let mode = use_signal(|| "Default".to_owned());
    let b = use_hook(|| f.c.bindings());
    *f.mapping.borrow_mut() = Some(b);
    let callbacks = b.batch;
    let done = Rc::clone(&f);
    let applied = Callback::new(move |count| done.completed.set(count));
    *f.applied.borrow_mut() = Some(applied);
    let owner = use_hook(|| {
        callbacks.install.call(Publication {
            view,
            mode,
            applied,
        })
    });
    use_drop(move || callbacks.dispose.call(owner));
    *f.batch.borrow_mut() = Some(callbacks);
    *f.view.borrow_mut() = Some(view);
    *f.mode.borrow_mut() = Some(mode);
    rsx! { "batch fixture" }
}
pub(super) fn mount() -> (VirtualDom, Rc<Fixture>, mpsc::Receiver<EngineCommand>) {
    let (_, state, _) =
        super::super::support::fixture(vec![inputforge_core::action::Action::Invert]);
    state.write().devices.push(DeviceState {
        info: DeviceInfo {
            id: DeviceId("batch".into()),
            name: "Stick".into(),
            axes: 2,
            buttons: 2,
            hats: 1,
            instance_path: None,
            axis_polarities: vec![],
        },
        connected: true,
        diagnostics: DeviceDiagnostics::default(),
    });
    state.write().virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![VJoyAxis::X, VJoyAxis::Y],
        button_count: 2,
        hat_count: 1,
    });
    let (tx, rx) = mpsc::channel();
    let f = Rc::new(Fixture {
        c: Controller::new(Arc::clone(&state), tx),
        state,
        batch: RefCell::default(),
        view: RefCell::default(),
        mode: RefCell::default(),
        completed: Cell::new(0),
        mapping: RefCell::default(),
        applied: RefCell::default(),
    });
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    (dom, f, rx)
}
impl Fixture {
    pub(super) fn model(&self) -> View {
        self.view.borrow().unwrap().peek().clone()
    }
    pub(super) fn request(&self, intent: Intent) -> Request {
        let v = self.model();
        Request {
            owner: v.owner,
            generation: v.generation,
            intent,
        }
    }
    pub(super) fn act(&self, intent: Intent) {
        self.batch
            .borrow()
            .unwrap()
            .dispatch
            .call(self.request(intent));
    }
    pub(super) fn refresh(&self) {
        self.mapping.borrow().unwrap().reconcile.call(());
    }
}
pub(super) fn address(index: u8) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("batch".into()),
        input: InputId::Axis { index },
    }
}
pub(super) fn commands(
    state: &Arc<RwLock<AppState>>,
    rx: &mpsc::Receiver<EngineCommand>,
) -> Vec<inputforge_core::action::BulkMapEntry> {
    let EngineCommand::SetMappingsBulk {
        entries,
        snapshot_label,
    } = rx.try_recv().unwrap()
    else {
        panic!("batch expected")
    };
    assert_eq!(snapshot_label, "Stick · Device 1");
    state
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mappings_bulk(&entries);
    let EngineCommand::MappingEditBarrier { owner, serial } = rx.try_recv().unwrap() else {
        panic!("barrier expected")
    };
    state.write().mapping_edit_barrier = Some((owner, serial));
    entries
}
