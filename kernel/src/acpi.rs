use acpi::AcpiTables;

pub fn init(rsdp_addr: u64) {
    log::trace!("rsdp at {:x}", rsdp_addr);
    let mm = crate::mem::get_memory_manager();

    let acpi_tables = unsafe { AcpiTables::from_rsdp(mm, rsdp_addr as usize).expect("couldn't get ACPI tables") };
    log::debug!("acpi revision: {}", acpi_tables.revision);
}
