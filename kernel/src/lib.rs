#![no_std]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(const_option)]
#![feature(custom_test_frameworks)]
#![feature(allocator_api)]
#![feature(alloc_layout_extra)]

//! akOS kernel

use core::sync::atomic::AtomicBool;

use ::acpi::AcpiTables;
use mem::MemoryManager;

extern crate alloc;

pub mod acpi;
pub mod fb;
pub mod gdt;
pub mod interrupts;
pub mod kbuf;
pub mod logger;
pub mod mem;
pub mod pci;
pub mod peripheral;
pub mod serial;
pub mod smp;
pub mod task;
pub mod time;
pub mod util;

#[cfg(feature = "test")]
pub mod test;

#[cfg(feature = "test")]
pub use test::*;

static PANICKING: AtomicBool = AtomicBool::new(false);

pub fn init(acpi_tables: Option<AcpiTables<MemoryManager>>) {
    gdt::init();
    if let Some(tables) = acpi_tables {
        let interrupt_model = tables.platform_info().map(|p| p.interrupt_model).ok();
        interrupts::init(interrupt_model);
        smp::init(&tables).ok();
        pci::init(&tables).ok();
    } else {
        interrupts::init(None);
    }
    peripheral::init();
}

pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(not(feature = "test"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    PANICKING.store(true, core::sync::atomic::Ordering::SeqCst);
    unsafe {
        interrupts::_panic_handle_all();
        serial::force_unlock();
        fb::force_unlock().ok();
    }
    println_serial!("\n{}", info);
    println_fb!("\n{}", info);
    halt();
}

#[cfg(feature = "test")]
#[panic_handler]
pub fn test_panic_handler(info: &core::panic::PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    unsafe {
        fb::force_unlock().ok();
        serial::force_unlock();
    }
    println!("[failed]");
    println!("Error: {}", info);
    exit_qemu(QemuExitCode::Failed);
}

#[alloc_error_handler]
fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
    unsafe { mem::force_unlock_allocator() };
    mem::dump_heap_state();
    panic!("out of memory");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
    halt();
}
