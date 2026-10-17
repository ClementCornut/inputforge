//! The actual demo host must expire real shortcut feedback and release deadlines.
use super::snapshot_document::Document;
use super::*;
use dioxus::prelude::*;
use std::{rc::Rc, time::Duration};
fn mount(state: &Arc<RwLock<AppState>>, document: &Rc<Document>) -> VirtualDom {
    let (commands, _) = mpsc::channel::<EngineCommand>();
    let mut dom = VirtualDom::new(demo_root);
    dom.provide_root_context((Arc::clone(state), commands));
    dom.provide_root_context(Rc::clone(document) as Rc<dyn document::Document>);
    dom.rebuild_in_place();
    dom
}
async fn drive(dom: &mut VirtualDom, seconds: u64) {
    tokio::time::advance(Duration::from_secs(seconds)).await;
    for _ in 0..4 {
        tokio::task::yield_now().await;
        dom.render_immediate_to_vec();
    }
}
#[test]
fn actual_demo_shortcut_feedback_expires_and_teardown_remount_does_not_duplicate() {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let state = Arc::new(RwLock::new(AppState::new()));
            let document = Rc::new(Document::default());
            let mut dom = mount(&state, &document);
            drive(&mut dom, 0).await;
            assert_eq!(document.listeners.borrow().len(), 1);
            document.shortcut(0, 1);
            drive(&mut dom, 0).await;
            assert!(dioxus_ssr::render(&dom).contains("Load a profile"));
            drive(&mut dom, 7).await;
            assert_eq!(
                dioxus_ssr::render(&dom)
                    .matches("if-toast__message")
                    .count(),
                1
            );
            drive(&mut dom, 1).await;
            assert!(!dioxus_ssr::render(&dom).contains("if-toast__message"));
            document.shortcut(0, 1);
            drive(&mut dom, 0).await;
            assert_eq!(
                dioxus_ssr::render(&dom)
                    .matches("if-toast__message")
                    .count(),
                1
            );
            drop(dom);
            assert_eq!(document.cleanups(), 2);
            assert_eq!(document.shutdowns(), 1);
            assert_eq!(Arc::strong_count(&state), 1);
            tokio::time::advance(Duration::from_secs(20)).await;
            let mut fresh = mount(&state, &document);
            drive(&mut fresh, 0).await;
            assert!(!dioxus_ssr::render(&fresh).contains("if-toast__message"));
            assert_eq!(document.listeners.borrow().len(), 2);
            document.shortcut(0, 1);
            document.shortcut(1, 1);
            drive(&mut fresh, 0).await;
            let html = dioxus_ssr::render(&fresh);
            assert_eq!(html.matches("if-toast__message").count(), 1);
            assert!(!html.contains("×2"));
            drive(&mut fresh, 8).await;
            assert!(!dioxus_ssr::render(&fresh).contains("if-toast__message"));
            drop(fresh);
            assert_eq!(document.cleanups(), 4);
            assert_eq!(document.shutdowns(), 2);
            assert_eq!(Arc::strong_count(&state), 1);
        });
}
