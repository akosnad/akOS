use alloc::collections::VecDeque;
use bootloader_api::info::{MemoryRegions as BootMemoryRegions, MemoryRegionKind as BootMemoryRegionKind};
use x86_64::{structures::paging::{PhysFrame, Size4KiB, FrameAllocator, FrameDeallocator}, PhysAddr};

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
impl MemoryRegion {
    fn size(&self) -> u64 {
        self.end - self.start
    }
    fn overlaps(&self, other: &Self) -> bool {
        (self.start > other.start && self.end < other.end) ||
        (other.start > self.start && other.end < self.end)
    }
}

#[derive(Debug)]
pub struct KernelFrameAllocator {
    free_usable_regions: VecDeque<MemoryRegion>,
    free_reserved_regions: VecDeque<MemoryRegion>,
    usable_regions: VecDeque<MemoryRegion>,
    reserved_regions: VecDeque<MemoryRegion>,
}

impl KernelFrameAllocator {
    pub fn init(boot_frame_allocator: &BootInfoFrameAllocator) -> Self {
        let mut usable_regions = VecDeque::new();
        let mut reserved_regions = VecDeque::new();
        for region in boot_frame_allocator.memory_regions.iter() {
            match region.kind {
                BootMemoryRegionKind::Usable => {
                    usable_regions.push_back(MemoryRegion {
                        start: region.start,
                        end: region.end,
                    });
                },
                _ => {
                    reserved_regions.push_back(MemoryRegion {
                        start: region.start,
                        end: region.end,
                    });
                },
            }
        }

        for region in reserved_regions.iter() {
            log::trace!("reserved memory region: {:x?}", region);
        }

        let mut free_usable_regions = VecDeque::new();
        let mut iter = boot_frame_allocator.usable_frames().skip(boot_frame_allocator.used_frame_count()).peekable();
        while let Some(frame) = iter.next() {
            let start = frame.start_address().as_u64();
            let mut end = start + frame.size();
            while let Some(next) = iter.peek() {
                let next_start = next.start_address().as_u64();
                let next_end = next_start + next.size();
                if end == next_start {
                    end = next_end;
                    iter.next();
                    continue;
                }
                break;
            }
            let region = MemoryRegion { start, end };
            log::trace!("free usable memory region: {:x?}", region);
            free_usable_regions.push_back(region);
        }

        Self {
            free_usable_regions,
            free_reserved_regions: VecDeque::new(),
            usable_regions,
            reserved_regions,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for KernelFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if let Some(region) = self.free_usable_regions.pop_front() {
            let frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(PhysAddr::new(region.start));
            let frame_end = frame.start_address().as_u64() + frame.size();
            let remaining_free_region = MemoryRegion { start: frame_end, end: region.end };
            if remaining_free_region.size() > 0 {
                self.free_usable_regions.push_front(remaining_free_region);
            }
            return Some(frame);
        }
        None
    }
}

impl FrameDeallocator<Size4KiB> for KernelFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        //TODO
    }
}
