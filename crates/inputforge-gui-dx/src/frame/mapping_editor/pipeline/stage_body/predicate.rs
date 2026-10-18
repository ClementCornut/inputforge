//! Predicate kind, numeric drafts and validation publication.
use dioxus::prelude::*;
#[path = "predicate_hat.rs"]
mod hat;
#[path = "predicate_input.rs"]
mod input;
use input::commit_condition;
#[path = "predicate_operands.rs"]
mod operands;
#[path = "predicate_range.rs"]
mod range;
use crate::capture::Target;
use crate::components::{Select, SelectOption};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::StageId;
use inputforge_core::action::{Action, Condition};
use inputforge_core::types::HatDirection;
const KIND_BUTTON_PRESSED: &str = "ButtonPressed";
const KIND_BUTTON_RELEASED: &str = "ButtonReleased";
const KIND_AXIS_IN_RANGE: &str = "AxisInRange";
const KIND_HAT_DIRECTION: &str = "HatDirection";
const KIND_ALL: &str = "All";
const KIND_ANY: &str = "Any";
const KIND_NOT: &str = "Not";
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
#[component]
pub(crate) fn PredicateEditor(
    mapping_key: MappingKey,
    stage_id: StageId,
    condition: Condition,
    if_true: Vec<Action>,
    if_false: Vec<Action>,
    root_actions: Vec<Action>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let field_prefix =
        use_hook(|| format!("if-predicate-field-{}", dioxus::core::current_scope_id().0));
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
    let kind_options = [
        (KIND_BUTTON_PRESSED, "Button pressed"),
        (KIND_BUTTON_RELEASED, "Button released"),
        (KIND_AXIS_IN_RANGE, "Axis in range"),
        (KIND_HAT_DIRECTION, "Hat direction"),
        (KIND_ALL, "All"),
        (KIND_ANY, "Any"),
        (KIND_NOT, "Not"),
    ]
    .into_iter()
    .map(|(value, label)| SelectOption {
        value: value.to_owned(),
        label: label.to_owned(),
        disabled: false,
        class: None,
    })
    .collect();
    let current_kind = condition_kind(&condition).to_owned();
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
    let mk_kind = mapping_key.clone();
    let sid_kind = stage_id.clone();
    let root_kind = root_actions.clone();
    let on_kind_change = move |evt: FormEvent| {
        let new_kind = evt.value();
        commit_condition(
            crate::mapping_edits::Field::ConditionKind(new_kind),
            &mk_kind.clone(),
            &sid_kind,
            &root_kind.clone(),
        );
    };
    rsx! {
        div {
            class: "if-predicate",
            Select {
                aria_label: "Predicate kind",
                id: format!("{field_prefix}-kind"),
                value: kind_signal,
                options: kind_options,
                onchange: on_kind_change,
            }
            operands::Operand {
                mapping_key,
                stage_id,
                condition,
                if_true,
                if_false,
                root_actions,
                capture_target,
                min_sig,
                max_sig,
                field_prefix,
            }
        }
    }
}
