//! Bounded action-tree traversal with branch-local stacks.
use super::{
    analyze::AnalysisContext,
    predicates::{PredicateDedupKey, collect_condition_predicates},
};
use inputforge_core::{
    action::{Action, ActionBranch, branch_actions},
    pipeline::{ActionPathSegment, BranchStep, evaluate_condition},
};
use inputforge_gui_dx::readouts::{Branch, ChainStep, LiveReadoutModel};
use std::collections::HashSet;
// Preserve the existing readout depth cap independently of persisted validation.
pub(super) const MAX_NESTED_ACTION_DEPTH: usize = 32;
#[expect(
    clippy::too_many_lines,
    reason = "Recursive analyzer keeps branch traversal state together for readability."
)]
#[expect(
    clippy::too_many_arguments,
    reason = "Recursive analyzer carries independent traversal stacks explicitly."
)]
pub(super) fn walk(
    context: &AnalysisContext<'_>,
    actions: &[Action],
    model: &mut LiveReadoutModel,
    chain_stack: &mut Vec<ChainStep>,
    branch_path: &mut Vec<BranchStep>,
    action_path: &mut Vec<ActionPathSegment>,
    predicate_keys: &mut HashSet<PredicateDedupKey>,
    depth: usize,
) {
    let stack_baseline = chain_stack.len();
    let action_path_baseline = action_path.len();
    if depth > MAX_NESTED_ACTION_DEPTH {
        chain_stack.truncate(stack_baseline);
        action_path.truncate(action_path_baseline);
        return;
    }

    for (i, action) in actions.iter().enumerate() {
        action_path.push(ActionPathSegment::Index(i));
        match action {
            Action::MergeAxis {
                second_input,
                operation,
            } => {
                model.pipeline_inputs.push(second_input.clone());
                let (display_value, polarity_at_step) = context.compute_merge_step_data(
                    branch_path,
                    chain_stack,
                    i,
                    second_input,
                    *operation,
                );
                chain_stack.push(ChainStep::Merge {
                    operation: *operation,
                    secondary_input: second_input.clone(),
                    display_value,
                    polarity_at_step,
                });
            }
            Action::MapToVJoy { .. } | Action::MapToKeyboard { .. } | Action::MapToMouse { .. } => {
                super::terminal::push(
                    context,
                    action,
                    model,
                    chain_stack,
                    branch_path,
                    action_path,
                    i,
                );
            }
            Action::Conditional {
                condition,
                if_true,
                if_false,
            } => {
                collect_condition_predicates(context, condition, model, predicate_keys);
                let condition_label = inputforge_gui_dx::readouts::labels::format_condition_label(
                    condition,
                    context.cfg,
                );
                let evaluated = evaluate_condition(condition, &context.state.input_cache);

                branch_path.push(BranchStep::Branch {
                    index: i,
                    branch: ActionBranch::ConditionalTrue,
                });
                action_path.push(ActionPathSegment::Branch(ActionBranch::ConditionalTrue));
                chain_stack.push(ChainStep::Conditional {
                    condition_label: condition_label.clone(),
                    active: evaluated,
                    branch: Branch::IfTrue,
                });
                walk(
                    context,
                    if_true,
                    model,
                    chain_stack,
                    branch_path,
                    action_path,
                    predicate_keys,
                    depth + 1,
                );
                chain_stack.pop();
                action_path.pop();
                branch_path.pop();

                branch_path.push(BranchStep::Branch {
                    index: i,
                    branch: ActionBranch::ConditionalFalse,
                });
                action_path.push(ActionPathSegment::Branch(ActionBranch::ConditionalFalse));
                chain_stack.push(ChainStep::Conditional {
                    condition_label,
                    active: !evaluated,
                    branch: Branch::IfFalse,
                });
                walk(
                    context,
                    if_false,
                    model,
                    chain_stack,
                    branch_path,
                    action_path,
                    predicate_keys,
                    depth + 1,
                );
                chain_stack.pop();
                action_path.pop();
                branch_path.pop();
            }
            Action::TapGesture { .. } | Action::PressGesture { .. } => {
                for &branch in gesture_branches_for(action) {
                    let Some(actions) = branch_actions(action, branch) else {
                        continue;
                    };
                    branch_path.push(BranchStep::Branch { index: i, branch });
                    action_path.push(ActionPathSegment::Branch(branch));
                    chain_stack.push(ChainStep::Conditional {
                        condition_label: branch.label().to_owned(),
                        active: false,
                        branch: Branch::Gesture(branch),
                    });
                    walk(
                        context,
                        actions,
                        model,
                        chain_stack,
                        branch_path,
                        action_path,
                        predicate_keys,
                        depth + 1,
                    );
                    chain_stack.pop();
                    action_path.pop();
                    branch_path.pop();
                }
            }
            Action::ResponseCurve { .. }
            | Action::Deadzone { .. }
            | Action::Invert
            | Action::ChangeMode { .. } => {}
        }
        action_path.pop();
    }

    chain_stack.truncate(stack_baseline);
    action_path.truncate(action_path_baseline);
}

fn gesture_branches_for(action: &Action) -> &'static [ActionBranch] {
    match action {
        Action::TapGesture { .. } => &[ActionBranch::TapSingle, ActionBranch::TapDouble],
        Action::PressGesture { .. } => &[ActionBranch::PressShort, ActionBranch::PressLong],
        _ => &[],
    }
}
