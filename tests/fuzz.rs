use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use priority_semaphore::{PrioritySemaphore, TryAcquireError};
use rand::{Rng, SeedableRng, rngs::StdRng};
use tokio::time::sleep;

const MAX_PERMITS: usize = 5;
const TASKS: usize = 10_000;
const SEED: u64 = 0xCAFEBABE;

enum OpFuture {
    Pending(tokio::task::JoinHandle<Result<(), ()>>),
    Cancelled,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn random_stress_test() {
    let sem = Arc::new(PrioritySemaphore::new(MAX_PERMITS));

    let in_flight = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));

    let mut rng = StdRng::seed_from_u64(SEED);
    let mut futures: Vec<OpFuture> = Vec::with_capacity(TASKS);

    for _ in 0..TASKS {
        let action = rng.random_range(0u8..=3);

        match action {
            0 => {
                let sem = sem.clone();
                let in_flight = in_flight.clone();
                let peak = peak.clone();
                let prio: i32 = rng.random_range(-10..=10);

                let handle = tokio::spawn(async move {
                    match sem.acquire(prio).await {
                        Ok(_permit) => {
                            let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                            peak.fetch_max(now, Ordering::SeqCst);
                            tokio::task::yield_now().await;
                            sleep(Duration::from_micros(10)).await;

                            in_flight.fetch_sub(1, Ordering::SeqCst);
                            Ok(())
                        }
                        Err(_) => Err(()),
                    }
                });
                futures.push(OpFuture::Pending(handle));
            }

            1 => {
                let prio: i32 = rng.random_range(-10..=10);
                match sem.try_acquire(prio) {
                    Ok(_permit) => {
                        let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(now, Ordering::SeqCst);
                        in_flight.fetch_sub(1, Ordering::SeqCst);
                    }
                    Err(TryAcquireError::NoPermits) => {}
                    Err(TryAcquireError::Closed) => {}
                }
            }

            2 => {
                sem.close();
            }

            3 => {
                let sem = sem.clone();
                let prio: i32 = rng.random_range(-10..=10);
                let fut = sem.acquire(prio);
                tokio::task::yield_now().await;
                drop(fut);
                futures.push(OpFuture::Cancelled);
            }

            _ => unreachable!(),
        }
    }

    for f in futures {
        if let OpFuture::Pending(h) = f {
            match h.await.unwrap() {
                Ok(()) | Err(()) => {}
            }
        }
    }

    assert!(
        peak.load(Ordering::SeqCst) <= MAX_PERMITS,
        "concurrency exceeded max permits: peak = {}, limit = {}",
        peak.load(Ordering::SeqCst),
        MAX_PERMITS
    );

    assert_eq!(
        sem.available_permits(),
        MAX_PERMITS,
        "all permits should have been returned"
    );

    assert_eq!(
        sem.queued(),
        0,
        "wait-queue should be empty after test completion"
    );
}
