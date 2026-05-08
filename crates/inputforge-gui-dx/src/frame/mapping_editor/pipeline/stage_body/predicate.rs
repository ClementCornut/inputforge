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
//! - `cmd_tx: mpsc::Sender<EngineCommand>` (cloned once per handler group)
//! - `cfg: Signal<ConfigSnapshot>` (`Copy`, safe to copy into every closure)
//!
//! `EditorState` IS `Copy` (all `Signal` fields) and is copied directly.
//! `MappingKey`, `StageId`, and `Vec<Action>` are `Clone`; each closure
//! clones them inside the body so the closure stays `FnMut`.
//!
//! # Amendments applied
//!
//! 1. Name preservation: current name read from `cfg.mapping_names`.
//! 2. Dispatch-before-undo: `push_edit` called only after `cmd.send` succeeds.
//! 3. `LiveCapture.session` ownership pattern (same as `MergeAxisBody`).
//! 4. Malformed-hint write for `AxisInRange` (min > max), `HatDirection`
//!    (empty directions), `All` / `Any` (zero conditions).

use std::sync::mpsc;

use dioxus::prelude::*;

use inputforge_core::action::{Action, Condition, Mapping};
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::{HatDirection, InputAddress};

use crate::components::{NumberInput, Select, SelectOption};
use crate::context::{AppContext, ConfigSnapshot};
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::replace_at_path;
use crate::frame::mapping_editor::undo_log::{LabelArgs, StageId, UndoKind, format_undo_label};
use crate::frame::mapping_list::source_label;
use crate::patterns::live_capture::{
    CAPTURE_PROMPT, CaptureFilter, LiveCapture, is_current_capture_session, rebind_composite_class,
};

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

/// Malformed-hint message shown when a leaf predicate's `input` is
/// [`InputAddress::Unbound`].
///
/// Surfaced by [`PredicateEditor`] for every leaf kind
/// (`ButtonPressed` / `ButtonReleased` / `AxisInRange` / `HatDirection`).
/// Takes priority over per-kind hints (inverted axis range, empty hat
/// directions) because no other validation is meaningful when there is no
/// input bound yet.
const HINT_PREDICATE_UNBOUND: &str = "Bind an input to complete this condition";

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

/// Build a default-shaped `Condition` for the given kind string, preserving
/// the `input` address where applicable so a kind-switch does not lose the
/// user's already-configured device/input.
///
/// When `prev_input` is `None`, the new leaf condition is seeded with
/// `InputAddress::Unbound` so the row renders the explicit `Unbound`
/// placeholder until the user picks an input. Previously a `Bound` sentinel
/// (empty device + button 0) silently rendered as `Btn 1` and looked like a
/// real binding the user had not chosen.
fn default_condition_for_kind(kind: &str, prev_input: Option<InputAddress>) -> Condition {
    let addr = prev_input.unwrap_or(InputAddress::Unbound);
    match kind {
        KIND_BUTTON_PRESSED => Condition::ButtonPressed { input: addr },
        KIND_BUTTON_RELEASED => Condition::ButtonReleased { input: addr },
        KIND_AXIS_IN_RANGE => Condition::AxisInRange {
            input: addr,
            min: -1.0,
            max: 1.0,
        },
        KIND_HAT_DIRECTION => Condition::HatDirection {
            input: addr,
            directions: vec![],
        },
        KIND_ALL => Condition::All { conditions: vec![] },
        KIND_ANY => Condition::Any { conditions: vec![] },
        // Default `Not` wraps a ButtonPressed so the inner editor is always valid.
        KIND_NOT => Condition::Not {
            condition: Box::new(Condition::ButtonPressed { input: addr }),
        },
        // Unrecognised kind (defensive): same shape as ButtonPressed.
        // The wildcard arm is intentional -- it provides a safe default
        // for any future kind strings that arrive before the picker is
        // updated, rather than panicking or producing an invalid state.
        #[expect(
            clippy::match_same_arms,
            reason = "wildcard is a defensive fallback; collapsing it into the \
                      ButtonPressed arm would obscure the intent"
        )]
        _ => Condition::ButtonPressed { input: addr },
    }
}

