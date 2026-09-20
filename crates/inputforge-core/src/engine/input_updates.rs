//! Ordered samples and edges share one path on every platform.
use super::Engine;
use crate::{device::InputUpdate, error::Result, types::InputValue};
impl Engine {
    pub(super) fn poll_input(&mut self) -> Result<()> {
        let mut updates = Vec::new();
        self.input.poll(&mut updates)?;
        let hotplug = self.input.hotplug_events();
        self.handle_hotplug(&hotplug);
        if (self.state.read().session.bindings.is_empty()
            || !hotplug.is_empty()
            || self
                .state
                .read()
                .active_profile
                .as_ref()
                .is_some_and(|p| p.controllers().is_none()))
            && let Err(error) = self.sync_controller_bindings()
        {
            self.state.write().session.notice =
                Some(format!("Could not save controller settings: {error}"));
        }
        let mut routed = false;
        let mut failure = None;
        for update in updates {
            match update {
                InputUpdate::AxisSample {
                    device,
                    index,
                    value,
                } => {
                    if let Err(error) = self.observe_axis(&device, index, value) {
                        self.state.write().session.notice = Some(error.to_string());
                    }
                }
                InputUpdate::Reset { device } => {
                    self.awaiting_snapshots.insert(device.clone());
                    if let Err(error) = self.reset_device(&device) {
                        self.fault(&error);
                        failure = Some(error);
                    }
                }
                InputUpdate::Snapshot { device, values, .. } => {
                    if let Err(error) = self.reset_device(&device) {
                        self.fault(&error);
                        failure = Some(error);
                    }
                    self.blocked_inputs.retain(|a| a.device() != Some(&device));
                    self.state.write().input_cache.evict_device(&device);
                    for mut event in values {
                        self.apply_axis_polarity(&mut event);
                        if matches!(
                            event.value,
                            InputValue::Button { pressed: true } | InputValue::Hat { .. }
                        ) {
                            self.blocked_inputs.insert(event.source.clone());
                        }
                        self.state
                            .write()
                            .input_cache
                            .update(&event.source, &event.value);
                    }
                    self.awaiting_snapshots.remove(&device);
                    self.mark_monitored(device);
                    self.pending_output_refresh = true;
                }
                InputUpdate::Frame(events) => self.process_frame(events, &mut routed, &mut failure),
            }
        }
        self.refresh_mapping_issues()?;
        self.event_buffer.clear();
        if !routed || self.pending_output_refresh {
            self.route_events()?;
        }
        failure.map_or(Ok(()), Err)
    }
    fn process_frame(
        &mut self,
        events: Vec<crate::types::InputEvent>,
        routed: &mut bool,
        failure: &mut Option<crate::error::EngineError>,
    ) {
        for mut event in events {
            self.apply_axis_polarity(&mut event);
            if let Some(device) = event.source.device() {
                if self.awaiting_snapshots.contains(device) {
                    continue;
                }
                self.mark_monitored(device.clone());
            }
            let previous_hat =
                crate::pipeline::InputCache::get_hat(&self.state.read().input_cache, &event.source);
            self.state
                .write()
                .input_cache
                .update(&event.source, &event.value);
            if self.blocked_inputs.contains(&event.source) {
                match event.value {
                    InputValue::Hat { direction } => {
                        if direction == previous_hat {
                            continue;
                        }
                        self.blocked_inputs.remove(&event.source);
                    }
                    InputValue::Button { pressed: false } => {
                        self.blocked_inputs.remove(&event.source);
                        continue;
                    }
                    _ => continue,
                }
            }
            self.event_buffer = vec![event];
            if let Err(error) = self
                .refresh_mapping_issues()
                .and_then(|()| self.route_events())
            {
                self.fault(&error);
                *failure = Some(error);
            }
            *routed = true;
        }
    }
    fn reset_device(&mut self, device: &crate::types::DeviceId) -> Result<()> {
        let mut state = self.state.write();
        state.session.monitored.retain(|id| id != device);
        state.session.ready = !state.session.monitored.is_empty();
        state.input_cache.evict_device(device);
        state.session.generation = state.session.generation.wrapping_add(1);
        drop(state);
        self.refresh_mapping_issues()
    }
    fn mark_monitored(&mut self, device: crate::types::DeviceId) {
        let mut state = self.state.write();
        if !state.session.monitored.contains(&device) {
            state.session.monitored.push(device);
            state.session.generation = state.session.generation.wrapping_add(1);
        }
        state.session.ready = !state.session.monitored.is_empty();
    }
    pub(super) fn block_held_input(&mut self, input: &crate::types::InputAddress) {
        let sampled = self.state.read().input_cache.clone_compact();
        if sampled.iter().any(|entry| {
            &entry.address == input
                && matches!(
                    entry.value,
                    InputValue::Button { pressed: true } | InputValue::Hat { .. }
                )
        }) {
            self.blocked_inputs.insert(input.clone());
        }
    }
    pub(super) fn block_sampled_inputs(&mut self) {
        for entry in self.state.read().input_cache.clone_compact() {
            if matches!(
                entry.value,
                InputValue::Button { pressed: true } | InputValue::Hat { .. }
            ) {
                self.blocked_inputs.insert(entry.address);
            }
        }
    }
}
