use core::{
    pin::Pin,
    task::{Context, Poll},
};

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt};

static PACKET_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub fn add_packet(packet: u8) {
    if let Ok(queue) = PACKET_QUEUE.try_get() {
        if queue.push(packet).is_err() {
            log::warn!("packet queue full; dropping packet");
        } else {
            WAKER.wake();
        }
    } else {
        log::warn!("packet queue not initialized");
    }
}

struct PacketStream {
    _private: (),
}
impl PacketStream {
    fn new() -> Self {
        PACKET_QUEUE
            .try_init_once(|| ArrayQueue::new(1024))
            .expect("packet queue already initialized");
        Self { _private: () }
    }
}
impl Stream for PacketStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = PACKET_QUEUE
            .try_get()
            .expect("packet queue should be initialized by now");

        if let Some(p) = queue.pop() {
            return Poll::Ready(Some(p));
        }

        WAKER.register(cx.waker());
        match queue.pop() {
            Some(p) => {
                WAKER.take();
                Poll::Ready(Some(p))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn process() {
    let mouse = crate::peripheral::mouse::get().expect("mouse should be initialized by now");
    let mut packets = PacketStream::new();
    while let Some(packet) = packets.next().await {
        mouse.add(packet).await;
    }
}
