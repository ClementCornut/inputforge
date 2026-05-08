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

use dioxus::prelude::*;

use inputforge_core::action::{Action, ModeChangeStrategy};

use crate::components::{Select, SelectOption, Tooltip, TooltipPlacement};
use crate::context::AppContext;
use crate::frame::MappingKey;
use crate::frame::mapping_editor::undo_log::StageId;

/// Hint copy. Centralised so tests can grep these strings unchanged.
pub(crate) const HINT_TARGET_EMPTY: &str = "Choose a target mode";
pub(crate) const HINT_HOLD_NOT_BUTTON: &str =
    "Hold requires a button input. Pick a button or change the strategy.";
pub(crate) const TOOLTIP_HOLD_NOT_BUTTON: &str = "Hold requires a button input.";

/// Set / Hold pill activation gate. Returns `false` when the pill is
/// `aria-disabled` or already in the active state. Both onclick handlers
/// call this; standalone-testable so acceptance #15 (Enter on aria-disabled
/// is a no-op) can be unit-verified without DOM event simulation.
#[allow(dead_code, reason = "wired in Task 13")]
pub(crate) fn pill_activates(disabled: bool, was_active: bool) -> bool {
    !disabled && !was_active
}

#[component]
pub(crate) fn ChangeModeBody(
    mapping_key: MappingKey,
    stage_id: StageId,
    /// Current strategy (destructured from `Action::ChangeMode { strategy }`
    /// in the dispatcher).
    strategy: ModeChangeStrategy,
    root_actions: Vec<Action>,
) -> Element {
    // Dispatch handlers are wired in Task 13 (pills) and Task 14 (Select).
    // Until then, `stage_id` and `root_actions` are intentionally unused.
    let _ = (&stage_id, &root_actions);

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

    let hold_pill = rsx! {
        button {
            r#type: "button",
            class: "if-stage__body-strategy-pill",
            "data-strategy": "hold",
            "aria-pressed": "{hold_aria_pressed}",
            "aria-disabled": "{hold_aria_disabled}",
            // onclick wired in Task 13.
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
                        // onclick wired in Task 13.
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
}
