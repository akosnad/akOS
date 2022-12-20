#![feature(alloc_error_handler)]
#![feature(panic_can_unwind)]
#![feature(generators)]
#![no_std]
#![no_main]

extern crate alloc;

use bootloader_api::{entry_point, BootInfo, BootloaderConfig, config::Mapping};
use ak_os_kernel::{mem, logger, task::{Task, executor::Executor, keyboard}};
use x86_64::VirtAddr;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    //config.frame_buffer.minimum_framebuffer_height = Some(1024);
    //config.frame_buffer.minimum_framebuffer_width = Some(768);
    config
};

entry_point!(main, config = &BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    log::set_logger(&logger::LOGGER).expect("failed to setup logger");
    log::set_max_level(log::LevelFilter::Trace);
    #[cfg(debug_assertions)]
    {
        log::set_max_level(log::LevelFilter::Trace);
    }
    log::info!("Hello world");

    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().expect("no physical_memory_offset"));
    unsafe { mem::init(physical_memory_offset, &boot_info.memory_regions) };

    let fb = boot_info.framebuffer.as_mut().expect("no framebuffer");
    let fb_info = fb.info();
    let fb_array = fb.buffer_mut();
    let fb_buffer = unsafe { core::slice::from_raw_parts_mut(fb_array.as_mut_ptr(), fb_array.len()) };
    logger::LOGGER.attach_framebuffer(fb_buffer, fb_info);
    log::debug!("switched to framebuffer");

    if let Some(rsdp_addr) = boot_info.rsdp_addr.into_option() {
        ak_os_kernel::acpi::init(rsdp_addr);
    } else {
        log::warn!("no RSDP address provided for the kernel, ACPI initialization not possible");
    }

    ak_os_kernel::init();

    let mut executor = Executor::new();
    executor.spawn(Task::new_with_name("keyboard", keyboard::process()));
    executor.spawn(Task::new(example_task()));
    executor.run();
}

async fn test() -> u32 {
    for _ in 0..10000000 {
    }
    42
}

async fn example_task() {
    for _ in 0..2 {
        let n = test().await;
        log::info!("async hello: {}", n);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = unsafe { logger::LOGGER.force_unlock() };
    log::error!("{}", info);
    x86_64::instructions::interrupts::disable();
    ak_os_kernel::halt();
}

#[alloc_error_handler]
fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
    panic!("out of memory");
}
