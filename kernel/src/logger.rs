use log::{Log, Record, Metadata};
use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use noto_sans_mono_bitmap::{get_bitmap, get_bitmap_width, BitmapChar, BitmapHeight, FontWeight};
use conquer_once::spin::OnceCell;
use spinning_top::Spinlock;

const VSPACE: usize = 14;

pub static LOGGER: OnceCell<LockedLogger> = OnceCell::uninit();

pub struct LockedLogger(Spinlock<Logger>);

impl LockedLogger {
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        LockedLogger(Spinlock::new(Logger::new(framebuffer, info)))
    }

    pub unsafe fn force_unlock(&self) {
        self.0.force_unlock();
    }
}

impl Log for LockedLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        use core::fmt::Write;
        let mut logger = self.0.lock();
        writeln!(logger, "[{}] {}", record.level(), record.args()).unwrap();
    }

    fn flush(&self) {}
}
pub struct Logger {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    x: usize,
    y: usize,
}

impl Logger {
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

unsafe impl Send for Logger {}
unsafe impl Sync for Logger {}

impl core::fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write(c);
        }
        Ok(())
    }
}
