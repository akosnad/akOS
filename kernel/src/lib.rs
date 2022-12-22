#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(map_first_last)]
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
