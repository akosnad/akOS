#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(alloc_error_handler)]
#![deny(unsafe_op_in_unsafe_fn)]

use uefi::prelude::*;
use core::fmt::Write;

#[macro_use]
extern crate alloc;

#[entry]
fn efi_main(_image: Handle, mut st: SystemTable<Boot>) -> Status {
    unsafe { uefi::alloc::init(st.boot_services()); }
    let stdout = st.stdout();
    stdout.write_str(format!("Hello from akOS bootloader!\nBuilt at: {} on {}\ncommit {}",
                     env!("BUILD_TIME"), env!("BUILD_HOST"), env!("BUILD_GIT_HASH")).as_str());

    halt();
}

fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("cli; hlt") };
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    halt();
}

#[alloc_error_handler]
fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
    panic!("out of memory");
}
