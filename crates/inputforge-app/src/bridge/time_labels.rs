//! Relative timestamp labels and local absolute timestamps.
/// Format a `DateTime<Utc>` as a short relative-time label, using
/// `now` as the reference. Pure function so tests can pin behaviour
/// deterministically without reading the wall clock.
///
/// Buckets: "just now" (under 1 min), "Nm ago" (under 1 hour),
/// "Nh ago" (under 1 day), "Nd ago" (under 1 week), `YYYY-MM-DD`
/// otherwise. Negative deltas (`taken_at` in the future, e.g. clock
/// skew) collapse to the date form so the user never sees "in 2m".
pub(crate) fn format_relative_at(
    taken_at: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let delta = now.signed_duration_since(taken_at);
    if delta.num_seconds() < 0 {
        return taken_at.format("%Y-%m-%d").to_string();
    }
    let mins = delta.num_minutes();
    let hours = delta.num_hours();
    let days = delta.num_days();
    if mins < 1 {
        "just now".to_owned()
    } else if mins < 60 {
        format!("{mins}m ago")
    } else if hours < 24 {
        format!("{hours}h ago")
    } else if days < 7 {
        format!("{days}d ago")
    } else {
        taken_at.format("%Y-%m-%d").to_string()
    }
}

/// Wall-clock variant of [`format_relative_at`]. Production callers
/// pass through here; tests prefer the pure form.
pub(super) fn format_chrono_time_relative(datetime: chrono::DateTime<chrono::Utc>) -> String {
    format_relative_at(datetime, chrono::Utc::now())
}

/// Format a UTC timestamp using the system's local clock and UTC offset.
/// Drops seconds and sub-second precision for the snapshot tooltip.
pub(crate) fn format_absolute_time(taken_at: chrono::DateTime<chrono::Utc>) -> String {
    taken_at
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M %:z")
        .to_string()
}
