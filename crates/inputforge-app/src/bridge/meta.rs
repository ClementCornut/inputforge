//! Prepare profile and session rows without filesystem access.
use super::time_labels::{format_absolute_time, format_relative_at};
use inputforge_core::state::AppState;
use inputforge_core::state::ProfileOrigin as CoreProfileOrigin;
use inputforge_gui_dx::snapshots::{
    MetaSnapshot, ProfileRowOrigin, ProfileRowView, SnapshotRowView,
};
pub(super) fn project(s: &AppState) -> MetaSnapshot {
    let active_profile_id = s
        .profile_path
        .as_ref()
        .map(|path| path.display().to_string());
    let mut profile_rows = s
        .profile_library_rows
        .iter()
        .map(|row| {
            let id = row.path.display().to_string();
            ProfileRowView {
                id,
                name: row.name.clone(),
                path_label: row.path.display().to_string(),
                is_active: row.is_active,
                origin: ProfileRowOrigin::Library,
                mode_count: row.mode_count,
                last_edited_label: row
                    .last_edited_at
                    .map(super::time_labels::format_chrono_time_relative),
                can_open: true,
                can_rename: true,
                can_duplicate: true,
                can_reveal: true,
                can_delete: true,
                can_add_to_library: false,
                can_snapshot_now: row.is_active,
            }
        })
        .collect::<Vec<_>>();
    if s.active_profile_origin == Some(CoreProfileOrigin::External)
        && let Some(row) = active_profile_row_from_state(s, ProfileRowOrigin::External)
    {
        profile_rows.push(row);
    }
    if profile_rows.is_empty()
        && let Some(row) = active_profile_row_from_state(s, ProfileRowOrigin::Library)
    {
        profile_rows.push(row);
    }
    let now_for_snapshots = chrono::Utc::now();
    let snapshot_rows = s
        .active_snapshot_rows
        .iter()
        .map(|row| SnapshotRowView {
            id: row.id,
            kind: row.kind,
            kind_label: match row.kind {
                inputforge_core::snapshot::SnapshotKind::AutoSessionStart => "Session start",
                inputforge_core::snapshot::SnapshotKind::AutoBeforeRestore => "Before restore",
                // The user-visible surface that creates this kind is the
                // "Batch map" tab; "bulk map" is engine vocabulary. Match
                // the label to the surface so the kind badge does not
                // stutter against the snapshot's own label which already
                // says "Before batch map".
                inputforge_core::snapshot::SnapshotKind::AutoBeforeBulkMap => "Before batch map",
                inputforge_core::snapshot::SnapshotKind::Manual => "Manual",
            }
            .to_owned(),
            label: row.label.clone(),
            time_relative: format_relative_at(row.taken_at, now_for_snapshots),
            time_absolute: format_absolute_time(row.taken_at),
            sort_key: row.taken_at.timestamp_millis(),
            pinned: row.pinned,
        })
        .collect();
    MetaSnapshot {
        session: s.session.clone(),
        engine_status: s.engine_status,
        current_mode: s.current_mode.clone(),
        profile_name: s.active_profile.as_ref().map(|p| p.name().to_owned()),
        profile_path: s.profile_path.clone(),
        warnings: s.warnings.clone(),
        modes: s
            .active_profile
            .as_ref()
            .map(|p| p.modes().as_slice().to_vec())
            .unwrap_or_default(),
        startup_mode: s
            .active_profile
            .as_ref()
            .map(|p| p.settings().startup_mode().to_owned()),
        active_profile_id,
        profile_rows,
        snapshot_rows,
    }
}
fn active_profile_row_from_state(s: &AppState, origin: ProfileRowOrigin) -> Option<ProfileRowView> {
    let path = s.profile_path.as_ref()?;
    let profile = s.active_profile.as_ref()?;
    let path_label = path.display().to_string();
    let is_external = origin == ProfileRowOrigin::External;
    let mode_count = u32::try_from(profile.modes().len()).unwrap_or(u32::MAX);

    Some(ProfileRowView {
        id: path_label.clone(),
        name: profile.name().to_owned(),
        path_label,
        is_active: true,
        origin,
        mode_count,
        last_edited_label: None,
        can_open: false,
        can_rename: !is_external,
        can_duplicate: !is_external,
        can_reveal: true,
        can_delete: !is_external,
        can_add_to_library: is_external,
        can_snapshot_now: true,
    })
}
