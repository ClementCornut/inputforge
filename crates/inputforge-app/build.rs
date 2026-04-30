// Rust guideline compliant 2026-04-26
//
// Build script for inputforge-app.
//
// Purpose: copy the SDL3 runtime DLL into both the cargo target output directory
// and the dx (dioxus-cli) bundled output directory so that running the binary via
// `cargo run` or `dx run` does not require a manually adjusted PATH.
//
// The sdl3 Rust crate's own build script may already drop SDL3.dll next to the
// cargo target binary, but we copy it defensively to guarantee placement and to
// also reach the dx output directory which the sdl3 crate has no knowledge of.
//
// Scope: Windows only, and only when an SDL/ source directory exists at the
// workspace root. Soft failures are reported via cargo:warning so the build
// still succeeds; the runtime dynamic loader will surface any actual problem.

fn main() {
    // Only Windows needs runtime DLL placement; on other platforms SDL3 is loaded
    // through the system loader (e.g., libSDL3.so on Linux, libSDL3.dylib on macOS).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo");
    let manifest_dir = std::path::Path::new(&manifest_dir);

    // Workspace root resolution: crates/inputforge-app/ -> crates/ -> workspace root.
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("cannot resolve workspace root from CARGO_MANIFEST_DIR");

    let sdl3_src = workspace_root.join("SDL").join("SDL3.dll");

    // Re-run the build script when the source DLL changes (e.g., after upgrading
    // SDL3) to force a fresh copy. If the file does not exist yet the rerun marker
    // will trigger when it appears.
    println!("cargo:rerun-if-changed={}", sdl3_src.display());

    if !sdl3_src.exists() {
        // The inputforge-core crate does not directly enable the sdl3-input feature
        // for inputforge-app, so we cannot reliably gate on CARGO_FEATURE_SDL3_INPUT
        // here. We instead treat the absence of SDL/SDL3.dll as the disable signal:
        // a workspace built without SDL3 simply skips the copy step.
        println!(
            "cargo:warning=SDL3.dll not found at {}; \
             gui-dioxus and SDL3-backed runtimes will fail to load the DLL at runtime. \
             Place SDL3 libraries at <workspace_root>/SDL/.",
            sdl3_src.display()
        );
        return;
    }

    // Derive `target/<profile>` from OUT_DIR.
    // OUT_DIR shape: target/<profile>/build/<crate>-<hash>/out
    // Walking 3 ancestors yields target/<profile>.
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let Some(cargo_target_profile) = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .map(std::path::Path::to_path_buf)
    else {
        println!(
            "cargo:warning=could not derive target/<profile> from OUT_DIR={out_dir}; \
             SDL3.dll will not be copied"
        );
        return;
    };

    // Copy 1: cargo target dir. Defensive, the sdl3 Rust crate may already place
    // it here, but we guarantee it. `fs::copy` overwrites the destination, so no
    // existence check is needed.
    let cargo_dst = cargo_target_profile.join("SDL3.dll");
    // Re-run if the destination is missing or modified, so manual deletion of the
    // copy triggers a fresh copy on the next build instead of silently leaving
    // the binary unable to load SDL3 at runtime.
    println!("cargo:rerun-if-changed={}", cargo_dst.display());
    if let Err(e) = std::fs::copy(&sdl3_src, &cargo_dst) {
        println!(
            "cargo:warning=failed to copy SDL3.dll to {}: {e}",
            cargo_dst.display()
        );
    }

    // Copy 2: dx (dioxus-cli) bundled output dir. dx runs the binary from
    // target/dx/<package>/<dx_profile>/windows/app/, which is outside the cargo
    // target tree and therefore does not get the sdl3 crate's own DLL drop.
    //
    // dx names its output dirs `debug` and `release` regardless of the
    // underlying cargo profile name. dx run with the default `desktop-dev`
    // cargo profile (which inherits from `dev`) still produces a bundle at
    // .../debug/windows/app/, NOT .../desktop-dev/windows/app/. Use cargo's
    // `PROFILE` env var, which collapses custom profiles to "debug" or
    // "release" based on inheritance, instead of the cargo profile name.
    //
    // CARGO_TARGET_DIR caveat: this path assumes the dx output lives under
    // `<workspace_root>/target/`. dioxus-cli 0.7.6 hardcodes `target/` relative
    // to the workspace and does not honor CARGO_TARGET_DIR for its bundle
    // output, so honoring it here would put the DLL where dx will not look.
    // The cargo-side copy above derives its target via OUT_DIR ancestry, so it
    // does honor CARGO_TARGET_DIR automatically. If a future dx-cli respects
    // CARGO_TARGET_DIR for bundle output, update both this path and the
    // recovery instructions in `crates/inputforge-gui-dx/README.md`.
    let dx_profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_owned());
    let dx_app_dir = workspace_root
        .join("target")
        .join("dx")
        .join("inputforge-app")
        .join(&dx_profile)
        .join("windows")
        .join("app");

    if let Err(e) = std::fs::create_dir_all(&dx_app_dir) {
        println!(
            "cargo:warning=cannot create dx output directory {}: {e}",
            dx_app_dir.display()
        );
        return;
    }

    let dx_dst = dx_app_dir.join("SDL3.dll");
    // Re-run if the dx-side copy is missing, same rationale as the cargo-side
    // rerun above. Especially relevant if `target/dx/` is wiped independently of
    // `target/<profile>/` (e.g., a future dx-cli upgrade reorganizes its output).
    println!("cargo:rerun-if-changed={}", dx_dst.display());
    if let Err(e) = std::fs::copy(&sdl3_src, &dx_dst) {
        println!(
            "cargo:warning=failed to copy SDL3.dll to dx output dir {}: {e}",
            dx_dst.display()
        );
    }
}
