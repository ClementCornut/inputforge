// Rust guideline compliant 2026-05-03

//! Human-readable labels for prepared live readout rows.
use inputforge_core::action::Condition;
use inputforge_core::types::{HatDirection, InputAddress};

use crate::context::ConfigSnapshot;
use crate::frame::mapping_list::source_label;

/// Render a condition as a single human label suitable for COND rows.
#[must_use]
pub fn format_condition_label(condition: &Condition, cfg: &ConfigSnapshot) -> String {
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
#[must_use]
pub fn format_predicate_chip_label(input: &InputAddress, cfg: &ConfigSnapshot) -> String {
    source_label::format(input, cfg)
}

/// Render hat directions in canonical compass order.
#[must_use]
pub fn render_hat_glyphs(directions: &[HatDirection]) -> String {
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
