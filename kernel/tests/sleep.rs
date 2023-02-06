#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(ak_os_kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use ak_os_kernel as lib;
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use lib::task::Task;
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
    executor.spawn(Task::new(sleep_test()));
    executor.run();
}

async fn sleep_test() {
    for _ in 0..10 {
        lib::time::sleep(5).await;
        log::info!("woken up");
    }
    let elapsed = lib::time::boot_elapsed();

    assert_eq!(elapsed, 50);

    lib::exit_qemu(lib::QemuExitCode::Success);
}
