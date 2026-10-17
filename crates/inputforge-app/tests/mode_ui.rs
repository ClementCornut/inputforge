//! App-owned mode presentation decisions without renderer event policy.
use dioxus::prelude::*;
use inputforge_core::{mode::Modes, profile::Profile, state::AppState};
use inputforge_gui_dx::mode_management::{
    Scope,
    ui::{Draft, Event, Publication, Request},
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};

type Values = Rc<RefCell<Option<(Publication, Draft, Callback<Request, bool>)>>>;
#[derive(Clone)]
struct Fixture {
    show: Rc<RefCell<Option<Signal<bool>>>>,
    draft_show: Rc<RefCell<Option<Signal<bool>>>>,
    state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<inputforge_core::engine::EngineCommand>,
    values: Values,
}
fn root() -> Element {
    let f = use_context::<Fixture>();
    let c = use_hook(|| {
        inputforge_app::mapping_edits::Controller::new(Arc::clone(&f.state), f.commands.clone())
    });
    let bindings = c.bindings();
    use_context_provider(|| bindings.modes);
    let show = use_signal(|| true);
    *f.show.borrow_mut() = Some(show);
    rsx! {if *show.read() { publication_child {} }}
}
fn publication_child() -> Element {
    let f = use_context::<Fixture>();
    let publication = Publication {
        adding: use_signal(|| None),
        renaming: use_signal(|| None),
        menu: use_signal(|| None),
        error: use_signal(|| None),
        delete: use_signal(|| None),
        focus: use_signal(|| None),
    };
    let show = use_signal(|| true);
    *f.draft_show.borrow_mut() = Some(show);
    rsx! {if *show.read() { draft_child { publication } }}
}
#[component]
fn draft_child(publication: Publication) -> Element {
    let f = use_context::<Fixture>();
    let bindings = use_context::<inputforge_gui_dx::mode_management::Bindings>();
    let draft = Draft {
        value: use_signal(|| "Approach".into()),
        error: use_signal(|| None),
        from: None,
    };
    *f.values.borrow_mut() = Some((publication, draft, bindings.ui));
    rsx! { span {} }
}
#[test]
fn failed_inline_delivery_retains_draft_and_cancel_is_app_owned() {
    let p = Profile::new(
        "P".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    let state = Arc::new(RwLock::new(AppState::with_profile(p)));
    let scope = Scope {
        revision: state.read().mode_revision,
        profile: state.read().active_profile.as_ref().map(|p| p.id().clone()),
        editing: "Default".into(),
    };
    let (commands, rx) = mpsc::channel();
    let values = Rc::new(RefCell::new(None));
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Fixture {
        show: Rc::default(),
        draft_show: Rc::default(),
        state,
        commands,
        values: Rc::clone(&values),
    });
    dom.rebuild_in_place();
    let (publication, draft, send) = values.borrow().clone().unwrap();
    dom.in_runtime(|| {
        send.call(Request {
            publication,
            scope: scope.clone(),
            event: Event::BeginAdd,
        })
    });
    drop(rx);
    dom.in_runtime(|| {
        send.call(Request {
            publication,
            scope: scope.clone(),
            event: Event::Commit {
                draft: draft.clone(),
                blur: false,
            },
        })
    });
    assert!(publication.adding.peek().is_some());
    assert_eq!(&*draft.value.peek(), "Approach");
    assert!(draft.error.peek().is_some());
    assert!(publication.focus.peek().is_none());
    dom.in_runtime(|| {
        send.call(Request {
            publication,
            scope,
            event: Event::CancelDraft(draft),
        })
    });
    assert!(publication.adding.peek().is_none());
}

#[test]
fn obsolete_mode_publications_cannot_change_state_after_child_teardown() {
    let (commands, rx) = mpsc::channel();
    let show = Rc::default();
    let values = Rc::default();
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Fixture {
        show: Rc::clone(&show),
        draft_show: Rc::default(),
        state: Arc::new(RwLock::new(AppState::new())),
        commands,
        values: Rc::clone(&values),
    });
    dom.rebuild_in_place();
    let (publication, _, send) = values.borrow().clone().unwrap();
    dom.in_runtime(|| show.borrow().unwrap().set(false));
    dom.render_immediate_to_vec();
    let delivered = dom.in_runtime(|| {
        send.call(Request {
            publication,
            scope: Scope {
                revision: 0,
                profile: None,
                editing: "Default".into(),
            },
            event: Event::BeginAdd,
        })
    });
    assert!(!delivered);
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
}

#[test]
fn obsolete_input_rejects_a_dropped_inline_draft_with_mode_tabs_still_mounted() {
    let (commands, rx) = mpsc::channel();
    let draft_show = Rc::default();
    let values = Rc::default();
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Fixture {
        show: Rc::default(),
        draft_show: Rc::clone(&draft_show),
        state: Arc::new(RwLock::new(AppState::new())),
        commands,
        values: Rc::clone(&values),
    });
    dom.rebuild_in_place();
    let (publication, draft, send) = values.borrow().clone().unwrap();
    dom.in_runtime(|| draft_show.borrow().unwrap().set(false));
    dom.render_immediate_to_vec();
    drop(publication.error.try_peek().unwrap());
    draft.value.try_peek().unwrap_err();
    let delivered = dom.in_runtime(|| {
        send.call(Request {
            publication,
            scope: Scope {
                revision: 0,
                profile: None,
                editing: "Default".into(),
            },
            event: Event::Input {
                draft,
                value: "obsolete".into(),
            },
        })
    });
    assert!(!delivered);
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
}
