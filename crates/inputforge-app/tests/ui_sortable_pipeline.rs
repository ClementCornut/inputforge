//! Nested gap events use final mapping identity/history validation.
use dioxus::prelude::*;
use inputforge_app::mapping_edits::Controller;
use inputforge_core::{
    action::{Action, ActionBranch, Condition, Mapping},
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    state::AppState,
    types::InputAddress,
};
use inputforge_gui_dx::{
    mapping_edits::{Publication, Request, StageId, StageIdSegment, View},
    sortable::{GapProps, GapView, HandleProps, HandleView, PipelineDrop, State},
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
struct Fixture {
    shared: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    output: RefCell<Option<Handles>>,
    source_index: usize,
}
#[derive(Clone)]
struct Handles {
    source: HandleView,
    ancestor: HandleView,
    gap: GapView,
    state: State<StageId>,
    controller: Controller,
}
fn branch(source: usize) -> StageId {
    StageId(vec![
        StageIdSegment::Index(usize::from(source == 0)),
        StageIdSegment::Branch(ActionBranch::ConditionalTrue),
    ])
}
fn mapping(actions: Vec<Action>) -> Mapping {
    Mapping {
        input: InputAddress::Unbound,
        mode: "Default".into(),
        name: None,
        actions,
    }
}
fn actions(source: usize) -> Vec<Action> {
    let mut result = vec![
        Action::Conditional {
            condition: Condition::ButtonPressed {
                input: InputAddress::Unbound,
            },
            if_true: vec![],
            if_false: vec![],
        },
        Action::Invert,
    ];
    if source == 0 {
        result.swap(0, 1);
    }
    result
}
fn profile(source: usize) -> Profile {
    Profile::new(
        "Sortable".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![mapping(actions(source))],
        vec![],
        "Default".into(),
    )
}
fn root() -> Element {
    let f = use_context::<Rc<Fixture>>();
    let index = f.source_index;
    let b = inputforge_app::ui::bindings();
    let state = (b.pipeline_sortable.state)();
    let source = (b.pipeline_sortable.handle)(HandleProps {
        state,
        index,
        group: StageId(vec![]),
        context: "original".into(),
        draggable: true,
    });
    let ancestor = (b.pipeline_sortable.handle)(HandleProps {
        state,
        index: 1 - index,
        group: StageId(vec![]),
        context: "original".into(),
        draggable: true,
    });
    let controller = use_hook(|| Controller::new(Arc::clone(&f.shared), f.commands.clone()));
    let cb = use_hook(|| controller.bindings());
    let history = use_signal(View::default);
    let mode = use_signal(|| "Default".into());
    let selection = use_signal(|| Some(("Default".into(), InputAddress::Unbound)));
    use_hook(|| {
        cb.install.call(Publication {
            history,
            mode,
            selection,
        });
    });
    let profile = f
        .shared
        .read()
        .active_profile
        .as_ref()
        .unwrap()
        .id()
        .clone();
    let on_drop = Callback::new(move |gap| {
        let profile = profile.clone();
        b.pipeline_drop.call(PipelineDrop {
            state,
            parent: branch(index),
            gap,
            title: Callback::new(|_| "Invert".to_owned()),
            submit: Callback::new(move |(edit, label)| {
                cb.edit
                    .call(Request {
                        profile: Some(profile.clone()),
                        before: mapping(actions(index)),
                        edit,
                        label,
                        group: None,
                        editor: true,
                    })
                    .unwrap_or(false)
            }),
        });
    });
    let gap = (b.pipeline_sortable.gap)(GapProps {
        state,
        gap_index: 0,
        group: branch(index),
        on_drop,
    });
    *f.output.borrow_mut() = Some(Handles {
        source,
        ancestor,
        gap,
        state,
        controller,
    });
    rsx! {}
}
fn mount_source(
    source: usize,
) -> (
    VirtualDom,
    Rc<Fixture>,
    Handles,
    mpsc::Receiver<EngineCommand>,
) {
    let (commands, rx) = mpsc::channel();
    let f = Rc::new(Fixture {
        shared: Arc::new(RwLock::new(AppState::with_profile(profile(source)))),
        commands,
        output: RefCell::new(None),
        source_index: source,
    });
    let mut dom = VirtualDom::new(root);
    dom.provide_root_context(Rc::clone(&f));
    dom.rebuild_in_place();
    let h = f.output.borrow().as_ref().unwrap().clone();
    (dom, f, h, rx)
}
fn event() -> DragEvent {
    Event::new(Rc::new(DragData::from(serde_json::from_value::<SerializedDragData>(serde_json::json!({"mouse":SerializedMouseData::default(),"data_transfer":{"items":[],"files":[],"effect_allowed":"move","drop_effect":"move"}})).unwrap())),true)
}
#[test]
fn empty_branch_drop_delivers_one_command_and_one_history_entry_and_undo_restores() {
    let (_dom, f, h, rx) = mount_source(1);
    h.source.start.call(event());
    h.gap.over.call(event());
    assert!(!h.state.drop_target.peek().as_ref().unwrap().invalid);
    h.gap.drop.call(event());
    h.gap.drop.call(event());
    let EngineCommand::SetMapping {
        actions: result, ..
    } = rx.try_recv().unwrap()
    else {
        panic!("mapping command expected")
    };
    assert_eq!(result.len(), 1);
    assert!(matches!(&result[0],Action::Conditional{if_true,..} if if_true==&vec![Action::Invert]));
    let EngineCommand::MappingEditBarrier { owner, serial } = rx.try_recv().unwrap() else {
        panic!("barrier expected")
    };
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    assert_eq!(
        h.controller.view().stacks[&("Default".into(), InputAddress::Unbound)]
            .undo
            .len(),
        1
    );
    f.shared
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(&InputAddress::Unbound, "Default", None, result);
    f.shared.write().mapping_edit_barrier = Some((owner, serial));
    assert!(
        h.controller
            .undo(&("Default".into(), InputAddress::Unbound))
    );
    assert!(
        matches!(rx.try_recv().unwrap(),EngineCommand::SetMapping{actions:restored,..} if restored==actions(1))
    );
}
#[test]
fn same_name_profile_replacement_during_drag_receives_no_command_or_success_announcement() {
    let (_dom, f, h, rx) = mount_source(1);
    h.source.start.call(event());
    f.shared.write().active_profile = Some(profile(1));
    h.gap.drop.call(event());
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    assert!(h.state.live_announcement.peek().is_empty());
    assert!(h.controller.view().stacks.is_empty());
}

#[test]
fn moving_an_earlier_source_into_a_later_branch_rebases_the_target_path() {
    let (_dom, _f, h, rx) = mount_source(0);
    h.source.start.call(event());
    h.gap.drop.call(event());
    let EngineCommand::SetMapping {
        actions: result, ..
    } = rx
        .try_recv()
        .expect("move must target the shifted Conditional")
    else {
        panic!("mapping command expected")
    };
    assert_eq!(result.len(), 1);
    assert!(matches!(&result[0],Action::Conditional{if_true,..} if if_true==&vec![Action::Invert]));
}

#[test]
fn ancestor_cannot_drop_into_its_own_empty_branch() {
    let (_dom, _f, h, rx) = mount_source(1);
    h.ancestor.start.call(event());
    h.gap.over.call(event());
    assert!(h.state.drop_target.peek().as_ref().unwrap().invalid);
    h.gap.drop.call(event());
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    assert!(h.controller.view().stacks.is_empty());
}

#[test]
fn action_tree_replacement_during_drag_receives_no_command_or_history() {
    let (_dom, f, h, rx) = mount_source(1);
    h.source.start.call(event());
    f.shared
        .write()
        .active_profile
        .as_mut()
        .unwrap()
        .set_mapping(
            &InputAddress::Unbound,
            "Default",
            None,
            vec![Action::Invert],
        );
    h.gap.drop.call(event());
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    assert!(h.state.live_announcement.peek().is_empty());
    assert!(h.controller.view().stacks.is_empty());
}

#[test]
fn final_dispatch_rejects_direct_cyclic_gap_intents_without_commands() {
    let (_dom, f, h, rx) = mount_source(1);
    let request = Request {
        profile: Some(
            f.shared
                .read()
                .active_profile
                .as_ref()
                .unwrap()
                .id()
                .clone(),
        ),
        before: mapping(actions(1)),
        edit: inputforge_gui_dx::mapping_edits::Edit::MoveGap {
            from: StageId(vec![StageIdSegment::Index(0)]),
            parent: branch(1),
            gap: 0,
        },
        label: "cycle".into(),
        group: None,
        editor: true,
    };
    assert!(!h.controller.dispatch(request, std::time::Instant::now()));
    assert_eq!(rx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty);
    assert!(h.controller.view().stacks.is_empty());
}
