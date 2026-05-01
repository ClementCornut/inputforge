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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Cap on per-mapping undo entries, per spec AC #25
/// ("Undo stack caps 50 entries; FIFO eviction").
const MAX_ENTRIES: usize = 50;

impl UndoLog {
    /// Append an edit entry. Clears the redo stack on this key.
    /// Enforces 50-entry FIFO cap.
    pub(crate) fn push_edit(
        &mut self,
        key: MappingKey,
        before: Mapping,
        kind: UndoKind,
        label: String,
    ) {
        let history = self.stacks.entry(key).or_default();
        history.redo.clear();
        history.undo.push(UndoEntry {
            kind,
            mapping_before: before,
            label,
        });
        if history.undo.len() > MAX_ENTRIES {
            let drain_count = history.undo.len() - MAX_ENTRIES;
            history.undo.drain(..drain_count);
        }
    }

    /// Pop the last undo entry and push it to redo. Caller dispatches
    /// `SetMapping` with `entry.mapping_before`.
    pub(crate) fn undo(&mut self, key: &MappingKey) -> Option<UndoEntry> {
        let history = self.stacks.get_mut(key)?;
        let entry = history.undo.pop()?;
        history.redo.push(entry.clone());
        Some(entry)
    }

    /// Pop the last redo entry and push it to undo.
    pub(crate) fn redo(&mut self, key: &MappingKey) -> Option<UndoEntry> {
        let history = self.stacks.get_mut(key)?;
        let entry = history.redo.pop()?;
        history.undo.push(entry.clone());
        Some(entry)
    }

    /// Clear both stacks for `key`.
    ///
    /// Removes the key entirely so that callers can test
    /// `stacks.get(&key).is_none()` as a cheap "nothing recorded" check.
    pub(crate) fn clear(&mut self, key: &MappingKey) {
        self.stacks.remove(key);
    }

    /// Return the label of the topmost undo entry, if any.
    ///
    /// Used by the editor footer recap to display "Undo: <label>".
    pub(crate) fn last_label(&self, key: &MappingKey) -> Option<String> {
        self.stacks
            .get(key)
            .and_then(|h| h.undo.last())
            .map(|e| e.label.clone())
    }

    /// Clear all per-mapping stacks. Used by Task 32's profile-flip handler.
    ///
    /// After this call `stacks` is empty, which means `has_pending_changes`
    /// returns `false` for every key.
    pub(crate) fn clear_all(&mut self) {
        self.stacks.clear();
    }

    /// Returns `true` if any mapping has a non-empty undo OR redo stack.
    ///
    /// Used by the profile-name click handler to decide whether to open
    /// `DirtyConfirmDialog` before navigating away. Returns `false` on a
    /// fresh or already-cleared log.
    pub(crate) fn has_pending_changes(&self) -> bool {
        self.stacks
            .values()
            .any(|h| !h.undo.is_empty() || !h.redo.is_empty())
    }
}

/// Argument bundle for [`format_undo_label`]. Each [`UndoKind`] reads a
/// specific subset of fields; the rest are ignored.
///
/// # Examples
///
/// ```rust,ignore
/// let label = format_undo_label(UndoKind::Rename, LabelArgs {
///     old_new: Some(("X axis", "Yaw")),
///     ..LabelArgs::default()
/// });
/// assert_eq!(label, "rename: 'X axis' -> 'Yaw'");
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LabelArgs<'a> {
    /// Human-readable stage variant or stage display name.
    pub stage_name: Option<&'a str>,
    /// Field name within a stage body (e.g. `"threshold"`, `"operation"`).
    pub field: Option<&'a str>,
    /// `(before, after)` field values stringified by the caller.
    pub before_after: Option<(&'a str, &'a str)>,
    /// Pipeline index for add / remove.
    pub index: Option<usize>,
    /// `(from_index, to_index)` for reorder.
    pub from_to: Option<(usize, usize)>,
    /// `(old, new)` for rename / rebind.
    pub old_new: Option<(&'a str, &'a str)>,
}

