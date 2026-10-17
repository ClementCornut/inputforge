// Rust guideline compliant 2026-05-02

//! Predicate editor: kind picker + operand fields per spec line 349.
//! Recursive for `All` / `Any` / `Not`.
//!
//! # Component tree
//!
//! ```text
//! PredicateEditor
//!   <select> kind picker -- 7 options
//!   (per-kind operand area)
//!     ButtonPressed / ButtonReleased -- PredicateInputRow (source label + rebind)
//!     AxisInRange   -- PredicateInputRow + 2x NumberInput (min, max)
//!     HatDirection  -- PredicateInputRow + 8 checkboxes in a 3-col grid
//!     All / Any     -- nested-list with one read-only PredicateEditor per child
//!     Not           -- single nested PredicateEditor for the inner condition
//! ```
//!
//! # Recursive All / Any / Not
//!
//! Nested `PredicateEditor` instances inside `All` / `Any` / `Not` are
//! display-only in this release (F9). Each nested editor renders the kind
//! badge and operand labels for the sub-condition but its dispatchers
//! operate against the outer `stage_id`. Full per-sub-slot editing is a
//! future task.
//!
//! # Closure / Copy convention
//!
//! `AppContext` is `Clone` but not `Copy` (it holds `Arc` and `mpsc::Sender`
//! fields). Rather than passing the whole context into `move` closures,
//! each handler captures only the two fields it needs:
//! - An app edit callback supplied through the mapping bindings
//! - `cfg: Signal<ConfigSnapshot>` (`Copy`, safe to copy into every closure)
//!
//! `EditorState` IS `Copy` (all `Signal` fields) and is copied directly.
//! `MappingKey`, `StageId`, and `Vec<Action>` are `Clone`; each closure
//! clones them inside the body so the closure stays `FnMut`.
//!
//! # Amendments applied
//!
//! 1. App dispatch preserves the exact mapping target's optional name.
//! 2. App dispatch records history only after successful command delivery.
//! 3. App-owned capture ownership and completion delivery.
//! 4. Malformed-hint write for `AxisInRange` (min > max), `HatDirection`
//!    (empty directions), `All` / `Any` (zero conditions).

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition};
use inputforge_core::types::{HatDirection, InputAddress};

use crate::capture::{CaptureFilter, Target};
use crate::components::{NumberInput, Select, SelectOption};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{CAPTURE_PROMPT, rebind_composite_class, use_capture};

// ---------------------------------------------------------------------------
// Kind string constants -- stable keys shared between picker and parser.
// ---------------------------------------------------------------------------

const KIND_BUTTON_PRESSED: &str = "ButtonPressed";
const KIND_BUTTON_RELEASED: &str = "ButtonReleased";
const KIND_AXIS_IN_RANGE: &str = "AxisInRange";
const KIND_HAT_DIRECTION: &str = "HatDirection";
const KIND_ALL: &str = "All";
const KIND_ANY: &str = "Any";
const KIND_NOT: &str = "Not";

/// Convert a `Condition` reference to its stable kind string.
fn condition_kind(c: &Condition) -> &'static str {
    match c {
        Condition::ButtonPressed { .. } => KIND_BUTTON_PRESSED,
        Condition::ButtonReleased { .. } => KIND_BUTTON_RELEASED,
        Condition::AxisInRange { .. } => KIND_AXIS_IN_RANGE,
        Condition::HatDirection { .. } => KIND_HAT_DIRECTION,
        Condition::All { .. } => KIND_ALL,
        Condition::Any { .. } => KIND_ANY,
        Condition::Not { .. } => KIND_NOT,
    }
}

// ---------------------------------------------------------------------------
// Eight hat-direction variants in display order.
// ---------------------------------------------------------------------------

/// Eight compass `HatDirection` values in display order.
///
/// `Center` is excluded: a "pointing at center" condition is better expressed
/// with an inversion or an `Any` wrapper, not a direction-set that includes
/// `Center` (which is the hat's idle / released position).
const HAT_VARIANTS: [(HatDirection, &str); 8] = [
    (HatDirection::N, "N"),
    (HatDirection::NE, "NE"),
    (HatDirection::E, "E"),
    (HatDirection::SE, "SE"),
    (HatDirection::S, "S"),
    (HatDirection::SW, "SW"),
    (HatDirection::W, "W"),
    (HatDirection::NW, "NW"),
];

