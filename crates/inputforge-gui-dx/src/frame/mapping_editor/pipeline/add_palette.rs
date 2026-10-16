// Rust guideline compliant 2026-05-01

//! Categorized action add palette.
//!
//! Clicking the `+` button (or `+ Add first stage` on an empty pipeline)
//! opens a small menu divided into three sections:
//!
//! - **Processing** -- Invert, Deadzone, Response curve
//! - **Output** -- Map to virtual device, Map to keyboard, Merge axis
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
//! 5. App history is recorded only after successful dispatch.

use dioxus::prelude::*;

use inputforge_core::action::Action;
use inputforge_core::types::KeyCombo;

use crate::components::{Anchor, Icon, MenuItem, MenuItems, MenuRoot, MenuTrigger};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::path_invalidated_by_mutation;
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, StageIdSegment, UndoKind, format_undo_label,
};
use crate::keyboard_capture::Destination;
use crate::patterns::keyboard_capture::{target, use_keyboard_capture};

use crate::icons::{Icon as IconKind, IconSize};
use crate::mapping_edits::StageKind;

struct PaletteItem {
    label: &'static str,
    kind: StageKind,
}

const PROCESSING_ITEMS: &[PaletteItem] = &[
    PaletteItem {
        label: "Invert",
        kind: StageKind::Invert,
    },
    PaletteItem {
        label: "Deadzone",
        kind: StageKind::Deadzone,
    },
    PaletteItem {
        label: "Response curve",
        kind: StageKind::ResponseCurve,
    },
];

const OUTPUT_ITEMS: &[PaletteItem] = &[
    PaletteItem {
        label: "Map to mouse",
        kind: StageKind::Mouse,
    },
    PaletteItem {
        label: "Merge axis",
        kind: StageKind::MergeAxis,
    },
];

