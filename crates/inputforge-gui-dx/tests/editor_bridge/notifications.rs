//! The existing production bridge advances warning delivery and expiry under contention.
use super::*;
#[test]
#[expect(
    clippy::await_holding_lock,
    reason = "hold writer to prove expiry does not wait for engine access"
)]
fn warning_delivery_expiry_contention_and_remount_share_one_editor_poller() {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(async {
            let (state, _) = fixture();
            state.write().warnings.push("historic warning".into());
            let starts = Rc::new(Cell::new(0));
            let mut dom = VirtualDom::new_with_props(
                editor,
                EditorProps {
                    state: Arc::clone(&state),
                    starts: Rc::clone(&starts),
                },
            );
            dom.rebuild_in_place();
            drive(&mut dom, 0).await;
            assert!(!dioxus_ssr::render(&dom).contains("if-toast__message"));
            state
                .write()
                .warnings
                .push("backend operation failed".into());
            drive(&mut dom, 16).await;
            let html = dioxus_ssr::render(&dom);
            assert!(html.contains("backend operation failed"));
            assert_eq!(html.matches("if-toast__message").count(), 1);
            drive(&mut dom, 16).await;
            assert!(!dioxus_ssr::render(&dom).contains("×2"));
            let writer = state.write();
            drive(&mut dom, 8000).await;
            assert!(!dioxus_ssr::render(&dom).contains("if-toast__message"));
            drop(writer);
            drive(&mut dom, 16).await;
            assert!(!dioxus_ssr::render(&dom).contains("if-toast__message"));
            assert_eq!(starts.get(), 1);
            drop(dom);
            assert_eq!(Arc::strong_count(&state), 1);
            let mut fresh = VirtualDom::new_with_props(
                editor,
                EditorProps {
                    state: Arc::clone(&state),
                    starts: Rc::clone(&starts),
                },
            );
            fresh.rebuild_in_place();
            drive(&mut fresh, 0).await;
            assert!(!dioxus_ssr::render(&fresh).contains("if-toast__message"));
            assert_eq!(starts.get(), 2);
            drop(fresh);
            assert_eq!(Arc::strong_count(&state), 1);
        });
}
