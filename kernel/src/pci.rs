use acpi::AcpiTables;

use crate::mem::MemoryManager;

pub fn init(acpi_tables: AcpiTables<MemoryManager>) -> Result<(), ()>{
    let _regions = acpi::PciConfigRegions::new(&acpi_tables)
        .or(Err(()))?;

    // TODO
    Ok(())
}
