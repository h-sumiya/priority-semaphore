//! Error types.

/// Returned by `try_acquire` when no permits are immediately available.
#[derive(Debug, Clone, Copy)]
pub enum TryAcquireError {
    /// All permits are currently in use.
    NoPermits,
    /// Semaphore has been closed.
    Closed,
}

/// Returned by async `acquire`.
#[derive(Debug)]
pub enum AcquireError {
    /// Semaphore was closed before acquisition succeeded.
    Closed,
}
