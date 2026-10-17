//! Shared sortable gestures, mounted cleanup and drop interpretation.
mod gap;
mod handle;
mod owner;
mod pipeline;
use inputforge_gui_dx::{mapping_edits::StageId, sortable::Bindings};
/// Hooks for mapping cards, including cross-group rejection.
#[must_use]
pub fn rail() -> Bindings<u32> {
    Bindings {
        state: || owner::use_state(rail()),
        handle: handle::use_handle,
        gap: |props| gap::use_gap(props, |_, a, b| a == b),
    }
}
/// Hooks for nested action pipelines, including cycle rejection.
#[must_use]
pub fn pipeline() -> Bindings<StageId> {
    Bindings {
        state: || owner::use_state(pipeline()),
        handle: handle::use_handle,
        gap: |props| gap::use_gap(props, pipeline::eligible),
    }
}
#[doc(inline)]
pub use pipeline::{
    drop_pipeline, gap_destination, gap_to_post_remove_slot, is_descendant, validate_pipeline_drop,
};

/// Shared acknowledgement phrase for keyboard, menu and pointer reorder paths.
#[must_use]
pub fn rail_announcement(
    (index, len, group): (usize, usize, inputforge_gui_dx::mapping_rail::Group),
) -> String {
    use inputforge_gui_dx::mapping_rail::Group;
    let group = match group {
        Group::Axes => "axes",
        Group::Buttons => "buttons",
        Group::Hats => "hats",
    };
    format!(
        "Mapping moved to position {} of {} in {}.",
        index + 1,
        len,
        group
    )
}
