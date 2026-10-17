//! Real drawer handlers must request confirmation and retain it on failed delivery.
use super::event_tests::listeners;
use crate::context::{AppContext, LiveSnapshot, SettingsSnapshot};
use dioxus::prelude::*;
use inputforge_core::{
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    snapshot::{SnapshotId, SnapshotKind},
    state::{ActiveSnapshotRow, AppState},
};
use parking_lot::RwLock;
use std::{
    rc::Rc,
    sync::{Arc, mpsc},
};
fn root() -> Element {
    let state = use_context::<Arc<RwLock<AppState>>>();
    let commands = use_context::<mpsc::Sender<EngineCommand>>();
    let c = use_hook(|| {
        inputforge_app::profiles::Controller::new(Arc::clone(&state), commands.clone())
    });
    use_context_provider(|| c.bindings());
    let frame = inputforge_app::bridge::prepare(&state.read(), None);
    let meta = use_signal(|| frame.meta);
    let config = use_signal(|| frame.config);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    use_context_provider(|| {
        crate::test_support::provide_engine(state, commands);
        AppContext {
            settings,
            meta,
            config,
            live,
        }
    });
    rsx! {super::ProfilesPanel{}}
}
fn fixture(connected: bool) -> (VirtualDom, mpsc::Receiver<EngineCommand>) {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let p = Profile::new(
        "Alpha".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    let mut s = AppState::with_profile(p);
    s.profile_path = Some("/profiles/Alpha.toml".into());
    s.active_snapshot_rows.push(ActiveSnapshotRow {
        id: SnapshotId("01K00K00000000000000000000".parse().unwrap()),
        kind: SnapshotKind::Manual,
        label: Some("Before edits".into()),
        taken_at: chrono::Utc::now(),
        pinned: true,
    });
    let (tx, rx) = mpsc::channel();
    let rx = if connected {
        rx
    } else {
        drop(rx);
        mpsc::channel().1
    };
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Arc::new(RwLock::new(s)));
    dom.provide_root_context(tx);
    dom.rebuild_in_place();
    dom.render_immediate_to_vec();
    (dom, rx)
}
fn click(dom: &mut VirtualDom, index_from_end: usize) {
    let mut ids = vec![];
    listeners(dom, dom.base_scope().root_node(), "click", &mut ids);
    let id = ids[ids.len() - index_from_end];
    dom.runtime().handle_event(
        "click",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(
                SerializedMouseData::default(),
            ))) as Rc<dyn std::any::Any>,
            false,
        ),
        id,
    );
    dom.render_immediate_to_vec();
}
#[test]
fn restore_row_requires_confirmation_and_cancel_sends_nothing() {
    let (mut dom, rx) = fixture(true);
    click(&mut dom, 2);
    assert!(
        rx.try_recv().is_err(),
        "restore row must ask before delivering destructive action"
    );
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Restore snapshot?"), "{html}");
    click(&mut dom, 2);
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    assert!(!dioxus_ssr::render(&dom).contains("Restore snapshot?"));
}
#[test]
fn failed_restore_confirm_keeps_dialog_and_accessible_feedback() {
    let (mut dom, _) = fixture(false);
    click(&mut dom, 2);
    click(&mut dom, 1);
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("Restore snapshot?"), "{html}");
    assert!(html.contains("role=\"alert\""), "{html}");
}
