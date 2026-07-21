//! Linux backend: writes `$XDG_CONFIG_HOME/autostart/InputForge.desktop`
//! via the `auto-launch` crate. Honored by GNOME, KDE, XFCE, Cinnamon,
//! MATE; ignored by tiling WMs without an XDG autostart implementation.

#![cfg(target_os = "linux")]

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

use crate::{AutostartError, AutostartManager};

const APP_NAME: &str = "InputForge";

#[derive(Debug)]
pub(crate) struct LinuxAutostart {
    app_name: String,
    app_path: String,
}

impl LinuxAutostart {
    /// Resolve the absolute exe path once at construction.
    ///
    /// # Errors
    ///
    /// Returns [`AutostartError::NotSupported`] when `std::env::current_exe`
    /// fails (e.g., `AppImage` mount weirdness).
    pub(crate) fn new() -> Result<Self, AutostartError> {
        let exe = std::env::current_exe().map_err(|_e| AutostartError::NotSupported)?;
        let app_path = exe.to_str().ok_or(AutostartError::NotSupported)?.to_owned();
        Ok(Self {
            app_name: APP_NAME.to_owned(),
            app_path,
        })
    }

    /// Construct a backend bound to a custom `.desktop` filename stem.
    ///
    /// Used by the integration test to namespace the autostart entry so it
    /// cannot collide with a real installation's `APP_NAME.desktop` file or
    /// with parallel `--ignored` runs. Production callers should use
    /// [`Self::new`].
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
        AutoLaunchBuilder::new()
            .set_app_name(&self.app_name)
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
    use std::{
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    const XDG_ROUND_TRIP_CHILD: &str = "INPUTFORGE_AUTOSTART_XDG_ROUND_TRIP_CHILD";

    /// Random-ish app-name suffix so the integration test cannot clobber a
    /// real `APP_NAME.desktop` file or collide with parallel `--ignored`
    /// runs.
    fn unique_test_app_name() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!("inputforge-autostart-test-{nanos}")
    }

    #[test]
    fn build_resolves_exe_path() {
        let l = LinuxAutostart::new().expect("current_exe must succeed in test runner");
        assert!(!l.app_path.is_empty());
    }

    /// Round-trip against a tempdir-rooted `XDG_CONFIG_HOME` to avoid touching
    /// the developer's real autostart dir. The parent launches an isolated
    /// child process because the auto-launch dependency may cache XDG paths.
    #[test]
    #[ignore = "writes a temporary XDG autostart entry; run with --ignored"]
    fn xdg_round_trip() {
        if std::env::var_os(XDG_ROUND_TRIP_CHILD).is_some() {
            run_xdg_round_trip_child();
            return;
        }

        let tmp = tempfile::tempdir().expect("temporary XDG config root must be created");
        let test_binary =
            std::env::current_exe().expect("current test binary path must be available");
        let output = Command::new(test_binary)
            .arg("linux::tests::xdg_round_trip")
            .args(["--exact", "--ignored", "--nocapture", "--test-threads=1"])
            .env("XDG_CONFIG_HOME", tmp.path())
            .env(XDG_ROUND_TRIP_CHILD, "1")
            .output()
            .expect("isolated XDG round-trip child must start");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success()
                && stdout.contains("running 1 test")
                && stdout.contains("test linux::tests::xdg_round_trip ... ok")
                && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured;"),
            "isolated XDG round-trip child did not run exactly the intended test\nstatus: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status
        );
    }

    fn run_xdg_round_trip_child() {
        let mut l = LinuxAutostart::with_app_name(unique_test_app_name()).unwrap();
        let _ = l.set_enabled(false, &[]);
        assert!(!l.is_enabled().unwrap());

        l.set_enabled(true, &["--start-minimized"]).unwrap();
        assert!(l.is_enabled().unwrap());

        l.set_enabled(false, &[]).unwrap();
        assert!(!l.is_enabled().unwrap());
    }
}
