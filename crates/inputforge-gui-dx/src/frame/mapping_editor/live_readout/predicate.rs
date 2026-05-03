// Rust guideline compliant 2026-05-03

//! Label formatting and leaf-state evaluation for conditional predicates.
//!
//! Composite condition evaluation stays in `inputforge_core::pipeline`.
//! This module renders human labels and snapshots per-leaf truth for the
//! live readout predicate chip section.

use inputforge_core::action::Condition;
use inputforge_core::pipeline::InputCache;
use inputforge_core::types::{HatDirection, InputAddress};

use crate::context::ConfigSnapshot;
use crate::frame::mapping_list::source_label;

use super::analyzer::PredicateKind;

/// Render a condition as a single human label suitable for COND rows.
pub(super) fn format_condition_label(condition: &Condition, cfg: &ConfigSnapshot) -> String {
    match condition {
        Condition::ButtonPressed { input } => {
            format!("{} pressed", source_label::format(input, cfg))
        }
        Condition::ButtonReleased { input } => {
            format!("{} released", source_label::format(input, cfg))
        }
        Condition::AxisInRange { input, min, max } => {
            format!(
                "{} in [{:.2}..{:.2}]",
                source_label::format(input, cfg),
                min,
                max
            )
        }
        Condition::HatDirection { input, directions } => {
            let glyphs = render_hat_glyphs(directions);
            format!("{} hat {glyphs}", source_label::format(input, cfg))
        }
        Condition::All { conditions } => {
            if conditions.is_empty() {
                "true".to_owned()
            } else {
                conditions
                    .iter()
                    .map(|condition| format_nested_condition_label(condition, cfg))
                    .collect::<Vec<_>>()
                    .join(" AND ")
            }
        }
        Condition::Any { conditions } => {
            if conditions.is_empty() {
                "false".to_owned()
            } else {
                conditions
                    .iter()
                    .map(|condition| format_nested_condition_label(condition, cfg))
                    .collect::<Vec<_>>()
                    .join(" OR ")
            }
        }
        Condition::Not { condition } => {
            format!("NOT {}", format_nested_condition_label(condition, cfg))
        }
    }
}

fn format_nested_condition_label(condition: &Condition, cfg: &ConfigSnapshot) -> String {
    let label = format_condition_label(condition, cfg);
    if needs_nested_parentheses(condition) {
        format!("({label})")
    } else {
        label
    }
}

fn needs_nested_parentheses(condition: &Condition) -> bool {
    match condition {
        Condition::All { conditions } | Condition::Any { conditions } => !conditions.is_empty(),
        Condition::Not { .. } => true,
        Condition::ButtonPressed { .. }
        | Condition::ButtonReleased { .. }
        | Condition::AxisInRange { .. }
        | Condition::HatDirection { .. } => false,
    }
}

/// Source label for an IN-block predicate chip.
pub(super) fn format_predicate_chip_label(input: &InputAddress, cfg: &ConfigSnapshot) -> String {
    source_label::format(input, cfg)
}

/// Per-leaf state snapshot used by IN-block predicate chip rendering.
pub(super) fn evaluate_leaf_state(
    input: &InputAddress,
    kind: &PredicateKind,
    cache: &dyn InputCache,
) -> bool {
    use inputforge_core::pipeline::evaluate_condition;

    let synthetic = match kind {
        PredicateKind::ButtonPressed => Condition::ButtonPressed {
            input: input.clone(),
        },
        PredicateKind::ButtonReleased => Condition::ButtonReleased {
            input: input.clone(),
        },
        PredicateKind::AxisInRange { min, max } => Condition::AxisInRange {
            input: input.clone(),
            min: *min,
            max: *max,
        },
        PredicateKind::HatDirection { directions } => Condition::HatDirection {
            input: input.clone(),
            directions: directions.clone(),
        },
    };
    evaluate_condition(&synthetic, cache)
}

