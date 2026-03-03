// Rust guideline compliant 2026-03-03

fn main() {
    // Emit linker search path for SDL3 libraries.
    // The SDL directory is at the workspace root, two levels up from this crate.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let sdl_dir = std::path::Path::new(&manifest_dir)
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("cannot resolve workspace root")
        .join("SDL");

    if sdl_dir.exists() {
        println!("cargo:rustc-link-search=native={}", sdl_dir.display());
    }
}
