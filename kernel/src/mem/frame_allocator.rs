use alloc::collections::BTreeSet;
use bootloader_api::info::{
    MemoryRegionKind as BootMemoryRegionKind, MemoryRegions as BootMemoryRegions,
};
use x86_64::{
    structures::paging::{FrameAllocator, FrameDeallocator, PageSize, PhysFrame, Size4KiB},
    PhysAddr,
};

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

        #[cfg(feature = "dbg-mem")]
        log::trace!("allocated frame: {:x?}, count so far: {}", frame, self.next);

        frame
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MemoryRegion {
    start: u64,
    end: u64,
}
impl MemoryRegion {
    #[inline]
    fn size(&self) -> u64 {
        self.end - self.start
    }

    #[inline]
    fn overlaps(&self, other: &Self) -> bool {
        (self.start > other.start && self.end < other.end)
            || (other.start > self.start && other.end < self.end)
    }

    #[inline]
    fn contains(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}
impl core::ops::RangeBounds<u64> for MemoryRegion {
    fn start_bound(&self) -> core::ops::Bound<&u64> {
        core::ops::Bound::Included(&self.start)
    }

    fn end_bound(&self) -> core::ops::Bound<&u64> {
        core::ops::Bound::Excluded(&self.end)
    }
}
impl core::fmt::Debug for MemoryRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("0x{:x} - 0x{:x}", self.start, self.end))
    }
}

#[derive(Debug)]
struct MemoryRegions {
    regions: BTreeSet<MemoryRegion>,
}
impl MemoryRegions {
    fn new() -> Self {
        Self {
            regions: BTreeSet::new(),
        }
    }

    fn add(&mut self, region: MemoryRegion) {
        if self.regions.is_empty() {
            self.regions.insert(region);
            return;
        }

        for r in self.regions.iter() {
            if r.contains(&region) {
                // already covered by larger existing region
                return;
            }
            if r.overlaps(&region) {
                if region.start < r.start {
                    // we're under the existing region, extend it down
                    self.regions.replace(MemoryRegion {
                        start: region.start,
                        end: r.end,
                    });
                } else if region.end > r.end {
                    // we're over the existing region, extend it up
                    self.regions.replace(MemoryRegion {
                        start: r.start,
                        end: region.end,
                    });
                }
                return;
            }
        }

        // at this point we found no intersections
        self.regions.insert(region);
    }

    fn cut(&mut self, region: MemoryRegion) -> Result<(), ()> {
        let mut containing_region = None;
        for r in self.regions.iter() {
            if r.contains(&region) {
                containing_region = Some(*r);
                break;
            }
        }

        if let Some(containing_region) = containing_region {
            self.regions.remove(&containing_region);

            let lower = MemoryRegion {
                start: containing_region.start,
                end: region.start,
            };
            let upper = MemoryRegion {
                start: region.end,
                end: containing_region.end,
            };
            if lower.size() > 0 {
                self.add(lower);
            }
            if upper.size() > 0 {
                self.add(upper);
            }

            return Ok(());
        }
        Err(())
    }

    fn find_with_page_size<S: PageSize>(&self) -> Option<&MemoryRegion> {
        self.regions.iter().find(|r| r.size() >= S::SIZE)
    }

    fn contains(&self, region: MemoryRegion) -> bool {
        for r in self.regions.iter() {
            if r.contains(&region) {
                return true;
            }
        }
        false
    }

    #[cfg(feature = "dbg-mem")]
    fn dump_state(&self) {
        for r in self.regions.iter() {
            log::trace!("{:?}", r);
        }
    }
}

#[derive(Debug)]
pub struct KernelFrameAllocator {
    free_usable_regions: MemoryRegions,
    free_reserved_regions: MemoryRegions,
    usable_regions: MemoryRegions,
    reserved_regions: MemoryRegions,
}

impl KernelFrameAllocator {
    pub fn init(boot_frame_allocator: &BootInfoFrameAllocator) -> Self {
        let mut usable_regions = MemoryRegions::new();
        let mut reserved_regions = MemoryRegions::new();
        for region in boot_frame_allocator.memory_regions.iter() {
            match region.kind {
                BootMemoryRegionKind::Usable => {
                    usable_regions.add(MemoryRegion {
                        start: region.start,
                        end: region.end,
                    });
                }
                _ => {
                    reserved_regions.add(MemoryRegion {
                        start: region.start,
                        end: region.end,
                    });
                }
            }
        }

        #[cfg(feature = "dbg-mem")]
        {
            log::trace!("reserved memory regions:");
            reserved_regions.dump_state();
        }

        let mut free_usable_regions = MemoryRegions::new();
        let mut iter = boot_frame_allocator
            .usable_frames()
            .skip(boot_frame_allocator.used_frame_count())
            .peekable();
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
            free_usable_regions.add(MemoryRegion { start, end });
        }

        #[cfg(feature = "dbg-mem")]
        {
            log::trace!("free usable memory regions:");
            free_usable_regions.dump_state();
        }

        Self {
            free_usable_regions,
            free_reserved_regions: MemoryRegions::new(),
            usable_regions,
            reserved_regions,
        }
    }
}

unsafe impl<S: PageSize> FrameAllocator<S> for KernelFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<S>> {
        if let Some(region) = self.free_usable_regions.find_with_page_size::<S>() {
            let start = PhysAddr::new(region.start).align_up(S::SIZE);
            let frame: PhysFrame<S> = PhysFrame::from_start_address(start).ok()?;

            let frame_end = frame.start_address().as_u64() + frame.size();
            let allocated_region = MemoryRegion {
                start: region.start,
                end: frame_end,
            };
            self.free_usable_regions.cut(allocated_region).ok()?;

            #[cfg(feature = "dbg-mem")]
            log::trace!("allocated frame: {:x?}", frame);

            return Some(frame);
        }
        None
    }
}

impl<S: PageSize> FrameDeallocator<S> for KernelFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<S>) {
        let start = frame.start_address().as_u64();
        let end = start + frame.size();
        let region = MemoryRegion { start, end };
        if self.usable_regions.contains(region) {
            self.free_usable_regions.add(region);

            #[cfg(feature = "dbg-mem")]
            log::trace!("deallocated frame: {:x?}", frame);
        } else if self.reserved_regions.contains(region) {
            self.free_reserved_regions.add(region);

            #[cfg(feature = "dbg-mem")]
            log::trace!("deallocated reserved frame: {:x?}", frame);
        } else {
            panic!("couldn't deallocate frame: {:x?}", frame);
        }
    }
}
