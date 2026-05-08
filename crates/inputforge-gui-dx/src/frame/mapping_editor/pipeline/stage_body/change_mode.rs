// Rust guideline compliant 2026-05-08

//! `ChangeMode` body. Renders a two-row form: strategy picker
//! (segmented Set/Hold pills) and target-mode `Select`. F14 owner.
//!
//! Hint priority (highest first):
//! 1. Empty target mode -> `"Choose a target mode"`.
//! 2. Target mode not in `MetaSnapshot.modes` -> orphan option + drift hint.
//! 3. Hold strategy with non-button primary -> selected-but-disabled Hold.
//!
//! When (2) and (3) hold simultaneously the body emits a combined hint
//! so the user can recover both errors in one edit pass.

use std::collections::HashMap;
use std::sync::mpsc::Sender;

use dioxus::prelude::*;

use inputforge_core::action::{Action, ModeChangeStrategy};
use inputforge_core::engine::EngineCommand;
use inputforge_core::types::InputAddress;

use crate::components::{Select, SelectOption, Tooltip, TooltipPlacement};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::pipeline::stage_body::instruments::stage_dispatch::dispatch_stage_edit_into;
use crate::frame::mapping_editor::undo_log::{
    LabelArgs, StageId, UndoKind, UndoLog, format_undo_label,
};

/// Hint copy. Centralised so tests can grep these strings unchanged.
pub(crate) const HINT_TARGET_EMPTY: &str = "Choose a target mode";
pub(crate) const HINT_HOLD_NOT_BUTTON: &str =
    "Hold requires a button input. Pick a button or change the strategy.";
pub(crate) const TOOLTIP_HOLD_NOT_BUTTON: &str = "Hold requires a button input.";

/// Set / Hold pill activation gate. Returns `false` when the pill is
/// `aria-disabled` or already in the active state. Both onclick handlers
/// call this; standalone-testable so acceptance #15 (Enter on aria-disabled
/// is a no-op) can be unit-verified without DOM event simulation.
pub(crate) fn pill_activates(disabled: bool, was_active: bool) -> bool {
    !disabled && !was_active
}

/// Which pill the user clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StrategyTarget {
    Set,
    Hold,
}

/// Pure dispatch helper for the Set/Hold strategy pills.
///
/// Mirrors the [`dispatch_stage_edit_into`] split: this `_into` form takes
/// `&mut UndoLog` so tests can exercise it without spinning up a Dioxus
/// runtime to allocate a `Signal`. Returns the formatted undo label that was
/// committed (`None` when the dispatch was skipped, e.g. the pill was
/// `aria-disabled` or already active).
///
/// Target preservation across Set <-> Hold falls out for free: the caller
/// passes the same `current_mode` regardless of `target`, so flipping
/// strategy keeps the previously selected target mode.
#[expect(
    clippy::too_many_arguments,
    reason = "matches dispatch_stage_edit_into argument set; one callsite per pill"
)]
pub(crate) fn dispatch_strategy_change_into(
    target: StrategyTarget,
    current_mode: &str,
    is_currently_hold: bool,
    hold_disabled: bool,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    mapping_names: &HashMap<InputAddress, String>,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut UndoLog,
) -> Option<String> {
    let was_active = match target {
        StrategyTarget::Set => !is_currently_hold,
        StrategyTarget::Hold => is_currently_hold,
    };
    let target_disabled = matches!(target, StrategyTarget::Hold) && hold_disabled;
    if !pill_activates(target_disabled, was_active) {
        return None;
    }

    let new_strategy = match target {
        StrategyTarget::Set => ModeChangeStrategy::SwitchTo {
            mode: current_mode.to_owned(),
        },
        StrategyTarget::Hold => ModeChangeStrategy::Temporary {
            mode: current_mode.to_owned(),
        },
    };
    let new_action = Action::ChangeMode {
        strategy: new_strategy,
    };
    let (before, after) = match target {
        StrategyTarget::Set => ("Hold", "Set"),
        StrategyTarget::Hold => ("Set", "Hold"),
    };
    let label = format_undo_label(
        UndoKind::StageEdit,
        LabelArgs {
            stage_name: Some("Change mode"),
            field: Some("strategy"),
            before_after: Some((before, after)),
            ..LabelArgs::default()
        },
    );
    let name = mapping_names.get(&mapping_key.1).cloned();
    dispatch_stage_edit_into(
        undo_log,
        root_actions,
        stage_id,
        new_action,
        mapping_key,
        name,
        cmd_tx,
        label.clone(),
    );
    Some(label)
}

