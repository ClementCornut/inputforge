//! Filter logic for the F8 mapping list.
//!
//! Single-substring, case-insensitive. Match domain is `name` (if
//! present) plus the source-label string from `source_label::format`.
//! Spec § "Mapping-list interactions" choice 10: "Reduces visible rows;
//! does not reorder. Empty groups (post-filter) are omitted entirely."

use crate::context::{ConfigSnapshot, MappingSummary};
use crate::frame::mapping_list::source_label;
use inputforge_core::types::DeviceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeviceChip {
    pub id: DeviceId,
    pub label: String,
}

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

pub(crate) fn matches_device_filter(row: &MappingSummary, selected: Option<&DeviceId>) -> bool {
    selected.is_none_or(|device| row.referenced_devices.iter().any(|d| d == device))
}

pub(crate) fn device_chips_for_mode(
    rows: &[MappingSummary],
    mode: &str,
    cfg: &ConfigSnapshot,
) -> Vec<DeviceChip> {
    let mut ids: Vec<DeviceId> = Vec::new();
    for row in rows.iter().filter(|row| row.mode == mode) {
        for device in &row.referenced_devices {
            if !ids.iter().any(|existing| existing == device) {
                ids.push(device.clone());
            }
        }
    }

    let mut chips: Vec<DeviceChip> = ids
        .into_iter()
        .map(|id| {
            let label = cfg.device_display_name(&id);
            DeviceChip { id, label }
        })
        .collect();

    let mut counts = std::collections::HashMap::<String, usize>::new();
    for chip in &chips {
        *counts.entry(chip.label.clone()).or_default() += 1;
    }
    for chip in &mut chips {
        if counts.get(&chip.label).copied().unwrap_or_default() > 1 {
            chip.label = format!("{} · {}", chip.label, chip.id.0);
        }
    }
    // Sort by display label, case-insensitive, after disambiguation so
    // the appended `· {id}` suffix breaks duplicate-name ties
    // deterministically. Insertion order (which equals row scan order)
    // would shift the filter strip whenever rows are reordered.
    chips.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    chips
}

#[cfg(test)]
mod tests {
    use super::*;

    use inputforge_core::state::DeviceState;
    use inputforge_core::types::{
        AxisPolarity, DeviceDiagnostics, DeviceInfo, InputAddress, InputId,
    };

    use crate::context::GlyphFlags;

    fn cfg_with_device() -> ConfigSnapshot {
        let device = DeviceState {
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
            diagnostics: DeviceDiagnostics::default(),
        };
        let device_display_names =
            std::collections::HashMap::from([(device.info.id.clone(), device.info.name.clone())]);
        ConfigSnapshot {
            devices: vec![device],
            device_display_names,
            ..ConfigSnapshot::default()
        }
    }

