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

use crate::components::{Anchor, Icon, MenuItem, MenuItems, MenuRoot, MenuTrigger};
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
    // silently rendered as `X` and looked like a real binding the user had
    // not chosen.
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

    let path_prefix_clone = path_prefix.clone();
    let mapping_key_clone = mapping_key.clone();
    let root_actions_clone = root_actions.clone();

    // Shared do_insert closure factory. Returns a MouseEvent handler that
    // inserts `action` at the target position. Menu auto-closes via
    // MenuItem; this closure no longer touches an open signal.
    let make_insert_handler = move |action: Action| {
        let key = mapping_key_clone.clone();
        let prefix = path_prefix_clone.clone();
        let root = root_actions_clone.clone();
        let cmd_tx = ctx.commands.clone();
        let cfg_sig = ctx.config;
        let mut undo_log = editor.undo_log;
        let mut expanded = editor.expanded_stages;
        let mut malformed = editor.malformed_hints;
        let mut tags = editor.malformed_summary_tags;
        let insert_len = target_len;

        move |_: MouseEvent| {
            let mut path_segs = prefix.clone();
            path_segs.push(StageIdSegment::Index(insert_len));
            let insert_path = StageId(path_segs);

            let Some(new_actions) = insert_at_path(&root, &insert_path, action.clone()) else {
                return;
            };

            let cfg = cfg_sig.read();
            let current_name = cfg.mapping_names.get(&key.1).cloned();
            drop(cfg);

            let before = Mapping {
                input: key.1.clone(),
                mode: key.0.clone(),
                name: current_name.clone(),
                actions: root.clone(),
            };

            let stage_title = action_palette_label(&action);

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
                return;
            }

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

            let parent_path = insert_path.0[..insert_path.0.len() - 1].to_vec();
            let insert_idx = insert_len;
            expanded
                .write()
                .retain(|p| !path_invalidated_by_mutation(p, &parent_path, insert_idx));
            malformed
                .write()
                .retain(|p, _| !path_invalidated_by_mutation(p, &parent_path, insert_idx));
            tags.write()
                .retain(|p, _| !path_invalidated_by_mutation(p, &parent_path, insert_idx));
            expanded.write().insert(insert_path);
        }
    };

    let trigger_class = if louder {
        "if-add-palette__trigger if-add-palette__trigger--louder"
    } else {
        "if-add-palette__trigger"
    };

    let compact_aria_label = if louder {
        None
    } else {
        // Icon-only trigger: must carry an accessible name. WCAG 2.1 SC 4.1.2.
        Some("Add stage".to_owned())
    };

    rsx! {
        MenuRoot { class: "if-add-palette if-menu--block".to_owned(),
            MenuTrigger {
                class: trigger_class.to_owned(),
                unstyled: true,
                aria_label: compact_aria_label,
                if louder {
                    Icon { name: IconKind::Plus, size: IconSize::Sm }
                    "Add first stage"
                } else {
                    Icon { name: IconKind::Plus, size: IconSize::Sm }
                }
            }
            MenuItems { class: "if-add-palette__menu".to_owned(), anchor: Anchor::Center,
                div { class: "if-add-palette__section is-processing",
                    div { class: "if-add-palette__section-title", "Processing" }
                    for item in PROCESSING_ITEMS {
                        MenuItem {
                            class: "if-add-palette__item".to_owned(),
                            onclick: make_insert_handler((item.make)()),
                            "{item.label}"
                        }
                    }
                }
                div { class: "if-add-palette__section is-output",
                    div { class: "if-add-palette__section-title", "Output" }
                    for item in OUTPUT_ITEMS {
                        MenuItem {
                            class: "if-add-palette__item".to_owned(),
                            onclick: make_insert_handler((item.make)()),
                            "{item.label}"
                        }
                    }
                }
                div { class: "if-add-palette__section is-control",
                    div { class: "if-add-palette__section-title", "Control" }
                    for item in CONTROL_ITEMS {
                        MenuItem {
                            class: "if-add-palette__item".to_owned(),
                            onclick: make_insert_handler((item.make)()),
                            "{item.label}"
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
