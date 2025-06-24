//! Simple synchronization primitive used internally by the semaphore.

#[cfg(feature = "std")]
pub(crate) mod imp {
    use std::sync::{Mutex as StdMutex, MutexGuard as StdGuard};

    /// Wrapper around `std::sync::Mutex`.
    #[derive(Debug)]
    pub(crate) struct Lock<T>(StdMutex<T>);

    impl<T> Lock<T> {
        /// Create a new locked value.
        pub const fn new(value: T) -> Self {
            Self(StdMutex::new(value))
        }

        /// Lock and get mutable access to the inner value.
        pub fn lock(&self) -> StdGuard<'_, T> {
            self.0.lock().unwrap()
        }
    }
}

#[cfg(not(feature = "std"))]
pub(crate) mod imp {
    use core::cell::{RefCell, RefMut};

    /// `RefCell` based lock used in `no_std` environments.
    #[derive(Debug)]
    pub(crate) struct Lock<T>(RefCell<T>);

    impl<T> Lock<T> {
        /// Create a new locked value.
        pub const fn new(value: T) -> Self {
            Self(RefCell::new(value))
        }

        /// Borrow the inner value mutably.
        pub fn lock(&self) -> RefMut<'_, T> {
            self.0.borrow_mut()
        }
    }
}

pub(crate) use imp::Lock;
