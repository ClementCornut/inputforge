use std::{backtrace::Backtrace, fmt, io, path::Path};

use crate::{
    device::evdev::{Device, Issue},
    types::DeviceId,
};

/// A capture failure, retaining the primary cause and any cleanup failures.
#[derive(Debug)]
pub struct CaptureError {
    context: Box<Context>,
}

#[derive(Debug)]
struct Context {
    issue: Issue,
    device: Option<DeviceId>,
    source: io::Error,
    cleanup: Vec<Issue>,
    backtrace: Backtrace,
}

impl CaptureError {
    pub(super) fn io(
        operation: &str,
        path: &Path,
        device: Option<&DeviceId>,
        source: io::Error,
    ) -> Self {
        Self {
            context: Box::new(Context {
                issue: Issue::new(operation, path, &source),
                device: device.cloned(),
                source,
                cleanup: Vec::new(),
                backtrace: Backtrace::capture(),
            }),
        }
    }

    pub(super) fn device(operation: &str, device: &Device, source: io::Error) -> Self {
        Self::io(
            operation,
            &device.metadata.node,
            device.identity.id.as_ref(),
            source,
        )
    }

    pub(super) fn state(operation: &str, message: &str) -> Self {
        Self::io(
            operation,
            Path::new(""),
            None,
            io::Error::new(io::ErrorKind::InvalidInput, message),
        )
    }

    pub(super) fn from_issue(issue: Issue, device: &Device) -> Self {
        let source = issue.raw_os_error.map_or_else(
            || io::Error::new(issue.kind, issue.message.clone()),
            io::Error::from_raw_os_error,
        );
        let mut error = Self::device(&issue.operation, device, source);
        error.context.issue = issue;
        error
    }

    pub(super) fn append(&mut self, other: Self) {
        self.context.cleanup.push(other.context.issue);
        self.context.cleanup.extend(other.context.cleanup);
    }

    /// The operation that first failed.
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.context.issue.operation
    }

    /// The primary failure's path, kind, message and optional native errno.
    #[must_use]
    pub fn issue(&self) -> &Issue {
        &self.context.issue
    }

    /// The selected device, when the failure pertains to one interface.
    #[must_use]
    pub fn device_id(&self) -> Option<&DeviceId> {
        self.context.device.as_ref()
    }

    /// The OS errno, if the failing operation supplied one.
    #[must_use]
    pub fn raw_os_error(&self) -> Option<i32> {
        self.context.issue.raw_os_error
    }

    /// Secondary failures encountered while still closing every capture descriptor.
    #[must_use]
    pub fn cleanup_failures(&self) -> &[Issue] {
        &self.context.cleanup
    }
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let issue = &self.context.issue;
        write!(
            f,
            "{} at {}: {}",
            issue.operation,
            issue.path.display(),
            issue.message
        )?;
        if let Some(id) = &self.context.device {
            write!(f, " (device {})", id.0)?;
        }
        for issue in &self.context.cleanup {
            write!(
                f,
                "; cleanup {} at {}: {}",
                issue.operation,
                issue.path.display(),
                issue.message
            )?;
        }
        if self.context.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "\n{}", self.context.backtrace)?;
        }
        Ok(())
    }
}

impl std::error::Error for CaptureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.context.source)
    }
}
