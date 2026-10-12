//! Count device usage across mappings and action branches.
use super::actions::{action_branches, derive_referenced_devices};
use inputforge_core::state::AppState;
use inputforge_core::{
    action::branch_actions,
    types::{DeviceId, DeviceInfo, InputAddress, InputId},
};
use inputforge_gui_dx::snapshots::{DeviceCoverage, DeviceUsageSummary};
use std::collections::HashSet;

pub(super) fn usage_for_device(
    device_id: &DeviceId,
    info: &DeviceInfo,
    s: &AppState,
) -> DeviceUsageSummary {
    let mut axes = HashSet::new();
    let mut buttons = HashSet::new();
    let mut hats = HashSet::new();
    let mut primary_mappings = 0;
    let mut secondary_mappings = 0;
    let mut touched_modes = Vec::new();
    let mut touched_mapping_names = Vec::new();

    if let Some(profile) = &s.active_profile {
        for mapping in profile.mappings() {
            let primary = mapping.input.device().is_some_and(|id| id == device_id);
            let referenced_devices = derive_referenced_devices(&mapping.input, &mapping.actions);
            let referenced = referenced_devices
                .iter()
                .any(|referenced| referenced == device_id);

            if primary {
                primary_mappings += 1;
                record_input_kind(&mapping.input, &mut axes, &mut buttons, &mut hats);
            }
            let action_referenced = record_referenced_input_kinds(
                device_id,
                &mapping.actions,
                &mut axes,
                &mut buttons,
                &mut hats,
            );
            if action_referenced {
                secondary_mappings += 1;
            }

            if primary || referenced || action_referenced {
                push_unique(&mut touched_modes, mapping.mode.clone());
                if let Some(name) = &mapping.name {
                    push_unique(&mut touched_mapping_names, name.clone());
                }
            }
        }
    }

    DeviceUsageSummary {
        axes: DeviceCoverage {
            mapped: set_len_as_u8(&axes),
            total: info.axes,
        },
        buttons: DeviceCoverage {
            mapped: set_len_as_u8(&buttons),
            total: info.buttons,
        },
        hats: DeviceCoverage {
            mapped: set_len_as_u8(&hats),
            total: info.hats,
        },
        primary_mappings,
        secondary_mappings,
        touched_modes,
        touched_mapping_names,
    }
}

fn record_input_kind(
    address: &InputAddress,
    axes: &mut HashSet<u8>,
    buttons: &mut HashSet<u8>,
    hats: &mut HashSet<u8>,
) {
    let InputAddress::Bound { input, .. } = address else {
        return;
    };
    match input {
        InputId::Axis { index } => {
            axes.insert(*index);
        }
        InputId::Button { index } => {
            buttons.insert(*index);
        }
        InputId::Hat { index } => {
            hats.insert(*index);
        }
    }
}

fn record_referenced_input_kinds(
    device_id: &DeviceId,
    actions: &[inputforge_core::action::Action],
    axes: &mut HashSet<u8>,
    buttons: &mut HashSet<u8>,
    hats: &mut HashSet<u8>,
) -> bool {
    use inputforge_core::action::Action;
    let mut found = false;
    for action in actions {
        match action {
            Action::MergeAxis { second_input, .. }
                if second_input.device().is_some_and(|id| id == device_id) =>
            {
                record_input_kind(second_input, axes, buttons, hats);
                found = true;
            }
            Action::Conditional { condition, .. } => {
                found |= record_condition_input_kinds(device_id, condition, axes, buttons, hats);
            }
            _ => {}
        }
        for branch in action_branches() {
            if let Some(actions) = branch_actions(action, branch) {
                found |= record_referenced_input_kinds(device_id, actions, axes, buttons, hats);
            }
        }
    }
    found
}

fn record_condition_input_kinds(
    device_id: &DeviceId,
    condition: &inputforge_core::action::Condition,
    axes: &mut HashSet<u8>,
    buttons: &mut HashSet<u8>,
    hats: &mut HashSet<u8>,
) -> bool {
    use inputforge_core::action::Condition;
    match condition {
        Condition::ButtonPressed { input }
        | Condition::ButtonReleased { input }
        | Condition::AxisInRange { input, .. }
        | Condition::HatDirection { input, .. } => {
            if input.device().is_some_and(|id| id == device_id) {
                record_input_kind(input, axes, buttons, hats);
                true
            } else {
                false
            }
        }
        Condition::All { conditions } | Condition::Any { conditions } => {
            let mut found = false;
            for child in conditions {
                found |= record_condition_input_kinds(device_id, child, axes, buttons, hats);
            }
            found
        }
        Condition::Not { condition } => {
            record_condition_input_kinds(device_id, condition, axes, buttons, hats)
        }
    }
}

fn set_len_as_u8(set: &HashSet<u8>) -> u8 {
    set.len().try_into().unwrap_or(u8::MAX)
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}
