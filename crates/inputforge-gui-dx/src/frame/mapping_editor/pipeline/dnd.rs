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

/// Validator for `SortableGap.validate_drop`. Returns `true` (drop
/// allowed) unless the source path is a strict prefix of the target path,
/// which would create a cycle in the action tree.
pub(crate) fn validate_pipeline_drop(src: &StageId, tgt: &StageId) -> bool {
    !is_descendant(src, tgt)
}

/// Convert a gap's pre-remove `gap_index` to the post-remove insertion
/// slot used by `insert_at_path`.
///
/// `gap_index` ranges over `[0, parent_pipeline_len]` inclusive (one
/// gap before each stage plus a trailing gap after the last). After
/// removing the source from its parent branch, the destination index
/// in `insert_at_path`'s call needs to account for the index shift:
///
/// * **Same-branch drop, source before gap** (e.g. dragging stage 0 to
///   gap 2 in `[A, B, C]`): the remove at index 0 leaves `[B, C]`, so
///   the gap formerly at index 2 is now at index 1. Subtract 1.
/// * **Same-branch drop, source after gap** (e.g. dragging stage 2 to
///   gap 0): the remove at index 2 leaves `[A, B]`, gap 0 is still
///   index 0. No change.
/// * **Cross-branch drop**: the source's removal happens in a different
///   branch, so the target branch's indices are unchanged.
#[must_use]
pub(crate) fn gap_to_post_remove_slot(
    src_parent_path: &StageId,
    parent_pipeline_path: &StageId,
    src_local_index: usize,
    gap_index: usize,
) -> usize {
    let same_branch = src_parent_path == parent_pipeline_path;
    if same_branch && src_local_index < gap_index {
        gap_index - 1
    } else {
        gap_index
    }
}
