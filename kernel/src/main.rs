#![no_std]
#![no_main]

extern crate alloc;

use core::sync::atomic::AtomicUsize;

use ak_os_kernel as lib;
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use lib::thread::surrender;

#[cfg(not(feature = "test"))]
pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    //config.frame_buffer.minimum_framebuffer_height = Some(1024);
    //config.frame_buffer.minimum_framebuffer_width = Some(768);
    config
};

#[cfg(not(feature = "test"))]
entry_point!(main, config = &BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    use lib::{fb, logger, mem, println};
    use x86_64::VirtAddr;

    println!(
        "akOS kernel {} ({}) {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("PROFILE"),
        env!("BUILD_TARGET"),
        env!("BUILD_DATE"),
    );
    println!("{}\n{}", env!("RUSTC_VERSION"), env!("CARGO_VERSION"),);

    log::set_logger(&logger::LOGGER).expect("failed to setup logger");
    log::set_max_level(log::LevelFilter::Trace);
    log::debug!("hello from logger");

    let fb = boot_info.framebuffer.as_mut().expect("no framebuffer");
    let fb_info = fb.info();
    let fb_array = fb.buffer_mut();
    let fb_buffer =
        unsafe { core::slice::from_raw_parts_mut(fb_array.as_mut_ptr(), fb_array.len()) };
    fb::init(fb_buffer, fb_info);

    let physical_memory_offset = VirtAddr::new(
        boot_info
            .physical_memory_offset
            .into_option()
            .expect("no physical_memory_offset"),
    );
    unsafe { mem::init(physical_memory_offset, &boot_info.memory_regions) };

    let acpi_info = boot_info.rsdp_addr.into_option().map(lib::acpi::init);
    if acpi_info.is_none() {
        log::warn!("no RSDP address provided for the kernel, ACPI initialization not possible");
    }

    lib::init(acpi_info);

    context_switch_test();

    //scheduler_test();

    lib::halt();
}

fn context_switch_test() {
    use alloc::{boxed::Box, sync::Arc};
    use lib::thread::schedule;
    use lib::thread::Thread;
    use lib::thread::TCB as _;

    let counter = Arc::new(AtomicUsize::new(0));

    let c = counter.clone();
    let mut thread2 = Box::new(Thread::new(Box::new(move || {
        for _ in 0..10 {
            c.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        }
    })));

    schedule(thread2);
    surrender();
    log::info!(
        "counter: {}",
        counter.load(core::sync::atomic::Ordering::SeqCst)
    );
}

fn scheduler_test() {
    use alloc::{boxed::Box, sync::Arc};
    use lib::thread::{schedule, surrender, Thread};

    let counter = Arc::new(AtomicUsize::new(0));
    log::trace!("Arc defined at: {:p}", counter.as_ref() as *const _);
    for _ in 0..1 {
        let c = counter.clone();
        let x = Thread::new(Box::new(move || {
            for _ in 0..10 {
                let val = c.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
                log::info!("counter: {}", val);
                surrender();
            }
        }));
        schedule(Box::new(x));
    }
    log::info!("scheduled all threads");
    while counter.load(core::sync::atomic::Ordering::SeqCst) < 100 {
        surrender();
    }
    log::info!(
        "counter: {}",
        counter.load(core::sync::atomic::Ordering::SeqCst)
    );
}

#[cfg(feature = "test")]
static TEST_BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

#[cfg(feature = "test")]
entry_point!(test_kernel_main, config = &TEST_BOOTLOADER_CONFIG);

#[cfg(feature = "test")]
fn test_kernel_main(boot_info: &'static mut BootInfo) -> ! {
    use lib::{fb, mem};

    let fb = boot_info.framebuffer.as_mut().expect("no framebuffer");
    let fb_info = fb.info();
    let fb_array = fb.buffer_mut();
    let fb_buffer =
        unsafe { core::slice::from_raw_parts_mut(fb_array.as_mut_ptr(), fb_array.len()) };
    fb::init(fb_buffer, fb_info);

    let physical_memory_offset = x86_64::VirtAddr::new(
        boot_info
            .physical_memory_offset
            .into_option()
            .expect("no physical_memory_offset"),
    );
    unsafe { mem::init(physical_memory_offset, &boot_info.memory_regions) };

    lib::init(None);
    lib::test_main();
}
