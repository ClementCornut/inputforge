// Rust guideline compliant 2026-03-03

fn main() {
    println!("cargo:rerun-if-env-changed=INPUTFORGE_SDL_DIR");
    println!("cargo:rerun-if-env-changed=VJOY_SDK_DIR");

    // SDL3 library search path (only needed when sdl3-input feature is enabled)
    if std::env::var("CARGO_FEATURE_SDL3_INPUT").is_ok() {
        let sdl_dir = if let Ok(dir) = std::env::var("INPUTFORGE_SDL_DIR") {
            std::path::PathBuf::from(dir)
        } else {
            let manifest_dir =
                std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
            std::path::Path::new(&manifest_dir)
                .parent()
                .and_then(|p| p.parent())
                .expect("cannot resolve workspace root")
                .join("SDL")
        };

        if sdl_dir.exists() {
            println!("cargo:rustc-link-search=native={}", sdl_dir.display());
        } else {
            println!(
                "cargo:warning=SDL directory not found at {}. \
                 Set INPUTFORGE_SDL_DIR or place SDL3 libraries at the workspace root/SDL.",
                sdl_dir.display()
            );
        }
    }

    // vJoy SDK library search path
    if let Ok(vjoy_dir) = std::env::var("VJOY_SDK_DIR") {
        let vjoy_path = std::path::Path::new(&vjoy_dir);
        if vjoy_path.exists() {
            println!("cargo:rustc-link-search=native={vjoy_dir}");
        } else {
            println!("cargo:warning=VJOY_SDK_DIR ({vjoy_dir}) does not exist");
        }
    }
}
