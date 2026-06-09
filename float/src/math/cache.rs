#[cfg(not(feature = "std"))]
use core::cell::UnsafeCell;

#[cfg(not(feature = "std"))]
use core::ops::{Deref, DerefMut};
#[cfg(not(feature = "std"))]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "std")]
use std::sync::{Condvar, Mutex, MutexGuard};

use alloc::sync::Arc;

pub enum CacheState<T> {
    Empty,
    Computing { target_terms: usize },
    Poisoned,
    Ready(Arc<T>),
}

#[cfg(feature = "std")]
pub struct MathCache<T> {
    inner: Mutex<CacheState<T>>,
    cond: Condvar,
}

#[cfg(not(feature = "std"))]
pub struct MathCache<T> {
    locked: AtomicUsize,
    data: UnsafeCell<CacheState<T>>,
}

// SAFETY: The MathCache safely encapsulates the state. Transferring ownership
// across thread boundaries is safe as long as the inner data `T` is `Send`.
#[cfg(not(feature = "std"))]
unsafe impl<T: Send> Send for MathCache<T> {}

// SAFETY: MathCache uses an AtomicUsize spinlock to protect interior mutability.
// Under concurrent access, threads busy-wait (spin) rather than sleeping.
// This is sound but may cause excessive CPU usage if the computing thread
// takes a long time (e.g. high-precision calculations). Recommended for
// single-threaded no_std environments only.
#[cfg(not(feature = "std"))]
unsafe impl<T: Send + Sync> Sync for MathCache<T> {}

#[cfg(not(feature = "std"))]
pub struct CacheGuard<'a, T> {
    lock: &'a MathCache<T>,
}

#[cfg(not(feature = "std"))]
impl<T> Deref for CacheGuard<'_, T> {
    type Target = CacheState<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

#[cfg(not(feature = "std"))]
impl<T> DerefMut for CacheGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

#[cfg(not(feature = "std"))]
impl<T> Drop for CacheGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(0, Ordering::Release);
    }
}

impl<T> MathCache<T> {
    pub const fn new() -> Self {
        #[cfg(feature = "std")]
        {
            Self {
                inner: Mutex::new(CacheState::Empty),
                cond: Condvar::new(),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            Self {
                locked: AtomicUsize::new(0),
                data: UnsafeCell::new(CacheState::Empty),
            }
        }
    }

    pub fn clear(&self) {
        let mut guard = self.lock();
        if matches!(*guard, CacheState::Ready(_)) {
            *guard = CacheState::Empty;
        }
    }

    pub fn peek<R, F: FnOnce(&T) -> R>(&self, f: F) -> Option<R> {
        let guard = self.lock();
        if let CacheState::Ready(state) = &*guard {
            Some(f(state))
        } else {
            None
        }
    }

    #[cfg(feature = "std")]
    fn lock(&self) -> MutexGuard<'_, CacheState<T>> {
        self.inner
            .lock()
            .expect("MathCache mutex should not be poisoned")
    }

    #[cfg(not(feature = "std"))]
    fn lock(&self) -> CacheGuard<'_, T> {
        loop {
            if self
                .locked
                .compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return CacheGuard { lock: self };
            }
            core::hint::spin_loop();
        }
    }

    #[cfg(feature = "std")]
    fn notify_all(&self) {
        self.cond.notify_all();
    }

    #[cfg(not(feature = "std"))]
    fn notify_all(&self) {}

    #[cfg(feature = "std")]
    fn wait<'a>(&'a self, guard: MutexGuard<'a, CacheState<T>>) -> MutexGuard<'a, CacheState<T>> {
        self.cond
            .wait(guard)
            .expect("MathCache condvar should not be poisoned")
    }

    #[cfg(not(feature = "std"))]
    fn wait<'a>(&'a self, guard: CacheGuard<'a, T>) -> CacheGuard<'a, T> {
        drop(guard);
        core::hint::spin_loop();
        self.lock()
    }

    /// Retrieve the cached state or compute it using the provided closure.
    /// `compute` takes `(Option<T>, usize)` and returns the new state, where
    /// `usize` is the `target_terms`.
    pub fn get_or_compute<F, G>(
        &self,
        required_terms: usize,
        get_terms: G,
        mut compute: F,
    ) -> Arc<T>
    where
        F: FnMut(Option<Arc<T>>, usize) -> T,
        G: Fn(&T) -> usize,
    {
        // ComputeGuard ensures that if the computing thread panics, the cache
        // is poisoned to avoid deadlocking other waiting threads.
        struct ComputeGuard<'a, T> {
            cache: &'a MathCache<T>,
            is_computing: bool,
            completed: bool,
        }
        impl<T> Drop for ComputeGuard<'_, T> {
            fn drop(&mut self) {
                if self.is_computing && !self.completed {
                    {
                        let mut guard = self.cache.lock();
                        *guard = CacheState::Poisoned;
                    }
                    self.cache.notify_all();
                }
            }
        }
        let mut compute_guard = ComputeGuard {
            cache: self,
            is_computing: false,
            completed: false,
        };

        let mut guard = self.lock();
        let mut existing_state = None;

        loop {
            match &mut *guard {
                CacheState::Empty => {
                    *guard = CacheState::Computing {
                        target_terms: required_terms,
                    };
                    break;
                }
                CacheState::Poisoned => {
                    panic!("MathCache is poisoned due to a panic during computation");
                }
                CacheState::Ready(state) => {
                    if get_terms(state) >= required_terms {
                        return state.clone();
                    }

                    let CacheState::Ready(state) = core::mem::replace(
                        &mut *guard,
                        CacheState::Computing {
                            target_terms: required_terms,
                        },
                    ) else {
                        unreachable!()
                    };
                    existing_state = Some(state);
                    break;
                }
                CacheState::Computing { target_terms } => {
                    if required_terms > *target_terms {
                        *target_terms = required_terms;
                    }
                    guard = self.wait(guard);
                }
            }
        }

        compute_guard.is_computing = true;
        let mut target = required_terms;

        loop {
            drop(guard);
            existing_state = Some(Arc::new(compute(existing_state, target)));
            guard = self.lock();

            match &mut *guard {
                CacheState::Computing { target_terms } => {
                    if *target_terms <= target {
                        let final_state = existing_state
                            .expect("existing_state must be Some when target is reached");
                        let ret = final_state.clone();
                        *guard = CacheState::Ready(final_state);
                        compute_guard.completed = true;

                        drop(guard);
                        self.notify_all();
                        return ret;
                    }

                    target = *target_terms;
                }
                _ => unreachable!(),
            }
        }
    }
}
