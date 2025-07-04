//! Future & waiter helpers.

use crate::{
    error::{AcquireError, TryAcquireError},
    permit::Permit,
    semaphore::PrioritySemaphore,
};
use alloc::sync::Arc;
use core::sync::atomic::Ordering;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Future returned by `PrioritySemaphore::acquire`.
#[derive(Debug)]
pub struct AcquireFuture {
    pub(crate) root: Arc<PrioritySemaphore>,
    pub(crate) prio: i32,
    pub(crate) in_queue: bool,
    pub(crate) wait_id: Option<usize>,
}

impl Future for AcquireFuture {
    type Output = Result<Permit, AcquireError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        loop {
            match this.root.try_acquire(this.prio) {
                Ok(permit) => {
                    if this.in_queue {
                        if let Some(id) = this.wait_id.take() {
                            this.root.remove_waiter(id);
                        }
                        this.in_queue = false;
                    }
                    return Poll::Ready(Ok(permit));
                }
                Err(TryAcquireError::Closed) => {
                    if this.in_queue {
                        if let Some(id) = this.wait_id.take() {
                            this.root.remove_waiter(id);
                        }
                        this.in_queue = false;
                    }
                    return Poll::Ready(Err(AcquireError::Closed));
                }
                Err(TryAcquireError::NoPermits) => {}
            }

            if this.root.closed.load(Ordering::Acquire) {
                return Poll::Ready(Err(AcquireError::Closed));
            }

            if !this.in_queue {
                let mut queue = this.root.waiters.lock();
                let id = queue.push(this.prio, cx.waker().clone());
                this.wait_id = Some(id);
                this.in_queue = true;
                // Try again in case a permit became available after queuing
                continue;
            } else if let Some(id) = this.wait_id {
                let mut queue = this.root.waiters.lock();
                queue.update_waker(id, cx.waker().clone());
            }

            return Poll::Pending;
        }
    }
}

impl Drop for AcquireFuture {
    fn drop(&mut self) {
        if self.in_queue {
            if let Some(id) = self.wait_id {
                self.root.remove_waiter(id);
            }
        }
    }
}
