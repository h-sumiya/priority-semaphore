//! Core implementation of [`PrioritySemaphore`].

use crate::lock::Lock;
use crate::{error::*, permit::Permit, queue::WaitQueue, waiter::AcquireFuture};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Priority value used by the semaphore.
///
/// Larger numbers represent higher priority.
pub type Priority = i32;

/// Async-aware priority semaphore.
pub struct PrioritySemaphore {
    permits: AtomicUsize,
    pub(crate) waiters: Lock<WaitQueue>,
    max_permit: usize,
    pub(crate) closed: AtomicBool,
}

impl core::fmt::Debug for PrioritySemaphore {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PrioritySemaphore")
            .field("available", &self.available_permits())
            .field("queued", &self.queued())
            .field("closed", &self.closed.load(Ordering::Acquire))
            .finish()
    }
}

impl PrioritySemaphore {
    /// Create new semaphore with given maximum concurrent permits.
    pub const fn new(permits: usize) -> Self {
        Self {
            permits: AtomicUsize::new(permits),
            waiters: Lock::new(WaitQueue::new()),
            max_permit: permits,
            closed: AtomicBool::new(false),
        }
    }

    /// Async acquire (cancellation-safe).
    pub fn acquire(self: &Arc<Self>, prio: Priority) -> AcquireFuture {
        AcquireFuture {
            root: self.clone(),
            prio,
            in_queue: false,
            wait_id: None,
        }
    }

    /// Try immediate acquire.
    pub fn try_acquire(self: &Arc<Self>, _prio: Priority) -> Result<Permit, TryAcquireError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(TryAcquireError::Closed);
        }
        let mut curr = self.permits.load(Ordering::Acquire);
        loop {
            if curr == 0 {
                return Err(TryAcquireError::NoPermits);
            }
            match self.permits.compare_exchange_weak(
                curr,
                curr - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(Permit::new(self.clone())),
                Err(actual) => curr = actual,
            }
        }
    }

    /// Close the semaphore: further acquires fail.
    pub fn close(&self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut waiters = self.waiters.lock();
        while let Some(entry) = waiters.pop() {
            entry.waker.wake();
        }
    }

    /// Number of currently available permits.
    pub fn available_permits(&self) -> usize {
        self.permits.load(Ordering::Acquire)
    }

    /// Number of tasks waiting in the queue.
    pub fn queued(&self) -> usize {
        self.waiters.lock().len()
    }

    /// (internal) Called when a permit is returned.
    pub(crate) fn dispatch_next(&self) {
        let closed = self.closed.load(Ordering::Acquire);
        let mut waiters = self.waiters.lock();

        if !closed {
            if let Some(entry) = waiters.pop() {
                entry.waker.wake();
                return;
            }
        }

        let prev = self.permits.fetch_add(1, Ordering::AcqRel);
        if prev >= self.max_permit {
            self.permits.store(self.max_permit, Ordering::Release);
        }
    }

    pub(crate) fn remove_waiter(&self, id: usize) {
        let mut waiters = self.waiters.lock();
        waiters.remove(id);
    }
}
