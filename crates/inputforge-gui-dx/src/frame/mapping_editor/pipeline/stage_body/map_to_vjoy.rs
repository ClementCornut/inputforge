// Rust guideline compliant 2026-05-01

//! `MapToVJoy` body: device + output pickers with malformed-hint management
//! and undo-aware dispatch.
//!
//! # Pickers
//!
//! Two stacked `Select` rows are rendered:
//!
//! 1. **Device picker** -- one option per `VirtualDeviceConfig` entry in the
//!    snapshot, labeled "Virtual device N".
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

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::types::{OutputAddress, OutputId, VJoyAxis};

use crate::components::{Select, SelectOption};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
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
fn available_outputs(config: &inputforge_core::types::VirtualDeviceConfig) -> Vec<OutputId> {
    config
        .axes
        .iter()
        .map(|id| OutputId::Axis { id: *id })
        .chain((1..=config.button_count).map(|id| OutputId::Button { id }))
        .chain((1..=config.hat_count).map(|id| OutputId::Hat { id }))
        .collect()
}

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
    use_context::<crate::mapping_edits::Bindings>()
        .ui
        .stage_hint
        .call(crate::mapping_edits::hints::Request {
            path: stage_id.clone(),
            input: crate::mapping_edits::hints::Input::Vjoy(
                output.clone(),
                cfg.virtual_devices.clone(),
            ),
            hints: editor.malformed_hints,
            tags: None,
        });

    // --- Build device picker options ---
    // Each option is ("N", "Virtual device N") so the value is the device_id
    // as a decimal string, which is compact and unambiguous.
    let device_options: Vec<SelectOption> = cfg
        .virtual_devices
        .iter()
        .map(|v| SelectOption {
            value: v.device_id.to_string(),
            label: format!("Virtual device {}", v.device_id),
            disabled: false,
            class: None,
        })
        .collect();

    // Current device value for the Select. Sync the local Signal to the
    // prop on every render: `use_signal` is initialized once per component
    // instance, so without this peek-and-set the dropdown would keep
    // displaying the original device after a SetMapping echo.
    let device_str = output.device.to_string();
    let mut device_value_str: Signal<String> = use_signal(|| device_str.clone());
    if *device_value_str.peek() != device_str {
        device_value_str.set(device_str);
    }

    // --- Build output picker options ---
    // If the device exists in the snapshot, list its outputs; otherwise fall
    // back to showing only the current (possibly stale) output so the UI
    // does not collapse entirely while the snapshot catches up.
    let output_options: Vec<SelectOption> = if let Some(vd) = device_cfg {
        available_outputs(vd)
            .into_iter()
            .map(|id| SelectOption {
                value: output_id_key(&id),
                label: match id {
                    OutputId::Axis { id } => axis_label(id).to_owned(),
                    OutputId::Button { id } => format!("Button {id}"),
                    OutputId::Hat { id } => format!("Hat {id}"),
                },
                disabled: false,
                class: None,
            })
            .collect()
    } else {
        // Fallback: show the current output so the picker is not empty.
        let key = output_id_key(&output.output);
        let label = match &output.output {
            OutputId::Axis { id } => axis_label(*id).to_owned(),
            OutputId::Button { id } => format!("Button {id}"),
            OutputId::Hat { id } => format!("Hat {id}"),
        };
        vec![SelectOption {
            value: key,
            label,
            disabled: false,
            class: None,
        }]
    };

    // Current output value for the Select. Same prop-sync rationale as
    // device_value_str above.
    let output_str = output_id_key(&output.output);
    let mut output_value_str: Signal<String> = use_signal(|| output_str.clone());
    if *output_value_str.peek() != output_str {
        output_value_str.set(output_str);
    }

    // --- Device picker change handler ---
    let mapping_key_dev = mapping_key.clone();
    let stage_id_dev = stage_id.clone();
    let root_actions_dev = root_actions.clone();
    let output_cloned_dev = output.clone();
    let on_device_change = move |evt: FormEvent| {
        let new_device_str = evt.value();
        let before_str = format!("Device {}", output_cloned_dev.device);
        let after_str = format!("Device {new_device_str}");
        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("Map to virtual device"),
                field: Some("device"),
                before_after: Some((&before_str, &after_str)),
                ..LabelArgs::default()
            },
        );
        super::instruments::stage_dispatch::dispatch_field(
            &root_actions_dev,
            &stage_id_dev,
            crate::mapping_edits::Field::OutputDevice(new_device_str),
            &mapping_key_dev,
            label,
        );
    };

    // --- Output picker change handler ---
    let mapping_key_out = mapping_key.clone();
    let stage_id_out = stage_id.clone();
    let root_actions_out = root_actions.clone();
    let output_cloned_out = output.clone();
    let on_output_change = move |evt: FormEvent| {
        let key_str = evt.value();
        let before_key = output_id_key(&output_cloned_out.output);
        let after_key = key_str.clone();
        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("Map to virtual device"),
                field: Some("output"),
                before_after: Some((&before_key, &after_key)),
                ..LabelArgs::default()
            },
        );
        super::instruments::stage_dispatch::dispatch_field(
            &root_actions_out,
            &stage_id_out,
            crate::mapping_edits::Field::OutputControl(key_str),
            &mapping_key_out,
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

#[cfg(test)]
mod linux_numbering_tests {
    use super::*;
    #[test]
    fn picker_offers_first_and_last_one_based_controls() {
        let config = inputforge_core::types::VirtualDeviceConfig {
            device_id: 1,
            axes: vec![],
            button_count: 2,
            hat_count: 1,
        };
        assert_eq!(
            available_outputs(&config),
            vec![
                OutputId::Button { id: 1 },
                OutputId::Button { id: 2 },
                OutputId::Hat { id: 1 }
            ]
        );
    }
}
