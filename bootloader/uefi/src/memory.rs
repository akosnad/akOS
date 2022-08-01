use uefi::table::boot::{MemoryDescriptor, MemoryType};
use x86_64::{PhysAddr, VirtAddr, structures::paging::{FrameAllocator, OffsetPageTable, PhysFrame, Size4KiB, PageTable}};
use ak_os_bootloader_common::memory::{MemoryRegion, MemoryRegionKind, PageTables};

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

pub fn create_page_tables(frame_allocator: &mut impl FrameAllocator<Size4KiB>) -> PageTables {
    let phys_offset = VirtAddr::new(0);

    log::trace!("switching to new level 4 table");
    let bootloader_page_table = {
        let old = {
            let frame = x86_64::registers::control::Cr3::read().0;
            let ptr: *const PageTable = (phys_offset + frame.start_address().as_u64()).as_ptr();
            unsafe { &*ptr }
        };
        let new_frame = frame_allocator
            .allocate_frame()
            .expect("failed to allocate frame for new level 4 table");
        let new_table: &mut PageTable = {
            let ptr: *mut PageTable =
                (phys_offset + new_frame.start_address().as_u64()).as_mut_ptr();
            unsafe { ptr.write(PageTable::new());
                &mut *ptr
            }
        };

        new_table[0] = old[0].clone();

        unsafe {
            x86_64::registers::control::Cr3::write(
                new_frame,
                x86_64::registers::control::Cr3Flags::empty(),
            );
            OffsetPageTable::new(&mut *new_table, phys_offset)
        }
    };

    let (kernel_page_table, kernel_lvl4_table) = {
        let frame: PhysFrame = frame_allocator.allocate_frame().expect("no unused frames");
        log::info!("new level 4 page table at {:#?}", &frame);
        let addr = phys_offset + frame.start_address().as_u64();
        let ptr = addr.as_mut_ptr();
        unsafe { *ptr = PageTable::new() };
        let level_4_table = unsafe { &mut *ptr };
        (
            unsafe { OffsetPageTable::new(level_4_table, phys_offset) },
            frame,
        )
    };

    ak_os_bootloader_common::memory::PageTables {
        bootloader: bootloader_page_table,
        kernel: kernel_page_table,
        kernel_lvl4_table,
    }
}
