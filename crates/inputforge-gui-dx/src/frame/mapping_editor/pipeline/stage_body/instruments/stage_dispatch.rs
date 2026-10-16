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
    let mapping = cfg
        .mappings
        .iter()
        .find(|m| m.input == key.1 && m.mode == key.0);
    let before = Mapping {
        input: key.1.clone(),
        mode: key.0.clone(),
        name: mapping.and_then(|m| m.name.clone()),
        actions: actions.to_vec(),
    };
    drop(cfg);
    let request = Request {
        profile: profile(),
        before,
        edit,
        label,
        group,
        editor,
    };
    let result = consume_context::<Bindings>().edit.call(request);
    if let Some(mut editor) = try_consume_context::<crate::frame::mapping_editor::EditorState>() {
        editor.error.set(result.clone().err());
    }
    result.unwrap_or(false)
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
/// Forward one raw field intent; the app owns parsing and action reconstruction.
pub(crate) fn dispatch_field(
    actions: &[Action],
    path: &StageId,
    field: crate::mapping_edits::Field,
    key: &MappingKey,
    label: String,
) -> bool {
    submit(
        key,
        actions,
        Edit::Field {
            path: path.clone(),
            field,
        },
        label,
        None,
        true,
    )
}

/// Publish the app's inline completion decision without closing on rejection.
pub(crate) fn submit_inline(
    key: &MappingKey,
    actions: &[Action],
    name: Option<String>,
    edit: Edit,
    label: String,
    cancel: bool,
) -> bool {
    let before = Mapping {
        input: key.1.clone(),
        mode: key.0.clone(),
        name,
        actions: actions.to_vec(),
    };
    let result = consume_context::<Bindings>()
        .inline
        .call(crate::mapping_edits::Inline {
            target: Some(crate::mapping_rail::Target {
                scope: crate::mapping_rail::Scope {
                    profile: profile(),
                    mode: key.0.clone(),
                },
                before,
            }),
            edit,
            label,
            editor: true,
            cancel,
        });
    let mut editor = consume_context::<crate::frame::mapping_editor::EditorState>();
    editor.error.set(result.clone().err());
    result.is_ok()
}

#[derive(Clone)]
struct Origin(Option<inputforge_core::profile::ProfileId>);
/// Capture the mounted editor's profile before delayed event delivery.
pub(crate) fn use_origin() {
    let ctx = use_context::<crate::context::AppContext>();
    use_context_provider(|| Origin(ctx.meta.peek().profile_id.clone()));
}
pub(crate) fn profile() -> Option<inputforge_core::profile::ProfileId> {
    consume_context::<Origin>().0
}
