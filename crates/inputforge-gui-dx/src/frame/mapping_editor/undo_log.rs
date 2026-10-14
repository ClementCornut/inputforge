//! Stage paths and undo-label formatting retained by rendering.
pub(crate) use crate::mapping_edits::{StageId, StageIdSegment, UndoKind, View as UndoLog};

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
