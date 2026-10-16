use dioxus::prelude::*;
use inputforge_app::profiles::Controller;
use inputforge_core::{
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    snapshot::{SnapshotId, SnapshotKind},
    state::{ActiveSnapshotRow, AppState},
};
use inputforge_gui_dx::profiles::{Bindings, View};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
pub(super) fn profile() -> Profile {
    Profile::new(
        "Alpha".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    )
}
pub(super) fn state() -> AppState {
    let mut s = AppState::with_profile(profile());
    s.profile_path = Some("/profiles/Alpha.toml".into());
    s.active_snapshot_rows = vec![ActiveSnapshotRow {
        id: SnapshotId(ulid::Ulid::generate()),
        kind: SnapshotKind::Manual,
        label: Some("Before edits".into()),
        taken_at: chrono::Utc::now(),
        pinned: true,
    }];
    s
}
pub(super) struct Fixture {
    pub c: Controller,
    pub bindings: Bindings,
    pub owner: u64,
    pub view: Signal<View>,
    pub state: Arc<RwLock<AppState>>,
    pub rx: mpsc::Receiver<EngineCommand>,
}
type Output = Rc<RefCell<Option<(Controller, Bindings, u64, Signal<View>)>>>;
fn root() -> Element {
    let state = use_context::<Arc<RwLock<AppState>>>();
    let tx = use_context::<mpsc::Sender<EngineCommand>>();
    let c = use_hook(|| Controller::new(state, tx));
    let bindings = c.bindings();
    let view = use_signal(View::default);
    let owner = use_hook(|| bindings.mount.call(view));
    *use_context::<Output>().borrow_mut() = Some((c, bindings, owner, view));
    rsx! {div{}}
}
pub(super) fn fixture(connected: bool) -> (VirtualDom, Fixture) {
    let state = Arc::new(RwLock::new(state()));
    let (tx, rx) = mpsc::channel();
    let rx = if connected {
        rx
    } else {
        drop(rx);
        mpsc::channel().1
    };
    let output: Output = Rc::default();
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Arc::clone(&state));
    dom.provide_root_context(tx);
    dom.provide_root_context(Rc::clone(&output));
    dom.rebuild_in_place();
    let (c, bindings, owner, view) = output.borrow_mut().take().unwrap();
    (
        dom,
        Fixture {
            c,
            bindings,
            owner,
            view,
            state,
            rx,
        },
    )
}
impl Fixture {
    pub(super) fn send(&self, intent: inputforge_gui_dx::profiles::snapshots::Intent) -> bool {
        let workflow = self.view.peek().workflow;
        self.bindings.dispatch.call((
            self.owner,
            workflow,
            inputforge_gui_dx::profiles::Intent::Snapshot(intent),
        ))
    }
}
