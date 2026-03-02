// Rust guideline compliant 2026-03-02

use std::path::PathBuf;

use crate::types::DeviceId;

/// All errors that can occur in the `InputForge` engine.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("profile not found: {path}")]
    ProfileNotFound { path: PathBuf },

    #[error("failed to parse profile: {0}")]
    ProfileParse(#[from] toml::de::Error),

    #[error("failed to serialize profile: {0}")]
    ProfileWrite(#[from] toml::ser::Error),

    #[error("vJoy device {device_id} is unavailable")]
    VJoyDeviceUnavailable { device_id: u8 },

    #[error("vJoy driver is not installed")]
    VJoyDriverMissing,

    #[error("SDL error: {0}")]
    Sdl(String),

    #[error("HidHide error: {0}")]
    HidHide(String),

    #[error("device not found: {device_id:?}")]
    DeviceNotFound { device_id: DeviceId },

    #[error("invalid mapping: {reason}")]
    InvalidMapping { reason: String },

    #[error("mode not found: {name}")]
    ModeNotFound { name: String },

    #[error("mode cycle detected: {path:?}")]
    ModeCycleDetected { path: Vec<String> },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Convenience Result alias for the engine.
pub type Result<T> = std::result::Result<T, EngineError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_error_display_profile_not_found() {
        let err = EngineError::ProfileNotFound {
            path: PathBuf::from("/some/path.toml"),
        };
        assert!(err.to_string().contains("/some/path.toml"));
    }

    #[test]
    fn engine_error_display_device_not_found() {
        let err = EngineError::DeviceNotFound {
            device_id: DeviceId("abc".to_owned()),
        };
        assert!(err.to_string().contains("abc"));
    }

    #[test]
    fn engine_error_display_invalid_mapping() {
        let err = EngineError::InvalidMapping {
            reason: "bad axis".to_owned(),
        };
        assert!(err.to_string().contains("bad axis"));
    }

    #[test]
    fn engine_error_display_mode_cycle() {
        let err = EngineError::ModeCycleDetected {
            path: vec!["A".to_owned(), "B".to_owned(), "A".to_owned()],
        };
        let msg = err.to_string();
        assert!(msg.contains("cycle"));
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: EngineError = io_err.into();
        assert!(matches!(err, EngineError::Io(_)));
    }

    #[test]
    fn result_alias_works() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<i32> = Err(EngineError::VJoyDriverMissing);
        assert!(err.is_err());
    }
}
