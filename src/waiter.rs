//! Acquire future and direct-handoff state.

use crate::{
    error::AcquireError,
    permit::Permit,
    queue::WaitKey,
    semaphore::{Priority, PrioritySemaphore, RegisterResult},
};
use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU8, Ordering},
    task::{Context, Poll},
};

const WAITING: u8 = 0;
const ASSIGNED: u8 = 1;
const CLOSED: u8 = 2;

/// State shared between a queued future and the thread returning a permit.
#[derive(Debug)]
pub(crate) struct Waiter(AtomicU8);

impl Waiter {
    pub(crate) const fn new() -> Self {
        Self(AtomicU8::new(WAITING))
    }

    pub(crate) fn assign(&self) {
        self.0.store(ASSIGNED, Ordering::Release);
    }

    pub(crate) fn close(&self) {
        self.0.store(CLOSED, Ordering::Release);
    }

    pub(crate) fn is_waiting(&self) -> bool {
        self.0.load(Ordering::Acquire) == WAITING
    }

    pub(crate) fn is_assigned(&self) -> bool {
        self.0.load(Ordering::Acquire) == ASSIGNED
    }

    fn status(&self) -> u8 {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug)]
enum Phase {
    Initial,
    Waiting { key: WaitKey, waiter: Arc<Waiter> },
    Complete,
}

/// Future returned by [`PrioritySemaphore::acquire`](crate::PrioritySemaphore::acquire).
///
/// Dropping this future is cancellation-safe in every state, including after
/// a permit has been assigned but before the executor polls it again.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled or awaited"]
pub struct AcquireFuture {
    // Option lets a completed future move its existing Arc directly into the
    // permit instead of paying for an increment/decrement pair per acquire.
    root: Option<Arc<PrioritySemaphore>>,
    priority: Priority,
    phase: Phase,
}

impl AcquireFuture {
    pub(crate) fn new(root: Arc<PrioritySemaphore>, priority: Priority) -> Self {
        Self {
            root: Some(root),
            priority,
            phase: Phase::Initial,
        }
    }
}

impl Future for AcquireFuture {
    type Output = Result<Permit, AcquireError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match &this.phase {
            Phase::Initial => match this.root.as_ref().unwrap().try_take() {
                Ok(()) => {
                    let root = this.root.take().unwrap();
                    this.phase = Phase::Complete;
                    Poll::Ready(Ok(Permit::new(root)))
                }
                Err(crate::TryAcquireError::Closed) => {
                    this.root = None;
                    this.phase = Phase::Complete;
                    Poll::Ready(Err(AcquireError::Closed))
                }
                Err(crate::TryAcquireError::NoPermits) => {
                    match this
                        .root
                        .as_ref()
                        .unwrap()
                        .register(this.priority, cx.waker())
                    {
                        RegisterResult::Acquired => {
                            let root = this.root.take().unwrap();
                            this.phase = Phase::Complete;
                            Poll::Ready(Ok(Permit::new(root)))
                        }
                        RegisterResult::Closed => {
                            this.root = None;
                            this.phase = Phase::Complete;
                            Poll::Ready(Err(AcquireError::Closed))
                        }
                        RegisterResult::Queued { key, waiter } => {
                            this.phase = Phase::Waiting { key, waiter };
                            Poll::Pending
                        }
                    }
                }
            },
            Phase::Waiting { key, waiter } => match waiter.status() {
                ASSIGNED => {
                    let root = this.root.take().unwrap();
                    this.phase = Phase::Complete;
                    Poll::Ready(Ok(Permit::new(root)))
                }
                CLOSED => {
                    this.root = None;
                    this.phase = Phase::Complete;
                    Poll::Ready(Err(AcquireError::Closed))
                }
                WAITING => {
                    this.root
                        .as_ref()
                        .unwrap()
                        .refresh_waker(*key, waiter, cx.waker());
                    // The status may have changed before refresh_waker took
                    // the queue lock. In that case the corresponding wake is
                    // already guaranteed, so Pending remains correct.
                    Poll::Pending
                }
                _ => unreachable!("invalid waiter state"),
            },
            Phase::Complete => panic!("AcquireFuture polled after completion"),
        }
    }
}

impl Drop for AcquireFuture {
    fn drop(&mut self) {
        if let (Some(root), Phase::Waiting { key, waiter }) = (&self.root, &self.phase) {
            root.cancel_waiter(*key, waiter);
        }
    }
}
