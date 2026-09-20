use inputforge_core::profile::Profile;

const PROFILE: &str = r#"
modes = ["Default"]
[profile]
id = "fixture"
name = "Linux"
startup_mode = "Default"
[linux]
version = 1
selected = ["evdev:v1:fixture"]
virtual_devices = []
[[linux.bindings]]
device = "evdev:v1:fixture"
buttons = [300, 704]
hats = [0]
[[linux.bindings.axes]]
code = 5
minimum = -10
maximum = 90
polarity = "Unipolar"
"#;

#[test]
fn linux_native_codes_survive_platform_neutral_edit_and_save() {
    let mut profile = Profile::from_toml(PROFILE).unwrap();
    assert!(profile.controllers().unwrap().legacy_axis_settings);
    profile.set_name("Renamed".into());
    let saved = profile.to_toml().unwrap();
    let value: toml::Value = toml::from_str(&saved).unwrap();
    assert_eq!(
        value["controllers"]["bindings"][0]["buttons"][1].as_integer(),
        Some(704)
    );
    assert_eq!(Profile::from_toml(&saved).unwrap().name(), "Renamed");
    assert!(value.get("linux").is_none());
}

#[test]
fn invalid_linux_tables_are_rejected_instead_of_reinterpreted() {
    for (from, to) in [
        ("version = 1", "version = 2"),
        ("[300, 704]", "[300, 300]"),
        ("maximum = 90", "maximum = -10"),
    ] {
        assert!(
            Profile::from_toml(&PROFILE.replace(from, to)).is_err(),
            "{to}"
        );
    }
}
