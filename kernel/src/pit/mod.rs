use alloc::{format, string::String};
use x86_64::instructions::port::Port;

use crate::util::Spinlock;

const PIT_DEFAULY_FREQUENCY: u32 = 1_193_182;
const PIT_MINIMUM_FREQUENCY: u32 = 19;

const PIT_COMMAND_REGISTER: u16 = 0x43;
// const PIT_CHANNEL_0_DATA_REGISTER: u16 = 0x40;
const PIT_CHANNEL_2_DATA_REGISTER: u16 = 0x42;

static mut PIT_COMMAND: Spinlock<Port<u8>> = Spinlock::new(Port::new(PIT_COMMAND_REGISTER));
//static mut PIT_CHANNEL_0: Spinlock<Port<u8>> =
//     Spinlock::new(Port::new(PIT_CHANNEL_0_DATA_REGISTER));
static mut PIT_CHANNEL_2: Spinlock<Port<u8>> =
    Spinlock::new(Port::new(PIT_CHANNEL_2_DATA_REGISTER));

pub fn pit_wait(microseconds: u32) -> Result<(), String> {
    let divisor = PIT_DEFAULY_FREQUENCY / (1_000_000 / microseconds);
    if divisor > (u16::MAX as u32) {
        return Err(format!(
            "pit_wait: wait time of {}us is too large, max value is {}!",
            microseconds,
            1_000_000 / PIT_MINIMUM_FREQUENCY
        ));
    }

    unsafe {
        let mut port_60 = Port::<u8>::new(0x60);
        let mut port_61 = Port::<u8>::new(0x61);

        let port_61_value = port_61.read();
        port_61.write(port_61_value & 0xFD | 0x1);
        PIT_COMMAND.lock_sync().write(0b10110010);

        PIT_CHANNEL_2.lock_sync().write(divisor as u8);
        let _ = port_60.read();
        PIT_CHANNEL_2.lock_sync().write((divisor >> 8) as u8);

        let port_61_value = port_61.read() & 0xFE;
        port_61.write(port_61_value);
        port_61.write(port_61_value | 0x1);

        while port_61.read() & 0x20 != 0 {
            // wait for PIT to finish
        }
        Ok(())
    }
}
