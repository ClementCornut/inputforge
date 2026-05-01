// Rust guideline compliant 2026-05-01

//! F9 pipeline drag-and-drop helpers.
//!
//! Drag/drop event handling is delegated to `components/sortable` with
//! `G = StageId`. This file carries the cycle-prevention validator and a
//! pure path-prefix helper used by the sortable item config.

use crate::frame::mapping_editor::undo_log::StageId;

/// Strict path-prefix check. Returns `true` when `ancestor` is a strict
/// prefix of `candidate`, meaning `candidate` is a descendant of `ancestor`
/// in the action tree.
///
/// A drop is rejected when the source path is a strict prefix of the target
/// path: moving a `Conditional` into one of its own descendant branches
/// would create a cycle. This predicate is pure with no allocation.
///
/// Equality (`ancestor == candidate`) returns `false` -- a node is not its
/// own strict ancestor.
#[must_use]
pub(crate) fn is_descendant(ancestor: &StageId, candidate: &StageId) -> bool {
    // Strict prefix: candidate must be strictly longer than ancestor, and
    // ancestor's segments must match the leading prefix of candidate.
    if candidate.0.len() <= ancestor.0.len() {
        return false;
    }
    candidate.0[..ancestor.0.len()] == ancestor.0[..]
}

/// Validator for `SortableItemConfig.validate_drop`. Returns `true` (drop
/// allowed) unless the source path is a strict prefix of the target path,
/// which would create a cycle in the action tree.
pub(crate) fn validate_pipeline_drop(src: &StageId, tgt: &StageId) -> bool {
    !is_descendant(src, tgt)
}