    fn row_named(name: &str, input: InputId) -> MappingSummary {
        MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("tfm".to_owned()),
                input,
            },
            mode: "Default".to_owned(),
            name: Some(name.to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("tfm".to_owned())],
            first_vjoy_output: None,
        }
    }

    fn cfg_with_named_devices<const N: usize>(devices: [(&str, &str); N]) -> ConfigSnapshot {
        let device_states: Vec<DeviceState> = devices
            .into_iter()
            .map(|(id, name)| DeviceState {
                info: DeviceInfo {
                    id: DeviceId(id.to_owned()),
                    name: name.to_owned(),
                    axes: 1,
                    buttons: 1,
                    hats: 0,
                    instance_path: None,
                    axis_polarities: vec![AxisPolarity::Bipolar],
                },
                connected: true,
                diagnostics: DeviceDiagnostics::default(),
            })
            .collect();
        let device_display_names = device_states
            .iter()
            .map(|d| (d.info.id.clone(), d.info.name.clone()))
            .collect();
        ConfigSnapshot {
            devices: device_states,
            device_display_names,
            ..ConfigSnapshot::default()
        }
    }

    fn row_in_mode_with_refs(mode: &str, name: &str, refs: Vec<&str>) -> MappingSummary {
        MappingSummary {
            input: InputAddress::Bound {
                device: DeviceId("primary".to_owned()),
                input: InputId::Button { index: 0 },
            },
            mode: mode.to_owned(),
            name: Some(name.to_owned()),
            glyphs: GlyphFlags::default(),
            referenced_devices: refs.into_iter().map(|id| DeviceId(id.to_owned())).collect(),
            first_vjoy_output: None,
        }
    }

    fn row_with_refs(name: &str, refs: Vec<&str>) -> MappingSummary {
        row_in_mode_with_refs("Default", name, refs)
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
            input: InputAddress::Bound {
                device: DeviceId("tfm".to_owned()),
                input: InputId::Axis { index: 2 },
            },
            mode: "Default".to_owned(),
            name: None,
            glyphs: GlyphFlags::default(),
            referenced_devices: vec![DeviceId("tfm".to_owned())],
            first_vjoy_output: None,
        };
        assert!(matches_filter(&row, "Z", &cfg));
        assert!(matches_filter(&row, "tfm", &cfg));
    }

    #[test]
    fn device_filter_matches_referenced_devices() {
        let row = row_with_refs("Axis", vec!["dev-a", "dev-b"]);
        assert!(matches_device_filter(
            &row,
            Some(&DeviceId("dev-b".to_owned()))
        ));
        assert!(!matches_device_filter(
            &row,
            Some(&DeviceId("dev-c".to_owned()))
        ));
        assert!(matches_device_filter(&row, None));
    }

    #[test]
    fn device_chips_sort_alphabetically_by_label_after_disambiguation() {
        // Two "Twin Stick" devices in Default mode get disambiguated to
        // "Twin Stick · dev-a" and "Twin Stick · dev-b"; sorting by the
        // disambiguated label puts dev-a before dev-b regardless of
        // first-appearance order in the row scan. The "Pedals" device
        // is referenced only in the "Other" mode, so it must NOT
        // surface in the Default-mode chip set.
        let cfg = cfg_with_named_devices([
            ("dev-a", "Twin Stick"),
            ("dev-b", "Twin Stick"),
            ("dev-c", "Pedals"),
        ]);
        let rows = vec![
            row_in_mode_with_refs("Default", "A", vec!["dev-b"]),
            row_in_mode_with_refs("Other", "Other", vec!["dev-c"]),
            row_in_mode_with_refs("Default", "B", vec!["dev-a"]),
            row_in_mode_with_refs("Default", "C", vec!["dev-b"]),
        ];

        let chips = device_chips_for_mode(&rows, "Default", &cfg);
        assert_eq!(
            chips.iter().map(|c| c.id.0.as_str()).collect::<Vec<_>>(),
            vec!["dev-a", "dev-b"],
            "chips must sort alphabetically by disambiguated label, not by first-seen order",
        );
        assert_ne!(chips[0].label, chips[1].label);
        assert!(
            !chips.iter().any(|c| c.id.0 == "dev-c"),
            "the Other-mode-only device must not surface in the Default chip set: {chips:?}",
        );
    }

    #[test]
    fn device_chips_use_alias_when_display_name_differs_from_hardware_name() {
        // Regression guard: the filter chip label must come from
        // `cfg.device_display_name(...)`, not `info.name`. Build a cfg
        // where `device_display_names` contains aliases distinct from
        // every device's hardware name, then assert chip labels echo
        // the aliases (and exclude both the hardware name and the raw
        // id, which would surface only on a regression).
        let device_states: Vec<DeviceState> = [
            ("dev-a", "Generic HID Joystick"),
            ("dev-b", "Generic HID Joystick"),
        ]
        .into_iter()
        .map(|(id, hardware_name)| DeviceState {
            info: DeviceInfo {
                id: DeviceId(id.to_owned()),
                name: hardware_name.to_owned(),
                axes: 1,
                buttons: 1,
                hats: 0,
                instance_path: None,
                axis_polarities: vec![AxisPolarity::Bipolar],
            },
            connected: true,
            diagnostics: DeviceDiagnostics::default(),
        })
        .collect();
        let device_display_names = std::collections::HashMap::from([
            (DeviceId("dev-a".to_owned()), "Rig Wheel".to_owned()),
            (DeviceId("dev-b".to_owned()), "Track Pedals".to_owned()),
        ]);
        let cfg = ConfigSnapshot {
            devices: device_states,
            device_display_names,
            ..ConfigSnapshot::default()
        };
        let rows = vec![
            row_in_mode_with_refs("Default", "A", vec!["dev-a"]),
            row_in_mode_with_refs("Default", "B", vec!["dev-b"]),
        ];

        let chips = device_chips_for_mode(&rows, "Default", &cfg);
        let labels: Vec<&str> = chips.iter().map(|c| c.label.as_str()).collect();
        assert!(
            labels.contains(&"Rig Wheel"),
            "expected 'Rig Wheel' chip label, got {labels:?}"
        );
        assert!(
            labels.contains(&"Track Pedals"),
            "expected 'Track Pedals' chip label, got {labels:?}"
        );
        for label in &labels {
            assert_ne!(*label, "Generic HID Joystick");
            assert_ne!(*label, "dev-a");
            assert_ne!(*label, "dev-b");
        }
    }
}
