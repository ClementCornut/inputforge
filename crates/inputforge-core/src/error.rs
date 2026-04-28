// Rust guideline compliant 2026-03-03

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

    #[error("output failed: {reason}")]
    OutputFailed { reason: String },

    #[error("device not found: {device_id:?}")]
    DeviceNotFound { device_id: DeviceId },

    #[error("invalid config: {reason}")]
    InvalidConfig { reason: String },

    #[error("invalid mapping: {reason}")]
    InvalidMapping { reason: String },

    #[error("mode not found: {name}")]
    ModeNotFound { name: String },

    #[error("mode cycle detected: {path:?}")]
    ModeCycleDetected { path: Vec<String> },

    #[error("snapshot not found: {id}")]
    SnapshotNotFound { id: String },

    #[error("snapshot file corrupt at {path}: {reason}")]
    SnapshotCorrupt { path: PathBuf, reason: String },

    #[error("snapshot directory I/O error at {path}: {source}")]
    SnapshotDirIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("snapshot id is not a valid ULID: {value}")]
    SnapshotIdInvalid { value: String },

    #[error("could not create snapshot directory at {path}: {source}")]
    SnapshotDirCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("profile path has no parent directory: {path}")]
    ProfilePathHasNoParent { path: PathBuf },

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
    fn engine_error_display_invalid_config() {
        let err = EngineError::InvalidConfig {
            reason: "low >= center_low".to_owned(),
        };
        assert!(err.to_string().contains("low >= center_low"));
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
    #[allow(
        clippy::unnecessary_literal_unwrap,
        reason = "test verifies the Result type alias"
    )]
    fn result_alias_works() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<i32> = Err(EngineError::VJoyDriverMissing);
        err.unwrap_err();
    }

    #[test]
    fn engine_error_display_snapshot_not_found() {
        let err = EngineError::SnapshotNotFound {
            id: "01H8ZK".to_owned(),
        };
        assert!(err.to_string().contains("01H8ZK"));
    }

    #[test]
    fn engine_error_display_snapshot_corrupt() {
        let err = EngineError::SnapshotCorrupt {
            path: PathBuf::from("/tmp/snap.toml"),
            reason: "missing meta".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/snap.toml"));
        assert!(msg.contains("missing meta"));
    }

    #[test]
    fn engine_error_display_snapshot_id_invalid() {
        let err = EngineError::SnapshotIdInvalid {
            value: "not-a-ulid".to_owned(),
        };
        assert!(err.to_string().contains("not-a-ulid"));
    }

    #[test]
    fn engine_error_display_profile_path_has_no_parent() {
        let err = EngineError::ProfilePathHasNoParent {
            path: PathBuf::from("foo.toml"),
        };
        assert!(err.to_string().contains("foo.toml"));
    }
}
