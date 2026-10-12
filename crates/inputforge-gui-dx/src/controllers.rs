//! Prepared controller rendering data and app-owned event callbacks.
use dioxus::prelude::*;
#[cfg(not(test))]
use inputforge_core::{
    output::traits::ControllerCapabilities,
    types::{DeviceId, VJoyAxis, VirtualDeviceConfig},
};

#[cfg(test)]
pub use inputforge_gui_dx::controllers::{Action, Bindings, Selection, View};

#[cfg(not(test))]
/// Controller operation requested by a rendered control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Change the routed controller selection.
    Select { device: DeviceId, selected: bool },
    /// Add the first free virtual slot.
    Add,
    /// Remove a virtual slot.
    Remove { slot: u8 },
    /// Commit a button-count field.
    Buttons { slot: u8, value: String },
    /// Commit a hat-count field.
    Hats { slot: u8, value: String },
    /// Enable or disable a virtual axis.
    Axis {
        slot: u8,
        axis: VJoyAxis,
        enabled: bool,
    },
    /// Apply a pending retained-output layout.
    Apply,
}
#[cfg(not(test))]
/// Prepared selection row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Stable controller identity.
    pub device: DeviceId,
    /// Controller display name.
    pub label: String,
    /// Whether the controller is currently connected.
    pub connected: bool,
    /// Whether the controller is selected for routing.
    pub selected: bool,
    /// Whether its selection control is available.
    pub enabled: bool,
}
#[cfg(not(test))]
/// App-prepared controller values and availability.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent prepared control availability and retained-output presentation"
)]
pub struct View {
    /// Selection and Apply controls may be used.
    pub editable: bool,
    /// Driver supports editing virtual capabilities in the current state.
    pub layout_editable: bool,
    /// Retained outputs need explicit reconfiguration.
    pub pending: bool,
    /// Apply is currently available.
    pub can_apply: bool,
    /// A virtual slot can be added.
    pub can_add: bool,
    /// Stopped routing retains connected neutral controllers.
    pub retained: bool,
    /// Driver-provided field limits and layout ownership.
    pub capabilities: ControllerCapabilities,
    /// Controller selection rows.
    pub selections: Vec<Selection>,
    /// Requested or native driver layouts to render.
    pub slots: Vec<VirtualDeviceConfig>,
}
#[cfg(not(test))]
/// App dispatch supplied by the editor host.
#[derive(Clone, Copy, Debug)]
pub struct Bindings {
    /// Deliver an operation and return prepared error feedback, or clear it on success.
    pub dispatch: Callback<Action, Option<String>>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Context {
    pub view: Signal<View>,
    pub error: Signal<Option<String>>,
    pub dispatch: Callback<Action>,
}
#[cfg(test)]
pub(crate) fn provide_fixture(view: View) {
    let view = use_signal(|| view);
    let error = use_signal(|| None);
    use_context_provider(|| Context {
        view,
        error,
        dispatch: Callback::new(|_| {}),
    });
}
