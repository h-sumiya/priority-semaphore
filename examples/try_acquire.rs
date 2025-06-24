use std::sync::Arc;
use priority_semaphore::{PrioritySemaphore, TryAcquireError};

fn main() {
    let sem = Arc::new(PrioritySemaphore::new(2));

    match sem.try_acquire(0) {
        Ok(_permit) => println!("first permit immediate"),
        Err(_) => println!("failed to acquire first permit"),
    }

    match sem.try_acquire(0) {
        Ok(_permit) => println!("second permit immediate"),
        Err(_) => println!("failed to acquire second permit"),
    }

    match sem.try_acquire(0) {
        Ok(_) => println!("unexpected third permit"),
        Err(TryAcquireError::NoPermits) => println!("no permits left"),
        Err(TryAcquireError::Closed) => unreachable!(),
    }
}
