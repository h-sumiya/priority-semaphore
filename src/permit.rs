//! RAII guard returned by [`PrioritySemaphore::acquire`].

use crate::semaphore::PrioritySemaphore;
use alloc::sync::Arc;

/// Returned by successful acquire; releases permit on `Drop`.
#[derive(Debug)]
pub struct Permit {
    root: Arc<PrioritySemaphore>,
}

impl Permit {
    pub(crate) fn new(root: Arc<PrioritySemaphore>) -> Self {
        Self { root }
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.root.release_one();
    }
}
