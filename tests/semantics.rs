use priority_semaphore::{AcquireError, AcquireFuture, Permit, PrioritySemaphore, TryAcquireError};
use std::time::Duration;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};
use tokio::sync::{mpsc, oneshot};

fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    let mut context = Context::from_waker(Waker::noop());
    future.poll(&mut context)
}

async fn wait_for_queue(semaphore: &PrioritySemaphore, expected: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while semaphore.queued() != expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "queue did not reach {expected}; actual={}",
            semaphore.queued()
        )
    });
}

#[tokio::test]
async fn highest_priority_first_and_fifo_for_ties() {
    let semaphore = Arc::new(PrioritySemaphore::new(1));
    let gate = semaphore.acquire(0).await.unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let jobs = [(1, "low"), (10, "high-a"), (5, "medium"), (10, "high-b")];
    let mut tasks = Vec::new();

    // Queue each future before introducing the next one so FIFO tie ordering
    // does not depend on executor scheduling.
    for (index, (priority, name)) in jobs.into_iter().enumerate() {
        let task_semaphore = semaphore.clone();
        let tx = tx.clone();
        tasks.push(tokio::spawn(async move {
            let _permit = task_semaphore.acquire(priority).await.unwrap();
            tx.send(name).unwrap();
        }));
        wait_for_queue(&semaphore, index + 1).await;
    }
    drop(tx);
    drop(gate);

    let mut actual = Vec::new();
    while let Some(name) = rx.recv().await {
        actual.push(name);
    }
    for task in tasks {
        task.await.unwrap();
    }
    assert_eq!(actual, ["high-a", "high-b", "medium", "low"]);
    assert_eq!(semaphore.available_permits(), 1);
}

#[tokio::test]
async fn returned_permit_is_reserved_and_cannot_be_stolen() {
    let semaphore = Arc::new(PrioritySemaphore::new(1));
    let gate = semaphore.acquire(0).await.unwrap();
    let (acquired_tx, acquired_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();

    let waiter = tokio::spawn({
        let semaphore = semaphore.clone();
        async move {
            let _permit = semaphore.acquire(50).await.unwrap();
            acquired_tx.send(()).unwrap();
            release_rx.await.unwrap();
        }
    });
    wait_for_queue(&semaphore, 1).await;

    drop(gate);
    for _ in 0..1_000 {
        assert_eq!(
            semaphore.try_acquire(i32::MAX).unwrap_err(),
            TryAcquireError::NoPermits
        );
    }
    tokio::time::timeout(Duration::from_secs(5), acquired_rx)
        .await
        .unwrap()
        .unwrap();
    release_tx.send(()).unwrap();
    waiter.await.unwrap();
    assert_eq!(semaphore.available_permits(), 1);
}

#[tokio::test]
async fn cancelling_after_direct_handoff_returns_the_reserved_permit() {
    let semaphore = Arc::new(PrioritySemaphore::new(1));
    let gate = semaphore.acquire(0).await.unwrap();
    let mut future = Box::pin(semaphore.acquire(10));

    assert!(poll_once(future.as_mut()).is_pending());
    assert_eq!(semaphore.queued(), 1);
    drop(gate); // assigns directly, but the future is not polled again
    assert_eq!(semaphore.queued(), 0);
    assert_eq!(semaphore.available_permits(), 0);

    drop(future);
    assert_eq!(semaphore.available_permits(), 1);
    assert!(semaphore.try_acquire(0).is_ok());
}

#[tokio::test]
async fn cancellation_removes_only_the_target_waiter() {
    let semaphore = Arc::new(PrioritySemaphore::new(1));
    let gate = semaphore.acquire(0).await.unwrap();
    let mut low = Box::pin(semaphore.acquire(1));
    let mut cancelled = Box::pin(semaphore.acquire(100));
    let mut high = Box::pin(semaphore.acquire(10));

    assert!(poll_once(low.as_mut()).is_pending());
    assert!(poll_once(cancelled.as_mut()).is_pending());
    assert!(poll_once(high.as_mut()).is_pending());
    assert_eq!(semaphore.queued(), 3);
    drop(cancelled);
    assert_eq!(semaphore.queued(), 2);

    drop(gate);
    let high_permit = high.await.unwrap();
    assert!(poll_once(low.as_mut()).is_pending());
    drop(high_permit);
    drop(low.await.unwrap());
    assert_eq!(semaphore.available_permits(), 1);
}

#[tokio::test]
async fn close_wakes_all_waiters_and_is_idempotent() {
    let semaphore = Arc::new(PrioritySemaphore::new(2));
    let permits = [
        semaphore.acquire(0).await.unwrap(),
        semaphore.acquire(0).await.unwrap(),
    ];
    let mut tasks = Vec::new();
    for priority in -16..16 {
        let semaphore = semaphore.clone();
        tasks.push(tokio::spawn(async move {
            semaphore.acquire(priority).await.map(drop)
        }));
    }
    wait_for_queue(&semaphore, 32).await;

    semaphore.close();
    semaphore.close();
    for task in tasks {
        assert_eq!(task.await.unwrap(), Err(AcquireError::Closed));
    }
    assert!(semaphore.is_closed());
    assert_eq!(semaphore.queued(), 0);
    assert_eq!(
        semaphore.try_acquire(0).unwrap_err(),
        TryAcquireError::Closed
    );

    drop(permits);
    assert_eq!(semaphore.available_permits(), 2);
}

#[tokio::test]
async fn a_permit_assigned_before_close_still_succeeds() {
    let semaphore = Arc::new(PrioritySemaphore::new(1));
    let gate = semaphore.acquire(0).await.unwrap();
    let mut waiter = Box::pin(semaphore.acquire(0));
    assert!(poll_once(waiter.as_mut()).is_pending());

    drop(gate);
    semaphore.close();
    drop(waiter.await.unwrap());
    assert_eq!(semaphore.available_permits(), 1);
}

#[test]
fn immediate_acquisition_zero_capacity_and_debug_state() {
    let semaphore = Arc::new(PrioritySemaphore::new(1));
    let permit = semaphore.try_acquire(0).unwrap();
    assert_eq!(semaphore.available_permits(), 0);
    assert_eq!(
        semaphore.try_acquire(100).unwrap_err(),
        TryAcquireError::NoPermits
    );
    assert!(format!("{semaphore:?}").contains("max_permits: 1"));
    drop(permit);
    assert_eq!(semaphore.available_permits(), 1);

    let zero = Arc::new(PrioritySemaphore::new(0));
    assert_eq!(zero.available_permits(), 0);
    assert_eq!(zero.try_acquire(0).unwrap_err(), TryAcquireError::NoPermits);
}

#[test]
#[should_panic(expected = "too many semaphore permits")]
fn rejects_a_count_that_overlaps_internal_state_bits() {
    let _ = PrioritySemaphore::new(PrioritySemaphore::MAX_PERMITS + 1);
}

#[test]
fn public_concurrency_types_are_send_and_sync() {
    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<PrioritySemaphore>();
    assert_send_sync::<Permit>();
    assert_send::<AcquireFuture>();
}
