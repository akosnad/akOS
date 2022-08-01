#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(alloc_error_handler)]
#![feature(ptr_to_from_bits)]

use uefi::prelude::*;
use core::fmt::Write;

extern crate alloc;
use alloc::boxed::Box;

extern crate backtracer_core;

mod loader;

#[entry]
fn efi_main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    unsafe {
        uefi::alloc::init(st.boot_services());
        *PANIC_SYSTEM_TABLE.get() = Some(st.unsafe_clone());
    }

    let logger: &'static uefi::logger::Logger = unsafe {
        // let st_clone = Box::new(st.unsafe_clone());
        // let mut st_leak: &'static mut SystemTable<Boot> = Box::leak(st_clone);
        let a = Box::new(uefi::logger::Logger::new(st.stdout()));
        Box::leak(a)
    };

    #[cfg(debug_assertions)]
    { log::set_max_level(log::LevelFilter::Trace); }
    #[cfg(not(debug_assertions))]
    { log::set_max_level(log::LevelFilter::Warn); }

    log::set_logger(logger).expect("failed to initialize logger");

    log::info!("Hello from akOS bootloader!\nBuilt at: {} on {}, commit {}",
        env!("BUILD_TIME"), env!("BUILD_HOST"), env!("BUILD_GIT_HASH")
    );

    let kernel = loader::load_kernel(image, &mut st);
    log::info!("found kernel file");

    ak_os_bootloader_common::load_and_start_kernel(kernel);
}

fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("cli; hlt") };
    }
}


use core::cell::UnsafeCell;
static PANIC_SYSTEM_TABLE: UnsafeSyncCell<Option<SystemTable<Boot>>> = UnsafeSyncCell::new(None);

struct UnsafeSyncCell<T>(UnsafeCell<T>);
impl<T> UnsafeSyncCell<T> {
    const fn new(x: T) -> Self {
        Self(UnsafeCell::new(x))
    }
}

unsafe impl<T> Sync for UnsafeSyncCell<T> {}
impl<T> core::ops::Deref for UnsafeSyncCell<T> {
    type Target = UnsafeCell<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(st) = unsafe { &mut *PANIC_SYSTEM_TABLE.get() } {
        writeln!(st.stdout(), "\n\n{}", info).ok();
    }

    halt();
}

#[alloc_error_handler]
fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
    panic!("out of memory");
}
