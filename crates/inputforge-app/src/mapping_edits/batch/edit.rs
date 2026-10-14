//! Wizard transitions and raw form-value validation.
use super::super::history::History;
use super::{Stamp, Wizard, policy, rows};
use inputforge_core::{
    action::BulkMapEntry,
    state::AppState,
    types::{OutputAddress, OutputId, VJoyAxis},
};
use inputforge_gui_dx::batch_mapping::{Intent, RowKind};
pub(super) fn apply(
    state: &AppState,
    history: &History,
    batch: &mut Wizard,
    intent: Intent,
) -> Option<(Vec<BulkMapEntry>, String)> {
    batch.view.error = None;
    // Refresh collisions and mode choices under the same validation guard as the action.
    policy::project(state, history, &mut batch.view);
    let view = &mut batch.view;
    match intent {
        Intent::Source(value) => {
            if let Some(source) = state
                .devices
                .iter()
                .find(|d| d.connected && d.info.id.0 == value)
            {
                view.source = Some(source.info.id.clone());
                view.rows = rows::derive(state, view);
            } else {
                view.error = Some("Source device is no longer available.".into());
            }
        }
        Intent::Target(value) => {
            if let Some(target) = value
                .parse::<u8>()
                .ok()
                .and_then(|id| state.virtual_devices.iter().find(|t| t.device_id == id))
            {
                view.target = Some(target.device_id);
                view.rows = rows::derive(state, view);
            } else {
                view.error = Some("Target device is no longer available.".into());
            }
        }
        Intent::Mode(mode) => {
            if view.modes.contains(&mode) {
                view.mode = mode;
            } else {
                view.error = Some("Mode is no longer available.".into());
            }
        }
        Intent::AllModes(all) => view.all_modes = all,
        Intent::RowTarget { kind, index, value } => {
            let parsed = parse(kind, &value, view.target).filter(|a| {
                rows::target(view, state)
                    .as_ref()
                    .is_some_and(|t| rows::available(kind, a, t))
            });
            if value != "(do not map)" && parsed.is_none() {
                view.error = Some("Target input is not available for this row.".into());
            } else if let Some(row) = view
                .rows
                .iter_mut()
                .find(|r| r.kind == kind && r.source_index == index)
            {
                row.target = parsed;
            } else {
                view.error = Some("Source input is no longer available.".into());
            }
        }
        Intent::Replace(kind, index) => {
            if let Some(row) = view
                .rows
                .iter_mut()
                .find(|r| r.kind == kind && r.source_index == index)
            {
                row.replace = !row.replace;
            } else {
                view.error = Some("Source input is no longer available.".into());
            }
        }
        Intent::ReplaceConflicts { kind, replace } => {
            for (row, conflict) in view.rows.iter_mut().zip(&view.conflicting) {
                if row.kind == kind && *conflict {
                    row.replace = replace;
                }
            }
        }
        Intent::Include(kind) => {
            if let Some(target) = rows::target(view, state) {
                for row in &mut view.rows {
                    if row.kind == kind && row.target.is_none() {
                        row.target = rows::auto_target(kind, row.source_index, &target);
                    }
                }
            }
        }
        Intent::Exclude(kind) => {
            for row in &mut view.rows {
                if row.kind == kind {
                    row.target = None;
                }
            }
        }
        Intent::Reset => view.rows = rows::derive(state, view),
        Intent::Apply => return super::dispatch::prepare(state, history, view),
    }
    let stamp = Stamp::new(state, view, batch.publication.mode.peek().clone());
    if batch.stamp != stamp {
        view.generation += 1;
        batch.stamp = stamp;
    }
    None
}
use dioxus::prelude::ReadableExt;
fn parse(kind: RowKind, value: &str, device: Option<u8>) -> Option<OutputAddress> {
    let (prefix, rest) = value.split_once(':')?;
    let output = match (kind, prefix) {
        (RowKind::Axis, "axis") => OutputId::Axis {
            id: match rest {
                "X" => VJoyAxis::X,
                "Y" => VJoyAxis::Y,
                "Z" => VJoyAxis::Z,
                "Rx" => VJoyAxis::Rx,
                "Ry" => VJoyAxis::Ry,
                "Rz" => VJoyAxis::Rz,
                "Slider0" => VJoyAxis::Slider0,
                "Slider1" => VJoyAxis::Slider1,
                _ => return None,
            },
        },
        (RowKind::Button, "button") => OutputId::Button {
            id: rest.parse().ok()?,
        },
        (RowKind::Hat, "hat") => OutputId::Hat {
            id: rest.parse().ok()?,
        },
        _ => return None,
    };
    Some(OutputAddress {
        device: device?,
        output,
    })
}
