// Rust guideline compliant 2026-05-01

//! `MapToVJoy` body: device + output pickers with malformed-hint management
//! and undo-aware dispatch.
//!
//! # Pickers
//!
//! Two stacked `Select` rows are rendered:
//!
//! 1. **Device picker** -- one option per `VirtualDeviceConfig` entry in the
//!    snapshot, labeled "vJoy device N".
//! 2. **Output picker** -- axes, buttons, and hats available on the selected
//!    device; filtered to only the outputs the device actually has.
//!
//! Changing either picker dispatches `EngineCommand::SetMapping` immediately.
//! The undo entry is pushed ONLY when the dispatch succeeds (Amendment 5).
//!
//! # Malformed hints (Amendment 1)
//!
//! On every render the component checks whether the current `output.device`
//! exists in `cfg.virtual_devices` and whether the selected output index is
//! within range. When invalid, it writes to `editor_state.malformed_hints`.
//! When valid, it clears any stale hint for this `stage_id`.
//!
//! # Name preservation (Amendment 2)
//!
//! `EngineCommand::SetMapping` requires a `name` field. On every dispatch we
//! read the current name from `cfg.mapping_names` so that user-set names are
//! never silently cleared.
//!
//! # External-edit subscription (Amendment 4)
//!
//! This body holds no local `Signal`s that mirror action fields, so the
//! `use_effect` that watches `external_edit_reset` is a documented no-op: it
//! reads the token to subscribe to the reactive graph, ensuring Dioxus
//! re-renders when the reconciliation token advances (Task 33).

use dioxus::prelude::*;

use inputforge_core::action::{Action, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis};

use crate::components::Select;
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};

/// Human-readable label for a [`VJoyAxis`] variant.
///
/// Matches the canonical labels used in the egui UI (see `vjoy.rs`).
fn axis_label(axis: VJoyAxis) -> &'static str {
    match axis {
        VJoyAxis::X => "X axis",
        VJoyAxis::Y => "Y axis",
        VJoyAxis::Z => "Z axis",
        VJoyAxis::Rx => "Rx axis",
        VJoyAxis::Ry => "Ry axis",
        VJoyAxis::Rz => "Rz axis",
        VJoyAxis::Slider0 => "Slider 0",
        VJoyAxis::Slider1 => "Slider 1",
    }
}

/// Stable string key for an [`OutputId`] used as a `Select` option value.
fn output_id_key(id: &OutputId) -> String {
    match id {
        OutputId::Axis { id: axis } => format!("axis:{}", axis_label(*axis)),
        OutputId::Button { id: n } => format!("button:{n}"),
        OutputId::Hat { id: n } => format!("hat:{n}"),
    }
}

/// Parse a `Select` option value back into an [`OutputId`]. Returns `None`
/// when the key string does not match any known pattern (should not occur
/// under normal operation).
fn parse_output_id(key: &str) -> Option<OutputId> {
    if let Some(rest) = key.strip_prefix("axis:") {
        let axis = ALL_AXES.iter().find(|&&a| axis_label(a) == rest).copied()?;
        return Some(OutputId::Axis { id: axis });
    }
    if let Some(rest) = key.strip_prefix("button:") {
        let n: u8 = rest.parse().ok()?;
        return Some(OutputId::Button { id: n });
    }
    if let Some(rest) = key.strip_prefix("hat:") {
        let n: u8 = rest.parse().ok()?;
        return Some(OutputId::Hat { id: n });
    }
    None
}

/// All vJoy axes in display order.
const ALL_AXES: [VJoyAxis; 8] = [
    VJoyAxis::X,
    VJoyAxis::Y,
    VJoyAxis::Z,
    VJoyAxis::Rx,
    VJoyAxis::Ry,
    VJoyAxis::Rz,
    VJoyAxis::Slider0,
    VJoyAxis::Slider1,
];

