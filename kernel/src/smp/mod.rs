//! Symmetric Multi-Processing (SMP) support
//!
//! This module relies on parsed ACPI tables to initialize additional CPUs.
//!
//! # CPU startup procedure
//!
//! The CPU startup procedure is described in the
//! [Intel® 64 and IA-32 Architectures Software Developer’s Manual, Volume 3A, Chapter 8.4](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-3a-part-1-manual.pdf).
//!
//! The procedure is as follows:
//!
//! 1. The AP startup code is written to the physical address `0x10000`, copied from the kernel section `.text.init`.
//! see [`ap_startup.s`](/src/ak_os_kernel/smp/ap_startup.s) for the startup code.
//! 2. The BSP writes the trampoline data to the AP startup address `0xF000` which tells the AP where to put its stack and where the rust entry point is.
//! 3. The BSP sends and INIT IPI, then two SIPI IPIs to the AP.
//! 4. The AP starts executing the startup code at `0x10000`, sets up paging, long mode, then jumps into rust code.
//! 5. The AP signals the BSP that is is done initializing, while the BSP waits.
//! 6. The BSP continues with the next AP, back to step 2.
//! 7. Once all APs have been started, they get scheduled to run in the [Executor](crate::task::executor::Executor).

use crate::mem::MemoryManager;
use acpi::{
    platform::{Processor, ProcessorState},
    AcpiError, AcpiTables,
};
use x86_64::{
    instructions::interrupts::without_interrupts,
    structures::paging::{mapper::MapToError, PageSize, PageTableFlags, Size4KiB},
    PhysAddr, VirtAddr,
};

mod ap_startup;

const AP_STARTUP_DEST: u32 = 0x10000;
const TRAMPOLINE: u32 = AP_STARTUP_DEST - Size4KiB::SIZE as u32;

extern "C" {
    static _init_section_start: u8;
    static _init_section_end: u8;
}

pub fn init(acpi_tables: &AcpiTables<MemoryManager>) -> Result<(), AcpiError> {
    let platform_info = acpi_tables.platform_info()?;
    let cpu_info = platform_info.processor_info.expect("no processor info");

    if cpu_info.application_processors.is_empty() {
        log::info!("system is single-processor, not starting additional cpus");
        return Ok(());
    }

    log::debug!(
        "system BSP cpu is {}, starting {} AP cpus",
        cpu_info.boot_processor.processor_uid,
        cpu_info.application_processors.len()
    );

    copy_init();
    copy_trampoline();

    for ap in cpu_info.application_processors.iter() {
        match ap.state {
            ProcessorState::Disabled => log::warn!("cpu {} is disabled", ap.processor_uid),
            ProcessorState::WaitingForSipi => {
                #[cfg(feature = "dbg-smp")]
                log::debug!("cpu {} is waiting for SIPI", ap.processor_uid);
                init_ap(ap);
            }
            ProcessorState::Running => log::warn!("cpu {} is already running", ap.processor_uid),
        }
    }
    Ok(())
}

fn init_ap(ap: &Processor) {
    let dest = ap.processor_uid << 24;

    setup_trampoline(ap);

    without_interrupts(|| {
        let mut lapic = crate::interrupts::LAPIC
            .get()
            .expect("LAPIC not initialized on BSP")
            .lock_sync();
        unsafe {
            #[cfg(feature = "dbg-smp")]
            log::trace!("INIT IPI to cpu {}", ap.processor_uid);

            // vector can be anything, it is ignored
            lapic.send_ipi(0, dest);
        }
    });
    crate::time::sleep_sync(1);

    // send SIPI twice
    for _ in 1..=2 {
        without_interrupts(move || {
            let mut lapic = crate::interrupts::LAPIC
                .get()
                .expect("LAPIC not initialized on BSP")
                .lock_sync();
            unsafe {
                let vector = (AP_STARTUP_DEST >> 12) & 0xFF;

                #[cfg(feature = "dbg-smp")]
                log::trace!(
                    "SIPI to AP#{} with vector 0x{:x}",
                    ap.processor_uid,
                    vector as u8
                );

                lapic.send_sipi(vector as u8, dest);
            }
        });
        crate::time::sleep_sync(1);
    }

    // wait for AP to signal it is ready
    while !ap_startup::AP_READY.load(core::sync::atomic::Ordering::SeqCst) {
        crate::time::sleep_sync(1);
    }
    crate::time::sleep_sync(1);
    ap_startup::AP_READY.store(false, core::sync::atomic::Ordering::SeqCst);
}

