use super::{document::Document, fixture};
use dioxus::prelude::*;
use inputforge_app::profiles::Controller;
use inputforge_core::{
    engine::{EngineCommand, SnapshotAction, SnapshotRequest},
    state::AppState,
};
use inputforge_gui_dx::profiles::Bindings;
use parking_lot::RwLock;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, mpsc},
};
#[derive(Clone)]
struct Surface {
    controller: Controller,
    bindings: Bindings,
    owner: Rc<Cell<u64>>,
    error: Signal<Option<String>>,
    feedback: Callback<String>,
}
type Output = Rc<RefCell<Option<Surface>>>;
fn root() -> Element {
    let state = use_context::<Arc<RwLock<AppState>>>();
    let tx = use_context::<mpsc::Sender<EngineCommand>>();
    let c = use_hook(|| Controller::new(state, tx));
    let bindings = c.bindings();
    let error = use_signal(|| None);
    let feedback = use_callback(move |message: String| {
        let mut error = error;
        error.set(Some(message));
    });
    let owner = use_hook(|| Rc::new(Cell::new(bindings.shortcut_mount.call(feedback))));
    let cleanup = Rc::clone(&owner);
    use_drop(move || bindings.shortcut_unmount.call(cleanup.get()));
    *use_context::<Output>().borrow_mut() = Some(Surface {
        controller: c,
        bindings,
        owner,
        error,
        feedback,
    });
    rsx! {div{}}
}
fn fixture(
    connected: bool,
) -> (
    VirtualDom,
    Surface,
    Rc<Document>,
    Arc<RwLock<AppState>>,
    mpsc::Receiver<EngineCommand>,
) {
    let state = Arc::new(RwLock::new(fixture::state()));
    let document = Rc::new(Document::default());
    let output: Output = Rc::default();
    let (tx, rx) = mpsc::channel();
    let rx = if connected {
        rx
    } else {
        drop(rx);
        mpsc::channel().1
    };
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Arc::clone(&state));
    dom.provide_root_context(tx);
    dom.provide_root_context(Rc::clone(&output));
    dom.provide_root_context(Rc::clone(&document) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    let surface = output.borrow_mut().take().unwrap();
    (dom, surface, document, state, rx)
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
async fn drive(dom: &mut VirtualDom) {
    for _ in 0..4 {
        tokio::task::yield_now().await;
        dom.render_immediate_to_vec();
    }
}
#[test]
fn root_shortcut_is_single_owned_listener_with_synchronous_cleanup_and_remount() {
    runtime().block_on(async {
        let (mut dom, f, document, _, rx) = fixture(true);
        assert_eq!(document.listeners.borrow().len(), 1);
        dom.render_immediate_to_vec();
        assert_eq!(document.listeners.borrow().len(), 1);
        document.shortcut(0, 1);
        drive(&mut dom).await;
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::Snapshot(request) if request.action==SnapshotAction::Manual
        ));
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        dom.in_scope(ScopeId::ROOT, || {
            f.bindings.shortcut_unmount.call(f.owner.get());
            f.owner.set(f.bindings.shortcut_mount.call(f.feedback));
        });
        assert_eq!(document.cleanups(), 1);
        assert_eq!(document.shutdowns(), 1);
        assert_eq!(document.listeners.borrow().len(), 2);
        document.shortcut(0, 1);
        document.shortcut(1, 1);
        drive(&mut dom).await;
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::Snapshot(request) if request.action==SnapshotAction::Manual
        ));
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        drop(dom);
        assert_eq!(document.cleanups(), 2);
        assert_eq!(document.shutdowns(), 2);
    });
}
#[test]
fn queued_shortcut_rejects_changed_profile_and_missing_profile_or_failed_delivery_is_visible() {
    runtime().block_on(async {
        let (mut dom, f, document, state, rx) = fixture(true);
        state.write().active_profile = Some(fixture::profile());
        document.shortcut(0, 1);
        drive(&mut dom).await;
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        dom.in_scope(ScopeId::ROOT, || {
            f.bindings
                .reconcile
                .call(inputforge_gui_dx::snapshots::MetaSnapshot::default());
        });
        document.shortcut(0, 1);
        drive(&mut dom).await;
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        document.shortcut(0, 2);
        drive(&mut dom).await;
        assert!(matches!(
            rx.try_recv().unwrap(),
            EngineCommand::Snapshot(request) if request.action==SnapshotAction::Manual
        ));
        state.write().active_profile = None;
        dom.in_scope(ScopeId::ROOT, || {
            f.bindings
                .reconcile
                .call(inputforge_gui_dx::snapshots::MetaSnapshot::default());
        });
        document.shortcut(0, 3);
        drive(&mut dom).await;
        assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
        assert!(f.error.peek().is_some());
        drop(dom);
        let (mut dom, f, document, _, _) = fixture(false);
        document.shortcut(0, 1);
        drive(&mut dom).await;
        assert!(f.error.peek().is_some());
    });
}

#[test]
fn root_shortcut_carries_old_identity_after_queued_profile_load() {
    runtime().block_on(async {
        let (mut dom, f, document, state, rx) = fixture(true);
        let old_id = state.read().active_profile.as_ref().unwrap().id().clone();
        dom.in_scope(ScopeId::ROOT, || {
            state
                .write()
                .profile_library_rows
                .push(inputforge_core::state::ProfileLibraryRow {
                    name: "Bravo".into(),
                    path: "/profiles/Bravo.toml".into(),
                    profile_id: None,
                    origin: inputforge_core::state::ProfileOrigin::Library,
                    is_active: false,
                    mode_count: 1,
                    last_edited_at: None,
                });
            let view = Signal::new(inputforge_gui_dx::profiles::View::default());
            let owner = f.controller.mount(view);
            let target = view
                .peek()
                .rows
                .iter()
                .find(|r| r.target.name == "Bravo")
                .unwrap()
                .target
                .clone();
            assert!(f.bindings.dispatch.call((
                owner,
                0,
                inputforge_gui_dx::profiles::Intent::Open(target)
            )));
        });
        document.shortcut(0, 1);
        drive(&mut dom).await;
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::LoadProfile("/profiles/Bravo.toml".into())
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            EngineCommand::Snapshot(Box::new(SnapshotRequest {
                profile: old_id,
                path: "/profiles/Alpha.toml".into(),
                action: SnapshotAction::Manual
            }))
        );
    });
}
