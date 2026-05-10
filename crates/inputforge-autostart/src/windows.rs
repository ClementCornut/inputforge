//! Windows backend: writes HKCU\Software\Microsoft\Windows\CurrentVersion\Run
//! via the `auto-launch` crate.

#![cfg(target_os = "windows")]

use auto_launch::{AutoLaunch, AutoLaunchBuilder, WindowsEnableMode};

use crate::{AutostartError, AutostartManager};

const APP_NAME: &str = "InputForge";

#[derive(Debug)]
pub(crate) struct WindowsAutostart {
    app_name: String,
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
    pub(crate) fn new() -> Result<Self, AutostartError> {
        let exe = std::env::current_exe().map_err(|_e| AutostartError::NotSupported)?;
        let app_path = exe.to_str().ok_or(AutostartError::NotSupported)?.to_owned();
        Ok(Self {
            app_name: APP_NAME.to_owned(),
            app_path,
        })
    }

    /// Construct a backend bound to a custom registry value name.
    ///
    /// Used by the integration test to namespace the registry write so it
    /// cannot collide with a real installation's `APP_NAME` entry. Production
    /// callers should use [`Self::new`].
    #[cfg(test)]
    pub(crate) fn with_app_name(name: String) -> Result<Self, AutostartError> {
        let exe = std::env::current_exe().map_err(|_e| AutostartError::NotSupported)?;
        let app_path = exe.to_str().ok_or(AutostartError::NotSupported)?.to_owned();
        Ok(Self {
            app_name: name,
            app_path,
        })
    }

    fn build(&self, args: &[&str]) -> AutoLaunch {
        let owned_args: Vec<String> = args.iter().map(|&s| s.to_owned()).collect();
        // Pin HKCU writes only. The default `Dynamic` mode probes HKLM first
        // and falls back to HKCU on access denied; under elevation that would
        // silently switch the persistence scope to admin. Per design choice 5
        // in the f16 spec, autostart is per-user only.
        //
        // Upstream sharp edge: auto-launch 0.6 writes the registry value as
        // `"{exe} {args}"` without quoting. Modern Windows handles unquoted
        // paths via the multi-attempt CreateProcess parser, so installs under
        // `C:\Program Files\...` work in practice. If a future Windows changes
        // that behavior, switch to a hand-written winreg call here.
        AutoLaunchBuilder::new()
            .set_app_name(&self.app_name)
            .set_app_path(&self.app_path)
            .set_args(&owned_args)
            .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
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
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    /// Random-ish app-name suffix so the integration test cannot clobber a
    /// real `APP_NAME` registry value or collide with parallel `--ignored`
    /// runs. Nanos resolution is plenty for that.
    fn unique_test_app_name() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!("inputforge-autostart-test-{nanos}")
    }

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
        let mut w = WindowsAutostart::with_app_name(unique_test_app_name()).unwrap();

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
