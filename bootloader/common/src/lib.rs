#![no_std]

extern crate alloc;

pub mod memory;
pub mod info;

use xmas_elf::ElfFile;
use memory::{MemoryRegion, FrameAllocator, PageTables};

pub struct Kernel<'a> {
    pub elf: ElfFile<'a>
}

impl<'a> Kernel<'a> {
    pub fn parse(slice: &'a [u8]) -> Self {
        Self {
            elf: ElfFile::new(slice).unwrap()
        }
    }
}

pub fn load_and_start_kernel<M, D>(
    kernel: Kernel,
    mut frame_allocator: FrameAllocator<M, D>,
    mut page_tables: PageTables
) -> !
where
    M: ExactSizeIterator<Item = D> + Clone,
    D: MemoryRegion,
{
    unimplemented!("load and start kernel");
}
