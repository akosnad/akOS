//! Kernel memory management
//!
//! This module provides [MemoryManager] for the kernel.

use acpi::AcpiHandler;
use alloc::{boxed::Box, collections::BTreeMap, sync::Arc};
use bootloader_api::info::MemoryRegions;
use conquer_once::spin::OnceCell;
use core::{
    ops::{DerefMut, Range},
    ptr,
};
use x86_64::{
    structures::paging::{
        mapper::{MapToError, UnmapError},
        FrameAllocator, FrameDeallocator, Mapper, OffsetPageTable, Page, PageSize, PageTable,
        PageTableFlags, PhysFrame, Size1GiB, Size2MiB, Size4KiB, Translate,
    },
    PhysAddr, VirtAddr,
};

use self::frame_allocator::{BootInfoFrameAllocator, KernelFrameAllocator};
use crate::util::Spinlock;

mod allocator;
mod frame_allocator;

pub(crate) use allocator::force_unlock_allocator;
pub use allocator::{dump_heap_state, AlignedAlloc};

static MEMORY_MANAGER: OnceCell<MemoryManager> = OnceCell::uninit();

pub fn get_memory_manager() -> MemoryManager<'static> {
    MEMORY_MANAGER
        .try_get()
        .expect("kernel memory manager is uninitialized")
        .clone()
}

#[derive(Clone)]
pub struct MemoryManager<'a> {
    page_table: Arc<Spinlock<OffsetPageTable<'a>>>,
    frame_allocator: Arc<Spinlock<KernelFrameAllocator>>,
    ap_page_tables: Arc<Spinlock<BTreeMap<u8, OffsetPageTable<'a>>>>,
}

impl MemoryManager<'_> {
    pub fn translate_addr(&self, addr: VirtAddr) -> Option<PhysAddr> {
        self.page_table.lock_sync().translate_addr(addr)
    }

    pub fn identity_map(
        &self,
        frame: PhysFrame,
        flags: Option<PageTableFlags>,
    ) -> Result<(), MapToError<Size4KiB>> {
        #[cfg(feature = "dbg-mem")]
        log::trace!("identity mapping frame: {:x?}", frame);

        let flags = flags.unwrap_or_else(|| {
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE
        });
        unsafe {
            self.page_table
                .lock_sync()
                .identity_map(frame, flags, self.frame_allocator.lock_sync().deref_mut())?
                .flush();
        }
        Ok(())
    }
    pub fn identity_map_address(
        &self,
        physical_address: u64,
        flags: Option<PageTableFlags>,
    ) -> Result<(), MapToError<Size4KiB>> {
        self.identity_map(
            PhysFrame::containing_address(PhysAddr::new(physical_address)),
            flags,
        )
    }

    pub fn identity_map_range(
        &self,
        range: Range<u64>,
        flags: Option<PageTableFlags>,
    ) -> Result<(), MapToError<Size4KiB>> {
        for frame in PhysFrame::range_inclusive(
            PhysFrame::containing_address(PhysAddr::new(range.start)),
            PhysFrame::containing_address(PhysAddr::new(range.end - 1)),
        ) {
            self.identity_map(frame, flags)?;
        }
        Ok(())
    }

    /// Initializes page table for AP CPUs
    ///
    /// Returns the physical address of the new level 4 page table and
    /// the virtual address of the mew stack base
    pub fn init_ap(
        &self,
        cpu_id: u8,
        stack_size: usize,
    ) -> Result<(PhysAddr, VirtAddr), MapToError<Size4KiB>> {
        let lvl4_table = {
            let mut new_table = PageTable::new();
            let mut bsp_table = self.page_table.lock_sync();
            new_table.clone_from(bsp_table.deref_mut().level_4_table());
            let b = Box::new(new_table);
            Box::leak(b)
        };
        let lvl4_table_addr = {
            let ptr = lvl4_table as *const _ as u64;
            self.translate_addr(VirtAddr::new(ptr))
                .expect("new lvl4 table not mapped")
        };

        let phys_offset = self.page_table.lock_sync().phys_offset();
        let mut ap_page_table = unsafe { OffsetPageTable::new(lvl4_table, phys_offset) };

        let ap_stack = self.map_stack(stack_size, &mut ap_page_table)?;

        self.ap_page_tables
            .lock_sync()
            .insert(cpu_id, ap_page_table);

        Ok((lvl4_table_addr, ap_stack))
    }

    fn map_stack(
        &self,
        size: usize,
        page_table: &mut OffsetPageTable,
    ) -> Result<VirtAddr, MapToError<Size4KiB>> {
        let start = VirtAddr::new(0xFFFF_FFFF_2000_0000);

        let guard_page: Page<Size4KiB> = Page::containing_address(start - 1u64);

        assert!(self.translate_addr(guard_page.start_address()).is_none());

        for page in Page::range(
            Page::containing_address(start),
            Page::containing_address(start + size as u64),
        ) {
            let frame = self
                .frame_allocator
                .lock_sync()
                .allocate_frame()
                .expect("out of memory");
            unsafe {
                page_table
                    .map_to(
                        page,
                        frame,
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        self.frame_allocator.lock_sync().deref_mut(),
                    )?
                    .ignore();
            }
        }

        #[cfg(feature = "dbg-mem")]
        log::trace!(
            "mapped stack at 0x{:x} with size 0x{:x}, guard page at 0x{:x}",
            start,
            size,
            guard_page.start_address()
        );

        Ok(start)
    }
}

