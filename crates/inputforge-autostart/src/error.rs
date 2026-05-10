//! Error types for autostart operations.

use std::io;

/// Errors returned by [`AutostartManager`](crate::AutostartManager) impls.
#[derive(Debug, thiserror::Error)]
pub enum AutostartError {
    /// The current platform has no supported autostart backend (or
    /// `std::env::current_exe()` failed during construction).
    #[error("autostart not supported on this platform")]
    NotSupported,

    /// HKCU\...\Run registry write was rejected.
    #[error("registry write denied")]
    RegistryDenied,

    /// XDG autostart directory is missing or read-only.
    #[error("autostart directory not writable: {0}")]
    DirectoryNotWritable(String),

    /// Other I/O error from the backend.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// Opaque wrapper around the underlying `auto_launch::Error` (or any
    /// non-classified backend error). The string is for log output only;
    /// callers must not pattern-match on its contents.
    #[error("backend error: {0}")]
    Backend(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_message_for_not_supported() {
        let err = AutostartError::NotSupported;
        assert_eq!(err.to_string(), "autostart not supported on this platform");
    }

    #[test]
    fn io_variant_wraps_via_from() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "no write");
        let err: AutostartError = io_err.into();
        assert!(matches!(err, AutostartError::Io(_)));
        assert!(err.to_string().starts_with("io error: "));
    }

    #[test]
    fn backend_string_propagates_to_display() {
        let err = AutostartError::Backend("oops".to_owned());
        assert_eq!(err.to_string(), "backend error: oops");
    }

    #[test]
    fn directory_not_writable_includes_path_in_display() {
        let err = AutostartError::DirectoryNotWritable("/tmp/x".to_owned());
        assert_eq!(err.to_string(), "autostart directory not writable: /tmp/x");
    }
}
