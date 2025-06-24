# priority-semaphore

[日本語](./README.ja.md)

Runtime-agnostic priority aware asynchronous semaphore for Rust.

This crate allows tasks to acquire permits with a signed priority. Higher
priorities wake first, making it easy to favour important work while still
preventing starvation.

## Features

- Works with **Tokio** or **async-std** (feature gated)
- Cancellation safe `acquire`
- Optional ageing strategy via the `ageing` feature
- Zero `unsafe` code

## Example

```rust
use std::sync::Arc;
use priority_semaphore::PrioritySemaphore;

#[tokio::main]
async fn main() {
    let sem = Arc::new(PrioritySemaphore::new(1));

    let hi = sem.clone();
    let h = tokio::spawn(async move {
        let _permit = hi.acquire(10).await.unwrap();
        println!("high priority job");
    });

    let lo = sem.clone();
    let l = tokio::spawn(async move {
        let _permit = lo.acquire(1).await.unwrap();
        println!("low priority job");
    });

    h.await.unwrap();
    l.await.unwrap();
}
```

More examples can be found in the [`examples`](./examples) directory.

## Crate features

| Feature     | Default | Description                                 |
| ----------- | ------- | ------------------------------------------- |
| `tokio`     | ✔       | Enable support for the Tokio runtime        |
| `async-std` | ❌      | Enable support for async-std                |
| `ageing`    | ❌      | Simple ageing strategy to reduce starvation |
| `std`       | ✔       | Use the standard library                    |
| `docsrs`    | ❌      | Internal feature used by docs.rs            |

## License

This project is licensed under either the MIT license or the
Apache License 2.0, at your option.
