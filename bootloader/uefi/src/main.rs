#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(alloc_error_handler)]
#![deny(unsafe_op_in_unsafe_fn)]

use uefi::prelude::*;
use core::fmt::Write;

extern crate alloc;

mod loader;

#[entry]
fn efi_main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    unsafe {
        uefi::alloc::init(st.boot_services());
        *PANIC_SYSTEM_TABLE.get() = Some(st.unsafe_clone());
    }
    writeln!(st.stdout(), "\n\nHello from akOS bootloader!\nBuilt at: {} on {}\ncommit {}",
        env!("BUILD_TIME"), env!("BUILD_HOST"), env!("BUILD_GIT_HASH")
    );


    let kernel = loader::load_kernel(image, &mut st);
    writeln!(st.stdout(), "found kernel file");

    halt();
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
        writeln!(st.stdout(), "{}", info);
    }

    halt();
}

#[alloc_error_handler]
fn alloc_error(_layout: alloc::alloc::Layout) -> ! {
    panic!("out of memory");
}
