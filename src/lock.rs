#[cfg(feature = "std")]
pub(crate) mod imp {
    use std::sync::{Mutex as StdMutex, MutexGuard as StdGuard};

    pub(crate) struct Lock<T>(StdMutex<T>);

    impl<T> Lock<T> {
        pub const fn new(value: T) -> Self { Self(StdMutex::new(value)) }
        pub fn lock(&self) -> StdGuard<'_, T> { self.0.lock().unwrap() }
    }

    pub(crate) type LockGuard<'a, T> = StdGuard<'a, T>;
}

#[cfg(not(feature = "std"))]
pub(crate) mod imp {
    use core::cell::{RefCell, RefMut};

    pub(crate) struct Lock<T>(RefCell<T>);

    impl<T> Lock<T> {
        pub const fn new(value: T) -> Self { Self(RefCell::new(value)) }
        pub fn lock(&self) -> RefMut<'_, T> { self.0.borrow_mut() }
    }

    pub(crate) type LockGuard<'a, T> = RefMut<'a, T>;
}

pub(crate) use imp::{Lock, LockGuard};
