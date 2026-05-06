use crate::context::{ProfileRowOrigin, ProfileRowView};
use crate::frame::profiles::projection::project_profile_rows;

fn sample_profile_rows(active: &str, names: &[&str]) -> Vec<ProfileRowView> {
    names
        .iter()
        .map(|name| ProfileRowView {
            id: (*name).to_owned(),
            name: (*name).to_owned(),
            path_label: format!("C:/Profiles/{name}.toml"),
            is_active: *name == active,
            origin: ProfileRowOrigin::Library,
            can_open: true,
            can_rename: true,
            can_duplicate: true,
            can_reveal: true,
            can_delete: true,
            can_add_to_library: false,
            can_snapshot_now: true,
        })
        .collect()
}

#[test]
fn projection_pins_active_and_sorts_inactive_alphabetically() {
    let rows = sample_profile_rows("Bravo", &["Zulu", "Alpha", "Bravo"]);

    let projected = project_profile_rows(&rows, "Bravo", "");

    assert_eq!(
        projected
            .iter()
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Bravo", "Alpha", "Zulu"]
    );
    assert!(projected[0].is_active);
}

#[test]
fn active_profile_stays_visible_when_filter_does_not_match() {
    let rows = sample_profile_rows("Bravo", &["Zulu", "Alpha", "Bravo"]);

    let projected = project_profile_rows(&rows, "Bravo", "alp");

    assert_eq!(
        projected
            .iter()
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Bravo", "Alpha"]
    );
}
