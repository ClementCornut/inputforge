//! Existing mapping presentation destinations and app-owned native handlers.
use crate::{
    capture::Capture,
    mapping_rail::{AddIntent, AddView, Menu, Scope, Target},
};
use dioxus::prelude::*;
use inputforge_core::{action::Mapping, profile::ProfileId, types::InputAddress};
/// Header rename publications.
#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub armed: Signal<bool>,
    pub value: Signal<String>,
    pub draft: Signal<Mapping>,
    pub error: Signal<Option<String>>,
}
/// Header native event observation.
#[derive(Clone, Debug)]
pub enum HeaderEvent {
    ArmKey(KeyboardEvent),
    ArmMouse(MouseEvent),
    Input(String),
    Key(KeyboardEvent),
    Blur,
}
/// Captured current target and header presentation destinations.
#[derive(Clone, Debug)]
pub struct HeaderRequest {
    pub publication: Header,
    pub profile: Option<ProfileId>,
    pub target: Mapping,
    pub event: HeaderEvent,
}
/// Inline rail rename publications, retaining its original full target.
#[derive(Clone, Debug)]
pub struct Rename {
    pub value: Signal<String>,
    pub error: Signal<Option<String>>,
    pub open: Signal<Option<InputAddress>>,
    pub target: Option<Target>,
}
/// Rail rename native event observation.
#[derive(Clone, Debug)]
pub enum RenameEvent {
    Key(KeyboardEvent),
    Input(String),
    Commit,
}
/// Rail menu and confirmation presentation destinations.
#[derive(Clone, Copy, Debug)]
pub struct Rail {
    pub filter: Filter,
    pub prepared: Signal<Option<Menu>>,
    pub menu: Signal<Option<(InputAddress, f64, f64)>>,
    pub rename: Signal<Option<InputAddress>>,
    pub duplicate: Signal<Option<Target>>,
    pub delete: Signal<Option<Target>>,
    pub error: Signal<Option<String>>,
    pub live: Signal<String>,
}
/// Rail menu/confirmation operation.
#[derive(Clone, Debug)]
pub enum RailEvent {
    Close,
    Open {
        lookup: crate::mapping_rail::Lookup,
        x: f64,
        y: f64,
    },
    Step {
        menu: Menu,
        down: bool,
    },
    Drop {
        drag: Signal<Option<Menu>>,
        group: crate::mapping_rail::Group,
        gap: usize,
    },
    Rename(Target),
    Duplicate(Target),
    BeginDelete(Target),
    Dispatch(crate::mapping_rail::Request),
    MenuDispatch(crate::mapping_rail::Request),
}
/// Add-pad presentation destinations and the existing capture callbacks.
#[derive(Clone, Copy, Debug)]
pub struct Add {
    pub state: Signal<AddView>,
    pub name: Signal<String>,
    pub capture: Capture,
}
/// Observations into the existing app add-pad state machine.
#[derive(Clone, Debug)]
pub enum AddEvent {
    Intent(AddIntent),
    Begin {
        scope: Scope,
        source: Option<Target>,
    },
    Observe {
        scope: Scope,
        pending: Signal<Option<Target>>,
        expanded: Signal<bool>,
    },
    Reconcile,
    Key(KeyboardEvent),
    Input(String),
    Commit,
    Cancel,
}
/// App callbacks for existing editor and rail presentation.
#[derive(Clone, Copy, Debug)]
pub struct Bindings {
    pub row_menu: Callback<RowMenuRequest>,
    pub drag: Callback<DragRequest>,
    pub header: Callback<HeaderRequest>,
    pub rename: Callback<(Rename, RenameEvent), Result<(), String>>,
    pub delete: Callback<DeleteRequest, bool>,
    pub rail: Callback<(Rail, RailEvent), bool>,
    pub add_state: Callback<(Signal<AddView>, Signal<String>, AddIntent), bool>,
    pub add: Callback<(Add, AddEvent)>,
    pub combo_sync: Callback<(
        inputforge_core::types::KeyCombo,
        bool,
        Signal<inputforge_core::types::KeyCombo>,
        Signal<inputforge_core::types::KeyCombo>,
    )>,
    pub stage_hint: Callback<super::hints::Request, super::hints::View>,
    pub stage_close: Callback<(
        Signal<Option<crate::navigation::StageMenu>>,
        Signal<bool>,
        crate::overlays::CloseReason,
    )>,
}

/// Row context-menu native observation and supplied anchor publication.
#[derive(Clone, Debug)]
pub struct RowMenuRequest {
    pub input: InputAddress,
    pub event: MouseEvent,
    pub send: Callback<(InputAddress, f64, f64)>,
}
/// Existing mapping confirmation observation and feedback destinations.
#[derive(Clone, Copy, Debug)]
pub struct DeleteRequest {
    pub target: Signal<Option<Target>>,
    pub error: Signal<Option<String>>,
    pub confirm: bool,
}

/// Raw current filter publications; interpretation belongs to the app.
#[derive(Clone, Copy, Debug)]
pub struct Filter {
    pub query: Signal<String>,
    pub device: Signal<Option<inputforge_core::types::DeviceId>>,
}
/// Current drag observations and supplied target publication.
#[derive(Clone, Debug)]
pub struct DragRequest {
    pub publication: Signal<Option<Menu>>,
    pub lookup: crate::mapping_rail::Lookup,
    pub filter: Filter,
}
