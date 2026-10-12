use crate::bridge::{config, live};
use inputforge_core::{
    state::{AppState, EngineStatus},
    types::{HatDirection, VirtualDeviceConfig},
};
#[test]
fn live_projection_uses_one_based_hats_and_changes_on_snapshot_generation() {
    let mut state = AppState::new();
    state.virtual_devices.push(VirtualDeviceConfig {
        device_id: 1,
        axes: vec![],
        button_count: 2,
        hat_count: 1,
    });
    state.output_cache.set_hat(1, 1, HatDirection::NE);
    let config = config::project(&state, None);
    let before = live::project(&state, &config);
    assert_eq!(before.output_values[0].hats, [HatDirection::NE]);
    state.session.generation += 1;
    assert_ne!(before, live::project(&state, &config));
}
#[test]
fn rebinding_depends_on_monitoring_and_thread_availability_not_routing() {
    let mut state = AppState::new();
    state.session.ready = true;
    for status in [
        EngineStatus::Stopped,
        EngineStatus::Running,
        EngineStatus::Faulted,
    ] {
        state.engine_status = status;
        assert!(live::capture_epoch(&state).allowed);
        assert!(live::capture_epoch(&state).ready);
    }
    state.session.ready = false;
    assert!(!live::capture_epoch(&state).ready);
    state.session.offline = true;
    assert!(!live::capture_epoch(&state).allowed);
}
