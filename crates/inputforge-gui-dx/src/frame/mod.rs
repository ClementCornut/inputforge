//! F7 application frame: top bar, banner, status bar, panel slot, layout.

mod banner;
mod bulk_map;
mod layout;
mod mapping_editor;
mod mapping_list;
mod panel_slot;
mod status_bar;
mod top_bar;
mod view_state;

pub(crate) use layout::Layout;
pub(crate) use mapping_editor::{MappingEditor, use_editor_state_provider};
pub(crate) use mapping_list::MappingList;
// `PanelSlot` and `ViewState` are not re-exported here, every consumer
// imports them directly via `crate::frame::view_state::*` so a single
// path style stays consistent across regions.
// `MappingKey` is re-exported here for Task 3+ consumers (ConfigSnapshot,
// UndoLog, EditorState). Until those tasks land it has no importers.
#[allow(
    unused_imports,
    reason = "Forward-exported for Task 3+ consumers (ConfigSnapshot, UndoLog, EditorState)."
)]
pub(crate) use view_state::{MappingKey, ViewState, use_view_state_provider};
