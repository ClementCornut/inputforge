// Rust guideline compliant 2026-03-07

//! `InputForge` application entry point.
//!
//! Parses portable command-line arguments before dispatching to the current
//! platform. Windows owns the desktop runtime; Linux exits during Slice 1
//! preflight before constructing unavailable input or output backends.

mod cli;
mod platform;
#[cfg(target_os = "windows")]
mod tray;
#[cfg(target_os = "windows")]
mod windows_app;

use anyhow::Result;
use clap::Parser;
use mimalloc::MiMalloc;

use crate::cli::Cli;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    run(cli)
}

#[cfg(target_os = "linux")]
fn run(_cli: Cli) -> Result<()> {
    platform::preflight()
}

#[cfg(target_os = "windows")]
fn run(cli: Cli) -> Result<()> {
    platform::preflight()?;
    windows_app::run(cli)
}
