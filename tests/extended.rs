use std::sync::Arc;
use priority_semaphore::{PrioritySemaphore, AcquireError, TryAcquireError};

#[tokio::test(start_paused = true)]
async fn permits_released_on_drop() {
    let sem = Arc::new(PrioritySemaphore::new(1));
    // first permit acquired
    let first = sem.acquire(1).await.unwrap();
    assert_eq!(sem.available_permits(), 0);

    // spawn task waiting for next permit
    let sem_clone = sem.clone();
    let handle = tokio::spawn(async move { sem_clone.acquire(1).await });

    // dropping the first permit should wake the waiting task
    drop(first);
    let second = handle.await.unwrap().unwrap();
    assert_eq!(sem.available_permits(), 0);

    drop(second);
    assert_eq!(sem.available_permits(), 1);
}

#[tokio::test(start_paused = true)]
async fn close_wakes_waiters() {
    let sem = Arc::new(PrioritySemaphore::new(1));
    let _permit = sem.acquire(1).await.unwrap();

    let wait_task = tokio::spawn({
        let sem = sem.clone();
        async move { sem.acquire(5).await }
    });

    sem.close();

    match wait_task.await.unwrap() {
        Err(AcquireError::Closed) => {}
        _ => panic!("unexpected result"),
    }

    assert_eq!(sem.queued(), 0);
    assert!(matches!(sem.try_acquire(1), Err(TryAcquireError::Closed)));
}


#[test]
fn try_acquire_behaviour() {
    let sem = Arc::new(PrioritySemaphore::new(1));
    let permit = sem.try_acquire(0).unwrap();
    assert_eq!(sem.available_permits(), 0);
    assert!(matches!(sem.try_acquire(0), Err(TryAcquireError::NoPermits)));
    drop(permit);
    assert!(sem.try_acquire(0).is_ok());
}