/// Extract the `InputAddress` from any leaf condition, if present.
///
/// Returns `None` for both branch conditions (`All` / `Any` / `Not`) and for
/// leaf conditions whose `input` is `Unbound`. The latter is intentional:
/// `default_condition_for_kind` calls this to seed a new condition's input
/// from the previous one, and propagating an `Unbound` sentinel back through
/// would defeat the explicit `Unbound` default.
fn condition_input(c: &Condition) -> Option<InputAddress> {
    match c {
        Condition::ButtonPressed { input }
        | Condition::ButtonReleased { input }
        | Condition::AxisInRange { input, .. }
        | Condition::HatDirection { input, .. } => match input {
            InputAddress::Bound { .. } => Some(input.clone()),
            InputAddress::Unbound => None,
        },
        Condition::All { .. } | Condition::Any { .. } | Condition::Not { .. } => None,
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

/// Commit a new `Condition` to the engine by rebuilding the outer
/// `Action::Conditional` at `stage_id`, dispatching `SetMapping`, and on
/// success pushing an undo entry.
///
/// Takes `cmd_tx` and `cfg` instead of a full `AppContext` so that `move`
/// closures can stay `FnMut` (neither `mpsc::Sender` nor `Signal` causes a
/// `FnOnce` degradation when cloned/copied into repeated calls).
/// `EditorState` is taken by value; it is `Copy`.
#[expect(
    clippy::too_many_arguments,
    reason = "all parameters carry necessary context for a SetMapping dispatch; \
              extracting a struct would add boilerplate without clarity gain"
)]
fn commit_condition(
    new_condition: Condition,
    mapping_key: MappingKey,
    stage_id: &StageId,
    root_actions: Vec<Action>,
    if_true: Vec<Action>,
    if_false: Vec<Action>,
    cmd_tx: &mpsc::Sender<EngineCommand>,
    cfg: Signal<ConfigSnapshot>,
    mut editor: EditorState,
) {
    let current_name = cfg.read().mapping_names.get(&mapping_key.1).cloned();

    // Re-assemble the Conditional with the updated condition but unchanged branches.
    let new_action = Action::Conditional {
        condition: new_condition,
        if_true,
        if_false,
    };

    let Some(new_actions) = replace_at_path(&root_actions, stage_id, new_action) else {
        // Invalid path -- skip to avoid phantom undo entry.
        return;
    };

    let before = Mapping {
        input: mapping_key.1.clone(),
        mode: mapping_key.0.clone(),
        name: current_name.clone(),
        actions: root_actions,
    };

    // Amendment 2: dispatch first; push undo only on success.
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
            action = "predicate_edit_drop_offline",
            "predicate edit dropped: engine channel disconnected"
        );
        return;
    }

    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Conditional"),
            field: Some("condition"),
            ..LabelArgs::default()
        },
    );
    editor
        .undo_log
        .write()
        .push_edit(mapping_key, before, UndoKind::StageEdit, label);
}

// ---------------------------------------------------------------------------
// PredicateInputRow -- shared source-label + rebind button sub-component.
// ---------------------------------------------------------------------------

