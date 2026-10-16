//! Profile presentation reuses metadata completed by the existing bridge cache.
use super::{Controller, Scope, policy};
use dioxus::prelude::{ReadableExt, WritableExt};
use inputforge_gui_dx::snapshots::MetaSnapshot;
impl Controller {
    /// Refresh values through the existing cadence, skipping contended reads.
    pub fn refresh(&self, owner: u64) {
        self.project(owner, None);
    }
    pub(super) fn reconcile(&self, meta: &MetaSnapshot) {
        self.reconcile_shortcut();
        let owner = self.session.borrow().as_ref().map(|s| s.owner);
        if let Some(owner) = owner {
            self.project(owner, Some(meta));
        }
    }
    fn project(&self, owner: u64, meta: Option<&MetaSnapshot>) {
        let Some(state) = self.state.try_read() else {
            return;
        };
        let mut sessions = self.session.borrow_mut();
        let Some(session) = sessions.as_mut().filter(|s| s.owner == owner) else {
            return;
        };
        let mut view = session.view.peek().clone();
        policy::project(&state, &mut view);
        if let Some(meta) = meta
            && (Scope {
                profile: meta.profile_id.clone(),
                path: meta.profile_path.clone(),
            }) == Scope::from_state(&state)
            && let Some(row) = view
                .rows
                .iter_mut()
                .find(|r| r.row.is_active && !r.row.can_open)
        {
            row.row.last_edited_label = meta
                .profile_rows
                .iter()
                .find(|r| r.is_active && !r.can_open)
                .and_then(|r| r.last_edited_label.clone());
        }
        if *session.view.peek() != view {
            session.view.set(view);
        }
    }
}
