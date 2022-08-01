use x86_64::{PhysAddr,
    structures::paging::{PhysFrame, Size4KiB, FrameAllocator as FrameAllocatorTrait, OffsetPageTable},
};
use core::mem::MaybeUninit;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub struct MemoryRegionDescriptor {
    pub start: u64,
    pub end: u64,
    pub kind: MemoryRegionKind,
}

impl MemoryRegionDescriptor {
    pub const fn empty() -> Self {
        Self {
            start: 0,
            end: 0,
            kind: MemoryRegionKind::Bootloader,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum MemoryRegionKind {
    Usable,
    Bootloader,
    UnknownUefi(u32),
    UnknownFirmware(u32)
}

#[repr(C)]
pub struct MemoryRegions {
    pub(crate) ptr: *mut MemoryRegionDescriptor,
    pub(crate) len: usize,
}

impl core::ops::Deref for MemoryRegions {
    type Target = [MemoryRegionDescriptor];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl core::ops::DerefMut for MemoryRegions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

pub trait MemoryRegion: Copy + core::fmt::Debug {
    fn start(&self) -> PhysAddr;
    fn len(&self) -> u64;
    fn kind(&self) -> MemoryRegionKind;

    /// Marks bootloader memory as usable for the kernel
    fn on_bootloader_exit(&mut self) {}
}

pub struct FrameAllocator<M, D> {
    original: M,
    memory_map: M,
    current_descriptor: Option<D>,
    next_frame: PhysFrame,
}

impl<M, D> FrameAllocator<M, D>
where
    M: ExactSizeIterator<Item = D> + Clone,
    M::Item: MemoryRegion,
{
    pub fn new(memory_map: M) -> Self {
        let start_frame = PhysFrame::containing_address(PhysAddr::new(0x1000));
        Self::new_starting_at(start_frame, memory_map)
    }

    pub fn new_starting_at(frame: PhysFrame, memory_map: M) -> Self {
        Self {
            original: memory_map.clone(),
            memory_map,
            current_descriptor: None,
            next_frame: frame,
        }
    }

    pub fn len(&self) -> usize {
        self.original.len()
    }

    pub fn create_memory_map(
        self,
        regions: &mut [MaybeUninit<MemoryRegionDescriptor>],
    ) -> &mut [MemoryRegionDescriptor] {
        let mut next_idx = 0;

        for mut descriptor in self.original {
            let mut start = descriptor.start();
            let end = start + descriptor.len();
            let next_free = self.next_frame.start_address();
            descriptor.on_bootloader_exit();
            let kind = match descriptor.kind() {
                MemoryRegionKind::Usable => {
                    if end <= next_free { MemoryRegionKind::Bootloader }
                    else if descriptor.start() >= next_free { MemoryRegionKind::Usable }
                    else {
                        let used_region = MemoryRegionDescriptor {
                            start: descriptor.start().as_u64(),
                            end: next_free.as_u64(),
                            kind: MemoryRegionKind::Bootloader,
                        };
                        Self::add_region(used_region, regions, &mut next_idx)
                            .expect("failed to add memory region");

                        start = next_free;
                        MemoryRegionKind::Usable
                        }
                    }
                    other => other,
                };

            let region = MemoryRegionDescriptor {
                start: start.as_u64(),
                end: end.as_u64(),
                kind,
            };
            Self::add_region(region, regions, &mut next_idx).unwrap();
        }

        let initialized = &mut regions[..next_idx];
        unsafe {
            &mut *(initialized as *mut [_] as *mut [_])
        }
    }

    fn add_region(
        region: MemoryRegionDescriptor,
        regions: &mut [MaybeUninit<MemoryRegionDescriptor>],
        next_index: &mut usize,
    ) -> Result<(), ()> {
        unsafe {
            regions.get_mut(*next_index)
                .ok_or(())?
                .as_mut_ptr()
                .write(region)
        };
        *next_index += 1;
        Ok(())
    }

    fn allocate_frame_from_descriptor(&mut self, descriptor: D) -> Option<PhysFrame> {
        let start_addr = descriptor.start();
        let start_frame = PhysFrame::containing_address(start_addr);
        let end_addr = start_addr + descriptor.len();
        let end_frame = PhysFrame::containing_address(end_addr - 1u64);

        if self.next_frame < start_frame {
            self.next_frame = start_frame;
        }

        if self.next_frame < end_frame {
            let result = self.next_frame;
            self.next_frame += 1;
            Some(result)
        } else { None }
    }
}

unsafe impl<M, D> FrameAllocatorTrait<Size4KiB> for FrameAllocator<M, D>
where
    M: ExactSizeIterator<Item = D> + Clone,
    M::Item: MemoryRegion,
{
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if let Some(current_descriptor) = self.current_descriptor {
            match self.allocate_frame_from_descriptor(current_descriptor) {
                Some(frame) => return Some(frame),
                None => { self.current_descriptor = None; }
            }
        }

        while let Some(descriptor) = self.memory_map.next() {
            if descriptor.kind() != MemoryRegionKind::Usable {
                continue;
            }
            if let Some(frame) = self.allocate_frame_from_descriptor(descriptor) {
                self.current_descriptor = Some(descriptor);
                return Some(frame);
            }
        }

        None
    }
}


pub struct PageTables {
    pub bootloader: OffsetPageTable<'static>,
    pub kernel: OffsetPageTable<'static>,
    pub kernel_lvl4_table: PhysFrame,
}
