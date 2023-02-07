use alloc::sync::Arc;
use conquer_once::spin::OnceCell;
use crossbeam_utils::atomic::AtomicCell;
use ps2_mouse::{Mouse as MouseDevice, MouseState};

use crate::util::Spinlock;

static MOUSE: OnceCell<Mouse> = OnceCell::uninit();

pub(super) fn init() {
    MOUSE.init_once(Mouse::new);
}

pub(crate) fn get() -> Option<Mouse> {
    MOUSE.get().cloned()
}

#[derive(Clone)]
pub(crate) struct Mouse {
    dev: Arc<Spinlock<MouseDevice>>,
    handler: fn(MouseState),
    state: Arc<AtomicCell<Option<MouseState>>>,
}
impl Mouse {
    fn new() -> Self {
        Self {
            dev: Arc::new(Spinlock::new(MouseDevice::new())),
            handler: Self::handler,
            state: Arc::new(AtomicCell::new(None)),
        }
    }

    fn handler(state: MouseState) {
        let this = MOUSE.get().expect("mouse not initialized");
        this.state.store(Some(state));
        log::trace!("mouse state: {:?}", state);
    }

    pub async fn add(&self, packet: u8) {
        let mut dev = self.dev.lock().await;
        dev.process_packet(packet);
        dev.set_on_complete(self.handler);
    }
}