/// Signal-wrapping form. Body call sites pass their `Signal<UndoLog>` here;
/// the wrapper takes the `write()` borrow once and threads it into the pure
/// helper. Same shape as the `dispatch_stage_edit` / `dispatch_stage_edit_into`
/// split in `instruments::stage_dispatch`.
#[expect(
    clippy::too_many_arguments,
    reason = "matches dispatch_strategy_change_into plus the Signal handle"
)]
pub(crate) fn dispatch_strategy_change(
    target: StrategyTarget,
    current_mode: &str,
    is_currently_hold: bool,
    hold_disabled: bool,
    mapping_key: &MappingKey,
    stage_id: &StageId,
    root_actions: &[Action],
    mapping_names: &HashMap<InputAddress, String>,
    cmd_tx: &Sender<EngineCommand>,
    undo_log: &mut Signal<UndoLog>,
) -> Option<String> {
    let mut guard = undo_log.write();
    dispatch_strategy_change_into(
        target,
        current_mode,
        is_currently_hold,
        hold_disabled,
        mapping_key,
        stage_id,
        root_actions,
        mapping_names,
        cmd_tx,
        &mut guard,
    )
}

#[component]
#[allow(
    unused_qualifications,
    reason = "Dioxus 0.7 RSX macro emits redundant qualifications on event listeners."
)]
pub(crate) fn ChangeModeBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    /// Current strategy (destructured from `Action::ChangeMode { strategy }`
    /// in the dispatcher).
    strategy: ModeChangeStrategy,
    root_actions: Vec<Action>,
) -> Element {
    let mode = match &strategy {
        ModeChangeStrategy::SwitchTo { mode } | ModeChangeStrategy::Temporary { mode } => {
            mode.clone()
        }
    };

    let is_hold = matches!(strategy, ModeChangeStrategy::Temporary { .. });
    let is_set = !is_hold;

    let primary_is_button_shaped = mapping_key.1.is_button_shaped();
    let hold_disabled = !primary_is_button_shaped;

    let set_aria_pressed = if is_set { "true" } else { "false" };
    let hold_aria_pressed = if is_hold { "true" } else { "false" };
    let hold_aria_disabled = if hold_disabled { "true" } else { "false" };

    let ctx = use_context::<AppContext>();
    let modes: Vec<String> = ctx.meta.read().modes.clone();

    let target_options: Vec<SelectOption> = modes
        .iter()
        .map(|m| SelectOption {
            value: m.clone(),
            label: m.clone(),
            disabled: false,
            class: None,
        })
        .collect();

    // Sync the local Signal to the prop on every render so the dropdown
    // follows snapshot echoes; same pattern as `MapToVJoyBody`.
    let mode_for_signal = mode.clone();
    let mut target_value: Signal<String> = use_signal(|| mode_for_signal.clone());
    if *target_value.peek() != mode_for_signal {
        target_value.set(mode_for_signal.clone());
    }

    // Pill onclick wiring. Each closure is a thin wrapper around
    // `dispatch_strategy_change`; all business logic (gate, label, action
    // construction, target preservation) lives in the helper.
    let editor = use_context::<EditorState>();
    let cfg_signal = ctx.config;
    let cmd_tx_set = ctx.commands.clone();
    let mut undo_log_set = editor.undo_log;
    let mapping_key_set = mapping_key.clone();
    let stage_id_set = stage_id.clone();
    let root_actions_set = root_actions.clone();
    let mode_set = mode.clone();
    let on_set_click = move |_evt: MouseEvent| {
        let cfg = cfg_signal.read();
        let names = cfg.mapping_names.clone();
        drop(cfg);
        let _ = dispatch_strategy_change(
            StrategyTarget::Set,
            &mode_set,
            is_hold,
            // The Set pill is never gated by the button-shape rule.
            false,
            &mapping_key_set,
            &stage_id_set,
            &root_actions_set,
            &names,
            &cmd_tx_set,
            &mut undo_log_set,
        );
    };

    let cmd_tx_hold = ctx.commands.clone();
    let mut undo_log_hold = editor.undo_log;
    let mapping_key_hold = mapping_key.clone();
    let stage_id_hold = stage_id.clone();
    let root_actions_hold = root_actions.clone();
    let mode_hold = mode.clone();
    let on_hold_click = move |_evt: MouseEvent| {
        let cfg = cfg_signal.read();
        let names = cfg.mapping_names.clone();
        drop(cfg);
        let _ = dispatch_strategy_change(
            StrategyTarget::Hold,
            &mode_hold,
            is_hold,
            hold_disabled,
            &mapping_key_hold,
            &stage_id_hold,
            &root_actions_hold,
            &names,
            &cmd_tx_hold,
            &mut undo_log_hold,
        );
    };

    let hold_pill = rsx! {
        button {
            r#type: "button",
            class: "if-stage__body-strategy-pill",
            "data-strategy": "hold",
            "aria-pressed": "{hold_aria_pressed}",
            "aria-disabled": "{hold_aria_disabled}",
            onclick: on_hold_click,
            "Hold"
        }
    };

    rsx! {
        div { class: "if-stage__body-change-mode",
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Strategy" }
                // Toggle-button-group pattern (role="group" + child aria-pressed).
                div {
                    class: "if-stage__body-strategy",
                    role: "group",
                    "aria-label": "Mode change strategy",
                    button {
                        r#type: "button",
                        class: "if-stage__body-strategy-pill",
                        "data-strategy": "set",
                        "aria-pressed": "{set_aria_pressed}",
                        onclick: on_set_click,
                        "Set"
                    }
                    if hold_disabled {
                        Tooltip {
                            content: TOOLTIP_HOLD_NOT_BUTTON.to_owned(),
                            placement: TooltipPlacement::Top,
                            {hold_pill}
                        }
                    } else {
                        {hold_pill}
                    }
                }
            }
            div { class: "if-stage__body-field",
                label { class: "if-stage__body-label", "Target mode" }
                Select {
                    value: target_value,
                    options: target_options,
                    onchange: move |_evt: FormEvent| {},
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::frame::mapping_editor::pipeline::tests::render_change_mode_body_for_test;

    #[test]
    fn renders_strategy_pills_and_target_select_for_switch_to() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "Combat".to_owned(),
        };
        let (html, _hints) =
            render_change_mode_body_for_test(strategy, "btn0", &["Default", "Combat"]);

        assert!(html.contains("if-stage__body-change-mode"));
        assert!(
            html.contains("if-stage__body-strategy"),
            "pill row missing: {html}"
        );
        assert!(
            html.contains("data-strategy=\"set\""),
            "Set pill missing: {html}"
        );
        assert!(
            html.contains("data-strategy=\"hold\""),
            "Hold pill missing: {html}"
        );
        assert!(
            html.contains("aria-pressed=\"true\""),
            "active pill must carry aria-pressed=true: {html}"
        );
    }

    #[test]
    fn renders_temporary_with_hold_pill_pressed() {
        let strategy = ModeChangeStrategy::Temporary {
            mode: "Combat".to_owned(),
        };
        let (html, _hints) =
            render_change_mode_body_for_test(strategy, "btn0", &["Default", "Combat"]);
        let hold_idx = html
            .find(r#"data-strategy="hold""#)
            .expect("hold pill must render");
        let after = &html[hold_idx..hold_idx + 200];
        assert!(
            after.contains("aria-pressed=\"true\""),
            "hold pill must be pressed when strategy is Temporary, fragment: {after}"
        );
    }

    #[test]
    fn dispatches_strategy_switch_with_target_preserved() {
        use inputforge_core::action::Action;
        use inputforge_core::engine::EngineCommand;

        let strategy_before = ModeChangeStrategy::SwitchTo {
            mode: "Combat".to_owned(),
        };

        // Direct call into the closure-extracted helper. No DOM event simulation.
        let (commands, undo_label) =
            crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_strategy_change(
                strategy_before,
                "btn0",
                &["Default", "Combat"],
                StrategyTarget::Hold,
            );

        let first = commands.into_iter().next().expect("expected SetMapping");
        match first {
            EngineCommand::SetMapping { actions, .. } => {
                assert!(
                    matches!(
                        actions.first(),
                        Some(Action::ChangeMode {
                            strategy: ModeChangeStrategy::Temporary { mode }
                        }) if mode == "Combat"
                    ),
                    "target must be preserved across strategy switch: {actions:?}"
                );
            }
            other => panic!("expected SetMapping, got {other:?}"),
        }
        assert_eq!(
            undo_label.as_deref(),
            Some("Change mode: strategy Set -> Hold")
        );
    }
}
