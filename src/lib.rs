#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Runtime-agnostic priority semaphore.
//!

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
