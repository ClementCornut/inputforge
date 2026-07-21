use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use inputforge_core::action::Action;
use inputforge_core::profile::Profile;
use inputforge_core::types::VJoyAxis;
use toml::Value;

struct FixtureCase {
    file_name: &'static str,
    profile_name: &'static str,
    mode_count: usize,
    profile_id: &'static str,
}

const FIXTURES: [FixtureCase; 2] = [
    FixtureCase {
        file_name: "default.toml",
        profile_name: "Default",
        mode_count: 2,
        profile_id: "00000000-0000-0000-0000-000000000001",
    },
    FixtureCase {
        file_name: "star-citizen.toml",
        profile_name: "Star Citizen",
        mode_count: 1,
        profile_id: "00000000-0000-0000-0000-000000000002",
    },
];

const DEVICE_IDS: [&str; 4] = [
    "00000000000000000000000000000001",
    "00000000000000000000000000000002",
    "00000000000000000000000000000003",
    "00000000000000000000000000000004",
];

fn fixture_path(file_name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0_2_0")
        .join(file_name)
}

fn count_map_to_vjoy(actions: &[Action]) -> usize {
    actions
        .iter()
        .map(|action| match action {
            Action::MapToVJoy { .. } => 1,
            Action::Conditional {
                if_true, if_false, ..
            } => count_map_to_vjoy(if_true) + count_map_to_vjoy(if_false),
            Action::TapGesture {
                single_tap,
                double_tap,
                ..
            } => count_map_to_vjoy(single_tap) + count_map_to_vjoy(double_tap),
            Action::PressGesture {
                short_press,
                long_press,
                ..
            } => count_map_to_vjoy(short_press) + count_map_to_vjoy(long_press),
            _ => 0,
        })
        .sum()
}

fn serialized_action_lists(document: &Value) -> Vec<Value> {
    document
        .get("mappings")
        .and_then(Value::as_array)
        .expect("fixture must contain a mappings array")
        .iter()
        .map(|mapping| {
            mapping
                .get("actions")
                .cloned()
                .expect("every mapping must contain actions")
        })
        .collect()
}

fn collect_string_devices(value: &Value, result: &mut BTreeSet<String>) {
    match value {
        Value::Table(table) => {
            for (key, child) in table {
                if key == "device"
                    && let Some(device) = child.as_str()
                {
                    result.insert(device.to_owned());
                }
                collect_string_devices(child, result);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_string_devices(child, result);
            }
        }
        _ => {}
    }
}

#[test]
fn v0_2_0_profiles_round_trip_without_data_loss() {
    let allowed_devices = DEVICE_IDS
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let mut combined_devices = BTreeSet::new();

    for fixture in FIXTURES {
        let path = fixture_path(fixture.file_name);
        let fixture_text = fs::read_to_string(&path).expect("fixture must be readable");
        let fixture_value =
            toml::from_str::<Value>(&fixture_text).expect("fixture must be valid TOML");
        let profile = Profile::load(&path).expect("fixture must load through Profile");

        assert_eq!(profile.name(), fixture.profile_name);
        assert_eq!(profile.id().as_str(), fixture.profile_id);
        assert_eq!(profile.modes().len(), fixture.mode_count);
        assert_eq!(profile.mappings().len(), 77);

        let map_to_vjoy_count = profile
            .mappings()
            .iter()
            .map(|mapping| count_map_to_vjoy(&mapping.actions))
            .sum::<usize>();
        assert_eq!(map_to_vjoy_count, 77);

        let emitted = profile.to_toml().expect("profile must serialize");
        assert!(emitted.contains("type = \"map_to_vjoy\""));
        assert!(!emitted.contains("map_to_virtual_device"));

        let emitted_value =
            toml::from_str::<Value>(&emitted).expect("serialized profile must be valid TOML");

        assert_eq!(
            serialized_action_lists(&fixture_value),
            serialized_action_lists(&emitted_value),
            "{} action sequence changed",
            fixture.file_name
        );
        assert_eq!(
            emitted_value, fixture_value,
            "{} did not round-trip losslessly",
            fixture.file_name
        );

        let mut fixture_devices = BTreeSet::new();
        collect_string_devices(&fixture_value, &mut fixture_devices);
        assert!(fixture_devices.is_subset(&allowed_devices));
        combined_devices.extend(fixture_devices);
    }

    assert_eq!(combined_devices, allowed_devices);
}

#[test]
fn vjoy_axis_wire_spellings_remain_exact() {
    let cases = [
        (VJoyAxis::X, "X"),
        (VJoyAxis::Y, "Y"),
        (VJoyAxis::Z, "Z"),
        (VJoyAxis::Rx, "Rx"),
        (VJoyAxis::Ry, "Ry"),
        (VJoyAxis::Rz, "Rz"),
        (VJoyAxis::Slider0, "Slider0"),
        (VJoyAxis::Slider1, "Slider1"),
    ];

    for (axis, spelling) in cases {
        let encoded = serde_json::to_string(&axis).expect("axis must serialize");
        assert_eq!(encoded, format!("\"{spelling}\""));

        let decoded = serde_json::from_str::<VJoyAxis>(&encoded).expect("axis must deserialize");
        assert_eq!(decoded, axis);
    }
}
