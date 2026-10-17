//! Application-owned overlay policy and document lifecycle contracts.
use dioxus::prelude::*;
/// Why a menu requests closure; Tab preserves the browser's focus traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    Escape,
    ClickOutside,
    Tab,
    ItemActivated,
}
/// Focus traversal requested by the app's keyboard interpreter.
#[derive(Debug, Clone, Copy)]
pub enum FocusAction {
    Next,
    Prev,
    First,
    Last,
}
/// Engine-independent overlay hooks, supplied through the controls provider.
#[derive(Clone, Copy, Debug)]
pub struct Bindings {
    pub menu: fn(MenuProps) -> MenuView,
    pub focus: fn(bool, bool, String),
    pub idle_escape: fn(ReadSignal<bool>, EventHandler<()>),
    pub click_away: fn(String, Option<String>, EventHandler<()>),
    pub dialog: fn(DialogProps) -> DialogView,
    pub resolve: Callback<(Signal<bool>, Callback<(), bool>)>,
}
/// Current root/menu observations; the renderer owns the open signal.
#[derive(Clone, Debug)]
pub struct MenuProps {
    pub open: Signal<bool>,
    pub trigger: String,
    pub onclose: Option<Callback<CloseReason>>,
}
/// App-selected event handlers for menu rendering.
#[derive(Clone, Copy, Debug)]
pub struct MenuView {
    pub toggle: EventHandler<MouseEvent>,
    pub close: Callback<CloseReason>,
    pub key: Callback<(KeyboardEvent, String)>,
    pub activate: Callback<Activation>,
}
/// Controlled dialog observations and acknowledged dismissal.
#[derive(Clone, Debug)]
pub struct DialogProps {
    pub open: Signal<bool>,
    pub id: String,
    pub dismissible: bool,
    pub backdrop: bool,
    /// Return true only when dismissal resolved; false retains the dialog and feedback.
    pub onclose: Callback<(), bool>,
}
/// Dialog event handlers; the renderer forwards native observations.
#[derive(Clone, Copy, Debug)]
pub struct DialogView {
    pub cancel: EventHandler<Event<CancelData>>,
    pub backdrop: EventHandler<MouseEvent>,
    pub panel: EventHandler<MouseEvent>,
}

/// One native item activation and its current consumer callbacks.
#[derive(Clone, Debug)]
pub struct Activation {
    pub event: MouseEvent,
    pub disabled: bool,
    pub onclick: Option<EventHandler<MouseEvent>>,
    pub onactivate: Option<Callback<MouseEvent, bool>>,
}
