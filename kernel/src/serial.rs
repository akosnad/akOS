//! Serial port driver
//!
//! The kernel uses the serial port to print the [kernel buffer](`crate::kbuf`).

use uart_16550::SerialPort;

use crate::util::Spinlock;

static SERIAL: Spinlock<Serial> = Spinlock::new(Serial::new());

/// # Safety
///
/// This function is unsafe because it only should be called by the panic handler.
/// This is needed to ensure that the framebuffer is available for printing panic messages.
pub(crate) unsafe fn force_unlock() {
    SERIAL.force_unlock();
}

pub struct Serial {
    port: SerialPort,
}

impl Serial {
    pub const fn new() -> Self {
        Self {
            port: unsafe { SerialPort::new(0x3f8) },
        }
    }

    pub fn write(&mut self, c: char) {
        self.port.send(c as u8);
    }
}

impl core::fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write(c);
        }
        Ok(())
    }
}

unsafe impl Send for Serial {}
unsafe impl Sync for Serial {}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        SERIAL.lock_sync().write_fmt(args).expect("print failed");
    });
}

#[macro_export]
macro_rules! print_serial {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println_serial {
    () => { $crate::print_serial!("\n"); };
    ($fmt:expr) => { $crate::print_serial!(concat!($fmt, "\n")); };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::print_serial!(concat!($fmt, "\n"), $($arg)*);
    };
}