const CONTROL_ITEMS: &[PaletteItem] = &[
    PaletteItem {
        label: "Conditional",
        kind: StageKind::Conditional,
    },
    PaletteItem {
        label: "Change mode",
        kind: StageKind::ChangeMode,
    },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
fn KeyboardCapturePrompt(onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { class: "if-key-capture", "aria-live": "polite",
            button {
                r#type: "button",
                class: "if-key-capture__surface is-listening",
                autofocus: true,
                "aria-label": "Press the physical key or shortcut to map. Escape cancels.",
                onclick,
                span { class: "if-key-capture__value",
                    "Press the physical key or shortcut to map. Esc cancels."
                }
            }
        }
    }
}

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
    crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::use_origin();
    let editor = use_context::<EditorState>();

    let availability = use_context::<crate::mapping_edits::Bindings>()
        .palette
        .call((mapping_key.clone(), path_prefix.clone()));
    let show_gestures = availability.gestures;

    let key = mapping_key.clone();
    let prefix = path_prefix.clone();
    let root = root_actions.clone();
    let mut expanded = editor.expanded_stages;
    let mut malformed = editor.malformed_hints;
    let mut tags = editor.malformed_summary_tags;
    let insert_len = target_len;
    let insert_action = use_callback(move |kind: StageKind| {
        let mut path_segs = prefix.clone();
        path_segs.push(StageIdSegment::Index(insert_len));
        let insert_path = StageId(path_segs);

        let stage_title = action_palette_label(&kind);

        let label = format_undo_label(
            UndoKind::StageAdd,
            LabelArgs {
                stage_name: Some(stage_title),
                index: Some(insert_len),
                ..LabelArgs::default()
            },
        );
        if !super::stage_body::instruments::stage_dispatch::submit(
            &key,
            &root,
            crate::mapping_edits::Edit::Add {
                path: insert_path.clone(),
                kind,
            },
            label,
            None,
            true,
        ) {
            return false;
        }

        let parent_path = insert_path.0[..insert_path.0.len() - 1].to_vec();
        expanded
            .write()
            .retain(|path| !path_invalidated_by_mutation(path, &parent_path, insert_len));
        malformed
            .write()
            .retain(|path, _| !path_invalidated_by_mutation(path, &parent_path, insert_len));
        tags.write()
            .retain(|path, _| !path_invalidated_by_mutation(path, &parent_path, insert_len));
        expanded.write().insert(insert_path);
        true
    });

    let mut insertion_path: Vec<_> = path_prefix
        .iter()
        .map(|segment| match segment {
            StageIdSegment::Index(i) => inputforge_core::pipeline::ActionPathSegment::Index(*i),
            StageIdSegment::Branch(branch) => {
                inputforge_core::pipeline::ActionPathSegment::Branch(*branch)
            }
        })
        .collect();
    insertion_path.push(inputforge_core::pipeline::ActionPathSegment::Index(
        target_len,
    ));
    let keyboard_capture = use_keyboard_capture(
        target(
            &ctx,
            &mapping_key,
            &root_actions,
            Destination::Insert(insertion_path),
        ),
        use_callback(move |combo: KeyCombo| {
            insert_action.call(StageKind::Keyboard(combo));
        }),
    );
    let start_keyboard_capture = move |_| keyboard_capture.start.call(());

    let make_insert_handler = move |kind: StageKind| {
        let insert = insert_action;
        move |_: MouseEvent| insert.call(kind.clone())
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

    let capture_hint = *keyboard_capture.hint.read();
    if *keyboard_capture.active.read() {
        let onclick = move |_| {
            keyboard_capture.cancel.call(());
        };
        return rsx! {
            KeyboardCapturePrompt { onclick }
        };
    }

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
                            onactivate: make_insert_handler(item.kind.clone()),
                            "{item.label}"
                        }
                    }
                }
                div { class: "if-add-palette__section is-output",
                    div { class: "if-add-palette__section-title", "Output" }
                    MenuItem {
                        class: "if-add-palette__item".to_owned(),
                        onactivate: make_insert_handler(StageKind::VirtualDevice),
                        "Map to virtual device"
                    }
                    MenuItem {
                        disabled: !availability.keyboard,
                        class: "if-add-palette__item".to_owned(),
                        onclick: start_keyboard_capture,
                        "Map to keyboard"
                    }
                    for item in OUTPUT_ITEMS {
                        MenuItem {
                            disabled: matches!(item.kind.clone(), StageKind::Mouse)
                                && !availability.mouse,
                            class: "if-add-palette__item".to_owned(),
                            onactivate: make_insert_handler(item.kind.clone()),
                            "{item.label}"
                        }
                    }
                }
                div { class: "if-add-palette__section is-control",
                    div { class: "if-add-palette__section-title", "Control" }
                    for item in CONTROL_ITEMS {
                        MenuItem {
                            class: "if-add-palette__item".to_owned(),
                            onactivate: make_insert_handler(item.kind.clone()),
                            "{item.label}"
                        }
                    }
                    if show_gestures {
                        MenuItem {
                            class: "if-add-palette__item".to_owned(),
                            onactivate: make_insert_handler(StageKind::Tap),
                            "Tap gesture"
                        }
                        MenuItem {
                            class: "if-add-palette__item".to_owned(),
                            onactivate: make_insert_handler(StageKind::Press),
                            "Press gesture"
                        }
                    }
                }
            }
        }
        if let Some(message) = capture_hint {
            span { class: "if-key-capture__hint", role: "status", "{message}" }
        }
    }
}

/// Return the palette display label for an action (used in undo label).
fn action_palette_label(action: &StageKind) -> &'static str {
    match action {
        StageKind::Invert => "Invert",
        StageKind::Deadzone => "Deadzone",
        StageKind::ResponseCurve => "Response curve",
        StageKind::VirtualDevice => "Map to virtual device",
        StageKind::Keyboard(_) => "Map to keyboard",
        StageKind::Mouse => "Map to mouse",
        StageKind::MergeAxis => "Merge axis",
        StageKind::Conditional => "Conditional",
        StageKind::ChangeMode => "Change mode",
        StageKind::Tap => "Tap gesture",
        StageKind::Press => "Press gesture",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture_prompt_harness() -> Element {
        rsx! { KeyboardCapturePrompt { onclick: |_| {} } }
    }

    #[test]
    fn capture_prompt_explains_physical_input_and_cancellation() {
        let mut vdom = VirtualDom::new(capture_prompt_harness);
        vdom.rebuild_in_place();
        let html = dioxus_ssr::render(&vdom);

        assert!(html.contains("Press the physical key or shortcut to map."));
        assert!(html.contains("Escape cancels."));
        assert!(html.contains(r#"aria-live="polite""#));
        assert!(html.contains("autofocus"));
    }
}
