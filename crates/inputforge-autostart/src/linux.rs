//! Linux backend: writes `$XDG_CONFIG_HOME/autostart/InputForge.desktop`
//! via the `auto-launch` crate. Honored by GNOME, KDE, XFCE, Cinnamon,
//! MATE; ignored by tiling WMs without an XDG autostart implementation.

#![cfg(target_os = "linux")]

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

use crate::{AutostartError, AutostartManager};

#[allow(
    dead_code,
    reason = "consumed by `new_for_current_platform` factory in Task 1.8"
)]
const APP_NAME: &str = "InputForge";

#[allow(
    dead_code,
    reason = "consumed by `new_for_current_platform` factory in Task 1.8"
)]
#[derive(Debug)]
pub(crate) struct LinuxAutostart {
    app_path: String,
}

impl LinuxAutostart {
    /// Resolve the absolute exe path once at construction.
    ///
    /// # Errors
    ///
    /// Returns [`AutostartError::NotSupported`] when `std::env::current_exe`
    /// fails (e.g., AppImage mount weirdness).
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
            .expect("LinuxAutostart: AutoLaunchBuilder::build cannot fail with valid app_path")
    }
}

impl AutostartManager for LinuxAutostart {
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
        let l = LinuxAutostart::new().expect("current_exe must succeed in test runner");
        assert!(!l.app_path.is_empty());
    }

    /// Round-trip against a tempdir-rooted XDG_CONFIG_HOME to avoid touching
    /// the developer's real autostart dir. Gated `#[ignore]` because it
    /// mutates an env var; not parallelizable with other env-var tests.
    #[test]
    #[ignore = "mutates XDG_CONFIG_HOME; run explicitly with --ignored"]
    fn xdg_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        // SAFETY: Test is single-threaded by --ignored in practice; for
        //         strict isolation, set XDG_CONFIG_HOME via a fixture.
        // The Rust 2024 edition requires `unsafe { ... }` around env::set_var.
        #[allow(
            unsafe_code,
            reason = "test mutates XDG_CONFIG_HOME against a tempdir; ignored test runs single-threaded"
        )]
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        }

        let mut l = LinuxAutostart::new().unwrap();
        let _ = l.set_enabled(false, &[]);
        assert!(!l.is_enabled().unwrap());

        l.set_enabled(true, &["--start-minimized"]).unwrap();
        assert!(l.is_enabled().unwrap());

        l.set_enabled(false, &[]).unwrap();
        assert!(!l.is_enabled().unwrap());
    }
}
