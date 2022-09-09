#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;

mod mem;
mod allocator;
mod logger;

use bootloader_api::{entry_point, BootInfo, BootloaderConfig, config::Mapping};
use x86_64::VirtAddr;
use alloc::boxed::Box;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(main, config = &BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().expect("no physical_memory_offset"));
    let mut mapper = unsafe { mem::init(physical_memory_offset) };
    let mut frame_allocator = unsafe { mem::BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    allocator::init_heap(&mut mapper, 1024 * 1024 * 4, &mut frame_allocator).expect("heap init failed");

//    let mut fb_box = Box::new(boot_info.framebuffer.into_option().expect("no framebuffer"));
//    let mut fb = Box::leak(fb_box);
//
//    let logger_box = Box::new(logger::KernelLogger::new(fb.buffer_mut(), fb.info()));
//    let logger = Box::leak(logger_box);
//    log::set_logger(logger).expect("failed to setup logger");

    log::info!("Hello world");
    halt();
}

fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    halt();
}

#[alloc_error_handler]
fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
    panic!("out of memory");
}
