use log::{Log, Record, Metadata};
use bootloader_api::info::FrameBufferInfo;

pub struct KernelLogger {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    x: usize,
    y: usize,
}

impl KernelLogger {
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        Self {
            framebuffer,
            info,
            x: 0,
            y: 0
        }
    }
}

impl Log for KernelLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {}

    fn flush(&self) {}
}
