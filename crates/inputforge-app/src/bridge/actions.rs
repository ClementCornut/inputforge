//! Existing action-tree readout preparation.
use inputforge_core::{
    action::{ActionBranch, branch_actions},
    types::{DeviceId, InputAddress, OutputAddress},
};
use inputforge_gui_dx::snapshots::GlyphFlags;
/// Walk an action tree in depth-first order, recording the first
/// `MergeAxis::second_input` and the first input-bearing `Conditional`
/// predicate. Returns early once both glyphs are populated, or after a
/// full traversal (whichever comes first).
pub(super) fn derive_glyphs(actions: &[inputforge_core::action::Action]) -> GlyphFlags {
    let mut out = GlyphFlags::default();
    walk_actions(actions, &mut out);
    out
}

pub(super) fn action_branches() -> [ActionBranch; 6] {
    [
        ActionBranch::ConditionalTrue,
        ActionBranch::ConditionalFalse,
        ActionBranch::TapSingle,
        ActionBranch::TapDouble,
        ActionBranch::PressShort,
        ActionBranch::PressLong,
    ]
}

fn walk_actions(actions: &[inputforge_core::action::Action], out: &mut GlyphFlags) {
    use inputforge_core::action::Action;
    for action in actions {
        if out.merge_secondary.is_some() && out.first_input_predicate.is_some() {
            return;
        }
        match action {
            Action::MergeAxis { second_input, .. } if out.merge_secondary.is_none() => {
                out.merge_secondary = Some(second_input.clone());
            }
            Action::Conditional { condition, .. } => {
                if out.first_input_predicate.is_none()
                    && let Some(addr) = first_input_predicate(condition)
                {
                    out.first_input_predicate = Some(addr);
                }
            }
            _ => {}
        }
        for branch in action_branches() {
            if let Some(actions) = branch_actions(action, branch) {
                walk_actions(actions, out);
            }
        }
    }
}

/// Recurse through `All | Any | Not` composites until an input-bearing
/// leaf (`ButtonPressed | ButtonReleased | AxisInRange | HatDirection`)
/// is found.
fn first_input_predicate(condition: &inputforge_core::action::Condition) -> Option<InputAddress> {
    use inputforge_core::action::Condition;
    match condition {
        Condition::ButtonPressed { input }
        | Condition::ButtonReleased { input }
        | Condition::AxisInRange { input, .. }
        | Condition::HatDirection { input, .. } => Some(input.clone()),
        Condition::All { conditions } | Condition::Any { conditions } => {
            conditions.iter().find_map(first_input_predicate)
        }
        Condition::Not { condition } => first_input_predicate(condition),
    }
}

pub(super) fn derive_referenced_devices(
    primary: &InputAddress,
    actions: &[inputforge_core::action::Action],
) -> Vec<DeviceId> {
    fn push_addr(out: &mut Vec<DeviceId>, addr: &InputAddress) {
        if let Some(device) = addr.device()
            && !out.iter().any(|existing| existing == device)
        {
            out.push(device.clone());
        }
    }

    fn walk_condition(out: &mut Vec<DeviceId>, condition: &inputforge_core::action::Condition) {
        use inputforge_core::action::Condition;
        match condition {
            Condition::ButtonPressed { input }
            | Condition::ButtonReleased { input }
            | Condition::AxisInRange { input, .. }
            | Condition::HatDirection { input, .. } => push_addr(out, input),
            Condition::All { conditions } | Condition::Any { conditions } => {
                for child in conditions {
                    walk_condition(out, child);
                }
            }
            Condition::Not { condition } => walk_condition(out, condition),
        }
    }

    fn walk_actions(out: &mut Vec<DeviceId>, actions: &[inputforge_core::action::Action]) {
        use inputforge_core::action::Action;
        for action in actions {
            match action {
                Action::MergeAxis { second_input, .. } => push_addr(out, second_input),
                Action::Conditional { condition, .. } => {
                    walk_condition(out, condition);
                }
                _ => {}
            }
            for branch in action_branches() {
                if let Some(actions) = branch_actions(action, branch) {
                    walk_actions(out, actions);
                }
            }
        }
    }

    let mut out = Vec::new();
    push_addr(&mut out, primary);
    walk_actions(&mut out, actions);
    out
}

pub(super) fn first_vjoy_output(
    actions: &[inputforge_core::action::Action],
) -> Option<OutputAddress> {
    use inputforge_core::action::Action;
    for action in actions {
        if let Action::MapToVJoy { output } = action {
            return Some(output.clone());
        }
        for branch in action_branches() {
            if let Some(actions) = branch_actions(action, branch)
                && let Some(output) = first_vjoy_output(actions)
            {
                return Some(output);
            }
        }
    }
    None
}
