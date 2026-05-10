//! Windows backend: writes HKCU\Software\Microsoft\Windows\CurrentVersion\Run
//! via the `auto-launch` crate.

#![cfg(target_os = "windows")]

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

use crate::{AutostartError, AutostartManager};

#[allow(dead_code, reason = "used by the factory in Task 1.8; not yet wired")]
const APP_NAME: &str = "InputForge";

#[allow(dead_code, reason = "used by the factory in Task 1.8; not yet wired")]
#[derive(Debug)]
pub(crate) struct WindowsAutostart {
    app_path: String,
}

impl WindowsAutostart {
    /// Resolve the absolute exe path once at construction.
    ///
    /// # Errors
    ///
    /// Returns [`AutostartError::NotSupported`] when `std::env::current_exe`
    /// fails (rare; e.g., AppImage-style mounts on non-Windows, kept here
    /// for symmetry).
    #[allow(dead_code, reason = "called by the factory in Task 1.8; not yet wired")]
    pub(crate) fn new() -> Result<Self, AutostartError> {
        let exe = std::env::current_exe().map_err(|_e| AutostartError::NotSupported)?;
        let app_path = exe.to_str().ok_or(AutostartError::NotSupported)?.to_owned();
        Ok(Self { app_path })
    }

    fn build(&self, args: &[&str]) -> AutoLaunch {
        let owned_args: Vec<String> = args.iter().map(|&s| s.to_owned()).collect();
        AutoLaunchBuilder::new()
            .set_app_name(APP_NAME)
            .set_app_path(&self.app_path)
            .set_args(&owned_args)
            .build()
            .expect("WindowsAutostart: AutoLaunchBuilder::build cannot fail with valid app_path")
    }
}

impl AutostartManager for WindowsAutostart {
    fn is_enabled(&self) -> Result<bool, AutostartError> {
        self.build(&[])
            .is_enabled()
            .map_err(|e| AutostartError::Backend(e.to_string()))
    }

    fn set_enabled(&mut self, enabled: bool, args: &[&str]) -> Result<(), AutostartError> {
        let launcher = self.build(args);
        let result = if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        };
        result.map_err(|e| AutostartError::Backend(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_resolves_exe_path() {
        let w = WindowsAutostart::new().expect("current_exe must succeed in test runner");
        assert!(!w.app_path.is_empty());
    }

    /// Round-trip enable -> `is_enabled` -> disable against the real registry.
    /// Gated `#[ignore]` so default `cargo test` does not touch HKCU.
    /// Run with: `cargo test --workspace -- --ignored`.
    #[test]
    #[ignore = "touches HKCU\\...\\Run; run explicitly with --ignored"]
    #[allow(
        clippy::items_after_statements,
        reason = "Cleanup guard struct must follow the variable it borrows"
    )]
    fn registry_round_trip() {
        let mut w = WindowsAutostart::new().unwrap();

        // Drop guard removes the registry value even on panic.
        struct Cleanup<'a>(&'a mut WindowsAutostart);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                let _ = self.0.set_enabled(false, &[]);
            }
        }
        let guard = Cleanup(&mut w);

        // Pre-clean to reduce flakiness from leftover state.
        let _ = guard.0.set_enabled(false, &[]);
        assert!(!guard.0.is_enabled().unwrap(), "must start clean");

        guard.0.set_enabled(true, &["--start-minimized"]).unwrap();
        assert!(guard.0.is_enabled().unwrap());

        guard.0.set_enabled(false, &[]).unwrap();
        assert!(!guard.0.is_enabled().unwrap());
    }
}
