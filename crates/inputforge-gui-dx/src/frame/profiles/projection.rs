use crate::context::ProfileRowView;

pub(crate) fn project_profile_rows(
    rows: &[ProfileRowView],
    active_id: &str,
    filter: &str,
) -> Vec<ProfileRowView> {
    let needle = filter.trim().to_lowercase();
    let mut active = rows
        .iter()
        .filter(|row| row.id == active_id)
        .cloned()
        .collect::<Vec<_>>();
    let mut inactive = rows
        .iter()
        .filter(|row| row.id != active_id)
        .filter(|row| needle.is_empty() || row.name.to_lowercase().contains(&needle))
        .cloned()
        .collect::<Vec<_>>();

    active.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    inactive.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    active.extend(inactive);
    active
}
