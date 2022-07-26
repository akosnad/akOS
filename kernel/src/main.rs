#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;

use uefi::prelude;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[alloc_error_handler]
fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
    panic!("out of memory");
}
