//! Kernel text buffer
//!
//! Used to print with [`print!`](crate::print) and [`println!`](crate::println).

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::SegQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt};
use heapless::{HistoryBuffer, String as StaticString};

use crate::util::Spinlock;

static mut KBUF: KernelBuffer = KernelBuffer::new();
static KBUF_LOCK: Spinlock<()> = Spinlock::new(());
static WAKER: AtomicWaker = AtomicWaker::new();

pub async fn read() -> Option<String> {
    unsafe { KBUF.next().await }
}

pub fn read_all() -> impl Iterator<Item = &'static str> {
    unsafe { KBUF.iter() }
}

/// Convert the kernel buffer to heap-allocated
///
/// # Safety
///
/// This function is unsafe because it only should be called once, after the kernel
/// heap has been initialized.
pub unsafe fn use_heap() {
    let temp: Vec<String> = KBUF.iter().map(String::from).collect();
    KBUF = KernelBuffer::Heap(HeapKernelBuffer::default());
    for s in temp {
        core::fmt::Write::write_str(&mut KBUF, &s).ok();
    }
    log::debug!("using heap for kernel buffer");
}

#[allow(clippy::large_enum_variant)]
enum KernelBuffer {
    Static(StaticKernelBuffer<96, 64>),
    Heap(HeapKernelBuffer),
}
impl KernelBuffer {
    pub const fn new() -> Self {
        Self::Static(StaticKernelBuffer::new())
    }
    fn iter(&self) -> impl Iterator<Item = &str> {
        let mut a = None;
        let mut b = None;
        match self {
            Self::Static(kbuf) => a = Some(kbuf.iter().map(|s| s.as_str())),
            Self::Heap(kbuf) => b = Some(kbuf.iter().map(|s| s.as_str())),
        }
        a.into_iter().flatten().chain(b.into_iter().flatten())
    }
}
impl core::fmt::Write for KernelBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self {
            Self::Static(kbuf) => kbuf.write_str(s),
            Self::Heap(kbuf) => kbuf.write_str(s),
        }
    }
}

impl Stream for KernelBuffer {
    type Item = String;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            KernelBuffer::Static(_) => panic!("static kernel buffer is not supported"),
            KernelBuffer::Heap(kbuf) => {
                if let Some(s) = kbuf.queue.pop() {
                    return Poll::Ready(Some(s));
                }

                WAKER.register(cx.waker());
                match kbuf.queue.pop() {
                    Some(s) => {
                        WAKER.take();
                        Poll::Ready(Some(s))
                    }
                    None => Poll::Pending,
                }
            }
        }
    }
}

struct StaticKernelBuffer<const LEN: usize, const CAP: usize> {
    buf: HistoryBuffer<StaticString<LEN>, CAP>,
}
impl<const LEN: usize, const CAP: usize> StaticKernelBuffer<LEN, CAP> {
    const fn new() -> Self {
        Self {
            buf: HistoryBuffer::new(),
        }
    }
    fn iter(&self) -> impl Iterator<Item = &StaticString<LEN>> {
        self.buf.oldest_ordered()
    }
}
impl<const LEN: usize, const CAP: usize> core::fmt::Write for StaticKernelBuffer<LEN, CAP> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.buf.write(StaticString::from(s));
        Ok(())
    }
}

#[derive(Default)]
struct HeapKernelBuffer {
    buf: Vec<String>,
    queue: SegQueue<String>,
}
impl HeapKernelBuffer {
    fn iter(&self) -> impl Iterator<Item = &String> {
        self.buf.iter()
    }
}
impl core::fmt::Write for HeapKernelBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.queue.push(s.to_string());
        WAKER.wake();
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;

    let _guard = KBUF_LOCK.lock_sync();

    crate::serial::_print(args);
    if !crate::task::executor::running() {
        crate::fb::_print(args);
    }

    unsafe {
        KBUF.write_fmt(args).expect("print failed");
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::kbuf::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n"); };
    ($fmt:expr) => { $crate::print!(concat!($fmt, "\n")); };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::print!(concat!($fmt, "\n"), $($arg)*);
    };
}
