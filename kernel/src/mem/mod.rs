use core::{ops::DerefMut, ptr};
use acpi::AcpiHandler;
use alloc::sync::Arc;
use bootloader_api::info::MemoryRegions;
use conquer_once::spin::OnceCell;
use spin::Mutex;
use x86_64::{structures::paging::{OffsetPageTable, PageTable, PhysFrame, Mapper, PageTableFlags, mapper::{MapToError, UnmapError}, Size4KiB, Page, FrameAllocator, FrameDeallocator}, VirtAddr, PhysAddr};

use self::paging::{BootInfoFrameAllocator, KernelFrameAllocator};

mod allocator;
mod paging;

static MEMORY_MANAGER: OnceCell<MemoryManager> = OnceCell::uninit();

pub fn get_memory_manager() -> MemoryManager<'static> {
    MEMORY_MANAGER.try_get().expect("kernel memory manager is uninitialized").clone()
}


#[derive(Debug, Clone)]
pub struct MemoryManager<'a> {
    page_table: Arc<Mutex<OffsetPageTable<'a>>>,
    frame_allocator: Arc<Mutex<KernelFrameAllocator>>,
}

impl MemoryManager<'_> {
    pub fn map(&self, page: Page) -> Result<PhysFrame, MapToError<Size4KiB>> {
        let frame = self.frame_allocator.lock().allocate_frame()
            .expect("cannot allocate frame");
        unsafe {
            self.page_table.lock().map_to(
                page,
                frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                self.frame_allocator.lock().deref_mut()
                )?.flush();
        }
        Ok(frame)
    }
    pub fn identity_map(&self, frame: PhysFrame) -> Result<(), MapToError<Size4KiB>> {
        unsafe {
        self.page_table.lock().identity_map(
            frame,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            self.frame_allocator.lock().deref_mut()
            )?.flush();
        }
        Ok(())
    }
    pub fn identity_map_address(&self, physical_address: u64) -> Result<(), MapToError<Size4KiB>> {
        self.identity_map(PhysFrame::containing_address(PhysAddr::new(physical_address)))
    }
    pub fn unmap(&self, page: Page) -> Result<(), UnmapError> {
        let frame = self.page_table.lock().unmap(page).and_then(|p| { p.1.flush(); Ok(p.0) })?;
        unsafe {
            self.frame_allocator.lock().deallocate_frame(frame);
        }
        Ok(())
    }
}

impl AcpiHandler for MemoryManager<'_> {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> acpi::PhysicalMapping<Self, T> {
        let start_address = PhysAddr::new(physical_address as u64);
        let end_address = PhysAddr::new((physical_address + size) as u64);
        let range = PhysFrame::range_inclusive(PhysFrame::containing_address(start_address), PhysFrame::containing_address(end_address));

        // TODO: don't necessarily identity map the requested addresses, this could be a bit
        // more smarter. It also panics if it cannot map the address
        for frame in range.into_iter() {
            self.identity_map(frame).or_else(|e| match e {
                MapToError::PageAlreadyMapped(_) => Ok(()), // if the page is already mapped, we
                                                            // leave it alone for now
                _=> Err(()),
            }).expect("failed to map page for acpi table parsing");
        }
        acpi::PhysicalMapping::new(
            start_address.as_u64() as usize,
            ptr::NonNull::new_unchecked(start_address.as_u64() as *mut _),
            size,
            size,
            self.clone()
        )
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        let mm = region.handler();
        mm.unmap(
            Page::containing_address(
                VirtAddr::new(region.virtual_start().as_ptr() as u64)
                )
            ).expect("should be able to unmap");
    }
}

/// Initialize the kernel memory management.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr, memory_regions: &'static MemoryRegions) {
    let level_4_table = active_level_4_table(physical_memory_offset);
    let mut page_table = OffsetPageTable::new(level_4_table, physical_memory_offset);

    let mut initial_frame_allocator = BootInfoFrameAllocator::init(memory_regions);

    allocator::init_heap(&mut page_table, 1024 * 1024 * 4, &mut initial_frame_allocator).unwrap_or_else(|e| panic!("heap init failed: {:#?}", e));

    MEMORY_MANAGER.init_once(|| MemoryManager {
        page_table: Arc::new(Mutex::new(page_table)),
        frame_allocator: Arc::new(Mutex::new(KernelFrameAllocator::init(&initial_frame_allocator))),
    });
}

/// Returns a mutable reference to the active level 4 table.
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

