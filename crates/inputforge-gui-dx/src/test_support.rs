//! Engine setup for unit fixtures; never part of the renderer contract.
use dioxus::prelude::*;
use inputforge_core::{engine::EngineCommand, state::AppState};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc};

#[derive(Clone, Debug)]
pub(crate) struct Fixture {
    pub state: Arc<RwLock<AppState>>,
    pub commands: mpsc::Sender<EngineCommand>,
}

pub(crate) fn provide_engine(state: Arc<RwLock<AppState>>, commands: mpsc::Sender<EngineCommand>) {
    provide_context(Fixture { state, commands });
}

pub(crate) fn engine() -> Fixture {
    consume_context::<Fixture>()
}
