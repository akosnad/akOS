#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(ak_os_kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use ak_os_kernel as lib;
use alloc::sync::Arc;
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use lib::{task::Task, util::Spinlock};
use x86_64::VirtAddr;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

pub fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    log::set_logger(&lib::logger::LOGGER).expect("failed to setup logger");
    log::set_max_level(log::LevelFilter::Trace);
    log::debug!("hello from logger");

    let physical_memory_offset = VirtAddr::new(
        boot_info
            .physical_memory_offset
            .into_option()
            .expect("no physical_memory_offset"),
    );
    unsafe { lib::mem::init(physical_memory_offset, &boot_info.memory_regions) };

    let acpi_info = boot_info.rsdp_addr.into_option().map(lib::acpi::init);
    if acpi_info.is_none() {
        log::warn!("no RSDP address provided for the kernel, ACPI initialization not possible");
    }

    lib::init(acpi_info);

    let mut executor = lib::task::Executor::default();

    let spinlock = Arc::new(Spinlock::new(0));

    executor.spawn(Task::new(task_one(spinlock.clone())));
    executor.spawn(Task::new(task_two(spinlock.clone())));
    executor.run();
}

async fn task_one(spinlock: Arc<Spinlock<i32>>) {
    for _ in 0..50 {
        let mut num = spinlock.lock().await;
        *num -= 1;
        log::trace!("task_one: {}", num);
        drop(num);
        lib::time::sleep(3).await;
    }

    // should be 25 if running on one core
    // with default timer interrupt settings
    assert_eq!(*spinlock.lock().await, 25);

    lib::exit_qemu(lib::QemuExitCode::Success);
}

async fn task_two(spinlock: Arc<Spinlock<i32>>) {
    loop {
        let mut num = spinlock.lock().await;
        *num += 1;
        log::trace!("task_two: {}", num);
        drop(num);
        lib::time::sleep(2).await;
    }
}
