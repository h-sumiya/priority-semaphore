use priority_semaphore::PrioritySemaphore;
use std::sync::Arc;
use tokio::sync::mpsc;

async fn wait_until_queued(semaphore: &PrioritySemaphore, count: usize) {
    while semaphore.queued() != count {
        tokio::task::yield_now().await;
    }
}

#[tokio::main]
async fn main() {
    let semaphore = Arc::new(PrioritySemaphore::new(1));
    let gate = semaphore.acquire(0).await.unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let low = {
        let semaphore = semaphore.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let _permit = semaphore.acquire(1).await.unwrap();
            tx.send("low priority").unwrap();
        })
    };
    wait_until_queued(&semaphore, 1).await;

    let high = {
        let semaphore = semaphore.clone();
        tokio::spawn(async move {
            let _permit = semaphore.acquire(100).await.unwrap();
            tx.send("high priority").unwrap();
        })
    };
    wait_until_queued(&semaphore, 2).await;

    // Both tasks are waiting now. Releasing the gate deterministically grants
    // the permit to priority 100 first.
    drop(gate);
    println!("acquired: {}", rx.recv().await.unwrap());
    println!("acquired: {}", rx.recv().await.unwrap());

    high.await.unwrap();
    low.await.unwrap();
}
