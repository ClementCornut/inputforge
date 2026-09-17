//! Standalone read-only Linux diagnostics; independent of the desktop application.
#[cfg(target_os = "linux")]
mod report;

fn main() -> anyhow::Result<()> {
    let _matches = clap::Command::new("inputforge-diagnostics")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Print read-only Linux input discovery diagnostics")
        .get_matches();
    run()
}

#[cfg(target_os = "linux")]
fn run() -> anyhow::Result<()> {
    report::run_with(
        inputforge_core::device::evdev::discover,
        std::io::stdout().lock(),
    )
}

#[cfg(not(target_os = "linux"))]
fn run() -> anyhow::Result<()> {
    anyhow::bail!("InputForge input diagnostics are only available on Linux")
}
