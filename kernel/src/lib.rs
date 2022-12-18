#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod mem;
pub mod allocator;
pub mod logger;
pub mod task;
pub mod gdt;
pub mod interrupts;

pub fn init() {
    gdt::init();
    interrupts::init();
}

pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

