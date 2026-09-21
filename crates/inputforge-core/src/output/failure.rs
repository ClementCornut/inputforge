use std::fmt;

/// Injection device affected by a native output failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    Keyboard,
    Mouse,
}

impl fmt::Display for OutputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keyboard => f.write_str("keyboard"),
            Self::Mouse => f.write_str("mouse"),
        }
    }
}

/// Native operation phase which failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputPhase {
    Initialization,
    Readiness,
    Emission,
    Release,
}

impl fmt::Display for OutputPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initialization => f.write_str("initialization"),
            Self::Readiness => f.write_str("readiness"),
            Self::Emission => f.write_str("emission"),
            Self::Release => f.write_str("release"),
        }
    }
}

/// Typed native injection failure with separately inspectable cleanup evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFailure {
    pub output: OutputKind,
    pub phase: OutputPhase,
    pub category: std::io::ErrorKind,
    pub details: String,
    pub cleanup: Vec<String>,
}

impl fmt::Display for OutputFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} failed ({:?}): {}",
            self.output, self.phase, self.category, self.details
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_shows_primary_but_not_cleanup_details() {
        let failure = OutputFailure {
            output: OutputKind::Keyboard,
            phase: OutputPhase::Emission,
            category: std::io::ErrorKind::BrokenPipe,
            details: "key packet failed".to_owned(),
            cleanup: vec!["release failed".to_owned(), "close failed".to_owned()],
        };

        assert!(failure.to_string().contains("key packet failed"));
        assert!(!failure.to_string().contains("release failed"));
    }
}
