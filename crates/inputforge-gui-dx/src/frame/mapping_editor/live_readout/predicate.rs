//! Renderer label formatting for prepared readout rows.
pub(super) use crate::readouts::labels::render_hat_glyphs;
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::ConfigSnapshot,
        readouts::labels::{format_condition_label, format_predicate_chip_label},
    };
    use inputforge_core::action::Condition;
    use inputforge_core::types::{DeviceId, InputId};
    use inputforge_core::types::{HatDirection, InputAddress};

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

        assert_ne!(label.len(), 0);
    }
}
