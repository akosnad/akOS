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

    setup_trampoline();

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

    let tmp_trampoline = ApTrampoline {
        ap_page_table: mm.lvl4_table_addr(),
        ..Default::default()
    };

    log::trace!("writing trampoline to 0x{:x}", TRAMPOLINE);
    let trampoline = unsafe { &mut *(TRAMPOLINE as *mut ApTrampoline) };
    unsafe {
        core::ptr::write(trampoline, tmp_trampoline);
    }
    log::trace!("written trampoline data: {:?}", trampoline);

    mm.identity_map_address(0x800, None).unwrap();
}

#[derive(Debug, Clone)]
#[repr(C)]
struct ApTrampoline {
    ap_ready: bool,
    ap_id: u8,
    ap_page_table: PhysAddr,
    ap_stack_start: VirtAddr,
    ap_stack_end: VirtAddr,
    ap_gdt: u32,
    ap_entry_code: VirtAddr,
}
impl Default for ApTrampoline {
    fn default() -> Self {
        Self {
            ap_ready: false,
            ap_id: 0,
            ap_page_table: PhysAddr::new(0),
            ap_stack_start: VirtAddr::new(0),
            ap_stack_end: VirtAddr::new(0),
            ap_gdt: 0,
            ap_entry_code: VirtAddr::new(0),
        }
    }
}

fn setup_trampoline() {}
