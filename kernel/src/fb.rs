use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use conquer_once::{spin::OnceCell, TryGetError};
use noto_sans_mono_bitmap::{
    get_raster, get_raster_width, FontWeight, RasterHeight, RasterizedChar,
};
use spinning_top::Spinlock;

const VSPACE: usize = noto_sans_mono_bitmap::RasterHeight::Size16 as usize;

static FRAMEBUFFER: LockedFramebuffer = LockedFramebuffer::uninit();
type LockedFramebuffer = OnceCell<Spinlock<Framebuffer>>;

pub fn init(buf: &'static mut [u8], info: FrameBufferInfo) {
    FRAMEBUFFER.init_once(|| Spinlock::new(Framebuffer::new(buf, info)));
    log::debug!("hello framebuffer");
}

/// # Safety
///
/// This function is unsafe because it only should be called by the panic handler.
/// This is needed to ensure that the framebuffer is available for printing panic messages.
pub(crate) unsafe fn force_unlock() -> Result<(), TryGetError> {
    FRAMEBUFFER.try_get()?.force_unlock();
    Ok(())
}

struct Framebuffer {
    buf: &'static mut [u8],
    info: FrameBufferInfo,
    x: usize,
    y: usize,
}

impl Framebuffer {
    pub fn new(buf: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut fb = Self {
            buf,
            info,
            x: 0,
            y: 0,
        };
        fb.clear();
        fb
    }

    pub fn clear(&mut self) {
        self.x = 0;
        self.y = 0;
        self.buf.fill(0);
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
                const TAB_SIZE: usize = 8;
                const BITMAP_WIDTH: usize =
                    get_raster_width(FontWeight::Regular, RasterHeight::Size16);

                if self.x / BITMAP_WIDTH >= self.width() / BITMAP_WIDTH {
                    self.newline();
                }
                if self.y >= (self.height() - BITMAP_WIDTH) {
                    self.clear();
                }
                if c == '\t' {
                    self.x += (TAB_SIZE - (self.x % TAB_SIZE)) * BITMAP_WIDTH;
                    return;
                }
                let bitmap_char = get_raster(c, FontWeight::Regular, RasterHeight::Size16).unwrap();
                self.write_rendered(bitmap_char);
            }
        }
    }

    fn write_rendered(&mut self, c: RasterizedChar) {
        for (y, row) in c.raster().iter().enumerate() {
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
        self.buf[byte_offset..(byte_offset + bpp)].copy_from_slice(&color[..bpp]);
        let _ = unsafe { core::ptr::read_volatile(&self.buf[byte_offset]) };
    }

    pub fn width(&self) -> usize {
        self.info.width
    }
    pub fn height(&self) -> usize {
        self.info.height
    }
}

impl core::fmt::Write for Framebuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write(c);
        }
        Ok(())
    }
}

unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        FRAMEBUFFER
            .try_get()
            .map(|fb| {
                fb.lock()
                    .write_fmt(args)
                    .expect("print to framebuffer failed");
            })
            .expect("failed to get framebuffer");
    });
}

#[macro_export]
macro_rules! print_fb {
    ($($arg:tt)*) => {
        $crate::fb::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println_fb {
    () => { $crate::print_fb!("\n"); };
    ($fmt:expr) => { $crate::print_fb!(concat!($fmt, "\n")); };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::print_fb!(concat!($fmt, "\n"), $($arg)*);
    };
}
