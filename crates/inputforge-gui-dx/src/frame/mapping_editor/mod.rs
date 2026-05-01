// Rust guideline compliant 2026-05-01

//! F9 mapping editor (center column). See
//! `docs/superpowers/specs/2026-04-30-f9-mapping-editor-design.md`.
//!
//! This is a stub; further sub-modules land in Tasks 7+.

#![allow(
    dead_code,
    reason = "Sub-modules expose APIs that the orchestrator + Tasks 12+ consume; \
              clippy's reachability check loses some pub(crate) items here."
)]

pub(crate) mod undo_log;