// ---------------------------------------------------------------------------
// Shared dispatch helper
// ---------------------------------------------------------------------------

/// Wire the updated predicate and unchanged branches to the app controller.
fn commit_condition(
    field: crate::mapping_edits::Field,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
) -> bool {
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Conditional"),
            field: Some("condition"),
            ..LabelArgs::default()
        },
    );
    super::instruments::stage_dispatch::dispatch_field(
        root_actions,
        stage_id,
        field,
        mapping_key,
        label,
    )
}

// ---------------------------------------------------------------------------
// PredicateInputRow -- shared source-label + rebind button sub-component.
// ---------------------------------------------------------------------------

/// Source label + rebind action for the `input` field of a condition.
///
/// Renders the shared `if-rebind-composite` cluster (same primitive used by
/// the editor header subtitle and the `MergeAxis` Secondary input row) so the
/// rebind affordance reads identically across all three call sites. The app
/// delivers only to the current capture owner.
#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant `dioxus_elements::*` qualifications \
              on per-element event listeners with bound closures."
)]
fn PredicateInputRow(
    /// The current input address displayed as a source label.
    input: InputAddress,
    /// Callback invoked with the new address after a successful capture.
    on_input_change: EventHandler<InputAddress>,
    target: Target,
) -> Element {
    let ctx = use_context::<AppContext>();
    let capture = use_capture(
        target,
        CaptureFilter::Any,
        move |new_addr| {
            on_input_change.call(new_addr);
        },
        |()| {},
    );

    let source = source_label::format(&input, &ctx.config.read());

    // Compose the rebind-composite class once so both the idle and listening
    // branches stay in lockstep. The `--unbound` modifier styles the
    // `__label` span muted/italic so the user sees at a glance that the
    // field is empty rather than mistaking the placeholder for a real
    // binding. Carry `--unbound` on both branches so the class doesn't
    // flicker when the user toggles rebind on an Unbound row. Today the
    // CSS rule only styles `__label` (idle branch), so the modifier is
    // inert in the listening state, but keeping it here means a future
    // rule keyed on `--unbound .__listening` would Just Work.
    let composite_class = rebind_composite_class(&input, false);
    let listening_class = rebind_composite_class(&input, true);

    let on_rebind = move |_: MouseEvent| capture.start.call(());
    let on_cancel = move |_: MouseEvent| capture.cancel.call(());
    let is_listening = *capture.listening.read();

    rsx! {
        div { class: "if-predicate__input-row",
            if is_listening {
                div { class: "{listening_class}",
                    span {
                        class: "if-rebind-composite__listening",
                        role: "status",
                        "aria-live": "polite",
                        "{CAPTURE_PROMPT}"
                    }
                    button {
                        class: "if-rebind-composite__action",
                        r#type: "button",
                        "aria-label": "Cancel predicate input capture",
                        onclick: on_cancel,
                        "Cancel"
                    }
                }
            } else {
                div { class: "{composite_class}",
                    span { class: "if-rebind-composite__label", "{source}" }
                    button {
                        class: "if-rebind-composite__action",
                        r#type: "button",
                        "aria-label": "Rebind predicate input",
                        onclick: on_rebind,
                        "rebind"
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PredicateEditor -- public component consumed by ConditionalBody.
// ---------------------------------------------------------------------------

/// Full predicate editor covering all 7 `Condition` variants.
///
/// Renders a `<select>` kind picker with all 7 options, plus per-kind
/// operand UI below the picker.
///
/// Leaf kinds (`ButtonPressed`, `ButtonReleased`, `AxisInRange`,
/// `HatDirection`) dispatch directly via `commit_condition`.
///
/// Recursive kinds (`All`, `Any`, `Not`) render nested `PredicateEditor`
/// instances. In this F9 release the nested editors are display-only.
#[component]
#[expect(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event \
              listener attribute shorthand (onchange: move |_|)."
)]
pub(crate) fn PredicateEditor(
    /// `(mode, InputAddress)` key for the mapping being edited.
    mapping_key: MappingKey,
    /// `StageId` of the enclosing `Conditional` stage (root-relative).
    stage_id: StageId,
    /// The condition to display and edit.
    condition: Condition,
    /// `if_true` branch forwarded unchanged into `commit_condition`.
    if_true: Vec<Action>,
    /// `if_false` branch forwarded unchanged into `commit_condition`.
    if_false: Vec<Action>,
    /// Full root-level actions vec, threaded unchanged per the Task 20 rule.
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let capture_target = Target {
        profile: ctx.meta.read().profile_id.clone(),
        mode: mapping_key.0.clone(),
        mapping: Some(mapping_key.1.clone()),
        name: ctx
            .config
            .read()
            .mappings
            .iter()
            .find(|mapping| mapping.input == mapping_key.1 && mapping.mode == mapping_key.0)
            .and_then(|mapping| mapping.name.clone()),
        actions: Some(root_actions.clone()),
        field: format!("predicate:{stage_id:?}:{condition:?}"),
    };

    // Build the 7-option kind picker options vec.
    let kind_options: Vec<SelectOption> = vec![
        SelectOption {
            value: KIND_BUTTON_PRESSED.to_owned(),
            label: KIND_BUTTON_PRESSED.to_owned(),
            disabled: false,
            class: None,
        },
        SelectOption {
            value: KIND_BUTTON_RELEASED.to_owned(),
            label: KIND_BUTTON_RELEASED.to_owned(),
            disabled: false,
            class: None,
        },
        SelectOption {
            value: KIND_AXIS_IN_RANGE.to_owned(),
            label: KIND_AXIS_IN_RANGE.to_owned(),
            disabled: false,
            class: None,
        },
        SelectOption {
            value: KIND_HAT_DIRECTION.to_owned(),
            label: KIND_HAT_DIRECTION.to_owned(),
            disabled: false,
            class: None,
        },
        SelectOption {
            value: KIND_ALL.to_owned(),
            label: "All".to_owned(),
            disabled: false,
            class: None,
        },
        SelectOption {
            value: KIND_ANY.to_owned(),
            label: "Any".to_owned(),
            disabled: false,
            class: None,
        },
        SelectOption {
            value: KIND_NOT.to_owned(),
            label: "Not".to_owned(),
            disabled: false,
            class: None,
        },
    ];

    let current_kind = condition_kind(&condition).to_owned();
    // Sync the Select's local Signal to the prop on every render.
    // `use_signal` returns persistent state per component instance, so its
    // initializer runs only once; without this peek-and-set the Select
    // would keep displaying the kind from the first render even after
    // `condition` is rebuilt with a different variant.
    let mut kind_signal: Signal<String> = use_signal(|| current_kind.clone());
    if *kind_signal.peek() != current_kind {
        kind_signal.set(current_kind);
    }

    let axis_min = match &condition {
        Condition::AxisInRange { min, .. } => *min,
        _ => -1.0,
    };
    let axis_max = match &condition {
        Condition::AxisInRange { max, .. } => *max,
        _ => 1.0,
    };
    let mut min_sig: Signal<f64> = use_signal(move || axis_min);
    let mut max_sig: Signal<f64> = use_signal(move || axis_max);
    if let Condition::AxisInRange { min, max, .. } = &condition {
        if (*min_sig.peek()).to_bits() != min.to_bits() {
            min_sig.set(*min);
        }
        if (*max_sig.peek()).to_bits() != max.to_bits() {
            max_sig.set(*max);
        }
    }

    use_context::<crate::mapping_edits::Bindings>()
        .ui
        .stage_hint
        .call(crate::mapping_edits::hints::Request {
            path: stage_id.clone(),
            input: crate::mapping_edits::hints::Input::Predicate(
                condition.clone(),
                (*min_sig.read(), *max_sig.read()),
            ),
            hints: editor.malformed_hints,
            tags: None,
        });

    // Capture clones for the kind-change handler.
    let mk_kind = mapping_key.clone();
    let sid_kind = stage_id.clone();
    let root_kind = root_actions.clone();
    // FnMut: clones mk_kind / sid_kind / root_kind etc. on each call.
    let on_kind_change = move |evt: FormEvent| {
        let new_kind = evt.value();
        commit_condition(
            crate::mapping_edits::Field::ConditionKind(new_kind),
            &mk_kind.clone(),
            &sid_kind,
            &root_kind.clone(),
        );
    };

    // Render per-kind operand UI.
    let operand_ui = match &condition {
        // ---------------------------------------------------------------
        // ButtonPressed / ButtonReleased: source-label + rebind button.
        // ---------------------------------------------------------------
        Condition::ButtonPressed { input } | Condition::ButtonReleased { input } => {
            let input_clone = input.clone();
            let mk = mapping_key.clone();
            let sid = stage_id.clone();
            let root = root_actions.clone();
            rsx! {
                                        PredicateInputRow {
                                            target: capture_target.clone(),
                                            input: input_clone,
                                            on_input_change: move |new_addr: InputAddress| {
                                                commit_condition(crate::mapping_edits::Field::ConditionInput(new_addr),
            &mk.clone(),
            &sid,
            &root.clone());
                                            },
                                        }
                                    }
        }

        // ---------------------------------------------------------------
        // AxisInRange: source-label + rebind + min/max NumberInput pair.
        // ---------------------------------------------------------------
        Condition::AxisInRange { input, .. } => {
            let input_addr = input.clone();

            // Clones for the PredicateInputRow rebind callback.
            let mk_in = mapping_key.clone();
            let sid_in = stage_id.clone();
            let root_in = root_actions.clone();
            let addr_in = input_addr.clone();
            // Clones for the min onstep callback.
            let mk_min = mapping_key.clone();
            let sid_min = stage_id.clone();
            let root_min = root_actions.clone();
            // Clones for the max onstep callback.
            let mk_max = mapping_key.clone();
            let sid_max = stage_id.clone();
            let root_max = root_actions.clone();
            rsx! {
                                        PredicateInputRow {
                                            target: capture_target.clone(),
                                            input: addr_in,
                                            on_input_change: move |new_addr: InputAddress| {
                                                commit_condition(crate::mapping_edits::Field::ConditionInput(new_addr),
            &mk_in.clone(),
            &sid_in,
            &root_in.clone());
                                            },
                                        }
                                        div { class: "if-predicate__range-row",
                                            label { class: "if-predicate__range-label", "min" }
                                            NumberInput {
                                                value: min_sig,
                                                min: -1.0,
                                                max: 1.0,
                                                step: 0.01,
                                                precision: 2_usize,
                                                // oninput updates the local signal only; no dispatch (avoids
                                                // flooding the engine channel with one entry per keystroke).
                                                oninput: move |evt: FormEvent| {
                                                    if let Some(v) = consume_context::<crate::mapping_edits::Bindings>().number.call(evt.value()) {
                                                        min_sig.set(v);
                                                    }
                                                },
                                                // onstep dispatches immediately (one click = one dispatch).
                                                onstep: move |v: f64| {
                                                    if commit_condition(crate::mapping_edits::Field::ConditionMin(v.to_string()),
            &mk_min.clone(),
            &sid_min,
            &root_min.clone()) { min_sig.set(v); }
                                                },
                                            }
                                            label { class: "if-predicate__range-label", "max" }
                                            NumberInput {
                                                value: max_sig,
                                                min: -1.0,
                                                max: 1.0,
                                                step: 0.01,
                                                precision: 2_usize,
                                                oninput: move |evt: FormEvent| {
                                                    if let Some(v) = consume_context::<crate::mapping_edits::Bindings>().number.call(evt.value()) {
                                                        max_sig.set(v);
                                                    }
                                                },
                                                onstep: move |v: f64| {
                                                    if commit_condition(crate::mapping_edits::Field::ConditionMax(v.to_string()),
            &mk_max.clone(),
            &sid_max,
            &root_max.clone()) { max_sig.set(v); }
                                                },
                                            }
                                        }
                                    }
        }

        // ---------------------------------------------------------------
        // HatDirection: source-label + rebind + 8-checkbox direction grid.
        // ---------------------------------------------------------------
        Condition::HatDirection { input, directions } => {
            let input_addr = input.clone();
            let dirs = directions.clone();

            // Clones for the rebind callback.
            let mk_in = mapping_key.clone();
            let sid_in = stage_id.clone();
            let root_in = root_actions.clone();
            rsx! {
                                        PredicateInputRow {
                                            target: capture_target.clone(),
                                            input: input_addr.clone(),
                                            on_input_change: move |new_addr: InputAddress| {
                                                commit_condition(crate::mapping_edits::Field::ConditionInput(new_addr),
            &mk_in.clone(),
            &sid_in,
            &root_in.clone());
                                            },
                                        }
                                        div { class: "if-hat-direction-grid",
                                            for (dir, label) in HAT_VARIANTS {
                                                {
                                                    let is_checked = dirs.contains(&dir);
                                                    // Per-checkbox copies -- each loop iteration owns
                                                    // its own clones for its move closure.
                                                    let mk_cb = mapping_key.clone();
                                                    let sid_cb = stage_id.clone();
                                                    let root_cb = root_actions.clone();
                                                    rsx! {
                                                        label { class: "if-hat-direction-grid__cell",
                                                            input {
                                                                r#type: "checkbox",
                                                                checked: is_checked,
                                                                onchange: move |_| {
                                                                    commit_condition(crate::mapping_edits::Field::ConditionDirection(dir),
            &mk_cb.clone(),
            &sid_cb,
            &root_cb.clone());
                                                                },
                                                            }
                                                            "{label}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
        }

        // ---------------------------------------------------------------
        // All / Any: nested-list of sub-condition editors (display-only).
        // ---------------------------------------------------------------
        Condition::All { conditions } | Condition::Any { conditions } => {
            let is_all = matches!(&condition, Condition::All { .. });
            let sub_conditions = conditions.clone();
            let sub_count = sub_conditions.len();

            // Amendment 5: malformed hint when no sub-conditions present.
            // REACTIVE-LOOP CONCERN (Task 40): both branches call
            // malformed_hints.write(), dirtying the Signal. No loop forms
            // because the effect is hoisted above the per-kind match and
            // captures is_empty by value (a plain bool, not a Signal read), so
            // dirtying malformed_hints does not re-trigger this effect. A
            // read-then-compare guard would be more explicit but is not
            // required for correctness here.

            rsx! {
                div { class: "if-predicate__nested-list",
                    for (idx, sub) in sub_conditions.iter().enumerate() {
                        {
                            let sub_cond = sub.clone();
                            let mk = mapping_key.clone();
                            let sid = stage_id.clone();
                            let root = root_actions.clone();
                            let it = if_true.clone();
                            let iff = if_false.clone();
                            rsx! {
                                div {
                                    class: "if-predicate__nested-item",
                                    key: "{idx}",
                                    PredicateEditor {
                                        mapping_key: mk,
                                        stage_id: sid,
                                        condition: sub_cond,
                                        if_true: it,
                                        if_false: iff,
                                        root_actions: root,
                                    }
                                }
                            }
                        }
                    }
                    if sub_count == 0 {
                        span { class: "if-predicate__nested-empty",
                            if is_all {
                                "No conditions (vacuously true)"
                            } else {
                                "No conditions (vacuously false)"
                            }
                        }
                    }
                }
            }
        }

        // ---------------------------------------------------------------
        // Not: single nested editor for the inner condition.
        // ---------------------------------------------------------------
        Condition::Not { condition: inner } => {
            let inner_cond = *inner.clone();
            let mk = mapping_key.clone();
            let sid = stage_id.clone();
            let root = root_actions.clone();
            let it = if_true.clone();
            let iff = if_false.clone();

            rsx! {
                div { class: "if-predicate__nested-list",
                    div { class: "if-predicate__nested-item",
                        PredicateEditor {
                            mapping_key: mk,
                            stage_id: sid,
                            condition: inner_cond,
                            if_true: it,
                            if_false: iff,
                            root_actions: root,
                        }
                    }
                }
            }
        }
    };

    rsx! {
        div { class: "if-predicate",
            // Kind picker: 7 options, changes dispatched immediately.
            Select {
                value: kind_signal,
                options: kind_options,
                onchange: on_kind_change,
            }
            // Per-kind operand UI.
            {operand_ui}
        }
    }
}
