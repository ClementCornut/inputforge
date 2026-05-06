use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use dioxus::prelude::*;
use dioxus_ssr::render;
use parking_lot::RwLock;

use inputforge_core::engine::EngineCommand;
use inputforge_core::mode::ModeTree;
use inputforge_core::profile::Profile;
use inputforge_core::settings::AppSettings;
use inputforge_core::snapshot::{SnapshotConfig, SnapshotId, SnapshotKind, create};
use inputforge_core::state::AppState;

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::context::{ProfileRowOrigin, ProfileRowView, SnapshotRowView};
use crate::frame::layout::EmptyState;
use crate::frame::profiles::ProfilesPanel;
use crate::frame::profiles::actions::{
    ConfirmationKind, ToastAction, profile_delete_action, snapshot_delete_action,
};
use crate::frame::profiles::new_profile::{
    NewProfileSource, add_external_to_library_command, create_new_profile_command,
    open_file_load_once_command,
};
use crate::frame::profiles::projection::project_profile_rows;
use crate::frame::profiles::snapshot_drawer::{
    FocusScope, SnapshotDrawer, should_handle_snapshot_shortcut,
};

fn simple_profile(name: &str) -> Profile {
    let map = HashMap::from([("Default".to_owned(), vec![])]);
    let modes = ModeTree::from_adjacency(&map).unwrap();
    Profile::new(
        name.to_owned(),
        vec![],
        modes,
        vec![],
        vec![],
        "Default".to_owned(),
    )
}

fn sample_profiles_context() -> AppState {
    AppState::with_profile(simple_profile("Bravo"))
}

#[component]
fn ProfilesHarness() -> Element {
    let state = Arc::new(RwLock::new(sample_profiles_context()));
    let (commands, _rx) = mpsc::channel();
    let meta = use_signal(MetaSnapshot::default);
    let config = use_signal(ConfigSnapshot::default);
    let live = use_signal(LiveSnapshot::default);
    use_context_provider(|| AppContext {
        state,
        commands,
        settings: Arc::new(AppSettings::default()),
        meta,
        config,
        live,
    });
    rsx! { ProfilesPanel {} }
}

#[component]
fn EmptyHarness() -> Element {
    rsx! { EmptyState {} }
}

fn render_profiles_panel(state: AppState) -> String {
    let _state = state;
    let mut vdom = VirtualDom::new(ProfilesHarness);
    vdom.rebuild_in_place();
    render(&vdom)
}

fn render_no_profile_frame() -> String {
    let mut vdom = VirtualDom::new(EmptyHarness);
    vdom.rebuild_in_place();
    render(&vdom)
}

fn sample_snapshot_context() -> Vec<SnapshotRowView> {
    vec![SnapshotRowView {
        id: sample_snapshot_id().to_string(),
        kind_label: "Manual".to_owned(),
        label: Some("Before trim".to_owned()),
        time_label: "2026-05-06T20:00:00Z".to_owned(),
        sort_key: 1,
        pinned: true,
    }]
}

#[component]
fn SnapshotDrawerHarness(rows: Vec<SnapshotRowView>, open: bool) -> Element {
    rsx! {
        SnapshotDrawer {
            active_profile_name: "Bravo".to_owned(),
            rows,
            open,
        }
    }
}

fn render_snapshot_drawer(rows: Vec<SnapshotRowView>, open: bool) -> String {
    let mut vdom = VirtualDom::new_with_props(
        SnapshotDrawerHarness,
        SnapshotDrawerHarnessProps { rows, open },
    );
    vdom.rebuild_in_place();
    render(&vdom)
}

fn sample_snapshot_id() -> SnapshotId {
    let dir = std::env::temp_dir().join(format!(
        "inputforge-gui-snapshot-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("profile.toml");
    simple_profile("Snapshot").save(&path).unwrap();
    let id = create(
        &path,
        SnapshotKind::Manual,
        None,
        &SnapshotConfig::default(),
    )
    .unwrap()
    .unwrap()
    .id;
    let _ = std::fs::remove_dir_all(&dir);
    id
}

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

#[test]
fn profiles_panel_replaces_placeholder_copy() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("data-testid=\"profile-library\""));
    assert!(!html.contains("Placeholder"));
}

#[test]
fn no_profile_state_shows_center_explanation_and_panel_actions() {
    let html = render_no_profile_frame();

    assert!(html.contains("No profile loaded"));
    assert!(html.contains("New profile"));
    assert!(html.contains("Open file"));
    assert!(!html.contains("mapping-list"));
}

#[test]
fn profile_delete_action_dispatches_real_engine_command() {
    let action = profile_delete_action("Alpha");

    assert_eq!(
        action.command,
        EngineCommand::DeleteProfile {
            name: "Alpha".to_owned()
        }
    );
    assert_eq!(action.confirmation, Some(ConfirmationKind::DestructiveF4));
}

#[test]
fn snapshot_delete_action_dispatches_real_engine_command_and_undo_toast() {
    let id = sample_snapshot_id();
    let action = snapshot_delete_action(id);

    assert_eq!(action.command, EngineCommand::DeleteSnapshot { id });
    assert_eq!(
        action.toast_action,
        Some(ToastAction::UndoSnapshotDelete { id })
    );
}

#[test]
fn new_blank_profile_dispatches_create_profile() {
    let command = create_new_profile_command(NewProfileSource::Blank, "Alpha", None).unwrap();

    assert_eq!(
        command,
        EngineCommand::CreateProfile {
            name: "Alpha".to_owned()
        }
    );
}

#[test]
fn open_file_load_once_dispatches_external_load() {
    let path = PathBuf::from("E:/Profiles/external.toml");
    let command = open_file_load_once_command(path.clone());

    assert_eq!(command, EngineCommand::LoadExternalProfileOnce(path));
}

#[test]
fn add_external_to_library_dispatches_import_command() {
    let path = PathBuf::from("E:/Profiles/external.toml");
    let command = add_external_to_library_command(path.clone(), "Imported").unwrap();

    assert_eq!(
        command,
        EngineCommand::AddExternalProfileToLibrary {
            path,
            name: "Imported".to_owned()
        }
    );
}

#[test]
fn drawer_header_uses_sibling_toggle_and_snapshot_now_button() {
    let html = render_snapshot_drawer(sample_snapshot_context(), true);

    assert!(html.contains("class=\"snapshot-drawer__header\""));
    assert!(html.contains("class=\"snapshot-drawer__toggle\""));
    assert!(html.contains("aria-label=\"Snapshot now\""));
    assert!(!html.contains("<button class=\"snapshot-drawer__toggle\"><button"));
}

#[test]
fn ctrl_s_is_suppressed_inside_editable_or_modal_context() {
    assert!(!should_handle_snapshot_shortcut(FocusScope::TextInput));
    assert!(!should_handle_snapshot_shortcut(FocusScope::InlineRename));
    assert!(!should_handle_snapshot_shortcut(FocusScope::Menu));
    assert!(!should_handle_snapshot_shortcut(FocusScope::Dialog));
    assert!(!should_handle_snapshot_shortcut(FocusScope::OsPickerReturn));
    assert!(should_handle_snapshot_shortcut(FocusScope::Panel));
}
