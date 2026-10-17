//! Constant-time access to app-prepared instrument tracking values.
use crate::{
    context::AppContext,
    mapping_edits::{StageId, StageIdSegment},
    snapshots::MappingKey,
};
use dioxus::prelude::ReadableExt;
pub(crate) fn value(stage_id: &StageId, key: &MappingKey, ctx: &AppContext) -> Option<f64> {
    let [StageIdSegment::Index(index)] = stage_id.0.as_slice() else {
        return None;
    };
    let live = ctx.live.read();
    let readout = live.readout.as_ref()?;
    if &readout.key.mapping != key {
        return None;
    }
    readout.instruments.get(*index).copied().flatten()
}