/// Format an undo-entry label per the F9 convention.
///
/// See spec § "`UndoLog` data shape" for the canonical label-format table.
///
/// Each [`UndoKind`] reads a specific subset of [`LabelArgs`] fields;
/// supplying `None` for a required field produces `"?"` as a safe fallback.
///
/// # Examples
///
/// ```rust,ignore
/// let label = format_undo_label(UndoKind::StageAdd, LabelArgs {
///     stage_name: Some("ResponseCurve"),
///     index: Some(2),
///     ..LabelArgs::default()
/// });
/// assert_eq!(label, "add stage: ResponseCurve at index 2");
/// ```
#[must_use]
pub(crate) fn format_undo_label(kind: UndoKind, args: LabelArgs<'_>) -> String {
    match kind {
        UndoKind::StageEdit => {
            let name = args.stage_name.unwrap_or("?");
            let field = args.field.unwrap_or("?");
            let (b, a) = args.before_after.unwrap_or(("?", "?"));
            format!("{name}: {field} {b} -> {a}")
        }
        UndoKind::StageAdd => {
            let name = args.stage_name.unwrap_or("?");
            let i = args.index.unwrap_or(0);
            format!("add stage: {name} at index {i}")
        }
        UndoKind::StageRemove => {
            let name = args.stage_name.unwrap_or("?");
            let i = args.index.unwrap_or(0);
            format!("remove stage: {name} at index {i}")
        }
        UndoKind::StageReorder => {
            let name = args.stage_name.unwrap_or("?");
            let (from, to) = args.from_to.unwrap_or((0, 0));
            format!("move stage {name} from {from} to {to}")
        }
        UndoKind::Rename => {
            let (old, new) = args.old_new.unwrap_or(("?", "?"));
            format!("rename: '{old}' -> '{new}'")
        }
        UndoKind::Rebind => {
            let (old, new) = args.old_new.unwrap_or(("?", "?"));
            format!("rebind: {old} -> {new}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_kind_variants_present() {
        // Compile-time presence check: exhaustive match ensures no variant
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

    // --- Task 7 tests ---

    #[expect(
        unused_imports,
        reason = "Action imported for API completeness; not all tests use it"
    )]
    use inputforge_core::action::Action;
    use inputforge_core::types::{DeviceId, InputAddress, InputId};

    fn synth_key() -> MappingKey {
        (
            "Default".to_owned(),
            InputAddress {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Button { index: 0 },
            },
        )
    }

    fn synth_mapping(name: &str) -> Mapping {
        Mapping {
            input: synth_key().1,
            mode: "Default".to_owned(),
            name: Some(name.to_owned()),
            actions: vec![],
        }
    }

    #[test]
    fn push_edit_appends_and_clears_redo() {
        let mut log = UndoLog::default();
        let key = synth_key();

        log.push_edit(
            key.clone(),
            synth_mapping("v1"),
            UndoKind::Rename,
            "rename: 'X' -> 'v1'".to_owned(),
        );

        let stack = &log.stacks[&key];
        assert_eq!(stack.undo.len(), 1);
        assert!(stack.redo.is_empty());
    }

    #[test]
    fn push_edit_clears_redo_stack_on_fresh_edit() {
        let mut log = UndoLog::default();
        let key = synth_key();
        log.push_edit(
            key.clone(),
            synth_mapping("a"),
            UndoKind::Rename,
            "a".to_owned(),
        );
        log.undo(&key);
        // redo now has 1 entry.
        log.push_edit(
            key.clone(),
            synth_mapping("b"),
            UndoKind::Rename,
            "b".to_owned(),
        );
        let stack = &log.stacks[&key];
        assert!(stack.redo.is_empty(), "fresh edit must clear redo");
    }

    #[test]
    fn push_edit_caps_at_fifty_with_fifo_eviction() {
        let mut log = UndoLog::default();
        let key = synth_key();
        for i in 0..60_u32 {
            log.push_edit(
                key.clone(),
                synth_mapping(&format!("v{i}")),
                UndoKind::Rename,
                format!("rename to v{i}"),
            );
        }
        let stack = &log.stacks[&key];
        assert_eq!(stack.undo.len(), 50);
        // Oldest entries (v0..v9) are evicted; the bottom of the stack is v10.
        assert_eq!(stack.undo[0].label, "rename to v10");
        assert_eq!(stack.undo[49].label, "rename to v59");
    }

    #[test]
    fn undo_pops_and_pushes_to_redo() {
        let mut log = UndoLog::default();
        let key = synth_key();
        log.push_edit(
            key.clone(),
            synth_mapping("a"),
            UndoKind::Rename,
            "a".to_owned(),
        );

        let entry = log.undo(&key).unwrap();
        assert_eq!(entry.label, "a");
        let stack = &log.stacks[&key];
        assert!(stack.undo.is_empty());
        assert_eq!(stack.redo.len(), 1);
    }

    #[test]
    fn undo_returns_none_when_stack_empty() {
        let mut log = UndoLog::default();
        let key = synth_key();
        assert!(log.undo(&key).is_none());
    }

    #[test]
    fn redo_pops_and_pushes_to_undo() {
        let mut log = UndoLog::default();
        let key = synth_key();
        log.push_edit(
            key.clone(),
            synth_mapping("a"),
            UndoKind::Rename,
            "a".to_owned(),
        );
        log.undo(&key);

        let entry = log.redo(&key).unwrap();
        assert_eq!(entry.label, "a");
        let stack = &log.stacks[&key];
        assert_eq!(stack.undo.len(), 1);
        assert!(stack.redo.is_empty());
    }

    #[test]
    fn clear_removes_both_stacks() {
        let mut log = UndoLog::default();
        let key = synth_key();
        log.push_edit(
            key.clone(),
            synth_mapping("a"),
            UndoKind::Rename,
            "a".to_owned(),
        );
        log.undo(&key);
        log.clear(&key);
        // Implementation removes the entry entirely; pin that behavior.
        assert!(!log.stacks.contains_key(&key), "clear must remove the key");
    }

    #[test]
    fn last_label_returns_top_of_undo() {
        let mut log = UndoLog::default();
        let key = synth_key();
        log.push_edit(
            key.clone(),
            synth_mapping("a"),
            UndoKind::Rename,
            "first".to_owned(),
        );
        log.push_edit(
            key.clone(),
            synth_mapping("b"),
            UndoKind::Rename,
            "second".to_owned(),
        );
        assert_eq!(log.last_label(&key).as_deref(), Some("second"));
    }

    #[test]
    fn mapping_history_isolated_across_switches() {
        // Switch A → B → A: A's undo/redo stacks must survive unchanged.
        let mut log = UndoLog::default();
        let key_a = synth_key();
        let key_b = (
            "Default".to_owned(),
            InputAddress {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Button { index: 1 },
            },
        );

        // Edit A.
        log.push_edit(
            key_a.clone(),
            synth_mapping("a1"),
            UndoKind::Rename,
            "a1".to_owned(),
        );
        log.undo(&key_a);
        // A now: undo=0, redo=1.

        // Switch to B and edit.
        log.push_edit(
            key_b.clone(),
            synth_mapping("b1"),
            UndoKind::Rename,
            "b1".to_owned(),
        );

        // Switch back to A. Verify A's stacks are intact.
        let a = &log.stacks[&key_a];
        assert_eq!(a.undo.len(), 0);
        assert_eq!(a.redo.len(), 1);
        let b = &log.stacks[&key_b];
        assert_eq!(b.undo.len(), 1);
        assert_eq!(b.redo.len(), 0);

        // Redo on A still works.
        let entry = log.redo(&key_a).unwrap();
        assert_eq!(entry.label, "a1");
    }

    // --- Task 32 tests ---

    #[test]
    fn clear_all_empties_all_stacks() {
        let mut log = UndoLog::default();
        let key_a = synth_key();
        let key_b = (
            "Default".to_owned(),
            InputAddress {
                device: DeviceId("dev-1".to_owned()),
                input: InputId::Button { index: 1 },
            },
        );
        log.push_edit(
            key_a.clone(),
            synth_mapping("a"),
            UndoKind::Rename,
            "a".to_owned(),
        );
        log.push_edit(
            key_b.clone(),
            synth_mapping("b"),
            UndoKind::Rename,
            "b".to_owned(),
        );
        assert!(
            log.has_pending_changes(),
            "should have changes before clear"
        );
        log.clear_all();
        assert!(
            log.stacks.is_empty(),
            "stacks must be empty after clear_all"
        );
        assert!(
            !log.has_pending_changes(),
            "has_pending_changes must be false after clear_all"
        );
    }

    #[test]
    fn has_pending_changes_detects_redo_only_stack() {
        let mut log = UndoLog::default();
        let key = synth_key();
        log.push_edit(
            key.clone(),
            synth_mapping("a"),
            UndoKind::Rename,
            "a".to_owned(),
        );
        // Move the entry to redo; undo stack becomes empty.
        log.undo(&key);
        // The redo stack is non-empty, so has_pending_changes must still be true.
        assert!(
            log.has_pending_changes(),
            "redo-only stack must still count as pending"
        );
    }

    // --- Task 8 tests ---

    #[test]
    fn label_format_stage_edit() {
        let label = format_undo_label(
            UndoKind::StageEdit,
            LabelArgs {
                stage_name: Some("deadzone outer"),
                field: Some("threshold"),
                before_after: Some(("92%", "95%")),
                index: None,
                from_to: None,
                old_new: None,
            },
        );
        assert_eq!(label, "deadzone outer: threshold 92% -> 95%");
    }

    #[test]
    fn label_format_stage_add() {
        let label = format_undo_label(
            UndoKind::StageAdd,
            LabelArgs {
                stage_name: Some("ResponseCurve"),
                index: Some(2),
                ..LabelArgs::default()
            },
        );
        assert_eq!(label, "add stage: ResponseCurve at index 2");
    }

    #[test]
    fn label_format_stage_remove() {
        let label = format_undo_label(
            UndoKind::StageRemove,
            LabelArgs {
                stage_name: Some("Deadzone"),
                index: Some(0),
                ..LabelArgs::default()
            },
        );
        assert_eq!(label, "remove stage: Deadzone at index 0");
    }

    #[test]
    fn label_format_stage_reorder() {
        let label = format_undo_label(
            UndoKind::StageReorder,
            LabelArgs {
                stage_name: Some("MergeAxis"),
                from_to: Some((1, 0)),
                ..LabelArgs::default()
            },
        );
        assert_eq!(label, "move stage MergeAxis from 1 to 0");
    }

    #[test]
    fn label_format_rename() {
        let label = format_undo_label(
            UndoKind::Rename,
            LabelArgs {
                old_new: Some(("X axis", "Yaw")),
                ..LabelArgs::default()
            },
        );
        assert_eq!(label, "rename: 'X axis' -> 'Yaw'");
    }

    #[test]
    fn label_format_rebind() {
        let label = format_undo_label(
            UndoKind::Rebind,
            LabelArgs {
                old_new: Some(("VPC Stick X", "VKB Pedals Y")),
                ..LabelArgs::default()
            },
        );
        assert_eq!(label, "rebind: VPC Stick X -> VKB Pedals Y");
    }
}
