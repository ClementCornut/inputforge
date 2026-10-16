//! One profile/mapping rendering scope, stable across ordinary field edits.
use super::{
    header::Header,
    inactive_hint::InactiveHint,
    live_readout::LiveReadout,
    mapping_issues::MappingIssues,
    pipeline::{Pipeline, StageActionsMenu},
    undo_recap::UndoRecap,
};
use crate::frame::MappingKey;
use dioxus::prelude::*;
use inputforge_core::action::Action;
#[component]
pub(super) fn MappingContent(
    name: String,
    mapping_key: MappingKey,
    actions: Vec<Action>,
) -> Element {
    rsx! {
        Header { name, input: mapping_key.1.clone(), mapping_key: mapping_key.clone(), actions: actions.clone() }
        MappingIssues { mapping_key: mapping_key.clone() }
        LiveReadout { primary: mapping_key.1.clone(), actions: actions.clone() }
        Pipeline { mapping_key: mapping_key.clone(), actions: actions.clone(), root_actions: actions.clone(), path_prefix: vec![], depth: 0 }
        StageActionsMenu { mapping_key: mapping_key.clone(), root_actions: actions }
        InactiveHint {}
        UndoRecap { mapping_key }
    }
}
