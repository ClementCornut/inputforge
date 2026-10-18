// Rust guideline compliant 2026-05-08

//! `ChangeMode` body. Renders a two-row form: strategy picker
//! (segmented Set/Hold pills) and target-mode `Select`. F14 owner.
//!
//! Hint priority (highest first):
//! 1. Empty target mode -> `"Choose a mode to switch to"`.
//! 2. Target mode not in `MetaSnapshot.modes` -> orphan option + drift hint.
//! 3. Hold strategy with non-button primary -> selected-but-disabled Hold.
//!
//! When (2) and (3) hold simultaneously the body emits a combined hint
//! so the user can recover both errors in one edit pass.

use dioxus::prelude::*;
#[path = "change_mode_dispatch.rs"]
mod dispatch;
pub(crate) use dispatch::{
    StrategyTarget, TOOLTIP_HOLD_NOT_BUTTON, dispatch_strategy_change, dispatch_target_change,
};

use inputforge_core::action::{Action, ModeChangeStrategy};

use crate::components::{
    SegmentedControl, SegmentedControlOption, Select, SelectOption, Tooltip, TooltipPlacement,
};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::EditorState;
use crate::frame::mapping_editor::undo_log::StageId;

/// Hint copy. Centralised so tests can grep these strings unchanged.
#[cfg(test)]
use crate::mapping_edits::hints::{
    HOLD_NOT_BUTTON as HINT_HOLD_NOT_BUTTON, TARGET_EMPTY as HINT_TARGET_EMPTY,
};
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

    let ctx = use_context::<AppContext>();
    let editor = use_context::<EditorState>();
    let modes = ctx.meta.read().modes.clone();
    let prepared = use_context::<crate::mapping_edits::Bindings>()
        .ui
        .stage_hint
        .call(crate::mapping_edits::hints::Request {
            path: stage_id.clone(),
            input: crate::mapping_edits::hints::Input::Mode(
                strategy.clone(),
                mapping_key.1.clone(),
                modes.clone(),
            ),
            hints: editor.malformed_hints,
            tags: Some(editor.malformed_summary_tags),
        });
    let hold_disabled = prepared.hold_disabled;
    let target_orphaned = prepared.target_orphaned;
    let hold_tabindex = if hold_disabled { "-1" } else { "0" };

    let mut target_options: Vec<SelectOption> = modes
        .iter()
        .map(|m| SelectOption {
            value: m.clone(),
            label: m.clone(),
            disabled: false,
            class: None,
        })
        .collect();
    if target_orphaned {
        // Prefix the orphan label so the closed Select trigger carries the
        // signal even on Chromium, where native <option> styling (color,
        // italic) only applies inside the open dropdown listbox and not on
        // the trigger's display of the selected option.
        target_options.insert(
            0,
            SelectOption {
                value: mode.clone(),
                label: format!("(removed) {mode}"),
                disabled: true,
                class: Some("if-select__option--orphan".into()),
            },
        );
    }

    // Sync the local Signal to the prop on every render so the dropdown
    // follows snapshot echoes; same pattern as `MapToVJoyBody`.
    let mode_for_signal = mode.clone();
    let mut target_value: Signal<String> = use_signal(|| mode_for_signal.clone());
    if *target_value.peek() != mode_for_signal {
        target_value.set(mode_for_signal.clone());
    }

    // Pill onclick wiring. Each closure is a thin wrapper around
    // `dispatch_strategy_change`; the app validates and reconstructs the action.
    let mapping_key_set = mapping_key.clone();
    let stage_id_set = stage_id.clone();
    let root_actions_set = root_actions.clone();
    let on_set_click = move |_evt: MouseEvent| {
        let _ = dispatch_strategy_change(
            StrategyTarget::Set,
            &mapping_key_set,
            &stage_id_set,
            &root_actions_set,
        );
    };

    let mapping_key_hold = mapping_key.clone();
    let stage_id_hold = stage_id.clone();
    let root_actions_hold = root_actions.clone();
    let on_hold_click = move |_evt: MouseEvent| {
        let _ = dispatch_strategy_change(
            StrategyTarget::Hold,
            &mapping_key_hold,
            &stage_id_hold,
            &root_actions_hold,
        );
    };

    // Target-mode Select onchange. Thin closure: pull `evt.value()` and
    // forward to `dispatch_target_change`. The helper guards the no-op case
    // (`new_mode == current`) so picking the same option twice is silent.
    let mapping_key_t = mapping_key.clone();
    let stage_id_t = stage_id.clone();
    let root_actions_t = root_actions.clone();
    let strategy_for_t = strategy.clone();
    let on_target_change = move |evt: FormEvent| {
        let _ = dispatch_target_change(
            &evt.value(),
            &strategy_for_t,
            &mapping_key_t,
            &stage_id_t,
            &root_actions_t,
        );
    };

    let field_prefix =
        use_hook(|| format!("if-stage-field-{}", dioxus::core::current_scope_id().0));
    let hold_description_id = format!("{field_prefix}-hold-description");
    let hold_pill = rsx! {
        SegmentedControlOption {
            value: "hold".to_owned(),
            data_strategy: Some("hold".to_owned()),
            aria_describedby: hold_disabled.then(|| hold_description_id.clone()),
            selected: is_hold,
            disabled: hold_disabled,
            tabindex: Some(hold_tabindex.to_owned()),
            onclick: on_hold_click,
            "Hold"
        }
    };

    rsx! {
        div { class: "if-stage__body-change-mode",
            div { class: "if-stage__body-field",
                span { class: "if-stage__body-label", "Strategy" }
                SegmentedControl { aria_label: "Mode change strategy".to_owned(),
                    SegmentedControlOption {
                        value: "set".to_owned(),
                        data_strategy: Some("set".to_owned()),
                        selected: is_set,
                        onclick: on_set_click,
                        "Set"
                    }
                    if hold_disabled {
                        Tooltip {
                            id: hold_description_id,
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
                crate::components::Label { for_id: format!("{field_prefix}-mode"), "Target mode" }
                Select {
                    aria_label: "Target mode",
                    id: format!("{field_prefix}-mode"),
                    value: target_value,
                    options: target_options,
                    onchange: on_target_change,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::frame::mapping_editor::pipeline::tests::render_change_mode_body_for_test;
    use crate::frame::mapping_editor::undo_log::StageIdSegment;

    fn hint_for_stage_zero(hints: &HashMap<StageId, String>) -> Option<&str> {
        hints
            .get(&StageId(vec![StageIdSegment::Index(0)]))
            .map(String::as_str)
    }

    #[test]
    fn renders_strategy_pills_and_target_select_for_switch_to() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "Combat".to_owned(),
        };
        let (html, _hints) =
            render_change_mode_body_for_test(strategy, "btn0", &["Default", "Combat"]);

        assert!(html.contains("if-stage__body-change-mode"));
        assert!(
            html.contains("if-segmented-control"),
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

    #[test]
    fn dispatches_target_change_with_unset_before_label() {
        use inputforge_core::action::Action;
        use inputforge_core::engine::EngineCommand;

        let strategy_before = ModeChangeStrategy::SwitchTo {
            mode: String::new(), // empty -> "<unset>"
        };
        let (commands, undo_label) =
            crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_target_change(
                strategy_before,
                "btn0",
                "Combat",
            );
        let first = commands.into_iter().next().expect("expected SetMapping");
        match first {
            EngineCommand::SetMapping { actions, .. } => {
                assert!(matches!(
                    actions.first(),
                    Some(Action::ChangeMode {
                        strategy: ModeChangeStrategy::SwitchTo { mode }
                    }) if mode == "Combat"
                ));
            }
            other => panic!("expected SetMapping, got {other:?}"),
        }
        assert_eq!(
            undo_label.as_deref(),
            Some("Change mode: target <unset> -> Combat")
        );
    }

    #[test]
    fn dispatches_target_change_with_explicit_before_label() {
        let strategy_before = ModeChangeStrategy::SwitchTo {
            mode: "Default".to_owned(),
        };
        let (_commands, undo_label) =
            crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_target_change(
                strategy_before,
                "btn0",
                "Combat",
            );
        assert_eq!(
            undo_label.as_deref(),
            Some("Change mode: target Default -> Combat")
        );
    }

    #[test]
    fn priority_1_hint_when_target_empty() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: String::new(),
        };
        let (_html, hints) =
            render_change_mode_body_for_test(strategy, "btn0", &["Default", "Combat"]);
        assert_eq!(hint_for_stage_zero(&hints), Some(HINT_TARGET_EMPTY));
    }

    #[test]
    fn priority_2_hint_when_target_not_in_modes() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "Ghost".to_owned(),
        };
        let (html, hints) =
            render_change_mode_body_for_test(strategy, "btn0", &["Default", "Combat"]);
        assert_eq!(
            hint_for_stage_zero(&hints),
            Some(r#"Mode "Ghost" no longer exists. Pick a current mode."#)
        );
        let ghost_idx = html
            .find(r#"<option value="Ghost""#)
            .expect("orphan option must render");
        let ghost_slice = &html[ghost_idx..ghost_idx + 200];
        assert!(
            ghost_slice.contains("disabled=true"),
            "orphan option must carry disabled=true: {ghost_slice}"
        );
        assert!(
            ghost_slice.contains("if-select__option--orphan"),
            "orphan option must carry the orphan class: {ghost_slice}"
        );
        assert!(
            ghost_slice.contains("(removed) Ghost"),
            "orphan label must be prefixed with \"(removed) \": {ghost_slice}"
        );
    }

    #[test]
    fn priority_3_hint_when_hold_on_non_button() {
        let strategy = ModeChangeStrategy::Temporary {
            mode: "Combat".to_owned(),
        };
        let (html, hints) =
            render_change_mode_body_for_test(strategy, "axis0", &["Default", "Combat"]);
        assert_eq!(hint_for_stage_zero(&hints), Some(HINT_HOLD_NOT_BUTTON));
        let hold_idx = html
            .find(r#"data-strategy="hold""#)
            .expect("hold pill must render");
        let after = &html[hold_idx..hold_idx + 200];
        assert!(after.contains(r#"aria-pressed="true""#));
        assert!(after.contains(r#"aria-disabled="true""#));
    }

    #[test]
    fn combined_hint_when_orphan_and_hold_disabled() {
        let strategy = ModeChangeStrategy::Temporary {
            mode: "Ghost".to_owned(),
        };
        let (_html, hints) =
            render_change_mode_body_for_test(strategy, "axis0", &["Default", "Combat"]);
        let hint = hint_for_stage_zero(&hints).expect("hint must surface");
        assert!(
            hint.contains("no longer exists") && hint.contains("button input"),
            "combined hint must mention both error conditions: {hint}"
        );
    }

    #[test]
    fn disabled_hold_pill_carries_tabindex_minus_one() {
        // When primary is non-button-shaped, the Hold pill is aria-disabled
        // and must drop out of sequential focus order. The app validation
        // gate already blocks the dispatch; tabindex="-1" hides the visual
        // "live but inert" misimpression for keyboard users.
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "Combat".to_owned(),
        };
        let (html, _hints) =
            render_change_mode_body_for_test(strategy, "axis0", &["Default", "Combat"]);
        let hold_idx = html
            .find(r#"data-strategy="hold""#)
            .expect("Hold pill must render");
        // Search a generous window around the data-strategy anchor for the
        // pill's own tabindex attribute.
        let window = &html[hold_idx.saturating_sub(200)..(hold_idx + 200).min(html.len())];
        assert!(
            window.contains(r#"tabindex="-1""#),
            "disabled Hold pill must carry tabindex=\"-1\": {window}"
        );
    }

    #[test]
    fn enabled_hold_pill_carries_tabindex_zero() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "Combat".to_owned(),
        };
        let (html, _hints) =
            render_change_mode_body_for_test(strategy, "btn0", &["Default", "Combat"]);
        let hold_idx = html
            .find(r#"data-strategy="hold""#)
            .expect("Hold pill must render");
        let window = &html[hold_idx.saturating_sub(200)..(hold_idx + 200).min(html.len())];
        assert!(
            window.contains(r#"tabindex="0""#),
            "enabled Hold pill must carry tabindex=\"0\": {window}"
        );
    }

    #[test]
    fn priority_3_disabled_hold_pill_click_is_noop() {
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "Combat".to_owned(),
        };
        let (commands, _label) =
            crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_strategy_change(
                strategy,
                "axis0",
                StrategyTarget::Hold,
            );
        assert!(
            commands.is_empty(),
            "click on aria-disabled Hold pill must not commit, got {commands:?}"
        );
    }

    #[test]
    fn priority_3_disabled_hold_pill_enter_is_noop() {
        // Native <button> Enter and Space activations route through onclick.
        // Since the dispatch helper calls pill_activates and bails when
        // aria-disabled, the keyboard path is gated by the same predicate as
        // the click path. The tabindex="-1" attribute (set by the parent
        // body) keeps the disabled pill out of sequential focus order, but
        // even if a user reaches it via Shift+Tab focus or programmatic
        // focus, Enter must remain a no-op. This test pins that contract.
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "Combat".to_owned(),
        };
        let (commands, _label) =
            crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_strategy_change(
                strategy,
                "axis0",
                StrategyTarget::Hold,
            );
        assert!(
            commands.is_empty(),
            "Enter on aria-disabled Hold pill must not commit, got {commands:?}"
        );
    }

    #[test]
    fn priority_3_set_pill_migration_preserves_target() {
        use inputforge_core::action::Action;
        use inputforge_core::engine::EngineCommand;
        let strategy = ModeChangeStrategy::Temporary {
            mode: "Combat".to_owned(),
        };
        let (commands, _label) =
            crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_strategy_change(
                strategy,
                "axis0",
                StrategyTarget::Set,
            );
        let first = commands.into_iter().next().expect("expected SetMapping");
        match first {
            EngineCommand::SetMapping { actions, .. } => {
                assert!(
                    matches!(
                        actions.first(),
                        Some(Action::ChangeMode {
                            strategy: ModeChangeStrategy::SwitchTo { mode }
                        }) if mode == "Combat"
                    ),
                    "Set click on selected-but-disabled Hold must migrate to SwitchTo with target preserved"
                );
            }
            other => panic!("expected SetMapping, got {other:?}"),
        }
    }

    #[test]
    fn orphan_pick_dispatches_action_without_orphan_mode() {
        use inputforge_core::action::Action;
        use inputforge_core::engine::EngineCommand;
        let strategy = ModeChangeStrategy::SwitchTo {
            mode: "Ghost".to_owned(),
        };
        let (commands, _label) =
            crate::frame::mapping_editor::pipeline::tests::simulate_dispatch_target_change(
                strategy, "btn0", "Combat",
            );
        let first = commands.into_iter().next().expect("expected SetMapping");
        match first {
            EngineCommand::SetMapping { actions, .. } => {
                assert!(
                    matches!(
                        actions.first(),
                        Some(Action::ChangeMode {
                            strategy: ModeChangeStrategy::SwitchTo { mode }
                        }) if mode == "Combat"
                    ),
                    "persisted action must use the picked mode, not the orphan: {actions:?}"
                );
            }
            other => panic!("expected SetMapping, got {other:?}"),
        }
    }
    #[test]
    fn adoption_disabled_hold_references_its_mounted_tooltip() {
        let hold = scraper::Selector::parse("button[data-strategy='hold']").unwrap();
        let tooltip = scraper::Selector::parse("[role='tooltip']").unwrap();
        for (primary, disabled) in [("axis0", true), ("btn0", false)] {
            let (html, _) = render_change_mode_body_for_test(
                ModeChangeStrategy::SwitchTo {
                    mode: "Default".into(),
                },
                primary,
                &["Default"],
            );
            let document = scraper::Html::parse_fragment(&html);
            let trigger = document.select(&hold).next().expect("native Hold option");
            let description = trigger.value().attr("aria-describedby");
            if disabled {
                let id = description.expect("disabled Hold must describe why it is unavailable");
                let bubble = document
                    .select(&tooltip)
                    .find(|bubble| bubble.value().attr("id") == Some(id))
                    .expect("description must reference the mounted tooltip bubble");
                assert_eq!(bubble.text().collect::<String>(), TOOLTIP_HOLD_NOT_BUTTON);
            } else {
                assert!(
                    description.is_none(),
                    "enabled Hold must omit absent tooltip reference: {html}"
                );
                assert_eq!(document.select(&tooltip).count(), 0);
            }
        }
    }
}
