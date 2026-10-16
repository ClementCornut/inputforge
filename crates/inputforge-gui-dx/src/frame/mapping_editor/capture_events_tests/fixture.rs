//! Actual capture buttons deliver through app bindings into existing edit handlers.
use super::super::{
    EditorState,
    header::Header,
    pipeline::stage_body::StageBody,
    undo_log::{StageId, StageIdSegment},
};
use crate::{
    context::{AppContext, LiveSnapshot, SettingsSnapshot},
    patterns::live_capture::use_live_capture_provider,
    toast::{ToastQueue, ToastState},
};
use dioxus::prelude::*;
use inputforge_app::live_capture::Controller;
use inputforge_core::{
    action::{Action, Condition, Mapping},
    engine::EngineCommand,
    mode::Modes,
    profile::Profile,
    state::AppState,
    types::{AxisPolarity, AxisValue, DeviceId, InputAddress, InputId, InputValue, MergeOp},
};
use parking_lot::RwLock;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};

#[derive(Clone, Copy)]
pub(super) enum Surface {
    Add,
    Header,
    Merge,
    Predicate,
}
pub(super) struct Fixture {
    pub(super) state: Arc<RwLock<AppState>>,
    commands: mpsc::Sender<EngineCommand>,
    surface: Surface,
    pub(super) controller: RefCell<Option<Controller>>,
    pub(super) editor: RefCell<Option<EditorState>>,
    pub(super) config: RefCell<Option<Signal<crate::context::ConfigSnapshot>>>,
    invalid_path: bool,
}
pub(super) fn address(input: InputId) -> InputAddress {
    InputAddress::Bound {
        device: DeviceId("capture-stick".into()),
        input,
    }
}
pub(super) fn primary() -> InputAddress {
    address(InputId::Button { index: 0 })
}
pub(super) fn actions() -> Vec<Action> {
    vec![
        Action::MergeAxis {
            second_input: InputAddress::Unbound,
            operation: MergeOp::Bidirectional,
        },
        Action::Conditional {
            condition: Condition::ButtonPressed {
                input: InputAddress::Unbound,
            },
            if_true: vec![],
            if_false: vec![],
        },
    ]
}
fn harness() -> Element {
    let fixture = use_context::<Rc<Fixture>>();
    let controller =
        use_hook(|| Controller::new(Arc::clone(&fixture.state), fixture.commands.clone()));
    *fixture.controller.borrow_mut() = Some(controller.clone());
    let bindings = controller.bindings();
    use_context_provider(|| bindings);
    let key = ("Default".into(), primary());
    let frame = inputforge_app::bridge::prepare(&fixture.state.read(), Some(&key));
    let meta = use_signal(|| frame.meta);
    let config = use_signal(|| frame.config);
    *fixture.config.borrow_mut() = Some(config);
    let live = use_signal(LiveSnapshot::default);
    let settings = use_signal(SettingsSnapshot::default);
    use_context_provider(|| AppContext {
        state: Arc::clone(&fixture.state),
        commands: fixture.commands.clone(),
        meta,
        config,
        live,
        settings,
    });
    let mut view = crate::frame::use_view_state_provider(meta);
    use_hook(|| view.selected_mapping.set(Some(key.clone())));
    use_context_provider(|| view);
    let editor = super::super::use_editor_state_provider();
    *fixture.editor.borrow_mut() = Some(editor);
    let toast = use_signal(ToastState::default);
    use_context_provider(|| ToastQueue { state: toast });
    use_live_capture_provider();
    let sortable = crate::components::sortable::use_sortable_state::<StageId>();
    use_context_provider(|| sortable);
    match fixture.surface {
        Surface::Add => rsx! { crate::frame::mapping_list::MappingList {} },
        Surface::Header => {
            let name = fixture
                .state
                .read()
                .active_profile
                .as_ref()
                .unwrap()
                .find_mapping(&primary(), "Default")
                .unwrap()
                .name
                .clone()
                .unwrap_or_else(|| "Untitled mapping".into());
            rsx! { Header { name, input: primary(), mapping_key: key, actions: config.read().selected_mapping_actions.clone().unwrap_or_default() } }
        }
        Surface::Predicate => {
            rsx! { super::super::pipeline::stage_body::predicate::PredicateEditor { mapping_key: key, stage_id: StageId(vec![StageIdSegment::Index(1)]), condition: Condition::ButtonPressed { input: InputAddress::Unbound }, if_true: vec![], if_false: vec![], root_actions: actions() } }
        }
        Surface::Merge => {
            rsx! { StageBody { mapping_key: key, stage_id: StageId(vec![StageIdSegment::Index(if fixture.invalid_path { 99 } else { 0 })]), action: actions()[0].clone(), root_actions: actions() } }
        }
    }
}
pub(super) fn fixture_with_name(
    surface: Surface,
    name: Option<String>,
    other_mode: bool,
    invalid_path: bool,
) -> (VirtualDom, Rc<Fixture>, mpsc::Receiver<EngineCommand>) {
    let profile = Profile::new(
        "Fixture".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Other".into()]).unwrap(),
        {
            let mut mappings = vec![Mapping {
                input: primary(),
                mode: "Default".into(),
                name,
                actions: actions(),
            }];
            if other_mode {
                mappings.push(Mapping {
                    input: primary(),
                    mode: "Other".into(),
                    name: Some("Other name".into()),
                    actions: actions(),
                });
            }
            mappings
        },
        vec![],
        "Default".into(),
    );
    let mut state = AppState::with_profile(profile);
    state.session.ready = true;
    state.input_cache.update(
        &address(InputId::Button { index: 3 }),
        &InputValue::Button { pressed: false },
    );
    state.input_cache.update(
        &address(InputId::Axis { index: 4 }),
        &InputValue::Axis {
            value: AxisValue::new(0.0),
            polarity: AxisPolarity::Bipolar,
        },
    );
    let (commands, received) = mpsc::channel();
    let fixture = Rc::new(Fixture {
        state: Arc::new(RwLock::new(state)),
        commands,
        surface,
        invalid_path,
        controller: RefCell::new(None),
        editor: RefCell::new(None),
        config: RefCell::default(),
    });
    let mut dom = VirtualDom::new(harness);
    dom.provide_root_context(Rc::clone(&fixture));
    dom.rebuild_in_place();
    (dom, fixture, received)
}
pub(super) fn fixture(
    surface: Surface,
) -> (VirtualDom, Rc<Fixture>, mpsc::Receiver<EngineCommand>) {
    fixture_with_name(surface, Some("Fixture".into()), false, false)
}
