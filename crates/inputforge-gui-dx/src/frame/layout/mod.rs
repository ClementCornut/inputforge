mod empty_state;

use dioxus::prelude::*;

use crate::context::AppContext;
use crate::frame::banner::Banner;
use crate::frame::panel_slot::PanelSlot as PanelSlotComponent;
use crate::frame::status_bar::StatusBar;
use crate::frame::top_bar::TopBar;
use crate::frame::view_state::ViewState;

pub(crate) use empty_state::EmptyState;

/// F7 layout shell: top bar, conditional banner, main row (rail + center +
/// panel slot) or empty-state, status bar.
#[component]
pub(crate) fn Layout() -> Element {
    let ctx = use_context::<AppContext>();
    // Calling `use_context::<ViewState>()` here is a structural panic guard:
    // every region component (Banner, ModeTabs, ToolsCluster, etc.) reads
    // ViewState via `use_context`, and Dioxus panics with an opaque error
    // if no provider is in scope. Failing here keeps the panic readable
    // ("ViewState provider missing in app_root") and centralized.
    let _view_state_panic_guard = use_context::<ViewState>();
    let has_profile = use_memo(move || ctx.meta.read().profile_name.is_some());
    let p = *has_profile.read();

    rsx! {
        div { class: "if-layout",
            TopBar {}
            Banner {}
            if p {
                div { class: "if-layout__main",
                    div { class: "if-layout__rail", "Mapping list — F8 owns content" }
                    div { class: "if-layout__center", "Mapping editor — F9 owns content" }
                    PanelSlotComponent {}
                }
            } else {
                EmptyState {}
            }
            StatusBar {}
        }
    }
}
