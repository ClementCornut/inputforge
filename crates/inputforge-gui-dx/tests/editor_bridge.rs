//! The public renderer installs the app poller once and owns its lifetime.
use dioxus::prelude::*;
use inputforge_core::{mode::Modes, profile::Profile, state::AppState};
use inputforge_gui_dx::{render_editor, routing::Bindings};
use parking_lot::RwLock;
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, mpsc},
    time::Duration,
};

#[derive(Clone, Props)]
struct EditorProps {
    state: Arc<RwLock<AppState>>,
    starts: Rc<Cell<usize>>,
}
impl PartialEq for EditorProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state) && Rc::ptr_eq(&self.starts, &other.starts)
    }
}
fn editor(props: EditorProps) -> Element {
    let (commands, _) = mpsc::channel();
    let state = Arc::clone(&props.state);
    let start = Callback::new(move |publisher| {
        props.starts.set(props.starts.get() + 1);
        inputforge_app::bridge::spawn_polling_task(Arc::clone(&state), publisher);
    });
    render_editor(
        props.state,
        commands,
        Bindings {
            dispatch: Callback::new(|_| {}),
        },
        start,
    )
}
fn profile(name: &str) -> Profile {
    Profile::new(
        name.into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    )
}
async fn drive(dom: &mut VirtualDom, millis: u64) {
    tokio::time::advance(Duration::from_millis(millis)).await;
    dom.render_immediate_to_vec();
}
#[test]
#[expect(
    clippy::await_holding_lock,
    reason = "hold the writer through a tick to verify nonblocking contention skips"
)]
fn renderer_preserves_tick_cadence_skips_contention_and_releases_polling_on_drop() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap();
    runtime.block_on(async {
        let state = Arc::new(RwLock::new(AppState::with_profile(profile(
            "Initial fixture",
        ))));
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
        assert!(dioxus_ssr::render(&dom).contains("Initial fixture"));
        state.write().active_profile = Some(profile("Next fixture"));
        drive(&mut dom, 15).await;
        assert!(!dioxus_ssr::render(&dom).contains("Next fixture"));
        drive(&mut dom, 1).await;
        assert!(dioxus_ssr::render(&dom).contains("Next fixture"));
        dom.mark_dirty(ScopeId::ROOT);
        dom.render_immediate_to_vec();
        assert_eq!(starts.get(), 1, "rerender must not install a second poller");
        let blocked_html = {
            let mut guard = state.write();
            guard.active_profile = Some(profile("Recovered fixture"));
            drive(&mut dom, 16).await;
            dioxus_ssr::render(&dom)
        };
        assert!(!blocked_html.contains("Recovered fixture"));
        drive(&mut dom, 16).await;
        assert!(dioxus_ssr::render(&dom).contains("Recovered fixture"));
        drop(dom);
        assert_eq!(
            Arc::strong_count(&state),
            1,
            "scope teardown must release every engine handle"
        );
        let mut remount = VirtualDom::new_with_props(
            editor,
            EditorProps {
                state: Arc::clone(&state),
                starts: Rc::clone(&starts),
            },
        );
        remount.rebuild_in_place();
        drive(&mut remount, 0).await;
        assert_eq!(starts.get(), 2);
        assert!(dioxus_ssr::render(&remount).contains("Recovered fixture"));
        drop(remount);
        assert_eq!(Arc::strong_count(&state), 1);
    });
}
