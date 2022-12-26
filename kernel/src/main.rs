#![no_std]
#![no_main]

extern crate alloc;

use bootloader_api::{entry_point, BootInfo, BootloaderConfig, config::Mapping};
use ak_os_kernel::{mem, logger, task::{Task, executor::Executor, keyboard}, println};
use x86_64::VirtAddr;
use core::env;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    //config.frame_buffer.minimum_framebuffer_height = Some(1024);
    //config.frame_buffer.minimum_framebuffer_width = Some(768);
    config
};

entry_point!(main, config = &BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    println!(
        "akOS kernel {} ({}) {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("PROFILE"),
        env!("BUILD_TARGET"),
        env!("BUILD_DATE"),
    );
    println!("{}\n{}",
        env!("RUSTC_VERSION"),
        env!("CARGO_VERSION"),
    );

    log::set_logger(&logger::LOGGER).expect("failed to setup logger");
    log::set_max_level(log::LevelFilter::Trace);
    log::debug!("hello from logger");

    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().expect("no physical_memory_offset"));
    unsafe { mem::init(physical_memory_offset, &boot_info.memory_regions) };

    let fb = boot_info.framebuffer.as_mut().expect("no framebuffer");
    let fb_info = fb.info();
    let fb_array = fb.buffer_mut();
    let fb_buffer = unsafe { core::slice::from_raw_parts_mut(fb_array.as_mut_ptr(), fb_array.len()) };
    logger::LOGGER.attach_framebuffer(fb_buffer, fb_info);
    log::debug!("switched to framebuffer");

    let acpi_info = boot_info.rsdp_addr.into_option()
        .and_then(|addr| Some(ak_os_kernel::acpi::init(addr)));
    if acpi_info.is_none() {
        log::warn!("no RSDP address provided for the kernel, ACPI initialization not possible");
    }

    ak_os_kernel::init(acpi_info);

    let mut executor = Executor::new();
    executor.spawn(Task::new_with_name("keyboard", keyboard::process()));
    executor.run();
}

