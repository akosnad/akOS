use acpi::{
    platform::{Processor, ProcessorState},
    AcpiError, AcpiTables,
};
use x86_64::{
    instructions::interrupts::without_interrupts, structures::paging::PhysFrame, PhysAddr,
};

mod ap_startup;

use crate::mem::MemoryManager;

const AP_STARTUP_DEST: u32 = 0x10000;

extern "C" {
    static _ap_section_start: u8;
    static _ap_section_end: u8;
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

    copy_ap_startup_code();

    for ap in cpu_info.application_processors.iter() {
        match ap.state {
            ProcessorState::Disabled => log::warn!("cpu {} is disabled", ap.processor_uid),
            ProcessorState::WaitingForSipi => {
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

    without_interrupts(|| {
        let mut lapic = crate::interrupts::LAPIC
            .get()
            .expect("LAPIC not initialized on BSP")
            .lock_sync();
        unsafe {
            log::trace!("INIT IPI to cpu {}", ap.processor_uid);

            // vector can be anything, it is ignored
            lapic.send_ipi(0, dest);
        }
    });
    crate::time::sleep_sync(10);

    // send SIPI twice
    for i in 1..=2 {
        without_interrupts(|| {
            let mut lapic = crate::interrupts::LAPIC
                .get()
                .expect("LAPIC not initialized on BSP")
                .lock_sync();
            unsafe {
                let vector = (AP_STARTUP_DEST >> 12) & 0xFF;
                log::trace!(
                    "SIPI#{} to AP#{} with vector 0x{:x}",
                    i,
                    ap.processor_uid,
                    vector as u8
                );
                lapic.send_sipi(vector as u8, dest);
            }
        });
        crate::time::sleep_sync(2);
    }
}

fn copy_ap_startup_code() {
    let start = unsafe { core::ptr::addr_of!(_ap_section_start) };
    let end = unsafe { core::ptr::addr_of!(_ap_section_end) };
    let size = end as usize - start as usize;

    log::trace!(
        "copying AP CPU startup code from 0x{:x} with size 0x{:x} to 0x{:x}",
        start as usize,
        size,
        AP_STARTUP_DEST
    );

    let slice = unsafe { core::slice::from_raw_parts(start, size) };
    let dest_slice = unsafe { core::slice::from_raw_parts_mut(AP_STARTUP_DEST as *mut u8, size) };

    crate::mem::get_memory_manager()
        .identity_map(PhysFrame::containing_address(PhysAddr::new(
            AP_STARTUP_DEST as u64,
        )))
        .expect("failed to identity map AP startup code");

    dest_slice.copy_from_slice(slice);
}
