// Rust guideline compliant 2026-05-11

//! `MapToMouse` body: target picker plus button behavior picker.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping, MouseTarget, OutputBehavior};
use inputforge_core::engine::EngineCommand;

use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, UndoKind, UndoLog, format_undo_label,
};

const MOUSE_TARGETS: &[MouseTarget] = &[
    MouseTarget::LeftButton,
    MouseTarget::RightButton,
    MouseTarget::MiddleButton,
    MouseTarget::BackButton,
    MouseTarget::ForwardButton,
    MouseTarget::WheelUp,
    MouseTarget::WheelDown,
];

#[component]
pub(crate) fn MapToMouseBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    target: MouseTarget,
    behavior: OutputBehavior,
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let cfg = ctx.config.read();
    let current_name = cfg.mapping_names.get(&mapping_key.1).cloned();
    let before_mapping = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: current_name.clone(),
        actions: root_actions.clone(),
    };
    drop(cfg);

    let cmd_tx = ctx.commands.clone();
    let undo_log = editor.undo_log;

    let mapping_key_hold = mapping_key.clone();
    let stage_id_hold = stage_id.clone();
    let root_actions_hold = root_actions.clone();
    let before_hold = before_mapping.clone();
    let current_name_hold = current_name.clone();
    let cmd_tx_hold = cmd_tx.clone();
    let mut undo_log_hold = undo_log;
    let on_hold = move |_| {
        dispatch_mouse_change(
            target,
            OutputBehavior::Hold,
            "behavior",
            &mapping_key_hold,
            &stage_id_hold,
            &root_actions_hold,
            &before_hold,
            current_name_hold.clone(),
            &cmd_tx_hold,
            &mut undo_log_hold,
        );
    };

    let mapping_key_pulse = mapping_key.clone();
    let stage_id_pulse = stage_id.clone();
    let root_actions_pulse = root_actions.clone();
    let before_pulse = before_mapping.clone();
    let current_name_pulse = current_name.clone();
    let cmd_tx_pulse = cmd_tx.clone();
    let mut undo_log_pulse = undo_log;
    let on_pulse = move |_| {
        dispatch_mouse_change(
            target,
            OutputBehavior::Pulse,
            "behavior",
            &mapping_key_pulse,
            &stage_id_pulse,
            &root_actions_pulse,
            &before_pulse,
            current_name_pulse.clone(),
            &cmd_tx_pulse,
            &mut undo_log_pulse,
        );
    };

    rsx! {
        div { class: "if-stage__body-mouse",
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Target" }
                div { class: "if-stage__body-segmented",
                    for candidate in MOUSE_TARGETS {
                        {
                            let candidate = *candidate;
                            let mapping_key_target = mapping_key.clone();
                            let stage_id_target = stage_id.clone();
                            let root_actions_target = root_actions.clone();
                            let before_target = before_mapping.clone();
                            let current_name_target = current_name.clone();
                            let cmd_tx_target = cmd_tx.clone();
                            let mut undo_log_target = undo_log;
                            let onclick = move |_| {
                                dispatch_mouse_change(
                                    candidate,
                                    behavior,
                                    "target",
                                    &mapping_key_target,
                                    &stage_id_target,
                                    &root_actions_target,
                                    &before_target,
                                    current_name_target.clone(),
                                    &cmd_tx_target,
                                    &mut undo_log_target,
                                );
                            };
                            rsx! {
                                button {
                                    class: if candidate == target { "if-stage__body-segment is-active" } else { "if-stage__body-segment" },
                                    onclick,
                                    "{candidate.label()}"
                                }
                            }
                        }
                    }
                }
            }
            if !target.is_wheel() {
                div { class: "if-stage__body-field",
                    label { class: "if-stage__body-label", "Behavior" }
                    div { class: "if-stage__body-segmented",
                        {
                            let onclick = on_hold;
                            rsx! {
                                button {
                                    class: if behavior == OutputBehavior::Hold { "if-stage__body-segment is-active" } else { "if-stage__body-segment" },
                                    onclick,
                                    "Hold"
                                }
                            }
                        }
                        {
                            let onclick = on_pulse;
                            rsx! {
                                button {
                                    class: if behavior == OutputBehavior::Pulse { "if-stage__body-segment is-active" } else { "if-stage__body-segment" },
                                    onclick,
                                    "Pulse"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "matches existing stage body dispatch helpers: one call site per control"
)]
fn dispatch_mouse_change(
    new_target: MouseTarget,
    new_behavior: OutputBehavior,
    field: &'static str,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    before: &Mapping,
    current_name: Option<String>,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<UndoLog>,
) {
    let effective_behavior = if new_target.is_wheel() {
        OutputBehavior::Pulse
    } else {
        new_behavior
    };
    let new_action = Action::MapToMouse {
        target: new_target,
        behavior: effective_behavior,
    };
    let Some(new_actions) = replace_at_path(root_actions, stage_id, new_action) else {
        return;
    };
    if cmd_tx
        .send(EngineCommand::SetMapping {
            input: mapping_key.1.clone(),
            mode: mapping_key.0.clone(),
            name: current_name,
            actions: new_actions,
        })
        .is_err()
    {
        tracing::warn!(
            target: "f9::mapping_editor",
            field,
            "mouse output change dropped: engine channel disconnected"
        );
        return;
    }
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Map to mouse"),
            field: Some(field),
            ..LabelArgs::default()
        },
    );
    undo_log.write().push_edit(
        mapping_key.clone(),
        before.clone(),
        UndoKind::StageEdit,
        label,
    );
}
