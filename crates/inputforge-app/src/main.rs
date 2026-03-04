// Rust guideline compliant 2026-03-03

//! `InputForge` — desktop application entry point.
//!
//! Sets up logging, creates an empty shared application state, and
//! launches the egui GUI window. The engine thread is not yet wired
//! — this serves as a visual preview of the GUI shell.

use std::sync::Arc;
use std::sync::mpsc;

use anyhow::Result;
use mimalloc::MiMalloc;
use parking_lot::RwLock;

use inputforge_core::state::AppState;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let state = Arc::new(RwLock::new(AppState::new()));
    let (tx, _rx) = mpsc::channel();

    inputforge_gui::launch_gui(state, tx).map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}
