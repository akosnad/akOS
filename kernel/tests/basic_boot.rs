#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(ak_os_kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use ak_os_kernel as lib;
use bootloader_api::{entry_point, BootInfo};

entry_point!(kernel_main);

pub fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    test_main();
    lib::exit_qemu(lib::QemuExitCode::Success);
    lib::halt();
}

#[test_case]
fn test_println() {
    lib::println!("print worked");
}
