# priority-semaphore (WIP,for ai agents)

Tiny, runtime-agnostic **priority-aware async semaphore** for Rust.  
_Currently under heavy development—APIs and internals will break._

---

## 📂 `src/` layout

| File               | Purpose (1-liner)                                          |
| ------------------ | ---------------------------------------------------------- |
| **`lib.rs`**       | Crate entry; feature gates & public re-exports.            |
| **`semaphore.rs`** | Core `PrioritySemaphore` logic (permits, dispatch, close). |
| **`permit.rs`**    | RAII guard that returns the permit on `Drop`.              |
| **`queue.rs`**     | High-performance heap queue (`push / pop / remove`).       |
| **`waiter.rs`**    | `AcquireFuture`; cancellation-safe `poll` & cleanup.       |
| **`error.rs`**     | Tiny error enums (`TryAcquireError`, `AcquireError`).      |
| **`util.rs`**      | Misc helpers/macros (`doc_cfg`, loom shims, etc.).         |

---

## ⚡ Quick idea

_Works like Tokio’s semaphore, but tasks carry a signed `priority`._  
Higher numbers wake first; starvation strategies (aging / round-robin) are feature-toggled.

```rust
let sem = Arc::new(PrioritySemaphore::new(3));

let hi = sem.clone();
tokio::spawn(async move {
    let _p = hi.acquire(10).await.unwrap(); // high prio
    /* … */
});

let lo = sem.clone();
tokio::spawn(async move {
    let _p = lo.acquire(1).await.unwrap(); // low prio
    /* … */
});
```
