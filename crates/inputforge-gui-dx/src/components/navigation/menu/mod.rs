//! Menu markup and placement; application bindings own interaction policy.
use crate::components::{ClickAwayListener, Portal, merge_class};
use crate::overlays::CloseReason;
use dioxus::prelude::*;
use std::sync::atomic::AtomicU64;
mod anchored;
mod items;
mod placement;
mod root;
pub use anchored::{AnchoredMenu, AnchoredMenuProps};
pub use items::{MenuItems, MenuItemsProps};
pub use placement::{Anchor, MenuAnchor};
use placement::{MenuPlacement, MenuSize, TriggerRect, Viewport, compute_placement};
pub use root::{MenuItem, MenuItemProps, MenuRoot, MenuRootProps, MenuTrigger, MenuTriggerProps};
static MENU_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy)]
struct MenuState {
    open: Signal<bool>,
    menu_id: Signal<String>,
    trigger_id: Signal<String>,
    behavior: crate::overlays::MenuView,
}
