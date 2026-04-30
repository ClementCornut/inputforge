// Rust guideline compliant 2026-03-06

//! CLI argument parsing for `InputForge`.

use std::path::PathBuf;

use clap::Parser;

/// `InputForge`, remap physical joystick, pedal, and throttle inputs
/// to virtual `vJoy` devices.
#[derive(Debug, Parser)]
#[command(name = "inputforge", version, about)]
pub(crate) struct Cli {
    /// Path to a TOML profile file to load on startup.
    #[arg(long, value_name = "PATH")]
    pub(crate) profile: Option<PathBuf>,

    /// Activate the profile immediately after loading.
    #[arg(long, requires = "profile")]
    pub(crate) enable: bool,

    /// Start minimized to the system tray (no GUI window).
    #[arg(long)]
    pub(crate) start_minimized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_args_parse_successfully() {
        let cli = Cli::try_parse_from(["inputforge"]).expect("default args should parse");
        assert!(cli.profile.is_none());
        assert!(!cli.enable);
        assert!(!cli.start_minimized);
    }

    #[test]
    fn profile_flag_accepts_path() {
        let cli = Cli::try_parse_from(["inputforge", "--profile", "test.toml"])
            .expect("--profile should parse");
        assert_eq!(
            cli.profile.as_deref(),
            Some(std::path::Path::new("test.toml"))
        );
    }

    #[test]
    fn enable_requires_profile() {
        let result = Cli::try_parse_from(["inputforge", "--enable"]);
        assert!(result.is_err(), "--enable without --profile should fail");
    }

    #[test]
    fn enable_with_profile_parses() {
        let cli = Cli::try_parse_from(["inputforge", "--profile", "test.toml", "--enable"])
            .expect("--enable with --profile should parse");
        assert!(cli.enable);
    }

    #[test]
    fn start_minimized_parses() {
        let cli = Cli::try_parse_from(["inputforge", "--start-minimized"])
            .expect("--start-minimized should parse");
        assert!(cli.start_minimized);
    }
}
