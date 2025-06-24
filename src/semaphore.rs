//! Core implementation of [`PrioritySemaphore`].

use crate::{error::*, permit::Permit, queue::WaitQueue, waiter::AcquireFuture};
use alloc::sync::Arc;
use core::sync::atomic::AtomicUsize;

/// 整数が大きいほど高優先度
pub type Priority = i32;

/// Async-aware priority semaphore.
pub struct PrioritySemaphore {
    permits: AtomicUsize,
    waiters: WaitQueue,
    max_permit: usize,
}

impl PrioritySemaphore {
    /// Create new semaphore with given maximum concurrent permits.
    pub const fn new(permits: usize) -> Self {
        Self {
            permits: AtomicUsize::new(permits),
            waiters: WaitQueue::new(),
            max_permit: permits,
        }
    }

    /// Async acquire (cancellation-safe).
    pub fn acquire(self: &Arc<Self>, prio: Priority) -> AcquireFuture {
        unimplemented!()
    }

    /// Try immediate acquire.
    pub fn try_acquire(self: &Arc<Self>, prio: Priority) -> Result<Permit, TryAcquireError> {
        unimplemented!()
    }

    /// Close the semaphore: further acquires fail.
    pub fn close(&self) {
        unimplemented!()
    }

    /// Number of currently available permits.
    pub fn available_permits(&self) -> usize {
        unimplemented!()
    }

    /// Number of tasks waiting in the queue.
    pub fn queued(&self) -> usize {
        unimplemented!()
    }

    /// (internal) Called when a permit is returned.
    pub(crate) fn dispatch_next(&self) {
        unimplemented!()
    }

    pub(crate) fn remove_waiter(&self, id: usize) {
        unimplemented!()
    }
}
