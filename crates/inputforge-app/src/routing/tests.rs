use super::policy;
use inputforge_core::{
    mode::Modes,
    profile::{Profile, controllers::ControllerConfig},
    state::{AppState, EngineStatus},
    types::VirtualDeviceConfig,
};
use inputforge_gui_dx::routing::Action;

fn state() -> AppState {
    let mut state = AppState::new();
    let mut profile = Profile::new(
        "routing".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    profile
        .set_controllers(ControllerConfig::default())
        .unwrap();
    state.active_profile = Some(profile);
    state
}

fn pending_state() -> AppState {
    let mut state = state();
    state.session.output_active = true;
    state.session.output_layout = vec![VirtualDeviceConfig {
        device_id: 1,
        axes: vec![],
        button_count: 2,
        hat_count: 0,
    }];
    state
}

#[test]
fn stopped_session_can_start_without_ready_inputs() {
    let state = state();
    assert!(!state.session.ready);
    let view = policy::view(&state);
    assert_eq!(view.toggle, Some(Action::Start));
    assert_eq!(view.start, Some(Action::Start));
    assert_eq!(view.stop, None);
    assert_eq!(view.refresh, Some(Action::Refresh));
}

#[test]
fn running_and_starting_can_stop_even_without_a_profile() {
    let mut state = state();
    state.active_profile = None;
    for status in [EngineStatus::Starting, EngineStatus::Running] {
        state.engine_status = status;
        let view = policy::view(&state);
        assert_eq!(view.toggle, Some(Action::Stop));
        assert_eq!(view.stop, Some(Action::Stop));
        assert_eq!(view.start, None);
    }
}

#[test]
fn retained_layout_requires_review_until_routing_is_active() {
    let mut state = pending_state();
    for status in [EngineStatus::Stopped, EngineStatus::Faulted] {
        state.engine_status = status;
        let view = policy::view(&state);
        assert!(view.pending);
        assert_eq!(view.toggle, Some(Action::ReviewDevices));
        assert_eq!(view.recovery, Some(Action::ReviewDevices));
        assert_eq!(view.start, None);
    }
    state.engine_status = EngineStatus::Running;
    assert!(!policy::view(&state).pending);
    assert_eq!(policy::view(&state).toggle, Some(Action::Stop));
}

#[test]
fn offline_session_disables_every_action_including_review() {
    let mut state = pending_state();
    state.session.offline = true;
    let view = policy::view(&state);
    assert_eq!(view.toggle, None);
    assert_eq!(view.start, None);
    assert_eq!(view.stop, None);
    assert_eq!(view.recovery, None);
    assert_eq!(view.refresh, None);
}

#[test]
fn faulted_session_offers_retry_but_missing_profile_only_allows_refresh_and_cleanup() {
    let mut state = state();
    state.engine_status = EngineStatus::Faulted;
    assert_eq!(policy::view(&state).start, Some(Action::Retry));
    assert_eq!(policy::view(&state).recovery, Some(Action::Retry));
    state.active_profile = None;
    let view = policy::view(&state);
    assert_eq!(view.toggle, None);
    assert_eq!(view.start, None);
    assert_eq!(view.recovery, None);
    assert_eq!(view.refresh, Some(Action::Refresh));
    assert_eq!(view.stop, Some(Action::Stop));
}

#[test]
fn stale_start_reviews_layout_without_holding_engine_lock() {
    let state = parking_lot::RwLock::new(pending_state());
    let (tx, rx) = std::sync::mpsc::channel();
    let reviewed = std::cell::Cell::new(false);
    super::dispatch(Action::Start, &state, &tx, || {
        assert!(state.try_write().is_some());
        reviewed.set(true);
    })
    .unwrap();
    assert!(reviewed.get());
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
}

#[test]
fn dispatch_rechecks_profile_offline_and_running_changes() {
    let state = parking_lot::RwLock::new(state());
    let (tx, rx) = std::sync::mpsc::channel();
    for change in 0..3 {
        {
            let mut current = state.write();
            *current = self::state();
            let state = &mut *current;
            match change {
                0 => state.active_profile = None,
                1 => state.session.offline = true,
                _ => {
                    state.session.offline = false;
                    state.engine_status = EngineStatus::Running;
                }
            }
        }
        super::dispatch(Action::Start, &state, &tx, || panic!("unexpected review")).unwrap();
        assert_eq!(
            rx.try_recv().unwrap_err(),
            std::sync::mpsc::TryRecvError::Empty
        );
    }
    super::dispatch(Action::Stop, &state, &tx, || panic!("unexpected review")).unwrap();
    assert_eq!(
        rx.try_recv().unwrap(),
        inputforge_core::engine::EngineCommand::Deactivate
    );
}

#[test]
fn recovery_and_refresh_send_their_commands_and_report_disconnection() {
    let mut initial = state();
    initial.engine_status = EngineStatus::Faulted;
    let state = parking_lot::RwLock::new(initial);
    let (tx, rx) = std::sync::mpsc::channel();
    for (action, expected) in [
        (Action::Retry, inputforge_core::engine::EngineCommand::Retry),
        (
            Action::Refresh,
            inputforge_core::engine::EngineCommand::RefreshInput,
        ),
    ] {
        super::dispatch(action, &state, &tx, || panic!("unexpected review")).unwrap();
        assert_eq!(rx.try_recv().unwrap(), expected);
    }
    drop(rx);
    let result = super::dispatch(Action::Retry, &state, &tx, || panic!("unexpected review"));
    let _ = result.unwrap_err();
}

#[test]
fn stale_review_does_not_start_routing_after_layout_was_applied() {
    let state = parking_lot::RwLock::new(state());
    let (tx, rx) = std::sync::mpsc::channel();
    super::dispatch(Action::ReviewDevices, &state, &tx, || {
        panic!("unexpected review")
    })
    .unwrap();
    assert_eq!(
        rx.try_recv().unwrap_err(),
        std::sync::mpsc::TryRecvError::Empty
    );
    super::dispatch(Action::Start, &state, &tx, || panic!("unexpected review")).unwrap();
    assert_eq!(
        rx.try_recv().unwrap(),
        inputforge_core::engine::EngineCommand::Activate
    );
}
