use crate::mem::{get_memory_manager, MemoryManager};
use acpi::AcpiTables;

pub fn init(rsdp_addr: u64) -> AcpiTables<MemoryManager<'static>> {
    #[cfg(feature = "dbg-acpi")]
    log::trace!("rsdp at {:x}", rsdp_addr);

    let mm = get_memory_manager();

    let acpi_tables =
        unsafe { AcpiTables::from_rsdp(mm, rsdp_addr as usize).expect("couldn't get ACPI tables") };

    log::debug!("found acpi tables with revision: {}", acpi_tables.revision);

    #[cfg(feature = "dbg-acpi")]
    {
        let info = acpi_tables
            .platform_info()
            .unwrap_or_else(|e| panic!("couldn't get platform information from ACPI: {:#?}", e));
        log::trace!("power profile: {:#x?}", info.power_profile);
        log::trace!("interrupt model: {:#x?}", info.interrupt_model);
        log::trace!(
            "boot processor: {:#x?}",
            info.processor_info.as_ref().unwrap().boot_processor
        );
    }

    acpi_tables
}
