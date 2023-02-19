use alloc::alloc::Global;
use core::alloc::Allocator;

use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size2MiB, Size4KiB,
    },
    VirtAddr,
};

use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// This virtual address marks where the extended heap will start. It is aligned to 1 GiB.
///
/// Below this address is where the initial heap is, which should be small.
pub const EXTENDED_HEAP_START: u64 = 0x_4446_0000_0000;

pub fn init_heap(
    mapper: &mut (impl Mapper<Size4KiB> + '_),
    initial_size: u64,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let heap_start = VirtAddr::new(EXTENDED_HEAP_START - initial_size);
    let page_range = {
        let heap_end = heap_start + initial_size;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range(heap_start_page, heap_end_page)
    };

    #[cfg(feature = "dbg-mem")]
    log::debug!(
        "initializing heap: {:?}; with initial size: {:?} KiB",
        page_range,
        initial_size / 1024
    );

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }
    }

    unsafe {
        ALLOCATOR
            .lock()
            .init(heap_start.as_mut_ptr(), initial_size as usize);
    }

    #[cfg(feature = "dbg-mem")]
    dump_heap_state();

    Ok(())
}

/// After we have an initial heap, the kernel has initialized its'
/// frame allocator, so we can use 2MiB pages.
pub fn extend(extension_size: usize) -> Result<(), MapToError<Size2MiB>> {
    let page_range = {
        let heap_extended_bottom = VirtAddr::new(ALLOCATOR.lock().top() as u64);
        let heap_extended_top = heap_extended_bottom + extension_size - 1u64;
        let heap_extended_bottom_page = Page::containing_address(heap_extended_bottom);
        let heap_extended_top_page = Page::containing_address(heap_extended_top);
        Page::range_inclusive(heap_extended_bottom_page, heap_extended_top_page)
    };

    #[cfg(feature = "dbg-mem")]
    log::debug!(
        "extending heap: {:x?}; with size: {} MiB",
        page_range,
        extension_size / 1024 / 1024
    );

    let mm = super::get_memory_manager();
    for page in page_range {
        mm.map_2m(page)?;
    }

    unsafe {
        ALLOCATOR.lock().extend(extension_size);
    }

    #[cfg(feature = "dbg-mem")]
    dump_heap_state();

    Ok(())
}

pub fn dump_heap_state() {
    let mut level = log::Level::Debug;
    const K: usize = 1024;

    let a = ALLOCATOR.lock();
    let used = a.used() / K;
    let size = a.size() / K;
    let ratio = used as f32 / size as f32;
    if ratio == 1. {
        level = log::Level::Error;
    } else if ratio > 0.8 {
        level = log::Level::Warn;
    }

    log::log!(
        level,
        "heap: used {} KiB ({} MiB) out of {} KiB ({} MiB) ({:.2}%)",
        used,
        used / K,
        size,
        size / K,
        ratio * 100.
    );
}

/// # Safety
///
/// This function is unsafe because it only should be called by the alloc error handler.
/// This is needed to ensure that we can dump the allocator state while panicking, as an
/// out-of-memory state panics for now.
pub(crate) unsafe fn force_unlock_allocator() {
    ALLOCATOR.force_unlock();
}

pub struct AlignedAlloc<const N: usize>;

unsafe impl<const N: usize> Allocator for AlignedAlloc<N> {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        Global.allocate(layout.align_to(N).unwrap())
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        Global.deallocate(ptr, layout.align_to(N).unwrap())
    }
}
