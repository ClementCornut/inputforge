//! Shared validation for engine commands and app mode workflows.
use crate::{
    error::{EngineError, Result},
    mode::Modes,
    profile::Profile,
};
/// Maximum mode-name length in extended grapheme clusters (UAX #29).
pub const MAX_MODE_NAME_GRAPHEMES: usize = 64;

/// Returns `Err(InvalidConfig)` if `name` is empty or exceeds the
/// grapheme-cluster cap. Shared by `AddMode`, `RenameMode`, and
/// `SetDefaultMode` so the handlers enforce identical policy.
///
/// # Errors
/// Returns `EngineError::InvalidConfig` for empty or overlong names.
pub fn validate_mode_name(name: &str, empty_reason: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(EngineError::InvalidConfig {
            reason: empty_reason.to_owned(),
        });
    }
    let grapheme_count =
        unicode_segmentation::UnicodeSegmentation::graphemes(trimmed, true).count();
    if grapheme_count > MAX_MODE_NAME_GRAPHEMES {
        return Err(EngineError::InvalidConfig {
            reason: format!(
                "mode name too long ({grapheme_count} graphemes, max {MAX_MODE_NAME_GRAPHEMES})"
            ),
        });
    }
    Ok(())
}

/// Validate deletion guards and prepare the surviving mode list.
///
/// # Errors
/// Rejects invalid, missing, first, startup or final modes.
pub fn validate_mode_deletion(profile: &Profile, name: &str) -> Result<Modes> {
    validate_mode_name(name, "mode name cannot be empty")?;
    if profile.modes().first() == name {
        return Err(EngineError::InvalidConfig {
            reason: "cannot delete first mode".into(),
        });
    }
    if !profile.modes().contains(name) {
        return Err(EngineError::ModeNotFound { name: name.into() });
    }
    if profile.settings().startup_mode() == name {
        return Err(EngineError::InvalidConfig {
            reason: format!("cannot delete startup mode '{name}'"),
        });
    }
    profile.modes().with_removed(name)
}
