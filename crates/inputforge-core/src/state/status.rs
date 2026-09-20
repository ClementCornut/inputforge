// Rust guideline compliant 2026-03-03

/// Engine lifecycle status.
///
/// Passive input monitoring continues while routing is stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EngineStatus {
    /// Actively polling input and executing pipelines.
    Running,
    /// Acquiring and initializing a complete snapshot.
    Starting,
    /// Ownership released after a failure; explicit retry required.
    Faulted,
    /// Routing stopped; virtual devices stay connected and neutral.
    #[default]
    Stopped,
}
