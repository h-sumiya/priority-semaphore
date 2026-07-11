//! Indexed, stable priority queue used by contended acquisitions.

use crate::{semaphore::Priority, waiter::Waiter};
use alloc::{sync::Arc, vec::Vec};
use core::{cmp::Ordering, task::Waker};

const VACANT: usize = usize::MAX;

/// Stable handle held by an acquire future while it is queued.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WaitKey {
    slot: usize,
    generation: usize,
}

#[derive(Debug)]
struct Slot {
    generation: usize,
    heap_index: usize,
    next_free: usize,
}

/// A queued waiter. Older waiters win ties at the same priority.
#[derive(Debug)]
pub(crate) struct WaiterEntry {
    priority: Priority,
    sequence: u64,
    key: WaitKey,
    pub(crate) waiter: Arc<Waiter>,
    pub(crate) waker: Waker,
}

impl WaiterEntry {
    fn outranks(&self, other: &Self) -> bool {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence.cmp(&self.sequence))
            == Ordering::Greater
    }
}

/// Binary max-heap plus a generational slot table.
///
/// The slot table makes cancellation and waker replacement O(log n) and O(1)
/// respectively, instead of scanning every queued waiter.
#[derive(Debug)]
pub(crate) struct WaitQueue {
    heap: Vec<WaiterEntry>,
    slots: Vec<Slot>,
    free_head: usize,
    next_sequence: u64,
}

impl WaitQueue {
    pub(crate) const fn new() -> Self {
        Self {
            heap: Vec::new(),
            slots: Vec::new(),
            free_head: VACANT,
            next_sequence: 0,
        }
    }

    pub(crate) fn push(
        &mut self,
        priority: Priority,
        waiter: Arc<Waiter>,
        waker: Waker,
    ) -> WaitKey {
        let key = self.allocate_slot();
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        let index = self.heap.len();
        self.heap.push(WaiterEntry {
            priority,
            sequence,
            key,
            waiter,
            waker,
        });
        self.slots[key.slot].heap_index = index;
        self.sift_up(index);
        key
    }

    pub(crate) fn pop(&mut self) -> Option<WaiterEntry> {
        (!self.heap.is_empty()).then(|| self.remove_at(0))
    }

    pub(crate) fn remove(&mut self, key: WaitKey) -> Option<WaiterEntry> {
        let index = self.index_of(key)?;
        Some(self.remove_at(index))
    }

    pub(crate) fn update_waker(&mut self, key: WaitKey, waker: &Waker) -> bool {
        let Some(index) = self.index_of(key) else {
            return false;
        };
        if !self.heap[index].waker.will_wake(waker) {
            self.heap[index].waker = waker.clone();
        }
        true
    }

    pub(crate) fn drain(&mut self) -> Vec<WaiterEntry> {
        // Closing does not need priority order. Taking the heap directly keeps
        // mass wake-up O(n), rather than repeatedly repairing it in O(n log n).
        let entries = core::mem::take(&mut self.heap);
        for entry in &entries {
            self.vacate_slot(entry.key);
        }
        entries
    }

    pub(crate) fn len(&self) -> usize {
        self.heap.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    fn allocate_slot(&mut self) -> WaitKey {
        if self.free_head == VACANT {
            let slot = self.slots.len();
            self.slots.push(Slot {
                generation: 0,
                heap_index: VACANT,
                next_free: VACANT,
            });
            WaitKey {
                slot,
                generation: 0,
            }
        } else {
            let slot = self.free_head;
            self.free_head = self.slots[slot].next_free;
            self.slots[slot].next_free = VACANT;
            WaitKey {
                slot,
                generation: self.slots[slot].generation,
            }
        }
    }

    fn index_of(&self, key: WaitKey) -> Option<usize> {
        let slot = self.slots.get(key.slot)?;
        (slot.generation == key.generation && slot.heap_index != VACANT).then_some(slot.heap_index)
    }

    fn remove_at(&mut self, index: usize) -> WaiterEntry {
        let removed = self.heap.swap_remove(index);
        self.vacate_slot(removed.key);

        if index < self.heap.len() {
            let moved_key = self.heap[index].key;
            self.slots[moved_key.slot].heap_index = index;
            if index > 0 && self.heap[index].outranks(&self.heap[(index - 1) / 2]) {
                self.sift_up(index);
            } else {
                self.sift_down(index);
            }
        }
        removed
    }

    fn vacate_slot(&mut self, key: WaitKey) {
        let slot = &mut self.slots[key.slot];
        slot.generation = slot.generation.wrapping_add(1);
        slot.heap_index = VACANT;
        slot.next_free = self.free_head;
        self.free_head = key.slot;
    }

    fn swap(&mut self, a: usize, b: usize) {
        self.heap.swap(a, b);
        self.slots[self.heap[a].key.slot].heap_index = a;
        self.slots[self.heap[b].key.slot].heap_index = b;
    }

    fn sift_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent = (index - 1) / 2;
            if !self.heap[index].outranks(&self.heap[parent]) {
                break;
            }
            self.swap(index, parent);
            index = parent;
        }
    }

    fn sift_down(&mut self, mut index: usize) {
        loop {
            let left = index * 2 + 1;
            if left >= self.heap.len() {
                return;
            }
            let right = left + 1;
            let best = if right < self.heap.len() && self.heap[right].outranks(&self.heap[left]) {
                right
            } else {
                left
            };
            if !self.heap[best].outranks(&self.heap[index]) {
                return;
            }
            self.swap(index, best);
            index = best;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop_waker() -> Waker {
        Waker::noop().clone()
    }

    #[test]
    fn priority_fifo_and_indexed_removal() {
        let mut queue = WaitQueue::new();
        let waker = noop_waker();
        let low = queue.push(1, Arc::new(Waiter::new()), waker.clone());
        let first_high = queue.push(9, Arc::new(Waiter::new()), waker.clone());
        let cancelled = queue.push(100, Arc::new(Waiter::new()), waker.clone());
        let second_high = queue.push(9, Arc::new(Waiter::new()), waker);

        assert!(queue.remove(cancelled).is_some());
        assert!(queue.remove(cancelled).is_none());
        assert_eq!(queue.pop().unwrap().key.slot, first_high.slot);
        assert_eq!(queue.pop().unwrap().key.slot, second_high.slot);
        assert_eq!(queue.pop().unwrap().key.slot, low.slot);
        assert!(queue.is_empty());
    }
}
