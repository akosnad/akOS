use uart_16550::SerialPort;

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
