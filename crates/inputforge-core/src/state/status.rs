// Rust guideline compliant 2026-03-03

/// Engine lifecycle status.
///
/// Represents the three possible states of the engine event loop.
/// `Running` actively processes input; `Paused` keeps the engine
/// alive but skips processing; `Stopped` means fully deactivated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineStatus {
    /// Actively polling input and executing pipelines.
    Running,
    /// Alive but dormant; input processing is skipped.
    Paused,
    /// Fully deactivated; virtual devices released.
    Stopped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_debug_format() {
        assert_eq!(format!("{:?}", EngineStatus::Running), "Running");
        assert_eq!(format!("{:?}", EngineStatus::Paused), "Paused");
        assert_eq!(format!("{:?}", EngineStatus::Stopped), "Stopped");
    }

    #[test]
    fn status_clone_and_copy() {
        let status = EngineStatus::Running;
        let cloned = status.clone();
        let copied = status;
        assert_eq!(status, cloned);
        assert_eq!(status, copied);
    }

    #[test]
    fn status_equality() {
        assert_eq!(EngineStatus::Running, EngineStatus::Running);
        assert_ne!(EngineStatus::Running, EngineStatus::Paused);
        assert_ne!(EngineStatus::Paused, EngineStatus::Stopped);
    }
}
