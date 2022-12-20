use acpi::{AcpiTables, PlatformInfo};

pub fn init(rsdp_addr: u64) -> PlatformInfo {
    log::trace!("rsdp at {:x}", rsdp_addr);
    let mm = crate::mem::get_memory_manager();

    let acpi_tables = unsafe { AcpiTables::from_rsdp(mm, rsdp_addr as usize).expect("couldn't get ACPI tables") };
    log::debug!("acpi revision: {}", acpi_tables.revision);

    let info = acpi_tables.platform_info().unwrap_or_else(|e| panic!("couldn't get platform information from ACPI: {:#?}", e));
    log::trace!("power profile: {:#x?}", info.power_profile);
    log::trace!("interrupt model: {:#x?}", info.interrupt_model);
    log::trace!("boot processor: {:#x?}", info.processor_info.as_ref().unwrap().boot_processor);
    info
}
