use uefi::table::boot::{MemoryDescriptor, MemoryType};
use x86_64::PhysAddr;
use ak_os_bootloader_common::memory::{MemoryRegion, MemoryRegionKind};

#[derive(Debug, Copy, Clone)]
pub struct UefiMemoryDescriptor(pub MemoryDescriptor);

const PAGE_SIZE: u64 = 4096;

impl<'a> MemoryRegion for UefiMemoryDescriptor {
    fn start(&self) -> PhysAddr {
        PhysAddr::new(self.0.phys_start)
    }

    fn len(&self) -> u64 {
        self.0.page_count * PAGE_SIZE
    }

    fn kind(&self) -> MemoryRegionKind {
        match self.0.ty {
            MemoryType::CONVENTIONAL => MemoryRegionKind::Usable,
            other => MemoryRegionKind::UnknownUefi(other.0),
        }
    }

    fn on_bootloader_exit(&mut self) {
        match self.0.ty {
            MemoryType::LOADER_CODE
            | MemoryType::LOADER_DATA
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA
            | MemoryType::RUNTIME_SERVICES_CODE
            | MemoryType::RUNTIME_SERVICES_DATA => {
                self.0.ty = MemoryType::CONVENTIONAL;
            }
            _ => {}
        }
    }
}

