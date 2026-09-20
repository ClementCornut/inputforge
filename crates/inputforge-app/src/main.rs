// Rust guideline compliant 2026-03-07

//! `InputForge` application entry point.
//!
//! Parses portable command-line arguments before dispatching to the current
//! platform. Hardware sessions begin only after an explicit engine command.

mod cli;
mod desktop;
mod engine_thread;
mod platform;
#[cfg(target_os = "windows")]
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
    desktop::run(&cli)
}
