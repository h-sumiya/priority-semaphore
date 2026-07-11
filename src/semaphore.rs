//! Core implementation of [`PrioritySemaphore`].

use crate::{
    error::{TryAcquireError, TryAcquireError::*},
    lock::Lock,
    permit::Permit,
    queue::{WaitKey, WaitQueue},
    waiter::{AcquireFuture, Waiter},
};
use alloc::sync::Arc;
use core::{
    sync::atomic::{AtomicUsize, Ordering},
    task::Waker,
};

/// Priority value used by the semaphore.
///
/// Larger numbers represent higher priority. Waiters with an equal priority
/// are served in first-in, first-out order.
pub type Priority = i32;

// Available permits and coordination flags share one atomic word. This closes
// the check-then-enqueue race without putting the uncontended path behind a
// mutex.
const CLOSED: usize = 1 << (usize::BITS - 1);
const HAS_WAITERS: usize = 1 << (usize::BITS - 2);
const PERMIT_MASK: usize = HAS_WAITERS - 1;

pub(crate) enum RegisterResult {
    Acquired,
    Queued { key: WaitKey, waiter: Arc<Waiter> },
    Closed,
}

/// A runtime-independent, priority-aware asynchronous semaphore.
///
/// Acquiring an immediately available permit is lock-free. Under contention,
/// returned permits are reserved directly for the highest-priority waiter, so
/// a newly arriving task cannot steal a wake-up.
pub struct PrioritySemaphore {
    state: AtomicUsize,
    pub(crate) waiters: Lock<WaitQueue>,
    max_permits: usize,
}

impl core::fmt::Debug for PrioritySemaphore {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PrioritySemaphore")
            .field("available", &self.available_permits())
            .field("queued", &self.queued())
            .field("max_permits", &self.max_permits)
            .field("closed", &self.is_closed())
            .finish()
    }
}

impl PrioritySemaphore {
    /// Creates a semaphore with `permits` concurrent permits.
    ///
    /// # Panics
    ///
    /// Panics when `permits` is larger than [`PrioritySemaphore::MAX_PERMITS`].
    pub const fn new(permits: usize) -> Self {
        assert!(permits <= Self::MAX_PERMITS, "too many semaphore permits");
        Self {
            state: AtomicUsize::new(permits),
            waiters: Lock::new(WaitQueue::new()),
            max_permits: permits,
        }
    }

    /// Largest supported initial permit count.
    pub const MAX_PERMITS: usize = PERMIT_MASK;

    /// Acquires one permit at `priority`.
    ///
    /// The returned future is cancellation-safe. If it is cancelled after a
    /// permit has already been assigned, that permit is immediately passed to
    /// the next waiter or returned to the semaphore.
    pub fn acquire(self: &Arc<Self>, priority: Priority) -> AcquireFuture {
        AcquireFuture::new(self.clone(), priority)
    }

    /// Attempts to acquire one immediately available permit.
    ///
    /// This method never bypasses already queued waiters. `priority` is
    /// accepted for API symmetry, but only affects queued acquisitions.
    pub fn try_acquire(self: &Arc<Self>, _priority: Priority) -> Result<Permit, TryAcquireError> {
        self.try_take()?;
        Ok(Permit::new(self.clone()))
    }

    /// Closes the semaphore and wakes every queued waiter.
    ///
    /// Closing is idempotent. Permits acquired before the close remain valid,
    /// while all subsequent acquisition attempts fail.
    pub fn close(&self) {
        let entries = {
            // The lock makes close and direct handoff linearisable with each
            // other. Wakers are deliberately invoked after it is released.
            let mut queue = self.waiters.lock();
            let previous = self.state.fetch_or(CLOSED, Ordering::AcqRel);
            if previous & CLOSED != 0 {
                return;
            }
            let entries = queue.drain();
            for entry in &entries {
                entry.waiter.close();
            }
            self.state.fetch_and(!HAS_WAITERS, Ordering::Release);
            entries
        };

        for entry in entries {
            entry.waker.wake();
        }
    }

    /// Returns the number of permits that can be acquired immediately.
    pub fn available_permits(&self) -> usize {
        self.state.load(Ordering::Acquire) & PERMIT_MASK
    }

    /// Returns the number of futures currently waiting in the priority queue.
    pub fn queued(&self) -> usize {
        self.waiters.lock().len()
    }

    /// Returns `true` after [`PrioritySemaphore::close`] has been called.
    pub fn is_closed(&self) -> bool {
        self.state.load(Ordering::Acquire) & CLOSED != 0
    }

