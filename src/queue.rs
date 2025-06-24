//! Priority-ordered wait-queue based on `BinaryHeap`.

use alloc::vec::Vec;
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
    heap: Vec<WaiterEntry>,
    next_id: usize,
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            heap: Vec::new(),
            next_id: 0,
        }
    }

    pub fn push(&mut self, prio: Priority, weight: u32, waker: Waker) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.heap.push(WaiterEntry { prio, waker, id, weight });
        self.sift_up(self.heap.len() - 1);
        id
    }

    pub fn pop(&mut self) -> Option<WaiterEntry> {
        if self.heap.is_empty() {
            return None;
        }
        let last = self.heap.pop().unwrap();
        if self.heap.is_empty() {
            return Some(last);
        }
        let ret = core::mem::replace(&mut self.heap[0], last);
        self.sift_down(0);
        Some(ret)
    }

    pub fn remove(&mut self, id: usize) {
        if let Some(pos) = self.heap.iter().position(|e| e.id == id) {
            let last = self.heap.pop().unwrap();
            if pos == self.heap.len() {
                return;
            }
            self.heap[pos] = last;
            if pos > 0 && self.heap[pos] > self.heap[(pos - 1) / 2] {
                self.sift_up(pos);
            } else {
                self.sift_down(pos);
            }
        }
    }

    pub fn update_waker(&mut self, id: usize, waker: Waker) {
        if let Some(entry) = self.heap.iter_mut().find(|e| e.id == id) {
            entry.waker = waker;
        }
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    #[inline]
    fn sift_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx - 1) >> 1;
            if self.heap[parent] >= self.heap[idx] {
                break;
            }
            self.heap.swap(parent, idx);
            idx = parent;
        }
    }

    #[inline]
    fn sift_down(&mut self, mut idx: usize) {
        let len = self.heap.len();
        loop {
            let left = (idx << 1) + 1;
            if left >= len {
                break;
            }
            let right = left + 1;
            let mut child = left;
            if right < len && self.heap[right] > self.heap[left] {
                child = right;
            }
            if self.heap[idx] >= self.heap[child] {
                break;
            }
            self.heap.swap(idx, child);
            idx = child;
        }
    }
}
