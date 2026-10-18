//! Actual hook mount/rerender/teardown with an observing document.
use super::*;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};

#[derive(Default)]
struct Document(RefCell<Vec<String>>);
impl document::Document for Document {
    fn eval(&self, js: String) -> document::Eval {
        self.0.borrow_mut().push(js);
        document::Document::eval(&document::NoOpDocument, String::new())
    }
}
#[derive(Default)]
struct Fixture {
    show: RefCell<Option<Signal<bool>>>,
    revision: RefCell<Option<Signal<usize>>>,
}
fn owner() -> Element {
    let fixture = use_context::<Rc<Fixture>>();
    let show = use_signal(|| true);
    let revision = use_signal(|| 0);
    *fixture.show.borrow_mut() = Some(show);
    *fixture.revision.borrow_mut() = Some(revision);
    let bindings = use_hook(|| {
        let (commands, _receiver) = mpsc::channel();
        inputforge_app::mapping_edits::Controller::new(
            Arc::new(parking_lot::RwLock::new(
                inputforge_core::state::AppState::new(),
            )),
            commands,
        )
        .bindings()
    });
    use_context_provider(|| bindings);
    let editor = crate::frame::mapping_editor::EditorState {
        error: use_signal(|| None),
        undo_log: use_signal(Default::default),
        expanded_stages: use_signal(Default::default),
        stage_menu: use_signal(|| None),
        malformed_hints: use_signal(Default::default),
        malformed_summary_tags: use_signal(Default::default),
    };
    use_context_provider(|| editor);
    rsx! { if *show.read() { Listener { revision: *revision.read() } } }
}
#[component]
fn Listener(revision: usize) -> Element {
    use_kb_listener();
    rsx! { div { "{revision}" } }
}
async fn settle(dom: &mut VirtualDom) {
    for _ in 0..6 {
        tokio::task::yield_now().await;
        dom.render_immediate_to_vec();
    }
}
#[test]
fn actual_shortcut_hook_installs_once_and_cleans_up_before_remount() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let fixture = Rc::new(Fixture::default());
            let doc = Rc::new(Document::default());
            let mut dom = VirtualDom::new(owner);
            dom.provide_root_context(Rc::clone(&fixture));
            dom.provide_root_context(Rc::clone(&doc) as Rc<dyn document::Document>);
            dom.rebuild_in_place();
            settle(&mut dom).await;
            let installs = || {
                doc.0
                    .borrow()
                    .iter()
                    .filter(|s| s.contains("window.addEventListener"))
                    .count()
            };
            let cleanups = || {
                doc.0
                    .borrow()
                    .iter()
                    .filter(|s| s.contains("delete window["))
                    .count()
            };
            assert_eq!(installs(), 1);
            fixture.revision.borrow().unwrap().set(1);
            settle(&mut dom).await;
            assert_eq!(installs(), 1, "rerenders must not duplicate root shortcuts");
            fixture.show.borrow().unwrap().set(false);
            settle(&mut dom).await;
            assert_eq!(cleanups(), 1);
            fixture.show.borrow().unwrap().set(true);
            settle(&mut dom).await;
            assert_eq!(installs(), 2);
            drop(dom);
            assert_eq!(cleanups(), 2);
        });
}
