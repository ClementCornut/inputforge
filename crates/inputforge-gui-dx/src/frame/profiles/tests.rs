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
use inputforge_core::state::{AppState, ProfileOrigin};

use crate::context::{AppContext, ConfigSnapshot, LiveSnapshot, MetaSnapshot};
use crate::context::{ProfileRowOrigin, ProfileRowView, SnapshotRowView};
use crate::frame::layout::EmptyState;
use crate::frame::profiles::ProfilesPanel;
use crate::frame::profiles::actions::{
    ConfirmationKind, ToastAction, create_manual_snapshot_action, profile_delete_action,
    profile_duplicate_action, profile_open_action, profile_rename_action, profile_reveal_action,
    snapshot_delete_action, snapshot_restore_action,
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
    let mut state = AppState::with_profile(simple_profile("Default"));
    state.profile_path = Some(PathBuf::from("C:/Profiles/Default.toml"));
    state.active_profile_origin = Some(ProfileOrigin::Library);
    state
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
        id: sample_snapshot_id(),
        kind_label: "Manual".to_owned(),
        label: Some("Before trim".to_owned()),
        time_label: "2026-05-06T20:00:00Z".to_owned(),
        sort_key: 1,
        pinned: true,
    }]
}

#[component]
fn SnapshotDrawerHarness(rows: Vec<SnapshotRowView>, open: bool) -> Element {
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

    rsx! {
        SnapshotDrawer {
            active_profile_name: "Default".to_owned(),
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
            mode_count: 1,
            last_edited_label: None,
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
fn active_profile_row_renders_when_library_rows_are_empty() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("Default"));
    assert!(html.contains("profile-row--active"));
    // Spec column contract: name + active pill + mode count + last-edited;
    // path label is intentionally absent.
    assert!(!html.contains("profile-row__path"));
}

#[test]
fn profiles_panel_uses_design_system_components() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("if-button"));
    assert!(html.contains("if-icon-button"));
    assert!(html.contains("if-badge"));
    assert!(html.contains("if-bottom-drawer"));
    assert!(html.contains("if-menu"));
    assert!(html.contains("if-menu__trigger"));
    assert!(html.contains("if-menu__item"));
    assert!(html.contains("aria-haspopup=\"true\""));
    assert!(!html.contains("button button--primary"));
    assert!(!html.contains("class=\"button\""));
    assert!(!html.contains("class=\"icon-button\""));
    assert!(!html.contains("class=\"badge\""));
    assert!(!html.contains("profile-row__actions"));
    assert!(!html.contains(">v<"));
}

#[test]
fn profiles_css_uses_flex_layout_and_flush_drawer_contract() {
    let css = include_str!("../../../assets/frame/profiles.css");

    assert!(css.contains(".profiles-panel {\n  display: flex;"));
    assert!(css.contains("flex-direction: column;"));
    assert!(css.contains(".profiles-panel__body {\n  flex: 1 1 auto;"));
    assert!(css.contains(".profile-row {\n  display: flex;"));
    assert!(css.contains(".snapshot-drawer {\n  margin-top: auto;"));
    assert!(css.contains("margin: calc(-1 * var(--space-3));"));
    assert!(css.contains("width: calc(100% + (2 * var(--space-3)));"));
    assert!(css.contains("scrollbar-gutter: auto;"));
    assert!(!css.contains("margin-bottom: calc(-1 * var(--space-3));"));
    assert!(!css.contains("grid-template-rows"));
    assert!(!css.contains("grid-row"));
}

#[test]
fn bottom_drawer_css_uses_flex_and_scrollable_body() {
    let css = include_str!("../../../assets/components/bottom-drawer.css");

    assert!(css.contains(".if-bottom-drawer {\n  display: flex;"));
    assert!(css.contains("flex-direction: column;"));
    assert!(css.contains("width: 100%;"));
    assert!(css.contains(".if-bottom-drawer__header {\n  flex: 0 0 auto;"));
    assert!(css.contains(".if-bottom-drawer__body {\n  flex: 1 1 auto;"));
    assert!(css.contains("overflow: auto;"));
    assert!(!css.contains("display: grid"));
    assert!(!css.contains("grid-template"));
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
fn profile_row_actions_dispatch_expected_commands() {
    let path = PathBuf::from("C:/Profiles/Alpha.toml");

    assert_eq!(
        profile_open_action(path.clone()),
        EngineCommand::LoadProfile(path.clone())
    );
    assert_eq!(
        profile_rename_action("Alpha", "Bravo"),
        Some(EngineCommand::RenameProfile {
            old_name: "Alpha".to_owned(),
            new_name: "Bravo".to_owned(),
        })
    );
    assert_eq!(
        profile_duplicate_action(path.clone(), "Alpha Copy"),
        Some(EngineCommand::DuplicateProfile {
            source_path: path.clone(),
            name: "Alpha Copy".to_owned(),
        })
    );
    assert_eq!(
        profile_reveal_action(path.clone()),
        EngineCommand::RevealProfile { path }
    );
    assert_eq!(
        create_manual_snapshot_action(),
        EngineCommand::CreateSnapshot {
            kind: SnapshotKind::Manual,
            label: None,
        }
    );
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

    assert!(html.contains("class=\"if-bottom-drawer__header\""));
    assert!(html.contains("if-bottom-drawer__toggle"));
    assert!(html.contains("aria-label=\"Snapshot now\""));
    assert!(!html.contains("<button class=\"if-bottom-drawer__toggle\"><button"));
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

#[test]
fn snapshot_restore_uses_f4_confirmation() {
    let id = sample_snapshot_id();
    let action = snapshot_restore_action(id);

    assert_eq!(action.confirmation, Some(ConfirmationKind::DestructiveF4));
    assert_eq!(action.command, EngineCommand::RestoreSnapshot { id });
}

#[test]
fn undo_toast_dispatches_undo_snapshot_delete() {
    let id = sample_snapshot_id();
    let toast_action = ToastAction::UndoSnapshotDelete { id };

    assert_eq!(
        toast_action.command(),
        EngineCommand::UndoSnapshotDelete { id }
    );
}

#[test]
fn profiles_surface_never_renders_mapping_counts() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(!html.contains("mapping"));
    assert!(!html.contains("mappings"));
}

#[test]
fn drawer_is_panel_scoped_not_global_drawer() {
    let html = render_profiles_panel(sample_profiles_context());

    assert!(html.contains("if-bottom-drawer"));
    assert!(!html.contains("app-global-drawer"));
}
