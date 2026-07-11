#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Runtime-agnostic priority semaphore.
//!
//! This crate provides [`PrioritySemaphore`], an asynchronous semaphore where
//! waiters supply a signed priority. Higher priorities are granted returned
//! permits before lower ones, and equal priorities use FIFO order. The
//! implementation uses only the standard `Future`/`Waker` contract and does
//! not depend on a particular async runtime.
//!
//! ```rust
//! use std::sync::Arc;
//! use priority_semaphore::PrioritySemaphore;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let semaphore = Arc::new(PrioritySemaphore::new(4));
//! let permit = semaphore.acquire(10).await.unwrap();
//! // The permit is returned automatically, including during unwinding.
//! drop(permit);
//! # }
//! ```

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod error;
mod lock;
mod permit;
mod queue;
mod semaphore;
mod util;
mod waiter;

pub use crate::error::{AcquireError, TryAcquireError};
pub use crate::permit::Permit;
pub use crate::semaphore::{Priority, PrioritySemaphore};
pub use crate::waiter::AcquireFuture;
