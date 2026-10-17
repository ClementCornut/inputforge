//! Deadline behavior is exercised with the real nonblocking bridge sampler.
use super::super::Poller;
use inputforge_core::{
    action::{Action, Mapping, MouseTarget, OutputBehavior},
    mode::Modes,
    pipeline::{ActionPathSegment, OutputDestination, OutputOwner},
    profile::Profile,
    state::{AppState, EngineStatus, OUTPUT_ACTIVITY_PREVIEW_LATCH, OutputActivityValue},
    types::InputAddress,
};
use parking_lot::RwLock;
use std::{
    sync::{Arc, mpsc},
    time::Duration,
};
fn fixture() -> (Arc<RwLock<AppState>>, Poller, OutputOwner) {
    let input = InputAddress::Unbound;
    let profile = Profile::new(
        "P".into(),
        vec![],
        Modes::new(vec!["Default".into(), "Combat".into()]).unwrap(),
        vec![Mapping {
            input: input.clone(),
            mode: "Default".into(),
            name: None,
            actions: vec![Action::TapGesture {
                threshold_ms: 200,
                fire_single_immediately: false,
                single_tap: vec![Action::MapToMouse {
                    target: MouseTarget::LeftButton,
                    behavior: OutputBehavior::Pulse,
                }],
                double_tap: vec![],
            }],
        }],
        vec![],
        "Default".into(),
    );
    let mut state = AppState::with_profile(profile);
    state.engine_status = EngineStatus::Running;
    let owner = OutputOwner {
        profile: "memory-profile".into(),
        mode: "Default".into(),
        input,
        action_path: vec![
            ActionPathSegment::Index(0),
            ActionPathSegment::Branch(inputforge_core::action::ActionBranch::TapSingle),
            ActionPathSegment::Index(0),
        ],
        destination: OutputDestination::Mouse(MouseTarget::LeftButton),
        behavior: OutputBehavior::Pulse,
    };
    let state = Arc::new(RwLock::new(state));
    let (tx, _rx) = mpsc::channel();
    let capture = crate::live_capture::Controller::new(Arc::clone(&state), tx);
    let poller = Poller::new(Arc::clone(&state), capture);
    (state, poller, owner)
}
#[tokio::test(start_paused = true)]
async fn output_preview_expires_on_the_existing_tick_even_when_engine_is_contended() {
    let (state, mut poller, owner) = fixture();
    let now = tokio::time::Instant::now().into_std();
    let key = ("Default".into(), InputAddress::Unbound);
    state
        .write()
        .output_activity
        .record(owner, OutputActivityValue::Mouse(true), now, false);
    let initial = poller.sample(Some(&key), now, |_| None).unwrap();
    assert!(initial.live.readout.unwrap().model.outputs[0].is_active);
    tokio::time::advance(OUTPUT_ACTIVITY_PREVIEW_LATCH + Duration::from_millis(16)).await;
    let _locked = state.write();
    let now = tokio::time::Instant::now().into_std();
    assert!(
        poller
            .sample(Some(&key), now, |_| panic!(
                "contended tick cannot do metadata I/O"
            ))
            .is_none()
    );
    let super::activity::Change::Value(live) = poller.readout.expire(Some(&key), now).unwrap()
    else {
        panic!("unchanged selection retains model")
    };
    assert!(!live.model.outputs[0].is_active);
    assert!(poller.readout.expire(Some(&key), now).is_none());
}
#[test]
fn unchanged_observation_does_not_extend_deadline_and_reselection_clears_cached_activity() {
    let (state, mut poller, owner) = fixture();
    let now = std::time::Instant::now();
    let key = ("Default".into(), InputAddress::Unbound);
    state.write().output_activity.record(
        owner.clone(),
        OutputActivityValue::Mouse(true),
        now,
        false,
    );
    poller.sample(Some(&key), now, |_| None).unwrap();
    poller
        .sample(Some(&key), now + Duration::from_millis(140), |_| None)
        .unwrap();
    state.write().output_activity.clear_owner(&owner);
    let expired = poller
        .sample(Some(&key), now + Duration::from_millis(160), |_| None)
        .unwrap();
    assert!(!expired.live.readout.unwrap().model.outputs[0].is_active);
    assert_eq!(
        poller.readout.expire(None, now),
        Some(super::activity::Change::Clear)
    );
    assert!(
        poller
            .readout
            .expire(Some(&key), now + Duration::from_millis(200))
            .is_none()
    );
}
#[test]
fn profile_replacement_drops_held_output_activity_before_its_deadline() {
    let (state, mut poller, owner) = fixture();
    let now = std::time::Instant::now();
    let key = ("Default".into(), InputAddress::Unbound);
    state
        .write()
        .output_activity
        .record(owner, OutputActivityValue::Mouse(true), now, false);
    poller.sample(Some(&key), now, |_| None).unwrap();
    let replacement = fixture().0.read().active_profile.clone();
    state.write().output_activity.clear();
    state.write().active_profile = replacement;
    let frame = poller
        .sample(Some(&key), now + Duration::from_millis(16), |_| None)
        .unwrap();
    assert!(!frame.live.readout.unwrap().model.outputs[0].is_active);
}
#[test]
fn long_observed_activity_also_expires_when_fresh_engine_reads_stop() {
    let (state, mut poller, owner) = fixture();
    let now = std::time::Instant::now();
    let key = ("Default".into(), InputAddress::Unbound);
    state
        .write()
        .output_activity
        .record(owner, OutputActivityValue::Mouse(true), now, false);
    poller.sample(Some(&key), now, |_| None).unwrap();
    poller
        .sample(Some(&key), now + Duration::from_millis(200), |_| None)
        .unwrap();
    let _locked = state.write();
    let super::activity::Change::Value(expired) = poller
        .readout
        .expire(Some(&key), now + Duration::from_millis(400))
        .expect("stale observed activity needs an expiry even after its original latch elapsed")
    else {
        panic!("unchanged selection retains model")
    };
    assert!(!expired.model.outputs[0].is_active);
    assert!(
        poller
            .readout
            .expire(Some(&key), now + Duration::from_millis(500))
            .is_none()
    );
}
#[test]
fn contended_mode_switch_publishes_a_reset_instead_of_discarding_expiry_state_silently() {
    let (state, mut poller, owner) = fixture();
    let actions = state.read().active_profile.as_ref().unwrap().mappings()[0]
        .actions
        .clone();
    state.write().active_profile.as_mut().unwrap().set_mapping(
        &InputAddress::Unbound,
        "Combat",
        None,
        actions,
    );
    let now = std::time::Instant::now();
    let key = ("Default".into(), InputAddress::Unbound);
    state
        .write()
        .output_activity
        .record(owner, OutputActivityValue::Mouse(true), now, false);
    poller.sample(Some(&key), now, |_| None).unwrap();
    let next = ("Combat".into(), InputAddress::Unbound);
    let _locked = state.write();
    assert!(
        poller
            .sample(Some(&next), now + Duration::from_millis(16), |_| None)
            .is_none()
    );
    assert_eq!(
        poller
            .readout
            .expire(Some(&next), now + Duration::from_millis(16)),
        Some(super::activity::Change::Clear)
    );
    assert!(
        poller
            .readout
            .expire(None, now + Duration::from_millis(32))
            .is_none()
    );
    assert!(
        poller
            .readout
            .expire(Some(&key), now + Duration::from_millis(48))
            .is_none()
    );
}
