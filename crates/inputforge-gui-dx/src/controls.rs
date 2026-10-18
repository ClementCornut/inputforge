//! App-supplied behavior for reusable controls and overlay renderers.
use crate::components::{IntegerInputError, TabsOrientation};
use dioxus::prelude::*;
/// Engine-independent hooks and event callbacks installed by the host.
#[derive(Clone, Copy, Debug)]
pub struct Bindings {
    pub rail_sortable: crate::sortable::Bindings<u32>,
    pub pipeline_sortable: crate::sortable::Bindings<crate::mapping_edits::StageId>,
    pub rail_announcement: Callback<(usize, usize, crate::mapping_rail::Group), String>,
    pub pipeline_drop: Callback<crate::sortable::PipelineDrop>,
    pub overlays: crate::overlays::Bindings,
    pub number: fn(&NumberProps) -> NumberView,
    pub integer: fn(&IntegerProps) -> InputView,
    pub text: fn(TextProps) -> TextView,
    pub text_key: Callback<(KeyboardEvent, bool, Option<EventHandler<KeyboardEvent>>)>,
    pub segmented_click: Callback<(MouseEvent, bool, Option<EventHandler<MouseEvent>>)>,
    pub tabs: fn(TabsProps) -> TabsView,
}
/// Browser-owned draft observations and the consumer's input callback.
#[derive(Clone, Copy, Debug)]
pub struct TextProps {
    pub value: ReadSignal<String>,
    pub disabled: bool,
    pub oninput: Option<EventHandler<FormEvent>>,
}
/// Stable mounted identity and app-owned input acknowledgement handling.
#[derive(Clone, Debug)]
pub struct TextView {
    pub owner: String,
    pub input: EventHandler<FormEvent>,
}
/// Numeric rendering observations and consumer callbacks.
#[derive(Clone, Debug)]
pub struct NumberProps {
    pub value: ReadSignal<f64>,
    pub display: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub disabled: bool,
    pub oninput: Option<EventHandler<FormEvent>>,
    pub oncommit: Option<EventHandler<f64>>,
    pub onstep: Option<EventHandler<f64>>,
}
/// Integer rendering observations and consumer callbacks.
#[derive(Clone, Debug)]
pub struct IntegerProps {
    pub value: ReadSignal<usize>,
    pub display: String,
    pub min: usize,
    pub max: usize,
    pub disabled: bool,
    pub oninput: Option<EventHandler<FormEvent>>,
    pub oncommit: Option<EventHandler<usize>>,
    pub oninvalid: Option<EventHandler<IntegerInputError>>,
}
/// Raw text for rendering and app-owned input handlers.
#[derive(Clone, Copy, Debug)]
pub struct InputView {
    pub text: ReadSignal<String>,
    pub input: EventHandler<FormEvent>,
    pub key: EventHandler<KeyboardEvent>,
    pub blur: EventHandler<FocusEvent>,
    pub focus: EventHandler<FocusEvent>,
}
/// Numeric input view including increment/decrement observations.
#[derive(Clone, Copy, Debug)]
pub struct NumberView {
    pub input: InputView,
    pub step: EventHandler<bool>,
}
/// Root tab observations; mounted DOM references remain in the renderer.
#[derive(Clone, Copy, Debug)]
pub struct TabsProps {
    pub value: ReadSignal<String>,
    pub disabled: ReadSignal<bool>,
    pub onchange: EventHandler<String>,
}
/// Current mounted tab identity and interaction availability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabState {
    pub id: String,
    pub disabled: bool,
}
/// Prepared focus instruction and request acknowledgement.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TabFocus {
    pub target: Option<String>,
    pub consumed: bool,
}
/// Application coordination callbacks shared by tab leaves and lists.
#[derive(Clone, Copy, Debug)]
pub struct TabsView {
    pub pending: ReadSignal<Option<String>>,
    pub key: Callback<(KeyboardEvent, Vec<TabState>, TabsOrientation)>,
    pub click: Callback<(String, bool)>,
    pub focus: Callback<(Option<String>, Vec<TabState>), TabFocus>,
}
