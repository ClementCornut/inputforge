//! Rendering callback wiring for app-owned mapping dispatch.
use crate::{
    mapping_edits::{Bindings, Edit, Group, Request, StageId},
    snapshots::MappingKey,
};
use dioxus::prelude::*;
use inputforge_core::action::{Action, Mapping};
/// Wire a prepared editor target to the app without reading mutable engine state.
pub(crate) fn submit(
    key: &MappingKey,
    actions: &[Action],
    edit: Edit,
    label: String,
    group: Option<Group>,
    editor: bool,
) -> bool {
    let ctx = consume_context::<crate::context::AppContext>();
    let cfg = ctx.config.peek();
    let Some(mapping) = cfg
        .mappings
        .iter()
        .find(|m| m.input == key.1 && m.mode == key.0)
    else {
        return false;
    };
    let before = Mapping {
        input: key.1.clone(),
        mode: key.0.clone(),
        name: mapping.name.clone(),
        actions: actions.to_vec(),
    };
    drop(cfg);
    let request = Request {
        profile: ctx.meta.peek().profile_id.clone(),
        before,
        edit,
        label,
        group,
        editor,
    };
    consume_context::<Bindings>().dispatch.call(request)
}

pub(crate) fn dispatch_stage_edit(
    actions: &[Action],
    path: &StageId,
    action: Action,
    key: &MappingKey,
    label: String,
) -> bool {
    submit(
        key,
        actions,
        Edit::Replace {
            path: path.clone(),
            action,
        },
        label,
        None,
        true,
    )
}
pub(crate) fn dispatch_stage_nudge(
    actions: &[Action],
    path: &StageId,
    action: Action,
    key: &MappingKey,
    label: String,
    group: String,
) -> bool {
    submit(
        key,
        actions,
        Edit::Replace {
            path: path.clone(),
            action,
        },
        label,
        Some(Group {
            path: path.clone(),
            key: group,
        }),
        true,
    )
}
