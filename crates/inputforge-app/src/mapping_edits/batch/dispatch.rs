//! Guard-free batch delivery, snapshot request and affected history invalidation.
use super::{
    super::{Controller, history::History},
    policy, publish,
};
use inputforge_core::{
    action::{Action, BulkMapEntry, Mapping},
    engine::EngineCommand,
    state::AppState,
};
use inputforge_gui_dx::batch_mapping::View;
pub(super) fn prepare(
    state: &AppState,
    history: &History,
    view: &View,
) -> Option<(Vec<BulkMapEntry>, String)> {
    let source = view
        .sources
        .iter()
        .find(|(id, _)| Some(id) == view.source.as_ref())?;
    let label = format!("{} · Device {}", source.1, view.target?);
    let mut entries = vec![];
    for row in &view.rows {
        let Some(output) = &row.target else { continue };
        for mode in policy::modes(view) {
            if !row.replace
                && history
                    .current(state, &(mode.clone(), row.input.clone()))
                    .is_some()
            {
                continue;
            }
            entries.push(BulkMapEntry {
                input: row.input.clone(),
                mode,
                output: output.clone(),
            });
        }
    }
    Some((entries, label))
}
impl Controller {
    pub(super) fn batch_deliver(&self, owner: u128, entries: Vec<BulkMapEntry>, label: String) {
        if entries.is_empty() {
            return;
        }
        let count = entries.len();
        if self
            .commands
            .send(EngineCommand::SetMappingsBulk {
                entries: entries.clone(),
                snapshot_label: label,
            })
            .is_err()
        {
            self.batch_failed(owner);
            return;
        }
        let Some(serial) = self.fence() else {
            self.batch_failed(owner);
            return;
        };
        let applied = {
            let state = self.state.read();
            let mut inner = self.inner.borrow_mut();
            for entry in entries {
                let key = (entry.mode.clone(), entry.input.clone());
                inner.history.forward(
                    &state,
                    &key,
                    Some(Mapping {
                        input: entry.input,
                        mode: entry.mode,
                        name: None,
                        actions: vec![Action::MapToVJoy {
                            output: entry.output,
                        }],
                    }),
                    serial,
                );
            }
            inner
                .batch
                .as_ref()
                .filter(|b| b.view.owner == owner)
                .map(|b| b.publication.applied)
        };
        self.publish();
        self.batch_refresh();
        if let Some(applied) = applied {
            applied.call(count);
        }
    }
    fn batch_failed(&self, owner: u128) {
        let publication = {
            let mut inner = self.inner.borrow_mut();
            let Some(batch) = inner.batch.as_mut().filter(|b| b.view.owner == owner) else {
                return;
            };
            batch.view.error =
                Some("Batch mapping command could not be delivered. Try again.".into());
            (batch.publication, batch.view.clone())
        };
        publish(publication.0, publication.1);
    }
}