/// Render hat directions in canonical compass order.
pub(super) fn render_hat_glyphs(directions: &[HatDirection]) -> String {
    const ORDER: &[(HatDirection, char)] = &[
        (HatDirection::N, '\u{2191}'),
        (HatDirection::NE, '\u{2197}'),
        (HatDirection::E, '\u{2192}'),
        (HatDirection::SE, '\u{2198}'),
        (HatDirection::S, '\u{2193}'),
        (HatDirection::SW, '\u{2199}'),
        (HatDirection::W, '\u{2190}'),
        (HatDirection::NW, '\u{2196}'),
        (HatDirection::Center, '\u{00b7}'),
    ];

    ORDER
        .iter()
        .filter(|(direction, _)| directions.contains(direction))
        .map(|(_, glyph)| *glyph)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use inputforge_core::types::{AxisPolarity, AxisValue, DeviceId, InputId, InputValue};

    fn cfg() -> ConfigSnapshot {
        ConfigSnapshot::default()
    }

    fn button(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Button { index },
        }
    }

    fn axis(index: u8) -> InputAddress {
        InputAddress::Bound {
            device: DeviceId("dev-1".to_owned()),
            input: InputId::Axis { index },
        }
    }

    #[test]
    fn render_hat_glyphs_n_only() {
        assert_eq!(render_hat_glyphs(&[HatDirection::N]), "\u{2191}");
    }

    #[test]
    fn render_hat_glyphs_n_ne_e() {
        assert_eq!(
            render_hat_glyphs(&[HatDirection::N, HatDirection::NE, HatDirection::E]),
            "\u{2191}\u{2197}\u{2192}"
        );
    }

    #[test]
    fn render_hat_glyphs_center_only() {
        assert_eq!(render_hat_glyphs(&[HatDirection::Center]), "\u{00b7}");
    }

    #[test]
    fn render_hat_glyphs_order_is_canonical_not_input() {
        assert_eq!(
            render_hat_glyphs(&[HatDirection::E, HatDirection::N]),
            "\u{2191}\u{2192}"
        );
    }

    #[test]
    fn format_condition_label_button_pressed_uses_source_label() {
        let label = format_condition_label(&Condition::ButtonPressed { input: button(0) }, &cfg());

        assert!(label.contains("pressed"));
    }

    #[test]
    fn format_condition_label_axis_range_shows_bounds() {
        let label = format_condition_label(
            &Condition::AxisInRange {
                input: axis(1),
                min: 0.25,
                max: 0.75,
            },
            &cfg(),
        );

        assert!(label.contains("[0.25..0.75]"));
    }

    #[test]
    fn format_condition_label_all_joins_with_and() {
        let condition = Condition::All {
            conditions: vec![
                Condition::ButtonPressed { input: button(0) },
                Condition::ButtonReleased { input: button(1) },
            ],
        };
        let label = format_condition_label(&condition, &cfg());

        assert!(label.contains(" AND "));
    }

    #[test]
    fn format_condition_label_any_joins_with_or() {
        let condition = Condition::Any {
            conditions: vec![
                Condition::ButtonPressed { input: button(0) },
                Condition::ButtonPressed { input: button(1) },
            ],
        };
        let label = format_condition_label(&condition, &cfg());

        assert!(label.contains(" OR "));
    }

    #[test]
    fn format_condition_label_any_parenthesizes_nested_all() {
        let first = Condition::ButtonPressed { input: button(0) };
        let second = Condition::ButtonPressed { input: button(1) };
        let third = Condition::ButtonPressed { input: button(2) };
        let expected = format!(
            "({} AND {}) OR {}",
            format_condition_label(&first, &cfg()),
            format_condition_label(&second, &cfg()),
            format_condition_label(&third, &cfg())
        );
        let condition = Condition::Any {
            conditions: vec![
                Condition::All {
                    conditions: vec![first, second],
                },
                third,
            ],
        };

        let label = format_condition_label(&condition, &cfg());

        assert_eq!(label, expected);
    }

    #[test]
    fn format_condition_label_not_prefixes() {
        let condition = Condition::Not {
            condition: Box::new(Condition::ButtonPressed { input: button(0) }),
        };
        let label = format_condition_label(&condition, &cfg());

        assert!(label.starts_with("NOT "));
    }

    #[test]
    fn format_condition_label_not_parenthesizes_nested_any() {
        let first = Condition::ButtonPressed { input: button(0) };
        let second = Condition::ButtonPressed { input: button(1) };
        let expected = format!(
            "NOT ({} OR {})",
            format_condition_label(&first, &cfg()),
            format_condition_label(&second, &cfg())
        );
        let condition = Condition::Not {
            condition: Box::new(Condition::Any {
                conditions: vec![first, second],
            }),
        };

        let label = format_condition_label(&condition, &cfg());

        assert_eq!(label, expected);
    }

    #[test]
    fn format_condition_label_empty_all_is_true() {
        let label = format_condition_label(&Condition::All { conditions: vec![] }, &cfg());

        assert_eq!(label, "true");
    }

    #[test]
    fn format_condition_label_empty_any_is_false() {
        let label = format_condition_label(&Condition::Any { conditions: vec![] }, &cfg());

        assert_eq!(label, "false");
    }

    #[test]
    fn format_predicate_chip_label_reuses_source_label() {
        let label = format_predicate_chip_label(&button(3), &cfg());

        assert!(!label.is_empty());
    }

    #[test]
    fn evaluate_leaf_state_uses_input_cache() {
        let watched = axis(1);
        let mut state = inputforge_core::state::AppState::new();
        state.input_cache.update(
            &watched,
            &InputValue::Axis {
                value: AxisValue::new(0.5),
                polarity: AxisPolarity::Bipolar,
            },
        );

        assert!(evaluate_leaf_state(
            &watched,
            &PredicateKind::AxisInRange {
                min: 0.25,
                max: 0.75
            },
            &state.input_cache,
        ));
    }
}
