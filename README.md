# priority-semaphore

[日本語](./README.ja.md)

A fast, runtime-independent, priority-aware asynchronous semaphore for Rust.

Each acquisition has an `i32` priority. The largest priority receives the next
returned permit; equal priorities are served FIFO.

## Why this implementation

- Lock-free atomic fast path when uncontended
- Direct permit handoff under contention: a newly arriving task cannot steal a
  permit reserved for a woken waiter
- O(log n) insertion/cancellation and O(1) waker replacement through an indexed
  generational heap
- Cancellation-safe before and after direct handoff
- Linearizable `close`, permit return, and queue registration
- No runtime dependency: works with Tokio, async-std, smol, or a custom executor
- Thread-safe in both `std` and `no_std + alloc` builds
- No unsafe code in this crate

## Installation

```toml
[dependencies]
priority-semaphore = "0.2.0"
```

## Example

```rust
use priority_semaphore::PrioritySemaphore;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let semaphore = Arc::new(PrioritySemaphore::new(8));

    let permit = semaphore.acquire(100).await.unwrap();
    // run priority work
    drop(permit); // returns it to the highest-priority queued task
}
```

`acquire` is called on `Arc<PrioritySemaphore>` because the returned RAII permit
owns the semaphore. Dropping an acquire future is always safe. `try_acquire`
does not bypass queued work, even when called with a larger priority.

See deterministic priority, cancellation, and immediate-acquisition examples:

```console
cargo run --example priority
cargo run --example cancellation
cargo run --example try_acquire
```

## Semantics

- Larger `i32` values mean higher priority.
- Equal priorities use FIFO order.
- Priority affects queued acquisitions only.
- Strict priority may starve a low-priority waiter if higher-priority work keeps
  arriving.
- `close()` rejects new acquisitions and wakes queued futures with
  `AcquireError::Closed`. Already assigned/acquired permits remain valid.
- A permit is returned on `Drop`, including unwinding and task cancellation.

## Feature flags

| Feature | Default | Description |
| --- | --- | --- |
| `std` | yes | Uses `parking_lot` for short contended queue operations |
| `docsrs` | no | docs.rs-only configuration |

Without `std`, the queue uses a small spin mutex and remains safe to share
between threads. The crate still requires `alloc`.

## Verification and performance

The test suite covers direct-handoff races, cancellation before and after
assignment, simultaneous close/release/cancellation, priority/FIFO ordering,
and sustained eight-thread churn. Criterion benchmarks include uncontended
acquire/release and contended handoff:

As a reference, one release-mode run on a local x86_64 machine measured about
**15.4 ns** per uncontended acquire/release (Tokio's owned permit measured about
**24.1 ns** in the same benchmark) and roughly **1.15 million** priority-aware
contended handoffs per second. Results vary by hardware and workload; the Tokio
comparison is illustrative because its semaphore provides FIFO rather than
priority ordering.

```console
cargo test --all-features
cargo test --release --all-features
cargo bench --bench throughput
```

## License

MIT OR Apache-2.0, at your option.
