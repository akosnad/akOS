// #![feature(alloc_error_handler)]
#![no_std]
#![no_main]

// extern crate alloc;

use bootloader_api::{entry_point, BootInfo};

entry_point!(main);

fn main(boot_info: &'static mut BootInfo) -> ! {
    if let Some(fb) = boot_info.framebuffer.as_mut() {
        for byte in fb.buffer_mut() {
            *byte = 0x90;
        }
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// #[alloc_error_handler]
// fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
//     panic!("out of memory");
// }
