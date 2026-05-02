// Rust guideline compliant 2026-05-01

//! Categorized action add palette.
//!
//! Clicking the `+` button (or `+ Add first stage` on an empty pipeline)
//! opens a small menu divided into three sections:
//!
//! - **Processing** -- Invert, Deadzone, Response curve
//! - **Output** -- Map to vJoy, Map to keyboard, Merge axis
//! - **Control** -- Conditional, Change mode
//!
//! Clicking an item appends a default-configured action at `target_len`
//! (i.e. after the last stage) in the branch identified by `path_prefix`.
//!
//! # Amendment notes applied here
//!
//! 1. Section accent uses `--color-stage-tint-{processing,output,control}` (Task 5).
//! 2. Both empty-pipeline and end-of-pipeline `+` buttons are wired here.
//! 3. Name is read from `cfg.mapping_names.get(&mapping_key.1)` so user-set
//!    names are never silently cleared.
//! 4. After insert: `editor_state.expanded_stages` AND
//!    `editor_state.malformed_hints` are cleared, then the new stage's
//!    `StageId` is re-inserted so it opens expanded.
//! 5. `push_edit` is only called when `cmd_tx.send` succeeds.

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::processing::{DeadzoneConfig, ResponseCurve};
use inputforge_core::types::{InputAddress, KeyCombo, MergeOp, OutputAddress, OutputId, VJoyAxis};

use crate::components::Icon;
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::{insert_at_path, path_invalidated_by_mutation};
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, StageIdSegment, UndoKind, format_undo_label,
};
use crate::icons::{Icon as IconKind, IconSize};

use inputforge_core::action::ModeChangeStrategy;

// ---------------------------------------------------------------------------
// Default-action constructors
// ---------------------------------------------------------------------------

/// Build the default-configured action for each palette entry.
fn default_invert() -> Action {
    Action::Invert
}

fn default_deadzone() -> Action {
    Action::Deadzone {
        config: DeadzoneConfig::default(),
    }
}

fn default_response_curve() -> Action {
    Action::ResponseCurve {
        curve: ResponseCurve::PiecewiseLinear {
            // Identity passthrough: two endpoints (in, out) = (-1, -1) and (1, 1).
            points: vec![(-1.0, -1.0), (1.0, 1.0)],
            symmetric: false,
        },
    }
}

fn default_map_to_vjoy() -> Action {
    Action::MapToVJoy {
        output: OutputAddress {
            device: 1,
            output: OutputId::Axis { id: VJoyAxis::X },
        },
    }
}

fn default_map_to_keyboard() -> Action {
    Action::MapToKeyboard {
        key: KeyCombo {
            key: "A".to_owned(),
            modifiers: vec![],
        },
    }
}

fn default_merge_axis() -> Action {
    // Seed the secondary slot as `Unbound` so the row renders the explicit
    // `Unbound` placeholder until the user picks an input. The previous
    // sentinel `Bound { device: DeviceId(""), input: Axis { index: 0 } }`
    // silently rendered as `X` (or `Btn 1` for buttons) and looked like a
    // real binding the user had not chosen.
    Action::MergeAxis {
        second_input: InputAddress::Unbound,
        operation: MergeOp::Average,
    }
}

fn default_conditional() -> Action {
    // Same Unbound-seed reasoning as `default_merge_axis`: a freshly added
    // Conditional has no predicate input chosen yet.
    Action::Conditional {
        condition: Condition::ButtonPressed {
            input: InputAddress::Unbound,
        },
        if_true: vec![],
        if_false: Vec::new(),
    }
}

fn default_change_mode() -> Action {
    Action::ChangeMode {
        strategy: ModeChangeStrategy::SwitchTo {
            mode: String::new(),
        },
    }
}

// ---------------------------------------------------------------------------
// Palette section descriptors
// ---------------------------------------------------------------------------

struct PaletteItem {
    label: &'static str,
    make: fn() -> Action,
}

