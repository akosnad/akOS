#![feature(alloc_error_handler)]
#![feature(panic_can_unwind)]
#![no_std]
#![no_main]

extern crate alloc;

mod mem;
mod allocator;
mod logger;

use bootloader_api::{entry_point, BootInfo, BootloaderConfig, config::Mapping};
use x86_64::VirtAddr;

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

    let fb = boot_info.framebuffer.as_mut().expect("no framebuffer");
    let fb_info = fb.info();
    let fb_array = fb.buffer_mut();
    let fb_buffer = unsafe { core::slice::from_raw_parts_mut(fb_array.as_mut_ptr(), fb_array.len()) };

    let logger = logger::LOGGER.get_or_init(move || logger::LockedLogger::new(fb_buffer, fb_info));
    log::set_logger(logger).expect("failed to setup logger");
    log::set_max_level(log::LevelFilter::Trace);
    log::info!("Hello world");

    halt();
}

fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(logger) = logger::LOGGER.get() {
        unsafe { logger.force_unlock(); }
    }
    log::error!("{}", info);
    x86_64::instructions::interrupts::disable();
    halt();
}

#[alloc_error_handler]
fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
    panic!("out of memory");
}
