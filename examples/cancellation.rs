use priority_semaphore::PrioritySemaphore;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let semaphore = Arc::new(PrioritySemaphore::new(1));
    let gate = semaphore.acquire(0).await.unwrap();

    let cancelled = {
        let semaphore = semaphore.clone();
        tokio::spawn(async move {
            let _permit = semaphore.acquire(100).await.unwrap();
        })
    };
    while semaphore.queued() != 1 {
        tokio::task::yield_now().await;
    }

    // Cancelling a queued acquire removes it without leaking its future
    // reservation. This is also safe if cancellation races with `drop(gate)`.
    cancelled.abort();
    let _ = cancelled.await;
    drop(gate);

    let permit = tokio::time::timeout(Duration::from_secs(1), semaphore.acquire(0))
        .await
        .expect("permit leaked during cancellation")
        .unwrap();
    println!("permit recovered after cancellation");
    drop(permit);
}