macro_rules! gen_map_impl {
    ($Size:ident, $map_name:ident, $unmap_name:ident) => {
        impl<'a> MemoryManager<'a>
        where
            OffsetPageTable<'a>: Mapper<$Size>,
        {
            pub fn $map_name(
                &self,
                page: Page<$Size>,
            ) -> Result<PhysFrame<$Size>, MapToError<$Size>> {
                #[cfg(feature = "dbg-mem")]
                log::trace!("mapping page: {:x?}", page);

                let frame: PhysFrame<$Size> = self
                    .frame_allocator
                    .lock_sync()
                    .allocate_frame()
                    .expect("cannot allocate frame");
                unsafe {
                    self.page_table
                        .lock_sync()
                        .map_to(
                            page,
                            frame,
                            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                            self.frame_allocator.lock_sync().deref_mut(),
                        )?
                        .flush();
                }
                Ok(frame)
            }
            pub fn $unmap_name(&self, page: Page<$Size>) -> Result<(), UnmapError> {
                #[cfg(feature = "dbg-mem")]
                log::trace!("unmapping page: {:x?}", page);

                let frame = self.page_table.lock_sync().unmap(page).and_then(|p| {
                    p.1.flush();
                    Ok(p.0)
                })?;
                unsafe {
                    self.frame_allocator.lock_sync().deallocate_frame(frame);
                }
                Ok(())
            }
        }
    };
}

gen_map_impl!(Size4KiB, map, unmap);
gen_map_impl!(Size2MiB, map_2m, unmap_2m);
gen_map_impl!(Size1GiB, map_1g, unmap_1g);

impl AcpiHandler for MemoryManager<'_> {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let start_address = PhysAddr::new(physical_address as u64);
        let end_address = PhysAddr::new((physical_address + size) as u64);
        let range = PhysFrame::range_inclusive(
            PhysFrame::containing_address(start_address),
            PhysFrame::containing_address(end_address),
        );

        // TODO: don't necessarily identity map the requested addresses, this could be a bit
        // more smarter. It also panics if it cannot map the address
        for frame in range {
            self.identity_map(frame, None)
                .or_else(|e| match e {
                    MapToError::PageAlreadyMapped(_) => Ok(()), // if the page is already mapped, we
                    // leave it alone for now
                    _ => Err(()),
                })
                .expect("failed to map page for acpi table parsing");
        }
        acpi::PhysicalMapping::new(
            start_address.as_u64() as usize,
            ptr::NonNull::new_unchecked(start_address.as_u64() as *mut _),
            size,
            size,
            self.clone(),
        )
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        let mm = region.handler();
        mm.unmap(Page::containing_address(VirtAddr::new(
            region.virtual_start().as_ptr() as u64,
        )))
        .expect("should be able to unmap");
    }
}

/// Initialize the kernel memory management.
///
/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr, memory_regions: &'static MemoryRegions) {
    let level_4_table = active_level_4_table(physical_memory_offset);
    let mut page_table = OffsetPageTable::new(level_4_table, physical_memory_offset);

    let mut initial_frame_allocator = BootInfoFrameAllocator::init(memory_regions);

    allocator::init_heap(
        &mut page_table,
        4 * Size4KiB::SIZE,
        &mut initial_frame_allocator,
    )
    .unwrap_or_else(|e| panic!("heap init failed: {:#?}", e));

    MEMORY_MANAGER.init_once(|| MemoryManager {
        page_table: Arc::new(Spinlock::new(page_table)),
        frame_allocator: Arc::new(Spinlock::new(KernelFrameAllocator::init(
            &initial_frame_allocator,
        ))),
        ap_page_tables: Arc::new(Spinlock::new(BTreeMap::new())),
    });

    allocator::extend(4 * Size2MiB::SIZE as usize)
        .unwrap_or_else(|e| panic!("failed to extend heap: {:#?}", e));

    crate::kbuf::use_heap();
    crate::time::init();
}

/// Returns a mutable reference to the active level 4 table.
///
/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // unsafe
}
