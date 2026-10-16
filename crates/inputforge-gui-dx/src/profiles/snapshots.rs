//! Prepared snapshot rows and app-owned snapshot input.
use super::Target as Profile;
use crate::snapshots::SnapshotRowView;
use inputforge_core::state::ActiveSnapshotRow;
/// Captured profile and complete engine row identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub profile: Profile,
    pub snapshot: ActiveSnapshotRow,
}
/// Prepared presentation and availability for one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub row: SnapshotRowView,
    pub target: Target,
    pub can_delete: bool,
}
/// Snapshot drawer and destructive confirmation prepared by the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub profile: Option<Profile>,
    pub rows: Vec<Row>,
    pub open: bool,
    pub restore: Option<Target>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            profile: None,
            rows: vec![],
            open: true,
            restore: None,
        }
    }
}
/// Snapshot input captured by the rendered surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Create(Profile),
    Toggle(Profile),
    Restore(Target),
    Delete(Target),
    Confirm,
    Cancel,
}
