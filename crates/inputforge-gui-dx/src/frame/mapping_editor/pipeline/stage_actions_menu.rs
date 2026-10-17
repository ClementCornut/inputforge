//! Stage menu wiring for app-owned remove and relative-move intents.
use super::stage::stage_title_for;
use crate::{
    components::{AnchoredMenu, CloseReason, MenuAnchor, MenuItem},
    frame::{
        MappingKey,
        mapping_editor::{
            EditorState,
            undo_log::{LabelArgs, UndoKind, format_undo_label},
        },
    },
    mapping_edits::{Bindings, Edit},
};
use dioxus::prelude::*;
use inputforge_core::action::Action;

#[component]
pub(crate) fn StageActionsMenu(mapping_key: MappingKey, root_actions: Vec<Action>) -> Element {
    let mut editor = use_context::<EditorState>();
    crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::use_origin();
    let bindings = use_context::<Bindings>();
    let activation = use_signal(|| false);
    let menu = editor.stage_menu.read().clone();
    let Some(menu) = menu else {
        return rsx! {};
    };
    let Some(prepared) = bindings
        .stage_menu
        .call((root_actions.clone(), menu.stage.clone()))
    else {
        return rsx! {};
    };
    let stage_name = super::at_path(&root_actions, &menu.stage).map_or("stage", stage_title_for);
    let up = prepared.up;
    let down = prepared.down;
    let index = prepared.index;
    let activate = move |direction: Option<bool>| {
        let key = mapping_key.clone();
        let root = root_actions.clone();
        let path = menu.stage.clone();
        let mut activation = activation;
        move |_: MouseEvent| {
            let (edit, kind, destination, field_index) = match direction {
                Some(down) => (
                    Edit::Step {
                        path: path.clone(),
                        down,
                    },
                    UndoKind::StageReorder,
                    Some((
                        index,
                        if down {
                            index + 1
                        } else {
                            index.saturating_sub(1)
                        },
                    )),
                    None,
                ),
                None => (
                    Edit::Remove(path.clone()),
                    UndoKind::StageRemove,
                    None,
                    Some(index),
                ),
            };
            let label = format_undo_label(
                kind,
                LabelArgs {
                    stage_name: Some(stage_name),
                    from_to: destination,
                    index: field_index,
                    ..LabelArgs::default()
                },
            );
            let delivered = super::stage_body::instruments::stage_dispatch::submit(
                &key, &root, edit, label, None, true,
            );
            activation.set(delivered);
        }
    };
    rsx! {
        AnchoredMenu {
            open: Some(MenuAnchor { x: menu.x, y: menu.y }),
            class: "if-stage-menu".to_owned(),
            on_close: move |reason: CloseReason| {
                if reason != CloseReason::ItemActivated || *activation.peek() { editor.stage_menu.set(None); }
            },
            MenuItem { disabled: !up, onclick: activate(Some(false)), "Move up" }
            MenuItem { disabled: !down, onclick: activate(Some(true)), "Move down" }
            MenuItem { class: "if-stage-menu__item--danger".to_owned(), onclick: activate(None), "Delete" }
            if let Some(message) = editor.error.read().as_ref() { span { role: "alert", "{message}" } }
        }
    }
}
