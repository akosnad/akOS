#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(anonymous_lifetime_in_impl_trait)]

extern crate alloc;

pub mod util;
pub mod mem;
pub mod logger;
pub mod task;
pub mod gdt;
pub mod interrupts;
pub mod acpi;

pub fn init() {
    gdt::init();
    interrupts::init();
}

pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