fn copy_init() {
    let mm = crate::mem::get_memory_manager();

    let start = unsafe { core::ptr::addr_of!(_init_section_start) };
    let end = unsafe { core::ptr::addr_of!(_init_section_end) };
    let size = end as usize - start as usize;

    let range = (AP_STARTUP_DEST as u64)..(AP_STARTUP_DEST as u64 + size as u64);
    if let Err(e) = mm.identity_map_range(
        range,
        Some(PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::GLOBAL),
    ) {
        match e {
            MapToError::PageAlreadyMapped(_) => {
                log::warn!("AP startup code already mapped, skipping");
                return;
            }
            _ => panic!("failed to identity map AP startup code: {:?}", e),
        }
    }

    #[cfg(feature = "dbg-smp")]
    log::trace!(
        "copying AP CPU startup code from 0x{:x} with size 0x{:x} to 0x{:x}",
        start as usize,
        size,
        AP_STARTUP_DEST
    );

    let slice = unsafe { core::slice::from_raw_parts(start, size) };
    let dest_slice = unsafe { core::slice::from_raw_parts_mut(AP_STARTUP_DEST as *mut u8, size) };

    dest_slice.copy_from_slice(slice);
}

fn copy_trampoline() {
    let mm = crate::mem::get_memory_manager();
    if let Err(e) = mm.identity_map_address(TRAMPOLINE as u64, None) {
        match e {
            MapToError::PageAlreadyMapped(_) => {
                log::warn!("AP trampoline already mapped, skipping");
                return;
            }
            _ => panic!("failed to identity map AP trampoline: {:?}", e),
        }
    }

    let tmp_trampoline = ApTrampoline::default();

    #[cfg(feature = "dbg-smp")]
    log::trace!("writing trampoline to 0x{:x}", TRAMPOLINE);

    let trampoline = unsafe { &mut *(TRAMPOLINE as *mut ApTrampoline) };
    unsafe {
        core::ptr::write(trampoline, tmp_trampoline);
    }

    // temporary GDT
    mm.identity_map_address(0x800, None)
        .expect("failed to map temporary GDT");
}

#[derive(Debug, Clone)]
#[repr(C)]
struct ApTrampoline {
    ap_id: u8,
    ap_page_table: PhysAddr,
    ap_stack_start: VirtAddr,
    ap_stack_end: VirtAddr,
    ap_entry_code: VirtAddr,
}
impl Default for ApTrampoline {
    fn default() -> Self {
        Self {
            ap_id: 0,
            ap_page_table: PhysAddr::new(0),
            ap_stack_start: VirtAddr::new(0),
            ap_stack_end: VirtAddr::new(0),
            ap_entry_code: VirtAddr::new(0),
        }
    }
}

fn setup_trampoline(ap: &Processor) {
    let mm = crate::mem::get_memory_manager();

    const STACK_SIZE: usize = 0x1000 * 8;
    let (ap_page_table, ap_stack_start) = mm
        .init_ap(ap.processor_uid as u8, STACK_SIZE)
        .expect("failed to init AP memory management");
    let ap_stack_end = ap_stack_start + STACK_SIZE as u64;

    let tmp_trampoline = ApTrampoline {
        ap_id: ap.processor_uid as u8,
        ap_page_table,
        ap_stack_start,
        ap_stack_end,
        ap_entry_code: VirtAddr::new(ap_startup::kernel_ap_main as *const () as u64),
    };

    let trampoline = unsafe { &mut *(TRAMPOLINE as *mut ApTrampoline) };
    unsafe {
        core::ptr::write(trampoline, tmp_trampoline);
    }

    #[cfg(feature = "dbg-smp")]
    log::trace!("written trampoline data: {:?}", trampoline);
}
