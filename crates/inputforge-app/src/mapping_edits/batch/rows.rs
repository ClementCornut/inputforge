//! Input identities, automatic targets and capability reconciliation.
use inputforge_core::{
    profile::controllers::DeviceBinding,
    state::AppState,
    types::{DeviceId, InputAddress, InputId, OutputAddress, OutputId, VirtualDeviceConfig},
};
use inputforge_gui_dx::batch_mapping::{RowKind, RowState, View};
pub(super) fn binding<'a>(state: &'a AppState, id: &DeviceId) -> Option<&'a DeviceBinding> {
    state
        .session
        .bindings
        .iter()
        .find(|b| &b.device == id)
        .or_else(|| {
            state
                .active_profile
                .as_ref()?
                .controllers()?
                .bindings
                .iter()
                .find(|b| &b.device == id)
        })
}
pub(super) fn target(view: &View, state: &AppState) -> Option<VirtualDeviceConfig> {
    state
        .virtual_devices
        .iter()
        .find(|t| Some(t.device_id) == view.target)
        .cloned()
}
pub(super) fn auto_target(
    kind: RowKind,
    index: u8,
    target: &VirtualDeviceConfig,
) -> Option<OutputAddress> {
    let output = match kind {
        RowKind::Axis => OutputId::Axis {
            id: *target.axes.get(usize::from(index))?,
        },
        RowKind::Button => OutputId::Button {
            id: index
                .checked_add(1)
                .filter(|id| *id <= target.button_count)?,
        },
        RowKind::Hat => OutputId::Hat {
            id: index.checked_add(1).filter(|id| *id <= target.hat_count)?,
        },
    };
    Some(OutputAddress {
        device: target.device_id,
        output,
    })
}
pub(super) fn available(
    kind: RowKind,
    address: &OutputAddress,
    target: &VirtualDeviceConfig,
) -> bool {
    address.device == target.device_id
        && match (kind, &address.output) {
            (RowKind::Axis, OutputId::Axis { id }) => target.axes.contains(id),
            (RowKind::Button, OutputId::Button { id }) => *id > 0 && *id <= target.button_count,
            (RowKind::Hat, OutputId::Hat { id }) => *id > 0 && *id <= target.hat_count,
            _ => false,
        }
}
pub(super) fn derive(state: &AppState, view: &View) -> Vec<RowState> {
    let Some(source) = state
        .devices
        .iter()
        .find(|d| d.connected && Some(&d.info.id) == view.source.as_ref())
    else {
        return vec![];
    };
    let Some(target) = target(view, state).filter(|t| t.validate().is_ok()) else {
        return vec![];
    };
    let binding = binding(state, &source.info.id);
    if binding.is_some_and(|b| b.validate().is_err()) {
        return vec![];
    }
    let counts = binding.map_or(
        (source.info.axes, source.info.buttons, source.info.hats),
        |b| {
            (
                u8::try_from(b.axes.len()).unwrap_or(0),
                u8::try_from(b.buttons.len()).unwrap_or(0),
                u8::try_from(b.hats.len()).unwrap_or(0),
            )
        },
    );
    let mut rows = vec![];
    for (kind, count) in [
        (RowKind::Axis, counts.0),
        (RowKind::Button, counts.1),
        (RowKind::Hat, counts.2),
    ] {
        for index in 0..count {
            let input = match kind {
                RowKind::Axis => InputId::Axis { index },
                RowKind::Button => InputId::Button { index },
                RowKind::Hat => InputId::Hat { index },
            };
            if binding.is_some_and(|b| !b.resolves(&input)) {
                continue;
            }
            rows.push(RowState {
                kind,
                source_index: index,
                input: InputAddress::Bound {
                    device: source.info.id.clone(),
                    input,
                },
                target: auto_target(kind, index, &target),
                replace: false,
            });
        }
    }
    rows
}
pub(super) fn valid(state: &AppState, view: &View) -> bool {
    let baseline = derive(state, view);
    let target = target(view, state);
    target.is_some()
        && view.rows.len() == baseline.len()
        && view.rows.iter().zip(&baseline).all(|(row, b)| {
            row.kind == b.kind
                && row.input == b.input
                && row.source_index == b.source_index
                && row
                    .target
                    .as_ref()
                    .is_none_or(|a| target.as_ref().is_some_and(|t| available(row.kind, a, t)))
        })
}
pub(super) fn reconcile(state: &AppState, view: &mut View) {
    view.source = view
        .source
        .take()
        .filter(|id| {
            state
                .devices
                .iter()
                .any(|d| d.connected && &d.info.id == id)
        })
        .or_else(|| {
            state
                .devices
                .iter()
                .find(|d| d.connected)
                .map(|d| d.info.id.clone())
        });
    view.target = view
        .target
        .filter(|id| state.virtual_devices.iter().any(|t| t.device_id == *id))
        .or_else(|| state.virtual_devices.first().map(|t| t.device_id));
    if let Some(profile) = &state.active_profile
        && !profile.modes().contains(&view.mode)
    {
        profile.settings().startup_mode().clone_into(&mut view.mode);
    }
    if !valid(state, view) {
        let mut baseline = derive(state, view);
        let target = target(view, state);
        for row in &mut baseline {
            if let Some(old) = view.rows.iter().find(|old| {
                old.input == row.input
                    && old.kind == row.kind
                    && old.source_index == row.source_index
            }) && old
                .target
                .as_ref()
                .is_none_or(|a| target.as_ref().is_some_and(|t| available(row.kind, a, t)))
            {
                *row = old.clone();
            }
        }
        view.rows = baseline;
    }
}
