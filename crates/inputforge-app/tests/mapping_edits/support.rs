use inputforge_app::mapping_edits::Controller;
use inputforge_core::{
    action::{Action, Mapping},
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    state::AppState,
    types::InputAddress,
};
use inputforge_gui_dx::mapping_edits::{Edit, Request, StageId, StageIdSegment};
use parking_lot::RwLock;
use std::{
    sync::{Arc, mpsc},
    time::Instant,
};

pub(super) fn mapping(actions: Vec<Action>) -> Mapping {
    Mapping {
        input: InputAddress::Unbound,
        mode: "Default".into(),
        name: None,
        actions,
    }
}
pub(super) fn fixture(
    actions: Vec<Action>,
) -> (
    Controller,
    Arc<RwLock<AppState>>,
    mpsc::Receiver<EngineCommand>,
) {
    let profile = Profile::new(
        "Fixture".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Other".into()]).unwrap(),
        vec![mapping(actions)],
        vec![],
        "Default".into(),
    );
    let state = Arc::new(RwLock::new(AppState::with_profile(profile)));
    let (tx, rx) = mpsc::channel();
    (Controller::new(Arc::clone(&state), tx), state, rx)
}
pub(super) fn request(state: &Arc<RwLock<AppState>>, before: Mapping, edit: Edit) -> Request {
    Request {
        profile: state
            .read()
            .active_profile
            .as_ref()
            .map(Profile::id)
            .cloned(),
        before,
        edit,
        label: "edit".into(),
        group: None,
        editor: true,
    }
}
pub(super) fn stage(index: usize) -> StageId {
    StageId(vec![StageIdSegment::Index(index)])
}
pub(super) fn replace(
    state: &Arc<RwLock<AppState>>,
    before: Mapping,
    index: usize,
    action: Action,
) -> Request {
    request(
        state,
        before,
        Edit::Replace {
            path: stage(index),
            action,
        },
    )
}
pub(super) fn apply(state: &Arc<RwLock<AppState>>, rx: &mpsc::Receiver<EngineCommand>) -> Mapping {
    let mapping = match rx.try_recv().unwrap() {
        EngineCommand::SetMapping {
            input,
            mode,
            name,
            actions,
        } => {
            let mapping = Mapping {
                input,
                mode,
                name,
                actions,
            };
            state.write().active_profile.as_mut().unwrap().set_mapping(
                &mapping.input,
                &mapping.mode,
                mapping.name.clone(),
                mapping.actions.clone(),
            );
            mapping
        }
        EngineCommand::RemoveMapping { input, mode } => {
            state
                .write()
                .active_profile
                .as_mut()
                .unwrap()
                .remove_mapping(&input, &mode);
            Mapping {
                input,
                mode,
                name: None,
                actions: vec![],
            }
        }
        other => panic!("unexpected command {other:?}"),
    };
    let EngineCommand::MappingEditBarrier { owner, serial } = rx.try_recv().unwrap() else {
        panic!("mapping fence expected")
    };
    state.write().mapping_edit_barrier = Some((owner, serial));
    mapping
}

pub(super) fn now() -> Instant {
    Instant::now()
}
pub(super) fn key() -> (String, InputAddress) {
    ("Default".into(), InputAddress::Unbound)
}

pub(super) fn value(max: f64) -> Action {
    Action::ResponseCurve {
        curve: inputforge_core::processing::curves::ResponseCurve::piecewise_linear(
            vec![(-1.0, -1.0), (1.0, max)],
            false,
        )
        .unwrap(),
    }
}

pub(super) fn bind(state: &Arc<RwLock<AppState>>, input: InputAddress) -> Mapping {
    let mut state = state.write();
    let profile = state.active_profile.as_mut().unwrap();
    let mut m = profile.find_mapping(&key().1, &key().0).unwrap().clone();
    profile.remove_mapping(&m.input, &m.mode);
    m.input = input;
    profile.set_mapping(&m.input, &m.mode, m.name.clone(), m.actions.clone());
    m
}
