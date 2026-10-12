//! Sample coherent input, output and capture-session values.
use inputforge_core::pipeline::InputCache;
use inputforge_core::state::AppState;
use inputforge_core::types::{InputAddress, InputId};
use inputforge_gui_dx::snapshots::{
    CaptureEpoch, ConfigSnapshot, DeviceInputValues, LiveSnapshot, OutputActivitySnapshot,
    VjoyOutputValues,
};
use std::time::Instant;
/// Takes a pre-built `ConfigSnapshot` so device / virtual-device shape is
/// read from a single coherent source.
#[cfg(test)]
pub(super) fn project(s: &AppState, cfg: &ConfigSnapshot) -> LiveSnapshot {
    project_at(s, cfg, Instant::now())
}

/// Project live state at a caller-supplied time for deterministic tests.
pub(super) fn project_at(s: &AppState, cfg: &ConfigSnapshot, now: Instant) -> LiveSnapshot {
    let device_inputs: Vec<DeviceInputValues> = cfg
        .devices
        .iter()
        .map(|device| {
            let did = &device.info.id;
            DeviceInputValues {
                axes: (0..device.info.axes)
                    .map(|i| {
                        let addr = InputAddress::Bound {
                            device: did.clone(),
                            input: InputId::Axis { index: i },
                        };
                        // Polarity source: device.info.axis_polarities
                        // (the lazy-classification table updated on
                        // re-probe). Fall back to Bipolar when the
                        // device entry is short, matching pre-Task-1
                        // behavior. The cache's polarity tag is
                        // unused here intentionally.
                        let (value, _cache_polarity) = s.input_cache.get_axis(&addr);
                        let pol = device
                            .info
                            .axis_polarities
                            .get(usize::from(i))
                            .copied()
                            .unwrap_or_default();
                        (value, pol)
                    })
                    .collect(),
                buttons: (0..device.info.buttons)
                    .map(|i| {
                        let addr = InputAddress::Bound {
                            device: did.clone(),
                            input: InputId::Button { index: i },
                        };
                        s.input_cache.get_button(&addr)
                    })
                    .collect(),
                hats: (0..device.info.hats)
                    .map(|i| {
                        let addr = InputAddress::Bound {
                            device: did.clone(),
                            input: InputId::Hat { index: i },
                        };
                        s.input_cache.get_hat(&addr)
                    })
                    .collect(),
            }
        })
        .collect();

    let output_values: Vec<VjoyOutputValues> = cfg
        .virtual_devices
        .iter()
        .map(|v| VjoyOutputValues {
            axes: v
                .axes
                .iter()
                .map(|&a| (a, s.output_cache.get_axis(v.device_id, a)))
                .collect(),
            buttons: (1..=v.button_count)
                .map(|i| s.output_cache.get_button(v.device_id, i))
                .collect(),
            hats: (1..=v.hat_count)
                .map(|i| s.output_cache.get_hat(v.device_id, i))
                .collect(),
        })
        .collect();

    let mut output_activity: Vec<OutputActivitySnapshot> = s
        .output_activity
        .active_entries(now)
        .map(|(owner, value)| OutputActivitySnapshot {
            owner: owner.clone(),
            value,
        })
        .collect();
    output_activity.sort_by_cached_key(|entry| format!("{:?}", entry.owner));

    LiveSnapshot {
        capture_epoch: capture_epoch(s),
        device_inputs,
        output_values,
        output_activity,
    }
}

pub(super) fn capture_epoch(state: &AppState) -> CaptureEpoch {
    CaptureEpoch {
        generation: state.session.generation,
        ready: state.session.ready,
        allowed: !state.session.offline,
    }
}
