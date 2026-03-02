// InputForge App - binary entry point with system tray
// Rust guideline compliant 2026-03-02

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[expect(clippy::print_stdout, reason = "CLI version output is intentional")]
fn main() {
    println!("InputForge v{}", env!("CARGO_PKG_VERSION"));
}
