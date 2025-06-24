use std::sync::Arc;

use priority_semaphore::PrioritySemaphore;

#[tokio::test(start_paused = true)]
async fn high_beats_low() {
    use std::sync::atomic::*;
    let sem = Arc::new(PrioritySemaphore::new(1));

    let hit = Arc::new(AtomicUsize::new(0));

    // low
    let l_hit = hit.clone();
    let l = tokio::spawn({
        let sem = sem.clone();
        async move {
            let _p = sem.acquire(1).await.unwrap();
            l_hit.fetch_add(1, Ordering::SeqCst);
        }
    });

    // high
    let h_hit = hit.clone();
    let h = tokio::spawn({
        let sem = sem.clone();
        async move {
            let _p = sem.acquire(10).await.unwrap();
            h_hit.fetch_add(10, Ordering::SeqCst);
        }
    });

    h.await.unwrap();
    l.await.unwrap();

    assert_eq!(hit.load(Ordering::SeqCst), 11);
}
