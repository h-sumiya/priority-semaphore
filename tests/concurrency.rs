use priority_semaphore::PrioritySemaphore;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

async fn wait_for_queue(semaphore: &PrioritySemaphore, expected: usize) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while semaphore.queued() != expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn heavy_multithreaded_churn_never_exceeds_capacity() {
    const PERMITS: usize = 7;
    const TASKS: usize = 256;
    const ACQUIRES_PER_TASK: usize = 200;

    tokio::time::timeout(Duration::from_secs(30), async {
        let semaphore = Arc::new(PrioritySemaphore::new(PERMITS));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::with_capacity(TASKS);

        for task_id in 0..TASKS {
            let semaphore = semaphore.clone();
            let active = active.clone();
            let peak = peak.clone();
            let completed = completed.clone();
            tasks.push(tokio::spawn(async move {
                for iteration in 0..ACQUIRES_PER_TASK {
                    let priority = ((task_id * 31 + iteration * 17) % 101) as i32 - 50;
                    let permit = semaphore.acquire(priority).await.unwrap();
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    assert!(now <= PERMITS, "capacity exceeded: {now} > {PERMITS}");
                    if iteration % 3 == 0 {
                        tokio::task::yield_now().await;
                    }
                    active.fetch_sub(1, Ordering::SeqCst);
                    completed.fetch_add(1, Ordering::Relaxed);
                    drop(permit);
                }
            }));
        }

        for task in tasks {
            task.await.unwrap();
        }
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(completed.load(Ordering::Relaxed), TASKS * ACQUIRES_PER_TASK);
        assert_eq!(semaphore.available_permits(), PERMITS);
        assert_eq!(semaphore.queued(), 0);
        assert!(peak.load(Ordering::SeqCst) > 1);
    })
    .await
    .expect("multithreaded churn deadlocked");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn mass_cancellation_preserves_every_permit() {
    const WAITERS: usize = 4_096;
    const PERMITS: usize = 4;

    tokio::time::timeout(Duration::from_secs(30), async {
        let semaphore = Arc::new(PrioritySemaphore::new(PERMITS));
        let mut gates = Vec::new();
        for _ in 0..PERMITS {
            gates.push(semaphore.acquire(0).await.unwrap());
        }

        let mut tasks = Vec::with_capacity(WAITERS);
        for index in 0..WAITERS {
            let semaphore = semaphore.clone();
            tasks.push(tokio::spawn(async move {
                drop(semaphore.acquire((index % 97) as i32).await.unwrap());
            }));
        }
        wait_for_queue(&semaphore, WAITERS).await;

        for (index, task) in tasks.iter().enumerate() {
            if index % 3 != 0 {
                task.abort();
            }
        }
        for (index, task) in tasks.iter_mut().enumerate() {
            if index % 3 != 0 {
                assert!(task.await.unwrap_err().is_cancelled());
            }
        }

        drop(gates);
        for (index, task) in tasks.into_iter().enumerate() {
            if index % 3 == 0 {
                task.await.unwrap();
            }
        }
        assert_eq!(semaphore.queued(), 0);
        assert_eq!(semaphore.available_permits(), PERMITS);
    })
    .await
    .expect("mass cancellation deadlocked");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn close_release_and_cancellation_can_race_repeatedly() {
    tokio::time::timeout(Duration::from_secs(30), async {
        for round in 0..200 {
            let semaphore = Arc::new(PrioritySemaphore::new(1));
            let gate = semaphore.acquire(0).await.unwrap();
            let mut tasks = Vec::new();
            for priority in 0..32 {
                let semaphore = semaphore.clone();
                tasks.push(tokio::spawn(async move {
                    let _ = semaphore.acquire(priority).await;
                }));
            }
            wait_for_queue(&semaphore, 32).await;

            for task in tasks.iter().skip(round % 4).step_by(4) {
                task.abort();
            }
            let closer = {
                let semaphore = semaphore.clone();
                tokio::spawn(async move { semaphore.close() })
            };
            drop(gate);
            closer.await.unwrap();
            for task in tasks {
                let _ = task.await;
            }
            assert_eq!(semaphore.queued(), 0);
            assert_eq!(semaphore.available_permits(), 1);
        }
    })
    .await
    .expect("close/release/cancellation race deadlocked");
}
