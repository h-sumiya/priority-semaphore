//! Future & waiter helpers.

use crate::{error::AcquireError, permit::Permit, semaphore::PrioritySemaphore};
use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Future returned by `PrioritySemaphore::acquire`.
pub struct AcquireFuture {
    root: Arc<PrioritySemaphore>,
    prio: i32,
    in_queue: bool,
    wait_id: Option<usize>,
}

impl Future for AcquireFuture {
    type Output = Result<Permit, AcquireError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unimplemented!()
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