const PROCESSING_ITEMS: &[PaletteItem] = &[
    PaletteItem {
        label: "Invert",
        make: default_invert,
    },
    PaletteItem {
        label: "Deadzone",
        make: default_deadzone,
    },
    PaletteItem {
        label: "Response curve",
        make: default_response_curve,
    },
];

const OUTPUT_ITEMS: &[PaletteItem] = &[
    PaletteItem {
        label: "Map to vJoy",
        make: default_map_to_vjoy,
    },
    PaletteItem {
        label: "Map to keyboard",
        make: default_map_to_keyboard,
    },
    PaletteItem {
        label: "Merge axis",
        make: default_merge_axis,
    },
];

const CONTROL_ITEMS: &[PaletteItem] = &[
    PaletteItem {
        label: "Conditional",
        make: default_conditional,
    },
    PaletteItem {
        label: "Change mode",
        make: default_change_mode,
    },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Categorized action add palette.
///
/// Renders either a louder `+ Add first stage` label (when `louder` is true,
/// i.e. the pipeline is empty) or a compact `+` button. Clicking opens a
/// three-section menu; clicking an item inserts a default-configured action at
/// `[path_prefix.., Index(target_len)]` and closes the menu.
#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
pub(crate) fn AddPalette(
    /// `(mode, InputAddress)` key for the mapping. Named `mapping_key` to
    /// avoid collision with Dioxus's reserved `key` prop.
    mapping_key: MappingKey,
    /// `StageId` prefix segments from ancestor pipelines (empty at the outer
    /// pipeline; `Conditional` recursion appends branch segments).
    path_prefix: Vec<StageIdSegment>,
    /// Insertion index inside the branch: actions are inserted at this
    /// position (which equals the current length of the branch, i.e. append).
    target_len: usize,
    /// Mapping's outermost actions vec. `insert_at_path` is called against
    /// this to build the new root action tree.
    root_actions: Vec<Action>,
    /// When true renders the louder `+ Add first stage` text (empty pipeline).
    louder: bool,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();

    // Local open/close state for the dropdown menu.
    let mut open = use_signal(|| false);

    // Build the StageId for the new stage (path_prefix + Index(target_len)).
    // Cloned here for use inside closures below.
    let path_prefix_clone = path_prefix.clone();
    let mapping_key_clone = mapping_key.clone();
    let root_actions_clone = root_actions.clone();

    // Shared do_insert closure factory. Returns a MouseEvent handler that
    // inserts `action` at the target position.
    let make_insert_handler = move |action: Action| {
        let key = mapping_key_clone.clone();
        let prefix = path_prefix_clone.clone();
        let root = root_actions_clone.clone();
        let cmd_tx = ctx.commands.clone();
        let cfg_sig = ctx.config;
        let mut undo_log = editor.undo_log;
        let mut expanded = editor.expanded_stages;
        let mut malformed = editor.malformed_hints;
        let mut open_sig = open;
        let insert_len = target_len;

        move |_: MouseEvent| {
            // Build the insertion path.
            let mut path_segs = prefix.clone();
            path_segs.push(StageIdSegment::Index(insert_len));
            let insert_path = StageId(path_segs);

            // Call insert_at_path against root_actions.
            let Some(new_actions) = insert_at_path(&root, &insert_path, action.clone()) else {
                // Invalid path: skip edit and phantom undo entry.
                open_sig.set(false);
                return;
            };

            // Amendment 3: read current name from snapshot.
            let cfg = cfg_sig.read();
            let current_name = cfg.mapping_names.get(&key.1).cloned();
            drop(cfg);

            // Build before-Mapping snapshot.
            let before = Mapping {
                input: key.1.clone(),
                mode: key.0.clone(),
                name: current_name.clone(),
                actions: root.clone(),
            };

            // Derive stage title for the undo label.
            let stage_title = action_palette_label(&action);

            // Amendment 5: dispatch first; push undo only on success.
            if cmd_tx
                .send(EngineCommand::SetMapping {
                    input: key.1.clone(),
                    mode: key.0.clone(),
                    name: current_name,
                    actions: new_actions,
                })
                .is_err()
            {
                tracing::warn!(
                    target: "f9::mapping_editor",
                    action = "add_palette_drop_offline",
                    "stage add dropped: engine channel disconnected"
                );
                open_sig.set(false);
                return;
            }

            // Push StageAdd undo entry.
            let label = format_undo_label(
                UndoKind::StageAdd,
                LabelArgs {
                    stage_name: Some(stage_title),
                    index: Some(insert_len),
                    ..LabelArgs::default()
                },
            );
            undo_log
                .write()
                .push_edit(key.clone(), before, UndoKind::StageAdd, label);

            // Amendment 4: invalidate only paths whose indices shifted from
            // the insert (paths in the same branch at-or-after the insert
            // point). Strict ancestors and unrelated branches survive, so
            // the parent Conditional / outer pipeline keeps its expanded
            // state. Then re-expand the freshly-inserted stage.
            let parent_path = insert_path.0[..insert_path.0.len() - 1].to_vec();
            let insert_idx = insert_len;
            expanded
                .write()
                .retain(|p| !path_invalidated_by_mutation(p, &parent_path, insert_idx));
            malformed
                .write()
                .retain(|p, _| !path_invalidated_by_mutation(p, &parent_path, insert_idx));
            expanded.write().insert(insert_path);

            // Close the menu.
            open_sig.set(false);
        }
    };

    // Class modifier depends on louder / compact mode. Both variants
    // share the dashed-violet "next slot" treatment (defined in CSS);
    // `--louder` raises the alpha to make the empty-pipeline placeholder
    // shout. Compact stays quiet, then matches the louder intensity on
    // hover so the affordance is discoverable without dominating the
    // pipeline at rest.
    let trigger_class = if louder {
        "if-add-palette__trigger if-add-palette__trigger--louder"
    } else {
        "if-add-palette__trigger"
    };

    rsx! {
        div { class: "if-add-palette",
            if louder {
                button {
                    r#type: "button",
                    class: "{trigger_class}",
                    onclick: move |_| open.set(!open()),
                    Icon { name: IconKind::Plus, size: IconSize::Sm }
                    "Add first stage"
                }
            } else {
                button {
                    r#type: "button",
                    class: "{trigger_class}",
                    "aria-label": "Add stage",
                    onclick: move |_| open.set(!open()),
                    Icon { name: IconKind::Plus, size: IconSize::Sm }
                }
            }
            if open() {
                div { class: "if-add-palette__menu",
                    // --- Processing section ---
                    div { class: "if-add-palette__section is-processing",
                        div { class: "if-add-palette__section-title", "Processing" }
                        for item in PROCESSING_ITEMS {
                            button {
                                r#type: "button",
                                class: "if-add-palette__item",
                                onclick: make_insert_handler((item.make)()),
                                "{item.label}"
                            }
                        }
                    }
                    // --- Output section ---
                    div { class: "if-add-palette__section is-output",
                        div { class: "if-add-palette__section-title", "Output" }
                        for item in OUTPUT_ITEMS {
                            button {
                                r#type: "button",
                                class: "if-add-palette__item",
                                onclick: make_insert_handler((item.make)()),
                                "{item.label}"
                            }
                        }
                    }
                    // --- Control section ---
                    div { class: "if-add-palette__section is-control",
                        div { class: "if-add-palette__section-title", "Control" }
                        for item in CONTROL_ITEMS {
                            button {
                                r#type: "button",
                                class: "if-add-palette__item",
                                onclick: make_insert_handler((item.make)()),
                                "{item.label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Return the palette display label for an action (used in undo label).
fn action_palette_label(action: &Action) -> &'static str {
    match action {
        Action::Invert => "Invert",
        Action::Deadzone { .. } => "Deadzone",
        Action::ResponseCurve { .. } => "Response curve",
        Action::MapToVJoy { .. } => "Map to vJoy",
        Action::MapToKeyboard { .. } => "Map to keyboard",
        Action::MergeAxis { .. } => "Merge axis",
        Action::Conditional { .. } => "Conditional",
        Action::ChangeMode { .. } => "Change mode",
    }
}
