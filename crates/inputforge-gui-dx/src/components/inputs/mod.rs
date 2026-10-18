//! Form controls that render values and forward observations to application bindings.

pub(super) mod capture;
pub(super) mod checkbox;
pub(super) mod inline_text_editor;
pub(super) mod integer_input;
pub(super) mod native_number;
pub(super) mod number_input;
pub(super) mod radio;
pub(super) mod segmented_control;
pub(super) mod select;
pub(super) mod slider;
pub(super) mod switch;
pub(super) mod text_input;

#[cfg(test)]
mod audit_tests;

#[cfg(test)]
mod metadata_tests;

#[cfg(test)]
mod event_tests;

#[cfg(test)]
mod presentation_tests;
