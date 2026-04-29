//! F7 application frame: top bar, banner, status bar, panel slot, layout.

mod banner;
mod layout;
mod panel_slot;
mod status_bar;
mod top_bar;
mod view_state;

#[allow(
    unused_imports,
    reason = "Layout exported for use in app_root (Task 18)"
)]
pub(crate) use layout::Layout;
#[allow(
    unused_imports,
    reason = "ViewState types exported for app_root provider (Task 18)"
)]
pub(crate) use view_state::{PanelSlot, ViewState, use_view_state_provider};
