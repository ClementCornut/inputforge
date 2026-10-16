//! Mode cascades invalidate affected restores while preserving queue acknowledgement.
use super::History;
use inputforge_core::{
    action::{Action, ActionBranch, Mapping, ModeChangeStrategy, branch_actions},
    engine::EngineCommand,
    state::AppState,
};
use inputforge_gui_dx::snapshots::MappingKey;
impl History {
    pub(in crate::mapping_edits) fn mode_profile(
        &self,
        state: &AppState,
    ) -> Option<inputforge_core::profile::Profile> {
        let mut profile = state.active_profile.clone()?;
        for key in self.pending.keys() {
            if let Some(m) = self.current(state, key) {
                profile.set_mapping(&m.input, &m.mode, m.name.clone(), m.actions.clone());
            } else {
                profile.remove_mapping(&key.1, &key.0);
            }
        }
        Some(profile)
    }

    pub(in crate::mapping_edits) fn mode_changes(
        &self,
        state: &AppState,
        command: &EngineCommand,
    ) -> Vec<(MappingKey, Option<Mapping>)> {
        let Some(mut profile) = self.mode_profile(state) else {
            return vec![];
        };
        let before = profile.mappings().to_vec();
        match command {
            EngineCommand::RenameMode { from, to } => {
                profile.rename_mode_refs(from, to);
            }
            EngineCommand::DeleteMode { name } => {
                profile.remove_mappings_for_mode(name);
            }
            _ => return vec![],
        }
        let mut changes = vec![];
        for mapping in before.iter().chain(profile.mappings()) {
            let key = (mapping.mode.clone(), mapping.input.clone());
            let old = before.iter().find(|m| m.mode == key.0 && m.input == key.1);
            let next = profile.find_mapping(&key.1, &key.0);
            if old != next && !changes.iter().any(|(k, _)| k == &key) {
                changes.push((key, next.cloned()));
            }
        }
        changes
    }
    pub(in crate::mapping_edits) fn mode_forward(
        &mut self,
        state: &AppState,
        changes: Vec<(MappingKey, Option<Mapping>)>,
        affected: Option<&str>,
        serial: u64,
    ) {
        if let Some(mode) = affected {
            self.stacks.retain(|key, stack| {
                key.0 != mode
                    && !stack.undo.iter().chain(&stack.redo).any(|e| {
                        e.target.0 == mode
                            || e.before
                                .iter()
                                .chain(&e.after)
                                .any(|m| mentions(&m.actions, mode))
                    })
            });
        }
        for (key, mapping) in changes {
            self.forward(state, &key, mapping, serial);
        }
        self.burst = None;
    }
}
fn mentions(actions: &[Action], mode: &str) -> bool {
    actions.iter().any(|action| {
        let direct = matches!(action,
            Action::ChangeMode {
                strategy: ModeChangeStrategy::SwitchTo {mode:name}
                    | ModeChangeStrategy::Temporary {mode:name}
            } if name==mode
        );
        direct
            || [
                ActionBranch::ConditionalTrue,
                ActionBranch::ConditionalFalse,
                ActionBranch::TapSingle,
                ActionBranch::TapDouble,
                ActionBranch::PressShort,
                ActionBranch::PressLong,
            ]
            .into_iter()
            .any(|branch| {
                branch_actions(action, branch).is_some_and(|children| mentions(children, mode))
            })
    })
}
