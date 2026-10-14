//! Current collisions, counts and control availability for rendering.
use super::{super::history::History, rows};
use inputforge_core::{
    state::AppState,
    types::{OutputAddress, OutputId},
};
use inputforge_gui_dx::batch_mapping::{Control, Counts, Group, RowKind, View};
pub(super) fn modes(view: &View) -> Vec<String> {
    if view.all_modes {
        view.modes.clone()
    } else {
        vec![view.mode.clone()]
    }
}
pub(super) fn collides(
    state: &AppState,
    history: &History,
    view: &View,
    input: &inputforge_core::types::InputAddress,
) -> bool {
    modes(view).iter().any(|m| {
        history
            .current(state, &(m.clone(), input.clone()))
            .is_some()
    })
}
pub(super) fn project(state: &AppState, history: &History, view: &mut View) {
    view.profile = state.active_profile.as_ref().map(|p| p.id().clone());
    view.sources = state
        .devices
        .iter()
        .filter(|d| d.connected)
        .map(|d| {
            (
                d.info.id.clone(),
                inputforge_core::settings::display_name_for_device(&state.device_aliases, &d.info),
            )
        })
        .collect();
    view.targets.clone_from(&state.virtual_devices);
    view.modes = state
        .active_profile
        .as_ref()
        .map_or_else(Vec::new, |p| p.modes().as_slice().to_vec());
    let modes = modes(view);
    view.counts = Counts::default();
    view.conflicting = view
        .rows
        .iter()
        .map(|r| collides(state, history, view, &r.input))
        .collect();
    for row in &view.rows {
        if row.target.is_none() {
            view.counts.excluded += 1;
            continue;
        }
        for mode in &modes {
            match (
                history
                    .current(state, &(mode.clone(), row.input.clone()))
                    .is_some(),
                row.replace,
            ) {
                (false, _) => view.counts.create += 1,
                (true, true) => view.counts.replace += 1,
                (true, false) => view.counts.skip += 1,
            }
        }
    }
    let target = rows::target(view, state);
    view.groups = [RowKind::Axis, RowKind::Button, RowKind::Hat]
        .into_iter()
        .map(|kind| group(view, target.as_ref(), kind))
        .collect();
    view.dirty = view.rows != rows::derive(state, view);
    view.can_apply = !state.session.offline
        && view.profile.is_some()
        && rows::valid(state, view)
        && view.counts.create + view.counts.replace > 0;
}

fn group(
    view: &View,
    target: Option<&inputforge_core::types::VirtualDeviceConfig>,
    kind: RowKind,
) -> Group {
    let targets = target.map_or_else(Vec::new, |t| match kind {
        RowKind::Axis => t
            .axes
            .iter()
            .map(|id| OutputAddress {
                device: t.device_id,
                output: OutputId::Axis { id: *id },
            })
            .collect(),
        RowKind::Button => (1..=t.button_count)
            .map(|id| OutputAddress {
                device: t.device_id,
                output: OutputId::Button { id },
            })
            .collect(),
        RowKind::Hat => (1..=t.hat_count)
            .map(|id| OutputAddress {
                device: t.device_id,
                output: OutputId::Hat { id },
            })
            .collect(),
    });
    let group: Vec<_> = view
        .rows
        .iter()
        .zip(&view.conflicting)
        .filter(|(r, _)| r.kind == kind)
        .collect();
    let controls = [
        (Control::Skip, group.iter().any(|(r, c)| r.replace && **c)),
        (
            Control::Replace,
            group
                .iter()
                .any(|(r, c)| !r.replace && **c && r.target.is_some()),
        ),
        (
            Control::Include,
            group.iter().any(|(r, _)| r.target.is_none()),
        ),
        (
            Control::Exclude,
            group.iter().any(|(r, _)| r.target.is_some()),
        ),
    ]
    .into_iter()
    .filter_map(|(control, enabled)| enabled.then_some(control))
    .collect();
    Group {
        kind,
        targets,
        controls,
    }
}
