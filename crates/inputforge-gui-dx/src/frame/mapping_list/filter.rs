//! Filter logic for the F8 mapping list.
//!
//! Single-substring, case-insensitive. Match domain is `name` (if
//! present) plus the source-label string from `source_label::format`.
//! Spec § "Mapping-list interactions" choice 10: "Reduces visible rows;
//! does not reorder. Empty groups (post-filter) are omitted entirely."

use crate::context::{ConfigSnapshot, MappingSummary};
use crate::frame::mapping_list::source_label;

/// Returns `true` if `row` survives the current filter `query`.
///
/// - Empty query (or whitespace-only) -> always `true`.
/// - Otherwise: case-insensitive substring against `name + " " + source_label`.
pub(crate) fn matches_filter(row: &MappingSummary, query: &str, cfg: &ConfigSnapshot) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    let q_lower = q.to_ascii_lowercase();
    let source = source_label::format(&row.input, cfg);
    let mut haystack = String::new();
    if let Some(name) = &row.name {
        haystack.push_str(name);
        haystack.push(' ');
    }
    haystack.push_str(&source);
    haystack.to_ascii_lowercase().contains(&q_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{AxisPolarity, DeviceId, DeviceInfo, InputAddress, InputId};

    use crate::context::GlyphFlags;

    fn cfg_with_device() -> ConfigSnapshot {
        ConfigSnapshot {
            devices: vec![DeviceState {
                info: DeviceInfo {
                    id: DeviceId("tfm".to_owned()),
                    name: "TFM Throttle".to_owned(),
                    axes: 4,
                    buttons: 32,
                    hats: 1,
                    instance_path: None,
                    axis_polarities: vec![AxisPolarity::Bipolar; 4],
                },
                connected: true,
            }],
            ..ConfigSnapshot::default()
        }
    }

    fn row_named(name: &str, input: InputId) -> MappingSummary {
        MappingSummary {
            input: InputAddress {
                device: DeviceId("tfm".to_owned()),
                input,
            },
            mode: "Default".to_owned(),
            name: Some(name.to_owned()),
            glyphs: GlyphFlags::default(),
        }
    }

    #[test]
    fn empty_query_matches_everything() {
        let cfg = cfg_with_device();
        let row = row_named("Boost", InputId::Button { index: 0 });
        assert!(matches_filter(&row, "", &cfg));
        assert!(matches_filter(&row, "   ", &cfg));
    }

    #[test]
    fn matches_name_case_insensitive() {
        let cfg = cfg_with_device();
        let row = row_named("Boost", InputId::Button { index: 0 });
        assert!(matches_filter(&row, "boost", &cfg));
        assert!(matches_filter(&row, "BOOST", &cfg));
        assert!(matches_filter(&row, "oo", &cfg));
    }

    #[test]
    fn matches_source_label() {
        let cfg = cfg_with_device();
        let row = row_named("Boost", InputId::Button { index: 0 });
        assert!(matches_filter(&row, "throttle", &cfg));
        assert!(matches_filter(&row, "Btn 1", &cfg));
    }

    #[test]
    fn no_match_returns_false() {
        let cfg = cfg_with_device();
        let row = row_named("Boost", InputId::Button { index: 0 });
        assert!(!matches_filter(&row, "ailerons", &cfg));
    }

    #[test]
    fn unnamed_row_matches_on_source_only() {
        let cfg = cfg_with_device();
        let row = MappingSummary {
            input: InputAddress {
                device: DeviceId("tfm".to_owned()),
                input: InputId::Axis { index: 2 },
            },
            mode: "Default".to_owned(),
            name: None,
            glyphs: GlyphFlags::default(),
        };
        assert!(matches_filter(&row, "Z", &cfg));
        assert!(matches_filter(&row, "tfm", &cfg));
    }
}