/// `MapToVJoy` body: device picker + axis/button/hat picker.
#[component]
pub(crate) fn MapToVJoyBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    output: OutputAddress,
    /// Full root-level action list for the mapping. Needed so that
    /// `replace_at_path` can build the new action tree on every edit.
    /// Named `root_actions` per Amendment 3 (the dispatcher uses this name).
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();

    // Amendment 4: subscribe to external_edit_reset. This body has no local
    // Signals that mirror action fields (the dropdowns are driven from the
    // `output` prop directly), so the effect is a documented no-op: it reads
    // the token purely to subscribe to the reactive graph. Dioxus will
    // re-render when Task 33's reconciliation logic advances the token.
    let reset_token = editor.external_edit_reset;
    use_effect(move || {
        let _ = *reset_token.read();
    });

    let cfg = ctx.config.read();

    // --- Amendment 1: malformed-hint write / clear ---
    // REACTIVE-LOOP CONCERN (Task 40): this write happens during the render
    // phase (not inside use_effect). Dioxus will schedule a re-render when
    // malformed_hints is dirtied, but the write value is derived solely from
    // the `output` prop and `cfg`, neither of which originate from
    // malformed_hints, so no loop forms. A read-then-compare guard would be
    // more explicit but is not required for correctness here.
    let device_cfg = cfg
        .virtual_devices
        .iter()
        .find(|v| v.device_id == output.device);
    let output_valid = device_cfg.is_some_and(|v| match &output.output {
        OutputId::Axis { id } => v.axes.contains(id),
        OutputId::Button { id } => (*id as usize) < v.button_count as usize,
        OutputId::Hat { id } => (*id as usize) < v.hat_count as usize,
    });
    let mut malformed = editor.malformed_hints;
    if output_valid {
        malformed.write().remove(&stage_id);
    } else {
        let msg = if device_cfg.is_none() {
            format!("vJoy device {} not configured", output.device)
        } else {
            "Output not available on this device".to_owned()
        };
        malformed.write().insert(stage_id.clone(), msg);
    }

    // --- Build device picker options ---
    // Each option is ("N", "vJoy device N") so the value is the device_id
    // as a decimal string, which is compact and unambiguous.
    let device_options: Vec<(String, String)> = cfg
        .virtual_devices
        .iter()
        .map(|v| {
            (
                v.device_id.to_string(),
                format!("vJoy device {}", v.device_id),
            )
        })
        .collect();

    // Current device value for the Select.
    // `Signal<String>` satisfies `ReadSignal<String>` via the Into impl used
    // by Dioxus prop conversion; we store as Signal and pass directly.
    let device_value_str: Signal<String> = use_signal(|| output.device.to_string());

    // --- Build output picker options ---
    // If the device exists in the snapshot, list its outputs; otherwise fall
    // back to showing only the current (possibly stale) output so the UI
    // does not collapse entirely while the snapshot catches up.
    let output_options: Vec<(String, String)> = if let Some(vd) = device_cfg {
        let mut opts: Vec<(String, String)> = vd
            .axes
            .iter()
            .map(|&a| {
                (
                    output_id_key(&OutputId::Axis { id: a }),
                    axis_label(a).to_owned(),
                )
            })
            .collect();
        for i in 0..vd.button_count {
            let id = OutputId::Button { id: i };
            opts.push((output_id_key(&id), format!("Button {i}")));
        }
        for i in 0..vd.hat_count {
            let id = OutputId::Hat { id: i };
            opts.push((output_id_key(&id), format!("Hat {i}")));
        }
        opts
    } else {
        // Fallback: show the current output so the picker is not empty.
        let key = output_id_key(&output.output);
        let label = match &output.output {
            OutputId::Axis { id } => axis_label(*id).to_owned(),
            OutputId::Button { id } => format!("Button {id}"),
            OutputId::Hat { id } => format!("Hat {id}"),
        };
        vec![(key, label)]
    };

    // Current output value for the Select.
    let output_value_str: Signal<String> = use_signal(|| output_id_key(&output.output));

    // --- Shared dispatch helper (captures by value via closures) ---

    // Amendment 2: look up the current name from the snapshot so we never
    // clear a user-set name by passing None.
    let current_name = cfg.mapping_names.get(&mapping_key.1).cloned();

    // Snapshot of the mapping before any edit, used for undo entries.
    let before_mapping = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: current_name.clone(),
        actions: root_actions.clone(),
    };

    // --- Device picker change handler ---
    let mapping_key_dev = mapping_key.clone();
    let stage_id_dev = stage_id.clone();
    let root_actions_dev = root_actions.clone();
    let before_dev = before_mapping.clone();
    let current_name_dev = current_name.clone();
    let cmd_tx_dev = ctx.commands.clone();
    let output_cloned_dev = output.clone();
    let mut undo_log_dev = editor.undo_log;

    let on_device_change = move |evt: FormEvent| {
        let new_device_str = evt.value();
        let Ok(new_device) = new_device_str.parse::<u8>() else {
            return;
        };
        if new_device == output_cloned_dev.device {
            return;
        }
        let new_output = OutputAddress {
            device: new_device,
            output: output_cloned_dev.output.clone(),
        };
        let new_action = Action::MapToVJoy {
            output: new_output.clone(),
        };
        let Some(new_actions) = replace_at_path(&root_actions_dev, &stage_id_dev, new_action)
        else {
            return;
        };
        // Amendment 5: dispatch first; skip push_edit if the channel is closed.
        if cmd_tx_dev
            .send(EngineCommand::SetMapping {
                input: mapping_key_dev.1.clone(),
                mode: mapping_key_dev.0.clone(),
                name: current_name_dev.clone(),
                actions: new_actions,
            })
            .is_err()
        {
            tracing::warn!(
                target: "f9::mapping_editor",
                action = "map_to_vjoy_device_drop_offline",
                "device change dropped: engine channel disconnected"
            );
            return;
        }
        let before_str = format!("vJoy device {}", output_cloned_dev.device);
        let after_str = format!("vJoy device {new_device}");
        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("Map to vJoy"),
                field: Some("device"),
                before_after: Some((&before_str, &after_str)),
                ..LabelArgs::default()
            },
        );
        undo_log_dev.write().push_edit(
            mapping_key_dev.clone(),
            before_dev.clone(),
            UndoKind::StageEdit,
            label,
        );
    };

    // --- Output picker change handler ---
    let mapping_key_out = mapping_key.clone();
    let stage_id_out = stage_id.clone();
    let root_actions_out = root_actions.clone();
    let before_out = before_mapping.clone();
    let current_name_out = current_name.clone();
    let cmd_tx_out = ctx.commands.clone();
    let output_cloned_out = output.clone();
    let mut undo_log_out = editor.undo_log;

    let on_output_change = move |evt: FormEvent| {
        let key_str = evt.value();
        let Some(new_output_id) = parse_output_id(&key_str) else {
            return;
        };
        if new_output_id == output_cloned_out.output {
            return;
        }
        let new_output = OutputAddress {
            device: output_cloned_out.device,
            output: new_output_id.clone(),
        };
        let new_action = Action::MapToVJoy {
            output: new_output.clone(),
        };
        let Some(new_actions) = replace_at_path(&root_actions_out, &stage_id_out, new_action)
        else {
            return;
        };
        // Amendment 5: dispatch first; skip push_edit if the channel is closed.
        if cmd_tx_out
            .send(EngineCommand::SetMapping {
                input: mapping_key_out.1.clone(),
                mode: mapping_key_out.0.clone(),
                name: current_name_out.clone(),
                actions: new_actions,
            })
            .is_err()
        {
            tracing::warn!(
                target: "f9::mapping_editor",
                action = "map_to_vjoy_output_drop_offline",
                "output change dropped: engine channel disconnected"
            );
            return;
        }
        let before_key = output_id_key(&output_cloned_out.output);
        let after_key = output_id_key(&new_output_id);
        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("Map to vJoy"),
                field: Some("output"),
                before_after: Some((&before_key, &after_key)),
                ..LabelArgs::default()
            },
        );
        undo_log_out.write().push_edit(
            mapping_key_out.clone(),
            before_out.clone(),
            UndoKind::StageEdit,
            label,
        );
    };

    rsx! {
        div { class: "if-stage__body-vjoy",
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Device" }
                Select {
                    value: device_value_str,
                    options: device_options,
                    onchange: on_device_change,
                }
            }
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Output" }
                Select {
                    value: output_value_str,
                    options: output_options,
                    onchange: on_output_change,
                }
            }
        }
    }
}
