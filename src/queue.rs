//! Priority-ordered wait-queue based on `BinaryHeap`.

use alloc::collections::BinaryHeap;
use core::cmp::Ordering;
use core::task::Waker;
use crate::semaphore::Priority;

/// Internal queue entry holding waker & metadata.
#[derive(Debug)]
pub(crate) struct WaiterEntry {
    pub prio:  Priority,
    pub waker: Waker,
    pub id:    usize,
    pub weight: u32,
}

impl Eq for WaiterEntry {}
impl PartialEq for WaiterEntry {
    fn eq(&self, other: &Self) -> bool { self.id == other.id }
}
impl Ord for WaiterEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // max-heap: larger prio comes first
        self.prio.cmp(&other.prio)
    }
}
impl PartialOrd for WaiterEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Heap-based queue wrapper.
pub(crate) struct WaitQueue {
    heap: BinaryHeap<WaiterEntry>,
    next_id: usize,
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            next_id: 0,
        }
    }

    pub fn push(&mut self, prio: Priority, weight: u32, waker: Waker) -> usize {
        unimplemented!()
    }

    pub fn pop(&mut self) -> Option<WaiterEntry> {
        unimplemented!()
    }

    pub fn remove(&mut self, id: usize) {
        unimplemented!()
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }
}
