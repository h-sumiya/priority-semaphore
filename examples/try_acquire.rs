use priority_semaphore::{PrioritySemaphore, TryAcquireError};
use std::sync::Arc;

fn main() {
    let semaphore = Arc::new(PrioritySemaphore::new(2));

    // Keep both guards alive: a permit is returned when its guard is dropped.
    let first = semaphore.try_acquire(0).unwrap();
    let second = semaphore.try_acquire(0).unwrap();
    assert_eq!(semaphore.available_permits(), 0);

    assert_eq!(
        semaphore.try_acquire(0).unwrap_err(),
        TryAcquireError::NoPermits
    );

    drop(first);
    let third = semaphore.try_acquire(0).unwrap();
    println!("one returned permit was acquired immediately");

    drop((second, third));
    assert_eq!(semaphore.available_permits(), 2);
}
