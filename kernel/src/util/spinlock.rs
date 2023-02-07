use core::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};
use futures_util::{task::AtomicWaker, Future};
use lock_api::{GuardSend, Mutex, MutexGuard, RawMutex};

pub struct RawSpinlock {
    locked: AtomicBool,
    waker: AtomicWaker,
}
impl RawSpinlock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            waker: AtomicWaker::new(),
        }
    }
}

unsafe impl RawMutex for RawSpinlock {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: RawSpinlock = RawSpinlock::new();

    type GuardMarker = GuardSend;

    fn lock(&self) {
        while !self.try_lock() {
            self.waker.wake();
        }
    }

    fn try_lock(&self) -> bool {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    unsafe fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

pub type SpinlockGuard<'a, T> = MutexGuard<'a, RawSpinlock, T>;

pub struct SpinlockGuardFuture<'a, T: 'a> {
    lock: Option<&'a Spinlock<T>>,
}
unsafe impl<'a, T> Send for SpinlockGuardFuture<'a, T> {}

impl<'a, T> Future for SpinlockGuardFuture<'a, T> {
    type Output = SpinlockGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        let lock = this
            .lock
            .expect("polled SpinlockGuardFuture after completion");
        let state = unsafe { lock.mutex.raw() };

        // fast
        if let Some(guard) = lock.mutex.try_lock() {
            this.lock = None;
            return Poll::Ready(guard);
        }

        state.waker.register(cx.waker());
        if let Some(guard) = lock.mutex.try_lock() {
            state.waker.take();
            this.lock = None;
            Poll::Ready(guard)
        } else {
            Poll::Pending
        }
    }
}

pub struct Spinlock<T> {
    mutex: Mutex<RawSpinlock, T>,
}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            mutex: Mutex::new(data),
        }
    }

    /// Lock the spinlock synchronously and return the underlying data as a `SpinlockGuard`.
    pub fn lock_sync(&self) -> SpinlockGuard<'_, T> {
        self.mutex.lock()
    }

    /// Lock the spinlock asynchronously and return the underlying data as a `SpinlockGuard` after
    /// resolving the future.
    pub fn lock(&self) -> SpinlockGuardFuture<T> {
        SpinlockGuardFuture { lock: Some(self) }
    }

    pub fn try_lock(&self) -> Option<SpinlockGuard<'_, T>> {
        self.mutex.try_lock()
    }

    pub fn is_locked(&self) -> bool {
        self.mutex.is_locked()
    }

    /// # Safety
    ///
    /// This function is unsafe because it only should be called if the lock is held by the current
    /// thread/task. Otherwise undefined behavior will occur.
    pub unsafe fn force_unlock(&self) {
        self.mutex.force_unlock()
    }
}

unsafe impl<T> Send for Spinlock<T> {}
unsafe impl<T> Sync for Spinlock<T> {}

impl<T: Default> Default for Spinlock<T> {
    fn default() -> Self {
        Self { mutex: Mutex::new(T::default()) }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for Spinlock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Spinlock").field("mutex", &self.mutex).finish()
    }
}
