// Rust guideline compliant 2026-05-03

//! Placeholder for Task 10. The full keyboard handler (and the real
//! `KeyKind` variants) lands then; this stub exists only so `state.rs` can
//! reference `KeyKind` for `NudgeCoalesce<KeyKind>` initialization.
//!
//! An empty enum compiles with `Copy + Eq` (auto-implemented on uninhabited
//! types), and `NudgeCoalesce::<KeyKind>::default()` only ever stores
//! `Option<KeyKind> = None`, so no `KeyKind` value is constructed here.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyKind {}
