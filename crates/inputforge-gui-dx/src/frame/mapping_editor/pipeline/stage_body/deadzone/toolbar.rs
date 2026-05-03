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

    // Annotate the bindings as `ReadSignal<f64>` so the `.into()` in each
    // `value:` slot below disambiguates: Dioxus 0.7 has multiple `SuperInto`
    // impls for `Signal<f64>` (one via `dioxus_core`, one via `dioxus_stores`)
    // and the `value: ReadSignal<f64>` prop type alone is not enough to
    // resolve the conversion at the call site.
    let low_signal: ReadSignal<f64> = use_signal(|| config.low()).into();
    let cl_signal: ReadSignal<f64> = use_signal(|| config.center_low()).into();
    let ch_signal: ReadSignal<f64> = use_signal(|| config.center_high()).into();
    let high_signal: ReadSignal<f64> = use_signal(|| config.high()).into();

    // Lots of clones for the 5 closures (4 commits + 1 reset).
    let stage_id_low = stage_id.clone();
    let stage_id_cl = stage_id.clone();
    let stage_id_ch = stage_id.clone();
    let stage_id_high = stage_id.clone();
    let stage_id_reset = stage_id.clone();
    let mapping_key_low = mapping_key.clone();
    let mapping_key_cl = mapping_key.clone();
    let mapping_key_ch = mapping_key.clone();
    let mapping_key_high = mapping_key.clone();
    let mapping_key_reset = mapping_key.clone();
    let cmd_tx_low = cmd_tx.clone();
    let cmd_tx_cl = cmd_tx.clone();
    let cmd_tx_ch = cmd_tx.clone();
    let cmd_tx_high = cmd_tx.clone();
    let cmd_tx_reset = cmd_tx.clone();
    let cfg_low = config.clone();
    let cfg_cl = config.clone();
    let cfg_ch = config.clone();
    let cfg_high = config.clone();
    let cfg_current = config.clone();
    let actions_low = root_actions.clone();
    let actions_cl = root_actions.clone();
    let actions_ch = root_actions.clone();
    let actions_high = root_actions.clone();
    let actions_reset = root_actions.clone();

    let on_low_commit = move |v: f64| {
        commit_field(
            &cfg_low,
            FieldId::Low,
            v,
            &actions_low,
            &stage_id_low,
            &mapping_key_low,
            &cmd_tx_low,
            &mut undo_log,
            &mut malformed_hints,
            config_signal,
        );
    };
    let on_cl_commit = move |v: f64| {
        commit_field(
            &cfg_cl,
            FieldId::CL,
            v,
            &actions_cl,
            &stage_id_cl,
            &mapping_key_cl,
            &cmd_tx_cl,
            &mut undo_log,
            &mut malformed_hints,
            config_signal,
        );
    };
    let on_ch_commit = move |v: f64| {
        commit_field(
            &cfg_ch,
            FieldId::CH,
            v,
            &actions_ch,
            &stage_id_ch,
            &mapping_key_ch,
            &cmd_tx_ch,
            &mut undo_log,
            &mut malformed_hints,
            config_signal,
        );
    };
    let on_high_commit = move |v: f64| {
        commit_field(
            &cfg_high,
            FieldId::High,
            v,
            &actions_high,
            &stage_id_high,
            &mapping_key_high,
            &cmd_tx_high,
            &mut undo_log,
            &mut malformed_hints,
            config_signal,
        );
    };
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
                    value: low_signal,
                    min: -1.0,
                    // The 0.001 epsilon is a UX nudge: gives the spinner a
                    // buffer above the validator's strict `<` boundary so a
                    // single up-arrow press does not immediately collide with
                    // center_low. NOT a correctness gate.
                    max: config.center_low() - 0.001,
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_low_commit,
                }
            }
            Field { label: "CL".to_owned(), for_id: Some("dz-cl".to_owned()), error: err.clone(),
                NumberInput {
                    id: Some("dz-cl".to_owned()),
                    value: cl_signal,
                    min: config.low() + 0.001,
                    max: config.center_high(),
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_cl_commit,
                }
            }
            Field { label: "CH".to_owned(), for_id: Some("dz-ch".to_owned()), error: err.clone(),
                NumberInput {
                    id: Some("dz-ch".to_owned()),
                    value: ch_signal,
                    min: config.center_low(),
                    max: config.high() - 0.001,
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_ch_commit,
                }
            }
            Field { label: "High".to_owned(), for_id: Some("dz-high".to_owned()), error: err.clone(),
                NumberInput {
                    id: Some("dz-high".to_owned()),
                    value: high_signal,
                    min: config.center_high() + 0.001,
                    max: 1.0,
                    step: 0.01,
                    precision: Some(2),
                    oncommit: on_high_commit,
                }
            }
            div { class: "if-deadzone__toolbar-spacer" }
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
