#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Runtime-agnostic priority semaphore.
//!
//! 有効な Cargo feature に応じて `tokio` / `async-std` などで動作します。

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod error;
mod permit;
mod queue;
mod semaphore;
mod util;
mod waiter;

pub use crate::error::{AcquireError, TryAcquireError};
pub use crate::permit::Permit;
pub use crate::semaphore::{Priority, PrioritySemaphore};