    pub(crate) fn register(&self, priority: Priority, waker: &Waker) -> RegisterResult {
        let mut queue = self.waiters.lock();
        let previous = self.state.fetch_or(HAS_WAITERS, Ordering::AcqRel);
        if previous & CLOSED != 0 {
            if queue.is_empty() {
                self.state.fetch_and(!HAS_WAITERS, Ordering::Release);
            }
            return RegisterResult::Closed;
        }

        // Only the first waiter can consume a permit that raced with queue
        // registration. Existing queued waiters must retain strict priority.
        if queue.is_empty() {
            let state = self.state.load(Ordering::Acquire);
            debug_assert_eq!(state & CLOSED, 0);
            if state & PERMIT_MASK != 0 {
                match self.state.compare_exchange(
                    state,
                    state - 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        self.state.fetch_and(!HAS_WAITERS, Ordering::Release);
                        return RegisterResult::Acquired;
                    }
                    Err(actual) => {
                        // A release that began before HAS_WAITERS was set may
                        // have changed the count once. It cannot keep changing
                        // while this queue lock is held.
                        if actual & PERMIT_MASK != 0 {
                            let taken = self.state.compare_exchange(
                                actual,
                                actual - 1,
                                Ordering::AcqRel,
                                Ordering::Acquire,
                            );
                            if taken.is_ok() {
                                self.state.fetch_and(!HAS_WAITERS, Ordering::Release);
                                return RegisterResult::Acquired;
                            }
                        }
                    }
                }
            }
        }

        let waiter = Arc::new(Waiter::new());
        let key = queue.push(priority, waiter.clone(), waker.clone());
        RegisterResult::Queued { key, waiter }
    }

    pub(crate) fn refresh_waker(&self, key: WaitKey, waiter: &Waiter, waker: &Waker) {
        let mut queue = self.waiters.lock();
        if waiter.is_waiting() {
            queue.update_waker(key, waker);
        }
    }

    pub(crate) fn cancel_waiter(&self, key: WaitKey, waiter: &Waiter) {
        let assigned = {
            let mut queue = self.waiters.lock();
            if waiter.is_waiting() {
                let removed = queue.remove(key);
                if removed.is_some() && queue.is_empty() {
                    self.state.fetch_and(!HAS_WAITERS, Ordering::Release);
                }
                false
            } else {
                waiter.is_assigned()
            }
        };

        if assigned {
            self.release_one();
        }
    }

    pub(crate) fn release_one(&self) {
        let mut state = self.state.load(Ordering::Acquire);
        loop {
            if state & HAS_WAITERS != 0 {
                self.release_slow();
                return;
            }
            debug_assert!((state & PERMIT_MASK) < self.max_permits);
            match self.state.compare_exchange_weak(
                state,
                state + 1,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(actual) => state = actual,
            }
        }
    }

    pub(crate) fn try_take(&self) -> Result<(), TryAcquireError> {
        let mut state = self.state.load(Ordering::Acquire);
        loop {
            if state & CLOSED != 0 {
                return Err(Closed);
            }
            if state & HAS_WAITERS != 0 || state & PERMIT_MASK == 0 {
                return Err(NoPermits);
            }
            match self.state.compare_exchange_weak(
                state,
                state - 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => state = actual,
            }
        }
    }

    fn release_slow(&self) {
        let wake = {
            let mut queue = self.waiters.lock();
            let state = self.state.load(Ordering::Acquire);
            if state & CLOSED == 0 {
                if let Some(entry) = queue.pop() {
                    entry.waiter.assign();
                    if queue.is_empty() {
                        self.state.fetch_and(!HAS_WAITERS, Ordering::Release);
                    }
                    Some(entry.waker)
                } else {
                    self.return_to_pool(&queue);
                    None
                }
            } else {
                // Close normally drained the queue before we could acquire the
                // lock. Keep this branch defensive for unusual interleavings.
                let entries = queue.drain();
                for entry in &entries {
                    entry.waiter.close();
                }
                self.return_to_pool(&queue);
                drop(queue);
                for entry in entries {
                    entry.waker.wake();
                }
                return;
            }
        };
        if let Some(waker) = wake {
            waker.wake();
        }
    }

    fn return_to_pool(&self, queue: &WaitQueue) {
        debug_assert!(queue.is_empty());
        self.state.fetch_and(!HAS_WAITERS, Ordering::Release);
        let mut state = self.state.load(Ordering::Acquire);
        loop {
            debug_assert!((state & PERMIT_MASK) < self.max_permits);
            match self.state.compare_exchange_weak(
                state,
                state + 1,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(actual) => state = actual,
            }
        }
    }
}
