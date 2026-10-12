//! Prepared profile, mode, warning, and session rendering data.
use inputforge_core::{snapshot::SnapshotId, state::EngineStatus};
use std::path::PathBuf;
/// Prepared session, profile, mode, and warning data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetaSnapshot {
    pub session: inputforge_core::state::SessionState,
    pub engine_status: EngineStatus,
    pub current_mode: String,
    pub profile_name: Option<String>,
    pub profile_path: Option<PathBuf>,
    pub warnings: Vec<String>,
    /// Ordered flat mode names.
    pub modes: Vec<String>,
    pub startup_mode: Option<String>,
    pub active_profile_id: Option<String>,
    pub profile_rows: Vec<ProfileRowView>,
    pub snapshot_rows: Vec<SnapshotRowView>,
}

/// Whether a profile row belongs to the library or an external file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileRowOrigin {
    Library,
    External,
}

/// Prepared profile-row content and available controls.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "profile row capabilities are explicit UI affordance flags"
)]
pub struct ProfileRowView {
    pub id: String,
    pub name: String,
    pub path_label: String,
    pub is_active: bool,
    pub origin: ProfileRowOrigin,
    /// Number of modes defined in the profile.
    pub mode_count: u32,
    /// Pre-formatted "last edited" label for display, when known.
    pub last_edited_label: Option<String>,
    pub can_open: bool,
    pub can_rename: bool,
    pub can_duplicate: bool,
    pub can_reveal: bool,
    pub can_delete: bool,
    pub can_add_to_library: bool,
    pub can_snapshot_now: bool,
}

/// Prepared saved-snapshot content and timestamps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRowView {
    pub id: SnapshotId,
    pub kind: inputforge_core::snapshot::SnapshotKind,
    pub kind_label: String,
    pub label: Option<String>,
    /// Glanceable relative time ("just now", "12m ago", "2d ago", or
    /// `YYYY-MM-DD` for older). Re-computed each projection cycle.
    pub time_relative: String,
    /// Local timestamp for the tooltip / `<time datetime>` value.
    /// `YYYY-MM-DD HH:MM +HH:MM`, with the applicable local UTC offset.
    pub time_absolute: String,
    pub sort_key: i64,
    pub pinned: bool,
}
