pub mod keyboard;
pub mod mouse;

pub(crate) fn init() {
    keyboard::init();
    mouse::init();
}
