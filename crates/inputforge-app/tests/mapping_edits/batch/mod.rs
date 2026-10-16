mod dispatch;
mod lifecycle;
mod policy;
mod reconciliation;
mod support;
use inputforge_core::{
    action::Action,
    engine::EngineCommand,
    types::{OutputId, VJoyAxis},
};
use inputforge_gui_dx::batch_mapping::{Intent, RowKind};
use support::*;
mod mode_transition;
