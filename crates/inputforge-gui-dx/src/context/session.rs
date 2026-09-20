//! Capture changes must wake the bridge even when sampled values are identical.
use dioxus::prelude::ReadableExt;
use inputforge_core::state::AppState;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct CaptureEpoch {
    pub generation: u64,
    pub ready: bool,
    pub allowed: bool,
}
impl CaptureEpoch {
    pub(crate) fn from_state(state: &AppState) -> Self {
        Self {
            generation: state.session.generation,
            ready: state.session.ready,
            allowed: !state.session.offline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ConfigSnapshot, LiveSnapshot};
    use inputforge_core::types::{HatDirection, VirtualDeviceConfig};
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
        let config = ConfigSnapshot::from_state(&state, None);
        let before = LiveSnapshot::from_state(&state, &config);
        assert_eq!(before.output_values[0].hats, [HatDirection::NE]);
        state.session.generation += 1;
        let after = LiveSnapshot::from_state(&state, &config);
        assert_ne!(before, after);
    }
}

#[cfg(test)]
mod monitoring_tests {
    use super::*;
    use inputforge_core::state::EngineStatus;

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
            assert!(CaptureEpoch::from_state(&state).allowed);
            assert!(CaptureEpoch::from_state(&state).ready);
        }
        state.session.ready = false;
        assert!(!CaptureEpoch::from_state(&state).ready);
        state.session.offline = true;
        assert!(!CaptureEpoch::from_state(&state).allowed);
    }
}

impl super::AppContext {
    /// Shared GUI gate for a retained virtual layout awaiting explicit Apply.
    pub(crate) fn controller_changes_pending(&self) -> bool {
        let meta = self.meta.read();
        if matches!(
            meta.engine_status,
            inputforge_core::state::EngineStatus::Running
                | inputforge_core::state::EngineStatus::Starting
        ) {
            return false;
        }
        self.config
            .read()
            .controllers
            .as_ref()
            .is_some_and(|config| {
                meta.session
                    .requires_output_reconfiguration(&config.virtual_devices)
            })
    }
}
