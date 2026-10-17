//! Action-path evaluation and output ownership.
use super::walk::walk;
use inputforge_core::{
    action::{Action, OutputBehavior, branch_actions},
    pipeline::{
        ActionPathSegment, BranchStep, InputCache, OutputDestination as ActivityDestination,
        OutputOwner, button_pressed_from_value, evaluate_actions_through_path,
    },
    state::AppState,
    types::{AxisPolarity, InputAddress, MergeOp},
};
use inputforge_gui_dx::{
    readouts::{ChainStep, LiveReadoutModel},
    snapshots::ConfigSnapshot,
};
use std::collections::HashSet;
pub(super) fn analyze(
    actions: &[Action],
    primary: &InputAddress,
    state: &AppState,
    cfg: &ConfigSnapshot,
) -> LiveReadoutModel {
    let mut model = LiveReadoutModel {
        pipeline_inputs: vec![primary.clone()],
        predicates: Vec::new(),
        outputs: Vec::new(),
    };
    let context = AnalysisContext {
        top_level: actions,
        primary,
        state,
        cfg,
        profile_id: active_profile_id(state),
        mode: cfg
            .selected_mapping_key
            .as_ref()
            .map_or_else(|| state.current_mode.clone(), |(mode, _)| mode.clone()),
    };
    let mut chain_stack = Vec::new();
    let mut branch_path = Vec::new();
    let mut action_path = Vec::new();
    let mut predicate_keys = HashSet::new();
    walk(
        &context,
        actions,
        &mut model,
        &mut chain_stack,
        &mut branch_path,
        &mut action_path,
        &mut predicate_keys,
        0,
    );
    model
}

fn active_profile_id(state: &AppState) -> String {
    state.profile_path.as_ref().map_or_else(
        || "memory-profile".to_owned(),
        |path| path.display().to_string(),
    )
}

pub(super) struct AnalysisContext<'a> {
    pub(super) top_level: &'a [Action],
    pub(super) primary: &'a InputAddress,
    pub(super) state: &'a AppState,
    pub(super) cfg: &'a ConfigSnapshot,
    pub(super) profile_id: String,
    pub(super) mode: String,
}

impl AnalysisContext<'_> {
    pub(super) fn compute_merge_step_data(
        &self,
        branch_path: &[BranchStep],
        chain_stack: &[ChainStep],
        local_idx: usize,
        second_input: &InputAddress,
        operation: MergeOp,
    ) -> (f64, AxisPolarity) {
        if self.primary.device().is_none() {
            return (0.0, AxisPolarity::Bipolar);
        }
        let actions = self.flatten_actions_to_merge(branch_path, local_idx);
        let input_value =
            evaluate_actions_through_path(&actions, self.state, self.primary, &[], actions.len());
        let display_value = super::polarity::axis_f64(&input_value);
        let primary_polarity = chain_stack
            .iter()
            .rev()
            .find_map(|step| match step {
                ChainStep::Merge {
                    polarity_at_step, ..
                } => Some(*polarity_at_step),
                ChainStep::Conditional { .. } => None,
            })
            .unwrap_or_else(|| self.state.input_cache.get_axis(self.primary).1);
        let secondary_polarity = self.state.input_cache.get_axis(second_input).1;
        let polarity_at_step =
            super::polarity::merge_output_polarity(operation, primary_polarity, secondary_polarity);

        (
            inputforge_core::processing::into_natural_domain(display_value, polarity_at_step),
            polarity_at_step,
        )
    }

    fn flatten_actions_to_merge(
        &self,
        branch_path: &[BranchStep],
        local_idx: usize,
    ) -> Vec<Action> {
        let mut flattened = Vec::new();
        let mut current = self.top_level;

        for step in branch_path {
            let BranchStep::Branch { index, branch } = *step;
            append_non_branch_actions(&current[..index], &mut flattened);
            let action = &current[index];
            if let Some(actions) = branch_actions(action, branch) {
                current = actions;
            } else {
                panic!(
                    "branch path target at index {index} must support {branch:?}, got {action:?}"
                );
            }
        }

        append_non_branch_actions(&current[..=local_idx], &mut flattened);
        flattened
    }

    pub(super) fn keyboard_pressed(&self, branch_path: &[BranchStep], local_idx: usize) -> bool {
        if self.primary.device().is_none() {
            return false;
        }
        let projected = evaluate_actions_through_path(
            self.top_level,
            self.state,
            self.primary,
            branch_path,
            local_idx,
        );
        button_pressed_from_value(super::polarity::axis_f64(&projected))
    }

    pub(super) fn output_owner(
        &self,
        action_path: &[ActionPathSegment],
        destination: ActivityDestination,
        behavior: OutputBehavior,
    ) -> OutputOwner {
        OutputOwner {
            profile: self.profile_id.clone(),
            mode: self.mode.clone(),
            input: self.primary.clone(),
            action_path: action_path.to_vec(),
            destination,
            behavior,
        }
    }
}

fn append_non_branch_actions(actions: &[Action], out: &mut Vec<Action>) {
    out.extend(
        actions
            .iter()
            .filter(|action| {
                !matches!(
                    action,
                    Action::Conditional { .. }
                        | Action::TapGesture { .. }
                        | Action::PressGesture { .. }
                )
            })
            .cloned(),
    );
}