/// Source label + rebind action for the `input` field of a condition.
///
/// Renders the shared `if-rebind-composite` cluster (same primitive used by
/// the editor header subtitle and the `MergeAxis` Secondary input row) so the
/// rebind affordance reads identically across all three call sites. Follows
/// the session-ownership pattern from `MergeAxisBody`: the row only reacts
/// when its stored `LiveCapture.session` still matches the current session.
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
) -> Element {
    let capture = use_context::<LiveCapture>();
    let ctx = use_context::<AppContext>();

    let mut armed_session: Signal<Option<u64>> = use_signal(|| None);

    // Watch `capture.captured`: when we armed it, forward the new address.
    let input_for_effect = input.clone();
    use_effect(move || {
        let captured_addr = capture.captured.read().clone();
        if !is_current_capture_session(*armed_session.peek(), *capture.session.peek()) {
            return;
        }
        let Some(new_addr) = captured_addr else {
            return;
        };

        // Disarm before calling the handler to prevent self-fire on re-render.
        armed_session.set(None);
        let mut cap = capture.captured;
        cap.set(None);

        // Skip no-op captures (same address re-confirmed).
        if new_addr == input_for_effect {
            return;
        }

        on_input_change.call(new_addr);
    });

    // External-cancel / supersede watcher. `active=false` handles Esc/cancel;
    // `session` mismatch handles another capture surface starting while the
    // global capture remains active.
    use_effect(move || {
        let active_now = *capture.active.read();
        let current_session = *capture.session.read();
        let owned_session = *armed_session.peek();
        if owned_session.is_none() {
            return;
        }
        if capture.captured.peek().is_some() {
            return;
        }
        if active_now && is_current_capture_session(owned_session, current_session) {
            return;
        }
        armed_session.set(None);
    });

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

    let on_rebind = move |_: MouseEvent| {
        capture.start.call(CaptureFilter::Any);
        armed_session.set(Some(*capture.session.peek()));
    };

    let on_cancel = move |_: MouseEvent| {
        capture.cancel.call(());
        armed_session.set(None);
    };

    let is_listening = is_current_capture_session(*armed_session.read(), *capture.session.read());

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

    // Extract the two context fields used by commit_condition.
    // cmd_tx is cloned once here; cfg is Copy.
    let cmd_tx = ctx.commands.clone();
    let cfg = ctx.config;

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

    // Capture clones for the kind-change handler.
    let mk_kind = mapping_key.clone();
    let sid_kind = stage_id.clone();
    let root_kind = root_actions.clone();
    let it_kind = if_true.clone();
    let iff_kind = if_false.clone();
    let cond_for_kind = condition.clone();
    let cmd_tx_kind = cmd_tx.clone();

    // FnMut: clones mk_kind / sid_kind / root_kind etc. on each call.
    let on_kind_change = move |evt: FormEvent| {
        let new_kind = evt.value();
        let prev_input = condition_input(&cond_for_kind);
        let new_condition = default_condition_for_kind(&new_kind, prev_input);
        commit_condition(
            new_condition,
            mk_kind.clone(),
            &sid_kind,
            root_kind.clone(),
            it_kind.clone(),
            iff_kind.clone(),
            &cmd_tx_kind,
            cfg,
            editor,
        );
    };

    // Render per-kind operand UI.
    let operand_ui = match &condition {
        // ---------------------------------------------------------------
        // ButtonPressed / ButtonReleased: source-label + rebind button.
        // ---------------------------------------------------------------
        Condition::ButtonPressed { input } | Condition::ButtonReleased { input } => {
            // Task 9: malformed-hint write / clear on every render. These
            // two leaf kinds have no per-kind validation hint of their own,
            // so the only condition that can flip the hint is the input
            // being `Unbound`. Render-time write so SSR observes it.
            //
            // REACTIVE-LOOP CONCERN (Task 40): the write value is derived
            // solely from the `input` prop, which does not originate from
            // malformed_hints, so no loop forms.
            {
                let mut malformed_hints = editor.malformed_hints;
                if input.is_unbound() {
                    malformed_hints
                        .write()
                        .insert(stage_id.clone(), HINT_PREDICATE_UNBOUND.to_owned());
                } else {
                    malformed_hints.write().remove(&stage_id);
                }
            }

            let input_clone = input.clone();
            let mk = mapping_key.clone();
            let sid = stage_id.clone();
            let root = root_actions.clone();
            let it = if_true.clone();
            let iff = if_false.clone();
            let cond_op = condition.clone();
            let cmd_tx_op = cmd_tx.clone();

            rsx! {
                PredicateInputRow {
                    input: input_clone,
                    on_input_change: move |new_addr: InputAddress| {
                        let new_condition = match &cond_op {
                            Condition::ButtonPressed { .. } => {
                                Condition::ButtonPressed { input: new_addr }
                            }
                            Condition::ButtonReleased { .. } => {
                                Condition::ButtonReleased { input: new_addr }
                            }
                            // Defensive: this arm is only reached for the two button
                            // kinds, but the match must be exhaustive.
                            other => other.clone(),
                        };
                        commit_condition(
                            new_condition,
                            mk.clone(),
                            &sid,
                            root.clone(),
                            it.clone(),
                            iff.clone(),
                            &cmd_tx_op,
                            cfg,
                            editor,
                        );
                    },
                }
            }
        }

        // ---------------------------------------------------------------
        // AxisInRange: source-label + rebind + min/max NumberInput pair.
        // ---------------------------------------------------------------
        Condition::AxisInRange { input, min, max } => {
            let input_addr = input.clone();
            let min_val = *min;
            let max_val = *max;

            // Local Signals drive NumberInput display; committed on step.
            let mut min_sig: Signal<f64> = use_signal(move || min_val);
            let mut max_sig: Signal<f64> = use_signal(move || max_val);

            // Task 9 + Amendment 5: malformed-hint write / clear on every
            // render. Priority: Unbound > inverted-range. The hint is
            // written during the render phase (matching `MapToVJoyBody` and
            // `MapToKeyboardBody`) so SSR observes it and the user sees the
            // guidance the same frame the condition becomes invalid.
            //
            // REACTIVE-LOOP CONCERN (Task 40): both branches call
            // malformed_hints.write(), dirtying the Signal. No loop forms
            // because the write value is derived from the `input` prop and
            // local min/max signals, none of which originate from
            // malformed_hints. A read-then-compare guard would be more
            // explicit but is not required for correctness here.
            {
                let mut malformed_hints = editor.malformed_hints;
                if input.is_unbound() {
                    malformed_hints
                        .write()
                        .insert(stage_id.clone(), HINT_PREDICATE_UNBOUND.to_owned());
                } else {
                    let lo = *min_sig.read();
                    let hi = *max_sig.read();
                    if lo > hi {
                        malformed_hints
                            .write()
                            .insert(stage_id.clone(), "min must not exceed max".to_owned());
                    } else {
                        malformed_hints.write().remove(&stage_id);
                    }
                }
            }

            // Clones for the PredicateInputRow rebind callback.
            let mk_in = mapping_key.clone();
            let sid_in = stage_id.clone();
            let root_in = root_actions.clone();
            let it_in = if_true.clone();
            let iff_in = if_false.clone();
            let addr_in = input_addr.clone();
            let cmd_tx_in = cmd_tx.clone();

            // Clones for the min onstep callback.
            let mk_min = mapping_key.clone();
            let sid_min = stage_id.clone();
            let root_min = root_actions.clone();
            let it_min = if_true.clone();
            let iff_min = if_false.clone();
            let addr_min = input_addr.clone();
            let cmd_tx_min = cmd_tx.clone();

            // Clones for the max onstep callback.
            let mk_max = mapping_key.clone();
            let sid_max = stage_id.clone();
            let root_max = root_actions.clone();
            let it_max = if_true.clone();
            let iff_max = if_false.clone();
            let addr_max = input_addr.clone();
            let cmd_tx_max = cmd_tx.clone();

            rsx! {
                PredicateInputRow {
                    input: addr_in,
                    on_input_change: move |new_addr: InputAddress| {
                        commit_condition(
                            Condition::AxisInRange {
                                input: new_addr,
                                min: *min_sig.peek(),
                                max: *max_sig.peek(),
                            },
                            mk_in.clone(),
                            &sid_in,
                            root_in.clone(),
                            it_in.clone(),
                            iff_in.clone(),
                            &cmd_tx_in,
                            cfg,
                            editor,
                        );
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
                            if let Ok(v) = evt.value().parse::<f64>() {
                                min_sig.set(v);
                            }
                        },
                        // onstep dispatches immediately (one click = one dispatch).
                        onstep: move |v: f64| {
                            min_sig.set(v);
                            commit_condition(
                                Condition::AxisInRange {
                                    input: addr_min.clone(),
                                    min: v,
                                    max: *max_sig.peek(),
                                },
                                mk_min.clone(),
                                &sid_min,
                                root_min.clone(),
                                it_min.clone(),
                                iff_min.clone(),
                                &cmd_tx_min,
                                cfg,
                                editor,
                            );
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
                            if let Ok(v) = evt.value().parse::<f64>() {
                                max_sig.set(v);
                            }
                        },
                        onstep: move |v: f64| {
                            max_sig.set(v);
                            commit_condition(
                                Condition::AxisInRange {
                                    input: addr_max.clone(),
                                    min: *min_sig.peek(),
                                    max: v,
                                },
                                mk_max.clone(),
                                &sid_max,
                                root_max.clone(),
                                it_max.clone(),
                                iff_max.clone(),
                                &cmd_tx_max,
                                cfg,
                                editor,
                            );
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

            // Task 9 + Amendment 5: malformed-hint write / clear on every
            // render. Priority: Unbound > empty-directions. The hint is
            // written during the render phase (matching `MapToVJoyBody` and
            // `MapToKeyboardBody`) so SSR observes it and the user sees the
            // guidance the same frame the condition becomes invalid.
            //
            // REACTIVE-LOOP CONCERN (Task 40): both branches call
            // malformed_hints.write(), dirtying the Signal. No loop forms
            // because the write value is derived from the `input` prop and
            // the `directions` Vec value, neither of which originates from
            // malformed_hints. A read-then-compare guard would be more
            // explicit but is not required for correctness here.
            {
                let mut malformed_hints = editor.malformed_hints;
                if input.is_unbound() {
                    malformed_hints
                        .write()
                        .insert(stage_id.clone(), HINT_PREDICATE_UNBOUND.to_owned());
                } else if dirs.is_empty() {
                    malformed_hints.write().insert(
                        stage_id.clone(),
                        "at least one direction must be selected".to_owned(),
                    );
                } else {
                    malformed_hints.write().remove(&stage_id);
                }
            }

            // Clones for the rebind callback.
            let mk_in = mapping_key.clone();
            let sid_in = stage_id.clone();
            let root_in = root_actions.clone();
            let it_in = if_true.clone();
            let iff_in = if_false.clone();
            let dirs_in = dirs.clone();
            let cmd_tx_in = cmd_tx.clone();

            rsx! {
                PredicateInputRow {
                    input: input_addr.clone(),
                    on_input_change: move |new_addr: InputAddress| {
                        commit_condition(
                            Condition::HatDirection {
                                input: new_addr,
                                directions: dirs_in.clone(),
                            },
                            mk_in.clone(),
                            &sid_in,
                            root_in.clone(),
                            it_in.clone(),
                            iff_in.clone(),
                            &cmd_tx_in,
                            cfg,
                            editor,
                        );
                    },
                }
                div { class: "if-hat-direction-grid",
                    for (dir, label) in HAT_VARIANTS {
                        {
                            let is_checked = dirs.contains(&dir);
                            // Per-checkbox copies -- each loop iteration owns
                            // its own clones for its move closure.
                            let dirs_cb = dirs.clone();
                            let mk_cb = mapping_key.clone();
                            let sid_cb = stage_id.clone();
                            let root_cb = root_actions.clone();
                            let it_cb = if_true.clone();
                            let iff_cb = if_false.clone();
                            let addr_cb = input_addr.clone();
                            let cmd_cb = cmd_tx.clone();
                            rsx! {
                                label { class: "if-hat-direction-grid__cell",
                                    input {
                                        r#type: "checkbox",
                                        checked: is_checked,
                                        onchange: move |_| {
                                            // Toggle this direction in the set.
                                            let mut new_dirs = dirs_cb.clone();
                                            if new_dirs.contains(&dir) {
                                                new_dirs.retain(|d| *d != dir);
                                            } else {
                                                new_dirs.push(dir);
                                            }
                                            commit_condition(
                                                Condition::HatDirection {
                                                    input: addr_cb.clone(),
                                                    directions: new_dirs,
                                                },
                                                mk_cb.clone(),
                                                &sid_cb,
                                                root_cb.clone(),
                                                it_cb.clone(),
                                                iff_cb.clone(),
                                                &cmd_cb,
                                                cfg,
                                                editor,
                                            );
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
            // because the effect captures is_empty by value (a plain bool,
            // not a Signal read), so dirtying malformed_hints does not
            // re-trigger this effect. A read-then-compare guard would be more
            // explicit but is not required for correctness here.
            let sid_hint = stage_id.clone();
            let is_empty = sub_count == 0;
            let mut malformed_hints = editor.malformed_hints;
            use_effect(move || {
                if is_empty {
                    malformed_hints.write().insert(
                        sid_hint.clone(),
                        "at least one condition is required".to_owned(),
                    );
                } else {
                    malformed_hints.write().remove(&sid_hint);
                }
            });

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
