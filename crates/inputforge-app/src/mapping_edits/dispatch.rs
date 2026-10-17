//! Current-target validation and successful-delivery-before-history ordering.
use super::Controller;
use inputforge_core::{action::Mapping, engine::EngineCommand};
use inputforge_gui_dx::{mapping_edits::Request, snapshots::MappingKey};
use std::time::Instant;
fn command(key: &MappingKey, mapping: Option<&Mapping>) -> EngineCommand {
    mapping.map_or_else(
        || EngineCommand::RemoveMapping {
            input: key.1.clone(),
            mode: key.0.clone(),
        },
        |m| EngineCommand::SetMapping {
            input: m.input.clone(),
            mode: m.mode.clone(),
            name: m.name.clone(),
            actions: m.actions.clone(),
        },
    )
}
impl Controller {
    /// Validate and deliver one edit, then record its restore snapshots.
    #[must_use]
    pub fn dispatch(&self, request: Request, now: Instant) -> bool {
        self.edit(request, now, false).unwrap_or(false)
    }
    pub(super) fn edit(
        &self,
        mut request: Request,
        now: Instant,
        captured: bool,
    ) -> Result<bool, String> {
        let allows_noop = matches!(
            request.edit,
            inputforge_gui_dx::mapping_edits::Edit::Field { .. }
                | inputforge_gui_dx::mapping_edits::Edit::Rename(_)
        );
        self.reconcile();
        let Some((key, entry)) = self.prepare(&mut request, captured) else {
            self.publish();
            return Err(
                "The mapping could not be updated. Check the current mapping and try again.".into(),
            );
        };
        if allows_noop && entry.before == entry.after {
            return Ok(false);
        }
        let cmd = command(&entry.target, entry.after.as_ref());
        if self.commands.send(cmd).is_err() {
            self.end_group();
            tracing::warn!(
                action = "mapping_edit_delivery_failed",
                "mapping edit dropped: receiver disconnected"
            );
            return Err(
                "The mapping could not be updated. Check the current mapping and try again.".into(),
            );
        }
        let Some(serial) = self.fence() else {
            self.end_group();
            return Err(
                "The mapping could not be updated. Check the current mapping and try again.".into(),
            );
        };
        {
            let state = self.state.read();
            let mut inner = self.inner.borrow_mut();
            inner.revision = inner.revision.wrapping_add(1);
            inner.history.watch(&state, &key);
            inner.history.delivered(
                &state,
                &entry.target,
                entry.after.clone(),
                serial,
                entry.structural,
            );
            inner.history.record(key, entry, request.group, now);
        };
        self.invalidate_editor(&request.edit);
        self.publish();
        Ok(true)
    }
    pub(super) fn restore(&self, key: &MappingKey, redo: bool) -> bool {
        self.reconcile();
        self.end_group();
        let Some(entry) = self.prepare_restore(key, redo) else {
            self.publish();
            return false;
        };
        let restore = if redo { &entry.after } else { &entry.before };
        if self
            .commands
            .send(command(&entry.target, restore.as_ref()))
            .is_err()
        {
            tracing::warn!(
                action = "mapping_restore_delivery_failed",
                "history restore dropped: receiver disconnected"
            );
            return false;
        }
        let Some(serial) = self.fence() else {
            return false;
        };
        {
            let state = self.state.read();
            let mut inner = self.inner.borrow_mut();
            inner.revision = inner.revision.wrapping_add(1);
            inner
                .history
                .delivered(&state, &entry.target, restore.clone(), serial, true);
            let stack = inner
                .history
                .stacks
                .get_mut(key)
                .expect("history still exists after synchronous delivery");
            if redo {
                stack.redo.pop();
                stack.undo.push(entry);
            } else {
                stack.undo.pop();
                stack.redo.push(entry);
            }
            stack.stale_start = stack.undo.len();
        };
        self.publish();
        true
    }
}
