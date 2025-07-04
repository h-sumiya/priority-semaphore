use std::sync::Arc;

use priority_semaphore::PrioritySemaphore;

#[tokio::main]
async fn main() {
    let semaphore = PrioritySemaphore::new(10);
    let semaphore = Arc::new(semaphore);

    for i in 0..10 {
        let permit = semaphore.acquire(i).await.unwrap();
        tokio::spawn(async move {
            let _permit = permit;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            println!("Permit {} released", i);
        });
    }

    for _ in 0..10 {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(4)) => {
                panic!("Timeout waiting for semaphore permits");
            }
            permit = semaphore.acquire(1) => {
                println!("Acquired permit: {:?}", permit);
            }
        }
    }
}
