// Rust guideline compliant 2026-05-01

//! Per-mapping session-undo log. See spec § "Per-mapping session-undo log".

use std::collections::HashMap;

use inputforge_core::action::Mapping;

use crate::frame::MappingKey;

/// Path of segments identifying a stage in a possibly-nested action tree.
///
/// Examples (using the `StageIdSegment` variants below):
/// - `[Index(0)]`                              outer-pipeline first stage
/// - `[Index(2)]`                              outer-pipeline third stage
/// - `[Index(2), IfTrue, Index(1)]`            Conditional at outer index 2, `if_true` branch, second stage
/// - `[Index(2), IfFalse, Index(0)]`           Conditional at outer index 2, `if_false` branch, first stage
///
/// Paths are positional, NOT identity-based. Structural mutations
/// (insert/remove) invalidate every `StageId` at or after the mutation point.
/// See Task 11 for the clear-on-mutation contract that keeps
/// `expanded_stages` and `malformed_hints` consistent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct StageId(pub Vec<StageIdSegment>);

/// One segment of a [`StageId`] path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum StageIdSegment {
    /// Zero-based index into a pipeline's stage list.
    Index(usize),
    /// The `if_true` branch of a `Conditional` stage.
    IfTrue,
    /// The `if_false` branch of a `Conditional` stage.
    IfFalse,
}

/// Kinds of change recorded in the undo stack.
///
/// Note: editing-mode changes (re-assigning a mapping to a different mode)
/// are encoded as `Rebind` because the mode is an axis of
/// `EngineCommand::SetMapping` alongside the input address. This keeps the
/// label-format helper (Task 8) compact; if F-future ever needs distinct
/// labelling, add an explicit `ChangeMode` variant then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UndoKind {
    /// A stage's fields were edited in place.
    StageEdit,
    /// A new stage was appended or inserted.
    StageAdd,
    /// A stage was removed.
    StageRemove,
    /// Stages were reordered via drag-and-drop or keyboard.
    StageReorder,
    /// The mapping's display name was changed.
    Rename,
    /// The mapping's input address or mode was changed.
    Rebind,
}

/// One entry in a mapping's undo stack.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UndoEntry {
    /// The kind of change that produced this entry.
    pub kind: UndoKind,
    /// Full `Mapping` snapshot for restore. Cheap; bounded by stage count.
    pub mapping_before: Mapping,
    /// Human-readable label per the F9 convention.
    /// See spec § "`UndoLog` data shape" for format.
    pub label: String,
}

/// Per-mapping FIFO-capped undo + redo stacks.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MappingHistory {
    /// Entries available for undo, ordered oldest-first.
    pub undo: Vec<UndoEntry>,
    /// Entries available for redo, ordered oldest-first.
    pub redo: Vec<UndoEntry>,
}

/// Per-mapping session-undo log. Keyed by [`MappingKey`].
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct UndoLog {
    /// One [`MappingHistory`] per active mapping in the session.
    pub stacks: HashMap<MappingKey, MappingHistory>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_kind_variants_present() {
        // Compile-time presence check — exhaustive match ensures no variant
        // is silently removed by a later refactor.
        let _ = UndoKind::StageEdit;
        let _ = UndoKind::StageAdd;
        let _ = UndoKind::StageRemove;
        let _ = UndoKind::StageReorder;
        let _ = UndoKind::Rename;
        let _ = UndoKind::Rebind;
    }

    #[test]
    fn stage_id_segment_variants_present() {
        let _ = StageIdSegment::Index(0);
        let _ = StageIdSegment::IfTrue;
        let _ = StageIdSegment::IfFalse;
    }
}
