//! Nested pipeline drop interpretation and pre-removal destination identity.

use dioxus::prelude::*;
use inputforge_gui_dx::mapping_edits::{Edit, StageId, StageIdSegment};
use inputforge_gui_dx::sortable::PipelineDrop;

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
pub fn is_descendant(ancestor: &StageId, candidate: &StageId) -> bool {
    // Strict prefix: candidate must be strictly longer than ancestor, and
    // ancestor's segments must match the leading prefix of candidate.
    if candidate.0.len() <= ancestor.0.len() {
        return false;
    }
    candidate.0[..ancestor.0.len()] == ancestor.0[..]
}

/// Reject a captured source path that would descend into itself.
#[must_use]
pub fn validate_pipeline_drop(src: &StageId, tgt: &StageId) -> bool {
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
pub fn gap_to_post_remove_slot(
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

pub(super) fn eligible(index: usize, parent: &StageId, target: &StageId) -> bool {
    let mut source = parent.clone();
    source.0.push(StageIdSegment::Index(index));
    !target.0.starts_with(&source.0)
}

/// Interpret a gap using the captured source; final mapping dispatch revalidates identity.
pub fn drop_pipeline(mut request: PipelineDrop) {
    if *request.state.closed.peek() {
        return;
    }
    let Some(index) = *request.state.drag_from.peek() else {
        return;
    };
    let Some(parent) = request.state.drag_group.peek().clone() else {
        return;
    };
    if !eligible(index, &parent, &request.parent) {
        return;
    }
    let to = gap_to_post_remove_slot(&parent, &request.parent, index, request.gap);
    if parent == request.parent && to == index {
        return;
    }
    let mut from = parent;
    from.0.push(StageIdSegment::Index(index));
    let title = request.title.call(from.clone());
    if gap_destination(&from, &request.parent, request.gap).is_none() {
        return;
    }
    if request.submit.call((
        Edit::MoveGap {
            from,
            parent: request.parent,
            gap: request.gap,
        },
        format!("move stage {title} from {index} to {to}"),
    )) {
        request.state.live_announcement.set(format!(
            "Stage moved from position {} to {}",
            index + 1,
            to + 1
        ));
    }
}

/// Resolve a pre-removal gap while rejecting cycles and preserving target branch identity.
#[must_use]
pub fn gap_destination(from: &StageId, parent: &StageId, gap: usize) -> Option<StageId> {
    let (StageIdSegment::Index(index), source_parent) = from.0.split_last()? else {
        return None;
    };
    if parent.0.starts_with(&from.0) {
        return None;
    }
    if parent.0 == source_parent && (gap == *index || index.checked_add(1) == Some(gap)) {
        return None;
    }
    let slot = if parent.0 == source_parent && *index < gap {
        gap - 1
    } else {
        gap
    };
    let mut target = parent.clone();
    if target.0.starts_with(source_parent)
        && let Some(StageIdSegment::Index(target_index)) = target.0.get_mut(source_parent.len())
        && *target_index > *index
    {
        *target_index -= 1;
    }
    target.0.push(StageIdSegment::Index(slot));
    Some(target)
}
