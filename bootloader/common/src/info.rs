use crate::memory::MemoryRegions;

#[repr(C)]
pub struct BootInfo {
    pub memory_regions: MemoryRegions
}

