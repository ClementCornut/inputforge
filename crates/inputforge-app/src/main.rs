// Rust guideline compliant 2026-03-07

//! `InputForge` application entry point.
//!
//! Parses portable command-line arguments before dispatching to the current
//! platform. Hardware sessions begin only after an explicit engine command.

mod cli;
mod desktop;
mod desktop_host;
mod engine_thread;
mod platform;
mod routing;
mod single_instance;
mod tray;

use anyhow::Result;
use clap::Parser;
use mimalloc::MiMalloc;

use crate::cli::Cli;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let Some(instance) =
        single_instance::Instance::acquire(&inputforge_core::settings::AppSettings::config_dir())?
    else {
        return Ok(());
    };
    // Keep ownership outside GUI scopes until engine shutdown has completed.
    let instance = std::sync::Arc::new(instance);
    desktop::run(&cli, std::sync::Arc::clone(&instance))
}
