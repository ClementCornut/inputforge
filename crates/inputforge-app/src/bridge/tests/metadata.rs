use super::super::{Poller, profile_metadata::Cache};
use inputforge_core::{
    mode::Modes,
    profile::Profile,
    state::{AppState, ProfileOrigin},
};
use inputforge_gui_dx::snapshots::{MetaSnapshot, ProfileRowOrigin, ProfileRowView};
use parking_lot::RwLock;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

fn meta(path: &str) -> MetaSnapshot {
    MetaSnapshot {
        profile_path: Some(PathBuf::from(path)),
        profile_rows: vec![ProfileRowView {
            id: path.into(),
            path_label: path.into(),
            name: "external".into(),
            is_active: true,
            origin: ProfileRowOrigin::External,
            mode_count: 1,
            last_edited_label: None,
            can_open: false,
            can_rename: false,
            can_duplicate: false,
            can_reveal: true,
            can_delete: false,
            can_add_to_library: true,
            can_snapshot_now: true,
        }],
        ..Default::default()
    }
}
#[test]
fn unchanged_path_reuses_timestamp_until_refresh() {
    let mut cache = Cache::default();
    let now = Instant::now();
    let mut first = meta("external.toml");
    cache.complete(&mut first, now, |_| Some(SystemTime::now()));
    assert_eq!(
        first.profile_rows[0].last_edited_label.as_deref(),
        Some("just now")
    );
    let mut second = meta("external.toml");
    cache.complete(&mut second, now + Duration::from_millis(16), |_| {
        panic!("cached tick must not read the file")
    });
    assert_eq!(
        second.profile_rows[0].last_edited_label,
        first.profile_rows[0].last_edited_label
    );
    cache.complete(&mut second, now + Duration::from_secs(1), |_| None);
    assert_eq!(second.profile_rows[0].last_edited_label, None);
}
#[test]
fn switching_paths_refreshes_immediately() {
    let mut cache = Cache::default();
    let now = Instant::now();
    cache.complete(&mut meta("first.toml"), now, |_| Some(SystemTime::now()));
    let mut second = meta("second.toml");
    cache.complete(&mut second, now + Duration::from_millis(16), |path| {
        assert_eq!(path, std::path::Path::new("second.toml"));
        Some(SystemTime::UNIX_EPOCH)
    });
    assert_eq!(
        second.profile_rows[0].last_edited_label.as_deref(),
        Some("1970-01-01")
    );
}
#[test]
fn metadata_failure_is_cached_then_retried() {
    let mut cache = Cache::default();
    let now = Instant::now();
    let mut frame = meta("missing.toml");
    cache.complete(&mut frame, now, |_| None);
    assert_eq!(frame.profile_rows[0].last_edited_label, None);
    cache.complete(&mut frame, now + Duration::from_millis(16), |_| {
        panic!("failed stat must be cached")
    });
    cache.complete(&mut frame, now + Duration::from_secs(1), |_| {
        Some(SystemTime::now())
    });
    assert_eq!(
        frame.profile_rows[0].last_edited_label.as_deref(),
        Some("just now")
    );
}
#[test]
fn metadata_reader_can_write_engine_state() {
    let profile = Profile::new(
        "external".into(),
        vec![],
        Modes::new(vec!["Default".into()]).unwrap(),
        vec![],
        vec![],
        "Default".into(),
    );
    let mut state = AppState::with_profile(profile);
    state.profile_path = Some(PathBuf::from("external.toml"));
    state.active_profile_origin = Some(ProfileOrigin::External);
    let state = Arc::new(RwLock::new(state));
    let (commands, _) = std::sync::mpsc::channel();
    let capture = crate::live_capture::Controller::new(Arc::clone(&state), commands);
    let mut poller = Poller::new(Arc::clone(&state), capture);
    let frame = poller
        .sample(None, Instant::now(), |_| {
            state
                .try_write()
                .expect("metadata I/O must release engine lock")
                .current_mode = "Changed".into();
            Some(SystemTime::now())
        })
        .unwrap();
    assert_eq!(frame.meta.current_mode, "Default");
    assert_eq!(state.read().current_mode, "Changed");
    assert_eq!(
        frame.meta.profile_rows[0].last_edited_label.as_deref(),
        Some("just now")
    );
}

#[test]
fn leaving_synthesized_rows_invalidates_cache_without_reading_library_files() {
    let mut cache = Cache::default();
    let now = Instant::now();
    cache.complete(&mut meta("external.toml"), now, |_| Some(SystemTime::now()));
    let mut library = meta("external.toml");
    library.profile_rows[0].can_open = true;
    library.profile_rows[0].last_edited_label = Some("library timestamp".into());
    cache.complete(&mut library, now, |_| {
        panic!("library timestamps already belong to core")
    });
    assert_eq!(
        library.profile_rows[0].last_edited_label.as_deref(),
        Some("library timestamp")
    );
    let mut external = meta("external.toml");
    cache.complete(&mut external, now, |_| Some(SystemTime::UNIX_EPOCH));
    assert_eq!(
        external.profile_rows[0].last_edited_label.as_deref(),
        Some("1970-01-01")
    );
}
#[test]
fn real_metadata_source_reports_removed_files_without_failing_the_frame() {
    let file = tempfile::NamedTempFile::new().unwrap();
    assert!(super::super::modified(file.path()).is_some());
    let path = file.path().to_owned();
    drop(file);
    assert_eq!(super::super::modified(&path), None);
}
