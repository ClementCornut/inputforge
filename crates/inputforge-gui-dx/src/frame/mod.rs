//! F7 application frame: top bar, banner, status bar, panel slot, layout.

mod banner;
mod layout;
mod panel_slot;
mod status_bar;
mod top_bar;
mod view_state;

pub(crate) use layout::Layout;
// `PanelSlot` and `ViewState` are not re-exported here — every consumer
// imports them directly via `crate::frame::view_state::*` so a single
// path style stays consistent across regions.
pub(crate) use view_state::use_view_state_provider;
