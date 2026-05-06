use std::collections::HashMap;
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
use crate::context::{ProfileRowOrigin, ProfileRowView};
use crate::frame::layout::EmptyState;
use crate::frame::profiles::ProfilesPanel;
use crate::frame::profiles::actions::{
    ConfirmationKind, ToastAction, profile_delete_action, snapshot_delete_action,
};
use crate::frame::profiles::projection::project_profile_rows;

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
