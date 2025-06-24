use priority_semaphore::PrioritySemaphore;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let sem = Arc::new(PrioritySemaphore::new(1));

    let high = sem.clone();
    let high_task = tokio::spawn(async move {
        let _permit = high.acquire(10).await.unwrap();
        println!("high priority task acquired permit");
    });

    let low = sem.clone();
    let low_task = tokio::spawn(async move {
        let _permit = low.acquire(1).await.unwrap();
        println!("low priority task acquired permit");
    });

    high_task.await.unwrap();
    low_task.await.unwrap();
}
