//! Typed terminal values and exact output-owner attribution.
use super::{
    analyze::AnalysisContext,
    polarity::{compute_is_active, terminal_polarity},
};
use inputforge_core::{
    action::{Action, OutputBehavior},
    pipeline::{ActionPathSegment, BranchStep, OutputDestination as ActivityDestination},
    types::AxisPolarity,
};
use inputforge_gui_dx::readouts::{
    ChainStep, LiveReadoutModel, OutputDescriptor, OutputDestination,
};
pub(super) fn push(
    context: &AnalysisContext<'_>,
    action: &Action,
    model: &mut LiveReadoutModel,
    chain_stack: &[ChainStep],
    branch_path: &[BranchStep],
    action_path: &[ActionPathSegment],
    i: usize,
) {
    match action {
        Action::MapToVJoy { output } => {
            let is_active = compute_is_active(chain_stack);
            let polarity = terminal_polarity(chain_stack, context.primary, context.state);
            model.outputs.push(OutputDescriptor {
                owner: context.output_owner(
                    action_path,
                    ActivityDestination::VJoy(output.clone()),
                    OutputBehavior::Hold,
                ),
                destination: OutputDestination::VJoy(output.clone()),
                chain: chain_stack.to_vec(),
                is_active,
                polarity,
                activity: None,
            });
        }
        Action::MapToKeyboard { key, behavior } => {
            let pressed = context.keyboard_pressed(branch_path, i);
            model.outputs.push(OutputDescriptor {
                owner: context.output_owner(
                    action_path,
                    ActivityDestination::Keyboard(key.clone()),
                    *behavior,
                ),
                destination: OutputDestination::Keyboard {
                    key: key.clone(),
                    behavior: *behavior,
                    pressed,
                },
                chain: chain_stack.to_vec(),
                is_active: compute_is_active(chain_stack),
                polarity: AxisPolarity::Bipolar,
                activity: None,
            });
        }
        Action::MapToMouse { target, behavior } => {
            let behavior = if target.is_wheel() {
                OutputBehavior::Pulse
            } else {
                *behavior
            };
            let active = context.keyboard_pressed(branch_path, i);
            model.outputs.push(OutputDescriptor {
                owner: context.output_owner(
                    action_path,
                    ActivityDestination::Mouse(*target),
                    behavior,
                ),
                destination: OutputDestination::Mouse {
                    target: *target,
                    behavior,
                    active,
                },
                chain: chain_stack.to_vec(),
                is_active: compute_is_active(chain_stack),
                polarity: AxisPolarity::Bipolar,
                activity: None,
            });
        }
        _ => unreachable!("terminal walker passes only output actions"),
    }
}
