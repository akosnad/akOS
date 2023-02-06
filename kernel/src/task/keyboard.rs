use core::{
    pin::Pin,
    task::{Context, Poll},
};

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt};

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_err() {
            log::error!("keyboard scancode queue full, dropping input!");
        } else {
            WAKER.wake();
        }
    } else {
        log::warn!("keyboard scancode queue uninitialized, dropping input!");
    }
}

struct ScancodeStream {
    _private: (),
}
impl ScancodeStream {
    fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(1024))
            .expect("scancode queue failed to init");
        Self { _private: () }
    }
}
impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue should be initialized by now");

        if let Some(sc) = queue.pop() {
            return Poll::Ready(Some(sc));
        }

        WAKER.register(cx.waker());
        match queue.pop() {
            Some(sc) => {
                WAKER.take();
                Poll::Ready(Some(sc))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn process() {
    let keyboard =
        crate::peripheral::keyboard::get().expect("keyboard should be initialized by now");
    let mut scancodes = ScancodeStream::new();
    while let Some(sc) = scancodes.next().await {
        keyboard.add(sc).await;
    }
}
