//! Error types.

/// Returned by `try_acquire` when no permits are immediately available.
#[derive(Debug, Clone, Copy)]
pub enum TryAcquireError {
    /// All permits are currently in use.
    NoPermits,
    /// Semaphore has been closed.
    Closed,
}

impl core::fmt::Display for TryAcquireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TryAcquireError::NoPermits => write!(f, "no permits available"),
            TryAcquireError::Closed => write!(f, "semaphore closed"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TryAcquireError {}

/// Returned by async `acquire`.
#[derive(Debug)]
pub enum AcquireError {
    /// Semaphore was closed before acquisition succeeded.
    Closed,
}

impl core::fmt::Display for AcquireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AcquireError::Closed => write!(f, "semaphore closed"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AcquireError {}
