//! Elapsed time tracking and task sleeping

use alloc::{sync::Arc, vec::Vec};
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_utils::atomic::AtomicCell;
use futures_util::{task::AtomicWaker, Future};

use crate::util::Spinlock;

static TIME: OnceCell<Time> = OnceCell::uninit();

pub(crate) fn init() {
    TIME.init_once(Time::default);
}

#[derive(Default)]
struct Time {
    boot_elapsed: AtomicCell<u64>,
    sleepers: Spinlock<Vec<Arc<SleepCounter>>>,
}

#[inline]
pub fn boot_elapsed() -> u64 {
    TIME.get().map(|t| t.boot_elapsed.load()).unwrap_or(0)
}

pub(crate) fn increment() {
    TIME.get()
        .expect("tried to increment timer before initialization")
        .boot_elapsed
        .fetch_add(1);
}

pub(crate) fn wake_sleepers() {
    for s in TIME
        .get()
        .expect("tried to wake sleeping timers on uninitialized timer")
        .sleepers
        .lock_sync()
        .iter()
    {
        if s.is_done() {
            s.waker.wake();
        }
    }
}

#[derive(Default, Debug)]
struct SleepCounter {
    dur: u64,
    start: u64,
    waker: AtomicWaker,
}
impl SleepCounter {
    pub fn new(dur: u64) -> Self {
        Self {
            dur,
            start: boot_elapsed(),
            ..Default::default()
        }
    }

    #[inline]
    fn is_done(&self) -> bool {
        self.start + self.dur <= boot_elapsed()
    }

    fn wait(&self) -> SleepCounterFuture {
        SleepCounterFuture { counter: self }
    }
}

struct SleepCounterFuture<'a> {
    counter: &'a SleepCounter,
}
impl Future for SleepCounterFuture<'_> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let counter = this.counter;

        // fast
        if counter.is_done() {
            return Poll::Ready(());
        }

        counter.waker.register(cx.waker());
        if counter.is_done() {
            counter.waker.take();
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub async fn sleep(n: u64) {
    let s = Arc::new(SleepCounter::new(n));
    TIME.get()
        .expect("tried to sleep before timer was initialized")
        .sleepers
        .lock()
        .await
        .push(s.clone());
    s.wait().await;
}

pub fn sleep_sync(n: u64) {
    let s = SleepCounter::new(n);
    while !s.is_done() {}
}
