#![no_std]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(const_option)]

use ::acpi::AcpiTables;
use mem::MemoryManager;

extern crate alloc;

pub mod util;
pub mod mem;
pub mod logger;
pub mod task;
pub mod gdt;
pub mod interrupts;
pub mod acpi;
pub mod serial;
pub mod time;
pub mod pci;

pub fn init(acpi_tables: Option<AcpiTables<MemoryManager>>) {
    gdt::init();
    if let Some(tables) = acpi_tables {
        let interrupt_model = tables.platform_info()
            .map(|p| p.interrupt_model)
            .ok();
        interrupts::init(interrupt_model);
        pci::init(tables).ok();
    } else {
        interrupts::init(None);
    }
}

pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = unsafe { logger::LOGGER.force_unlock() };
    log::error!("{}", info);
    x86_64::instructions::interrupts::disable();
    halt();
}

#[alloc_error_handler]
fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
    mem::dump_heap_state();
    panic!("out of memory");
}
