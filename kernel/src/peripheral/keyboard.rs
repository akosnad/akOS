use alloc::sync::Arc;
use conquer_once::spin::OnceCell;
use pc_keyboard::{
    layouts::Us104Key,
    DecodedKey::{RawKey, Unicode},
    HandleControl,
    KeyState::{Down, SingleShot, Up},
    Keyboard as KeyboardDevice, ScancodeSet1,
};

use crate::{print, util::Spinlock};

static KEYBOARD: OnceCell<Keyboard> = OnceCell::uninit();

pub(super) fn init() {
    KEYBOARD.init_once(Keyboard::new);
}

pub(crate) fn get() -> Option<Keyboard> {
    KEYBOARD.get().cloned()
}

#[derive(Clone)]
pub(crate) struct Keyboard {
    dev: Arc<Spinlock<KeyboardDevice<Us104Key, ScancodeSet1>>>,
}
impl Keyboard {
    pub fn new() -> Self {
        Self {
            dev: Arc::new(Spinlock::new(KeyboardDevice::new(HandleControl::Ignore))),
        }
    }
    pub async fn add(&self, scancode: u8) {
        let mut dev = self.dev.lock().await;
        if let Some(ev) = dev
            .add_byte(scancode)
            .expect("failed to add byte to keyboard device processor")
        {
            let key = dev.process_keyevent(ev.clone());
            drop(dev);

            match key {
                Some(key) => match key {
                    Unicode(c) => {
                        print!("{}", c);
                    }
                    RawKey(_) => {}
                },
                None => match ev.state {
                    Up => {}
                    Down | SingleShot => {}
                },
            }
        }
    }
}
