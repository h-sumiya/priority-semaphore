#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Runtime-agnostic priority semaphore.
//!
//! This crate provides [`PrioritySemaphore`], an asynchronous semaphore where
//! waiters supply a signed priority. Higher priorities are awakened before
//! lower ones. The implementation is runtime agnostic and works with either
//! **Tokio** or **async-std** when the corresponding feature is enabled.
//!
//! ```rust
//! use std::sync::Arc;
//! use priority_semaphore::PrioritySemaphore;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let sem = Arc::new(PrioritySemaphore::new(1));
//!
//! let hi = sem.clone();
//! tokio::spawn(async move {
//!     let _permit = hi.acquire(10).await.unwrap();
//!     println!("high priority acquired");
//! });
//!
//! let lo = sem.clone();
//! tokio::spawn(async move {
//!     let _permit = lo.acquire(1).await.unwrap();
//!     println!("low priority acquired");
//! });
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
