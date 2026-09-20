use std::{
    backtrace::Backtrace,
    fmt, io,
    path::{Path, PathBuf},
};

/// A failed output operation, with its native cause and secondary cleanup failures.
#[derive(Debug)]
pub struct Error {
    context: Box<Context>,
}

#[derive(Debug)]
struct Context {
    operation: &'static str,
    slot: Option<u8>,
    path: PathBuf,
    source: io::Error,
    cleanup: Vec<Error>,
    backtrace: Backtrace,
}

impl Error {
    pub(super) fn io(
        operation: &'static str,
        slot: Option<u8>,
        path: impl AsRef<Path>,
        source: io::Error,
    ) -> Self {
        Self {
            context: Box::new(Context {
                operation,
                slot,
                path: path.as_ref().into(),
                source,
                cleanup: vec![],
                backtrace: Backtrace::capture(),
            }),
        }
    }

    pub(super) fn invalid(operation: &'static str, slot: Option<u8>, message: &str) -> Self {
        Self::io(
            operation,
            slot,
            "",
            io::Error::new(io::ErrorKind::InvalidInput, message),
        )
    }

    pub(super) fn append(&mut self, other: Self) {
        self.context.cleanup.push(other);
    }

    /// Return the operation that failed first.
    #[must_use]
    pub fn operation(&self) -> &str {
        self.context.operation
    }
    /// Return the affected logical slot, when known.
    #[must_use]
    pub fn slot(&self) -> Option<u8> {
        self.context.slot
    }
    /// Return the affected path, or an empty path for configuration errors.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.context.path
    }
    /// Return the original I/O error kind.
    #[must_use]
    pub fn kind(&self) -> io::ErrorKind {
        self.context.source.kind()
    }
    /// Return the original errno, if available.
    #[must_use]
    pub fn raw_os_error(&self) -> Option<i32> {
        self.context.source.raw_os_error()
    }
    /// Return subsequent failures encountered while releasing all owned devices.
    #[must_use]
    pub fn cleanup_failures(&self) -> &[Self] {
        &self.context.cleanup
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = &self.context;
        write!(f, "{}", c.operation)?;
        if let Some(slot) = c.slot {
            write!(f, " (slot {slot})")?;
        }
        if !c.path.as_os_str().is_empty() {
            write!(f, " at {}", c.path.display())?;
        }
        write!(f, ": {}", c.source)?;
        for error in &c.cleanup {
            write!(f, "; cleanup: {error}")?;
        }
        if c.backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            write!(f, "\n{}", c.backtrace)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.context.source)
    }
}
