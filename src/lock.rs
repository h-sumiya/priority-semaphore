//! Small, non-poisoning synchronization primitive used by the short queue
//! critical sections.

#[cfg(feature = "std")]
pub(crate) mod imp {
    use parking_lot::{Mutex, MutexGuard};

    #[derive(Debug)]
    pub(crate) struct Lock<T>(Mutex<T>);

    impl<T> Lock<T> {
        /// Create a new locked value.
        pub const fn new(value: T) -> Self {
            Self(Mutex::new(value))
        }

        /// Lock and get mutable access to the inner value.
        pub fn lock(&self) -> MutexGuard<'_, T> {
            self.0.lock()
        }
    }
}

#[cfg(not(feature = "std"))]
pub(crate) mod imp {
    use spin::{Mutex, MutexGuard, relax::Spin};

    #[derive(Debug)]
    pub(crate) struct Lock<T>(Mutex<T>);

    impl<T> Lock<T> {
        /// Create a new locked value.
        pub const fn new(value: T) -> Self {
            Self(Mutex::new(value))
        }

        /// Borrow the inner value mutably.
        pub fn lock(&self) -> MutexGuard<'_, T, Spin> {
            self.0.lock()
        }
    }
}

pub(crate) use imp::Lock;
