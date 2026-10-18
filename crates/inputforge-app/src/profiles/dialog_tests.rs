//! Real profile callbacks own picker startup, cancellation and stale completion.
use super::*;
use inputforge_gui_dx::{PanelSlot, profiles::Mode};
use std::{path::PathBuf, pin::Pin};

type PickResult = Pin<Box<dyn Future<Output = Option<PathBuf>>>>;
fn selected_file() -> PickResult {
    Box::pin(async { Some(PathBuf::from("/external/Chosen.toml")) })
}
fn cancelled_file() -> PickResult {
    Box::pin(async { None })
}
#[derive(Clone)]
struct Fixture {
    controller: Controller,
    panel: Signal<PanelSlot>,
    view: Signal<View>,
    owner: u64,
}
fn root() -> Element {
    let output = use_context::<Rc<RefCell<Option<Fixture>>>>();
    let mut controller = use_hook(|| {
        let (commands, _) = mpsc::channel();
        Controller::new(Arc::new(RwLock::new(AppState::new())), commands)
    });
    controller.picker = selected_file;
    let panel = use_signal(PanelSlot::default);
    let view = use_signal(View::default);
    let owner = use_hook(|| controller.mount(view));
    *output.borrow_mut() = Some(Fixture {
        controller,
        panel,
        view,
        owner,
    });
    rsx! { div {} }
}
fn settle(dom: &mut VirtualDom) {
    dom.render_immediate_to_vec();
}
#[test]
fn open_callback_invokes_picker_and_cancel_retains_the_form() {
    let output = Rc::new(RefCell::new(None::<Fixture>));
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&output));
    dom.rebuild_in_place();
    let mut f = output.borrow().clone().unwrap();
    dom.in_scope(ScopeId::ROOT, || {
        f.controller.unmount(f.owner);
        f.controller.bindings().start.call((f.panel, Intent::Pick));
        f.owner = f.controller.mount(f.view);
    });
    settle(&mut dom);
    assert!(f.view.peek().error.is_none());
    assert_eq!(*f.panel.peek(), PanelSlot::Profiles);
    assert_eq!(
        f.view.peek().mode,
        Mode::Open {
            path: "/external/Chosen.toml".into()
        }
    );
    assert_eq!(f.view.peek().name, "Chosen");
    f.controller.picker = cancelled_file;
    let before = f.view.peek().clone();
    dom.in_scope(ScopeId::ROOT, || {
        assert!(f.controller.dispatch(f.owner, Intent::Pick));
    });
    settle(&mut dom);
    assert_eq!(*f.view.peek(), before);
}
#[test]
fn picker_completion_after_panel_replacement_is_ignored() {
    let output = Rc::new(RefCell::new(None::<Fixture>));
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&output));
    dom.rebuild_in_place();
    let f = output.borrow().clone().unwrap();
    dom.in_scope(ScopeId::ROOT, || {
        assert!(f.controller.dispatch(f.owner, Intent::Pick));
        f.controller.unmount(f.owner);
        let owner = f.controller.mount(f.view);
        assert!(f.controller.dispatch(owner, Intent::New));
    });
    let before = f.view.peek().clone();
    settle(&mut dom);
    assert_eq!(f.view.peek().mode, Mode::New);
    assert_eq!(*f.view.peek(), before);
}

#[test]
fn leaving_profiles_before_the_entry_mount_cancels_the_deferred_action() {
    let output = Rc::new(RefCell::new(None::<Fixture>));
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&output));
    dom.rebuild_in_place();
    let mut f = output.borrow().clone().unwrap();
    dom.in_scope(ScopeId::ROOT, || {
        f.controller.unmount(f.owner);
        f.controller.bindings().start.call((f.panel, Intent::New));
        f.panel.set(PanelSlot::Settings);
    });
    settle(&mut dom);
    dom.in_scope(ScopeId::ROOT, || {
        f.panel.set(PanelSlot::Profiles);
        let _ = f.controller.mount(f.view);
    });
    assert_eq!(f.view.peek().mode, Mode::Library);
}

#[derive(Default)]
struct Lifetime {
    show: RefCell<Option<Signal<bool>>>,
    bindings: RefCell<Option<Bindings>>,
    owner: std::cell::Cell<u64>,
    started: std::cell::Cell<usize>,
    dropped: std::cell::Cell<usize>,
}
struct PendingPicker(Rc<Lifetime>);
impl Drop for PendingPicker {
    fn drop(&mut self) {
        self.0.dropped.set(self.0.dropped.get() + 1);
    }
}
fn pending_picker() -> PickResult {
    let log = consume_context::<Rc<Lifetime>>();
    log.started.set(log.started.get() + 1);
    let pending = PendingPicker(log);
    Box::pin(async move {
        let _pending = pending;
        std::future::pending().await
    })
}
fn lifetime_root() -> Element {
    let log = use_context::<Rc<Lifetime>>();
    let bindings = use_hook(|| {
        let (commands, _) = mpsc::channel();
        let mut controller = Controller::new(Arc::new(RwLock::new(AppState::new())), commands);
        controller.picker = pending_picker;
        controller.bindings()
    });
    use_context_provider(|| bindings);
    *log.bindings.borrow_mut() = Some(bindings);
    let show = use_signal(|| true);
    *log.show.borrow_mut() = Some(show);
    rsx! { if show() { LifetimePanel {} } }
}
#[component]
fn LifetimePanel() -> Element {
    let log = use_context::<Rc<Lifetime>>();
    let bindings = use_context::<Bindings>();
    let view = use_signal(View::default);
    let owner = use_hook(|| bindings.mount.call(view));
    log.owner.set(owner);
    use_drop(move || bindings.unmount.call(owner));
    rsx! { div {} }
}
#[test]
fn root_callbacks_cancel_pending_picker_work_when_the_child_panel_unmounts() {
    let log = Rc::new(Lifetime::default());
    let mut dom = VirtualDom::new(lifetime_root);
    dom.provide_root_context(Rc::clone(&log));
    dom.rebuild_in_place();
    let bindings = log.bindings.borrow().unwrap();
    assert!(bindings.dispatch.call((log.owner.get(), 0, Intent::Pick)));
    settle(&mut dom);
    assert_eq!(log.started.get(), 1);
    assert_eq!(log.dropped.get(), 0);
    dom.in_scope(ScopeId::ROOT, || log.show.borrow().unwrap().set(false));
    settle(&mut dom);
    assert_eq!(
        log.dropped.get(),
        1,
        "panel teardown must cancel its pending picker future"
    );
}
