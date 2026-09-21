//! Structured mapping diagnostics; presentation belongs to the GUI.
use crate::{
    output::OutputFailure,
    types::{InputAddress, OutputAddress},
};

/// An editable mapping which cannot currently route safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingIssue {
    pub input: InputAddress,
    pub mode: String,
    pub reason: MappingIssueReason,
}

/// Where an unavailable input is used in the mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputRole {
    Primary,
    MergeAxis,
    Condition,
}

/// Actionable causes of an input dependency failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputIssueKind {
    Unbound,
    Unselected,
    Disconnected,
    NotReady,
    MissingControl,
    ChangedControl,
    WrongKind,
}

/// Actionable causes of a virtual output failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputIssueKind {
    MissingController,
    MissingControl,
    Incompatible,
}

/// Reason one mapping is disabled, including the affected dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingIssueReason {
    Input {
        address: InputAddress,
        role: InputRole,
        problem: InputIssueKind,
    },
    Output {
        address: OutputAddress,
        problem: OutputIssueKind,
    },
    KeyboardUnavailable,
    MouseUnavailable,
    InjectionFailed {
        failure: OutputFailure,
    },
    InvalidCondition {
        input: InputAddress,
    },
    MissingMode {
        mode: String,
    },
    /// Structural validation details for diagnostics, not default user-facing copy.
    InvalidActions {
        details: String,
    },
}
