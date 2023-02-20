//! PCI subsystem (stub)

use acpi::{AcpiError, AcpiTables};

use crate::mem::MemoryManager;

pub fn init(acpi_tables: &AcpiTables<MemoryManager>) -> Result<(), AcpiError> {
    let _regions = acpi::PciConfigRegions::new(acpi_tables)?;

    // TODO
    Ok(())
}
