//! Input dependency validation with enough context for actionable messages.
use crate::{
    action::Condition,
    profile::Profile,
    state::{AppState, InputIssueKind, InputRole, MappingIssueReason},
    types::{InputAddress, InputId},
};
type Result = std::result::Result<(), MappingIssueReason>;

pub(super) fn validate_input(
    profile: &Profile,
    state: &AppState,
    address: &InputAddress,
    role: InputRole,
) -> Result {
    let fail = |problem| Err(input_issue(address, role, problem));
    let InputAddress::Bound { device, input } = address else {
        return fail(InputIssueKind::Unbound);
    };
    let Some(config) = profile.controllers() else {
        return fail(InputIssueKind::Unselected);
    };
    if !config.selected.contains(device) {
        return fail(InputIssueKind::Unselected);
    }
    if !state.session.monitored.contains(device) {
        return fail(
            if state
                .devices
                .iter()
                .any(|d| d.info.id == *device && d.connected)
            {
                InputIssueKind::NotReady
            } else {
                InputIssueKind::Disconnected
            },
        );
    }
    let Some(binding) = config.bindings.iter().find(|b| b.device == *device) else {
        return fail(InputIssueKind::MissingControl);
    };
    if binding.unavailable.contains(input) {
        return fail(InputIssueKind::ChangedControl);
    }
    if !binding.resolves(input) {
        return fail(InputIssueKind::MissingControl);
    }
    Ok(())
}

pub(super) fn input_issue(
    address: &InputAddress,
    role: InputRole,
    problem: InputIssueKind,
) -> MappingIssueReason {
    MappingIssueReason::Input {
        address: address.clone(),
        role,
        problem,
    }
}

pub(super) fn validate_condition(
    profile: &Profile,
    state: &AppState,
    condition: &Condition,
) -> Result {
    let (input, kind_valid) = match condition {
        Condition::ButtonPressed { input } | Condition::ButtonReleased { input } => (
            input,
            matches!(input.input_id(), Some(InputId::Button { .. })),
        ),
        Condition::AxisInRange { input, min, max } => {
            validate_input(profile, state, input, InputRole::Condition)?;
            if !min.is_finite() || !max.is_finite() || min > max {
                return Err(MappingIssueReason::InvalidCondition {
                    input: input.clone(),
                });
            }
            (
                input,
                matches!(input.input_id(), Some(InputId::Axis { .. })),
            )
        }
        Condition::HatDirection { input, .. } => {
            (input, matches!(input.input_id(), Some(InputId::Hat { .. })))
        }
        Condition::All { conditions } | Condition::Any { conditions } => {
            for child in conditions {
                validate_condition(profile, state, child)?;
            }
            return Ok(());
        }
        Condition::Not { condition } => return validate_condition(profile, state, condition),
    };
    validate_input(profile, state, input, InputRole::Condition)?;
    if !kind_valid {
        return Err(input_issue(
            input,
            InputRole::Condition,
            InputIssueKind::WrongKind,
        ));
    }
    Ok(())
}
