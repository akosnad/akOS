#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(map_first_last)]
#![feature(const_option)]

use ::acpi::PlatformInfo;

extern crate alloc;

pub mod util;
pub mod mem;
pub mod logger;
pub mod task;
pub mod gdt;
pub mod interrupts;
pub mod acpi;
pub mod serial;
pub mod time;

pub fn init(platform_info: Option<PlatformInfo>) {
    gdt::init();
    interrupts::init(platform_info.and_then(|i| Some(i.interrupt_model)));
}

pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
