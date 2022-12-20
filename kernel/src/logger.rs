use log::{Log, Record, Metadata};
use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use noto_sans_mono_bitmap::{get_bitmap, get_bitmap_width, BitmapChar, BitmapHeight, FontWeight};
use spinning_top::Spinlock;

use crate::serial::Serial;

const VSPACE: usize = 14;

pub static LOGGER: LockedLogger = LockedLogger::new();

pub struct LockedLogger {
    logger: Spinlock<Logger<1024>>,
    serial: Spinlock<Serial>,
}

impl LockedLogger {
    pub const fn new() -> Self {
        LockedLogger {
            logger: Spinlock::new(Logger::new()),
            serial: Spinlock::new(Serial::new()),
        }
    }

    pub unsafe fn force_unlock(&self) {
        self.logger.force_unlock();
        self.serial.force_unlock();
    }

    pub fn attach_framebuffer(&self, framebuffer: &'static mut [u8], info: FrameBufferInfo) {
        self.logger.lock().attach_framebuffer(framebuffer, info)
    }
}

impl Log for LockedLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        use core::fmt::Write;
        x86_64::instructions::interrupts::without_interrupts(|| {
            writeln!(self.serial.lock(), "[{}] {}", record.level(), record.args()).unwrap();
            writeln!(self.logger.lock(), "[{}] {}", record.level(), record.args()).unwrap();
        });
    }

    fn flush(&self) {}
}

pub enum Logger<const N: usize> {
    MemoryBacked(MemLogger<N>),
    FramebufferBacked(FbLogger),
}

impl<const N: usize> Logger<N> {
    pub const fn new() -> Self {
        Self::MemoryBacked(MemLogger::new())
    }
    pub fn attach_framebuffer(&mut self, framebuffer: &'static mut [u8], info: FrameBufferInfo) {
        match self {
            Logger::MemoryBacked(l) => {
                let mut fb = FbLogger::new(framebuffer, info);
                for c in l.buf.iter().filter(|c| c.is_some()).map(|c| c.unwrap()) {
                    fb.write(c);
                }
                *self = Logger::FramebufferBacked(fb)
            },
            Logger::FramebufferBacked(_) => return,
        }
    }

    fn write(&mut self, c: char) {
        match self {
            Logger::MemoryBacked(l) => l.write(c),
            Logger::FramebufferBacked(l) => l.write(c),
        }
    }
}

pub struct MemLogger<const N: usize> {
    buf: [Option<char>; N],
    next: usize,
}
impl<const N: usize> MemLogger<N> {
    pub const fn new() -> Self {
        Self {
            buf: [None; N],
            next: 0,
        }
    }

    pub fn write(&mut self, c: char) {
        if self.next == N { return; }
        self.buf[self.next] = Some(c);
        self.next += 1;
    }
}

pub struct FbLogger {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    x: usize,
    y: usize,
}

impl FbLogger {
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut logger = Self {
            framebuffer,
            info,
            x: 0,
            y: 0
        };
        logger.clear();
        logger
    }

    pub fn clear(&mut self) {
        self.x = 0;
        self.y = 0;
        self.framebuffer.fill(0);
    }

    fn carriage_return(&mut self) {
        self.x = 0;
    }

    fn newline(&mut self) {
        self.y += VSPACE;
        self.carriage_return()
    }

    fn write(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                if self.x >= self.width() {
                    self.newline();
                }
                const BITMAP_WIDTH: usize = get_bitmap_width(FontWeight::Regular, BitmapHeight::Size14);
                if self.y >= (self.height() - BITMAP_WIDTH) {
                    self.clear();
                }
                let bitmap_char = get_bitmap(c, FontWeight::Regular, BitmapHeight::Size14).unwrap();
                self.write_rendered(bitmap_char);
            }
        }
    }

    fn write_rendered(&mut self, c: BitmapChar) {
        for (y, row) in c.bitmap().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.set(self.x + x, self.y + y, *byte);
            }
        }
        self.x += c.width();
    }

    fn set(&mut self, x: usize, y: usize, intensity: u8) {
        let offset = y * self.info.stride + x;
        let color = match self.info.pixel_format {
            PixelFormat::Rgb => [intensity, intensity, intensity, 0],
            PixelFormat::Bgr => [intensity, intensity, intensity, 0],
            PixelFormat::U8 => [if intensity > 200 { 0xf } else { 0 }, 0, 0, 0],
            other => {
                self.info.pixel_format = PixelFormat::Rgb;
                panic!("pixel format {:?} not supported", other);
            }
        };
        let bpp = self.info.bytes_per_pixel;
        let byte_offset = offset * bpp;
        self.framebuffer[byte_offset..(byte_offset + bpp)]
            .copy_from_slice(&color[..bpp]);
        let _ = unsafe { core::ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }

    pub fn width(&self) -> usize {
        self.info.width
    }
    pub fn height(&self) -> usize {
        self.info.height
    }
}

unsafe impl<const N: usize> Send for Logger<N> {}
unsafe impl<const N: usize> Sync for Logger<N> {}

impl<const N: usize> core::fmt::Write for Logger<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write(c);
        }
        Ok(())
    }
}
