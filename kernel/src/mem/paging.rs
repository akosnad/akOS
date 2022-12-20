use alloc::collections::VecDeque;
use bootloader_api::info::{MemoryRegions as BootMemoryRegions, MemoryRegionKind as BootMemoryRegionKind};
use x86_64::{structures::paging::{PhysFrame, Size4KiB, FrameAllocator}, PhysAddr};

pub struct BootInfoFrameAllocator {
    memory_regions: &'static BootMemoryRegions,
    next: usize,
}

/// A simple frame allocator for when the kernel doesn't yet have heap allocation
/// after boot
impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_regions: &'static BootMemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_regions,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        // get usable regions from memory map
        let regions = self.memory_regions.iter();
        let usable_regions = regions.filter(|r| r.kind == BootMemoryRegionKind::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.start..r.end);
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `UnusedPhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    fn used_frame_count(&self) -> usize {
        self.next
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

#[derive(Debug)]
struct MemoryRegion {
    start: u64,
    end: u64,
}

#[derive(Debug)]
pub struct KernelFrameAllocator {
    regions: VecDeque<MemoryRegion>,
    next: usize,
}

impl KernelFrameAllocator {
    pub fn init(boot_frame_allocator: &BootInfoFrameAllocator) -> Self {
        let mut regions = VecDeque::new();
        for region in boot_frame_allocator.memory_regions.iter().filter(|r| r.kind == BootMemoryRegionKind::Usable) {
            regions.push_back(MemoryRegion {
                start: region.start,
                end: region.end,
            });
        }

        Self {
            regions,
            next: boot_frame_allocator.used_frame_count(),
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        self.regions.iter()
            .map(|r| r.start..r.end)
            .flat_map(|r| r.step_by(4096))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for KernelFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
