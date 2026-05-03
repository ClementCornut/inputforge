// Rust guideline compliant 2026-05-03

//! Numeric toolbar above the plot. Four `NumberInput` rows wrapped in
//! `Field` form-rows (for label and inline error), plus a Reset button.

use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::engine::EngineCommand;
use inputforge_core::processing::deadzone::DeadzoneConfig;

use crate::components::{Button, ButtonSize, ButtonVariant, Field, NumberInput};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_body::deadzone::mutation::default_config;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::dispatch_stage_edit;
use crate::frame::mapping_editor::undo_log::StageId;

#[component]
pub(crate) fn Toolbar(
    config: DeadzoneConfig,
    stage_id: StageId,
    root_actions: Vec<Action>,
    mapping_key: MappingKey,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let mut undo_log = editor.undo_log;
    let mut malformed_hints = editor.malformed_hints;
    let cmd_tx = ctx.commands.clone();
    let config_signal = ctx.config;

    // Build a single `EventHandler<f64>` per field (4 fields), wired to BOTH
    // `oncommit` (text edit + Enter/blur) and `onstep` (+/- button). Without
    // the onstep wiring the spinner buttons would be silent. `EventHandler<T>`
    // is `Copy`, so the same handler can be passed to both prop slots.
    let on_low = make_field_handler(
        FieldId::Low,
        config.clone(),
        root_actions.clone(),
        stage_id.clone(),
        mapping_key.clone(),
        cmd_tx.clone(),
        undo_log,
        malformed_hints,
        config_signal,
    );
    let on_cl = make_field_handler(
        FieldId::CL,
        config.clone(),
        root_actions.clone(),
        stage_id.clone(),
        mapping_key.clone(),
        cmd_tx.clone(),
        undo_log,
        malformed_hints,
        config_signal,
    );
    let on_ch = make_field_handler(
        FieldId::CH,
        config.clone(),
        root_actions.clone(),
        stage_id.clone(),
        mapping_key.clone(),
        cmd_tx.clone(),
        undo_log,
        malformed_hints,
        config_signal,
    );
    let on_high = make_field_handler(
        FieldId::High,
        config.clone(),
        root_actions.clone(),
        stage_id.clone(),
        mapping_key.clone(),
        cmd_tx.clone(),
        undo_log,
        malformed_hints,
        config_signal,
    );

    let cfg_current = config.clone();
    let stage_id_reset = stage_id.clone();
    let mapping_key_reset = mapping_key.clone();
    let cmd_tx_reset = cmd_tx.clone();
    let actions_reset = root_actions.clone();
    let on_reset = move |_| {
        let default = default_config();
        // Exact float equality is intentional: a config that's been moved and
        // returned to literally the same f64 values is still a no-op reset.
        if cfg_current == default {
            return;
        }
        let cfg_snap = config_signal.read();
        let name = cfg_snap.mapping_names.get(&mapping_key_reset.1).cloned();
        drop(cfg_snap);
        dispatch_stage_edit(
            &actions_reset,
            &stage_id_reset,
            Action::Deadzone { config: default },
            &mapping_key_reset,
            name,
            &cmd_tx_reset,
            &mut undo_log,
            "deadzone: reset".to_owned(),
        );
        malformed_hints.write().remove(&stage_id_reset);
    };

    let err = malformed_hints.read().get(&stage_id).cloned();

    rsx! {
        div { class: "if-deadzone__toolbar",
            Field { label: "Low".to_owned(), for_id: Some("dz-low".to_owned()), error: err.clone(),
                NumberInput {
                    id: Some("dz-low".to_owned()),
                    // Pass the live `f64` directly: Dioxus 0.7 wraps it into a
                    // ReadSignal that re-syncs on each Toolbar re-render. This
                    // is what keeps the field text following the deadzone state
                    // when an outer edit (drag, keyboard nudge, Reset) updates
                    // the config through `config_signal`.
                    value: config.low(),
                    min: -1.0,
                    // The 0.001 epsilon is a UX nudge: gives the spinner a
                    // buffer above the validator's strict `<` boundary so a
                    // single up-arrow press does not immediately collide with
                    // center_low. NOT a correctness gate.
                    max: config.center_low() - 0.001,
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_low,
                    onstep: on_low,
                }
            }
            Field { label: "CL".to_owned(), for_id: Some("dz-cl".to_owned()), error: err.clone(),
                NumberInput {
                    id: Some("dz-cl".to_owned()),
                    value: config.center_low(),
                    min: config.low() + 0.001,
                    max: config.center_high(),
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_cl,
                    onstep: on_cl,
                }
            }
            Field { label: "CH".to_owned(), for_id: Some("dz-ch".to_owned()), error: err.clone(),
                NumberInput {
                    id: Some("dz-ch".to_owned()),
                    value: config.center_high(),
                    min: config.center_low(),
                    max: config.high() - 0.001,
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_ch,
                    onstep: on_ch,
                }
            }
            Field { label: "High".to_owned(), for_id: Some("dz-high".to_owned()), error: err.clone(),
                NumberInput {
                    id: Some("dz-high".to_owned()),
                    value: config.high(),
                    min: config.center_high() + 0.001,
                    max: 1.0,
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_high,
                    onstep: on_high,
                }
            }
            Button {
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                onclick: on_reset,
                "Reset"
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FieldId {
    Low,
    CL,
    CH,
    High,
}

/// Build a single `EventHandler<f64>` that calls `commit_field` for one named
/// threshold. Invoked once per field at component render. The returned handler
/// is `Copy` so the rsx site wires it to BOTH `oncommit` (text edit) and
/// `onstep` (+/- buttons) without further cloning.
#[expect(
    clippy::too_many_arguments,
    reason = "matches commit_field's argument set; one binding site per field"
)]
fn make_field_handler(
    field: FieldId,
    config: DeadzoneConfig,
    actions: Vec<Action>,
    stage_id: StageId,
    mapping_key: MappingKey,
    cmd_tx: Sender<EngineCommand>,
    mut undo_log: Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
    mut malformed_hints: Signal<std::collections::HashMap<StageId, String>>,
    config_signal: Signal<crate::context::ConfigSnapshot>,
) -> EventHandler<f64> {
    EventHandler::new(move |v: f64| {
        commit_field(
            &config,
            field,
            v,
            &actions,
            &stage_id,
            &mapping_key,
            &cmd_tx,
            &mut undo_log,
            &mut malformed_hints,
            config_signal,
        );
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "matches dispatch_stage_edit signature plus malformed_hints"
)]
fn commit_field(
    config: &DeadzoneConfig,
    field: FieldId,
    new_value: f64,
    actions: &[Action],
    stage_id: &StageId,
    mapping_key: &MappingKey,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<crate::frame::mapping_editor::undo_log::UndoLog>,
    malformed_hints: &mut Signal<std::collections::HashMap<StageId, String>>,
    config_signal: Signal<crate::context::ConfigSnapshot>,
) {
    let (low, cl, ch, high) = match field {
        FieldId::Low => (
            new_value,
            config.center_low(),
            config.center_high(),
            config.high(),
        ),
        FieldId::CL => (config.low(), new_value, config.center_high(), config.high()),
        FieldId::CH => (config.low(), config.center_low(), new_value, config.high()),
        FieldId::High => (
            config.low(),
            config.center_low(),
            config.center_high(),
            new_value,
        ),
    };
    let candidate = match DeadzoneConfig::new(low, cl, ch, high) {
        Ok(c) => c,
        Err(err) => {
            malformed_hints
                .write()
                .insert(stage_id.clone(), err.to_string());
            return;
        }
    };
    let cfg_snap = config_signal.read();
    let name = cfg_snap.mapping_names.get(&mapping_key.1).cloned();
    drop(cfg_snap);
    let label_field = match field {
        FieldId::Low => "low",
        FieldId::CL => "center_low",
        FieldId::CH => "center_high",
        FieldId::High => "high",
    };
    let old_value = match field {
        FieldId::Low => config.low(),
        FieldId::CL => config.center_low(),
        FieldId::CH => config.center_high(),
        FieldId::High => config.high(),
    };
    let label = format!("deadzone: {label_field} {old_value:+.2} -> {new_value:+.2}");
    dispatch_stage_edit(
        actions,
        stage_id,
        Action::Deadzone { config: candidate },
        mapping_key,
        name,
        cmd_tx,
        undo_log,
        label,
    );
    malformed_hints.write().remove(stage_id);
}
