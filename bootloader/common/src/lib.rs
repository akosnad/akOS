#![no_std]

use xmas_elf::ElfFile;

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

pub fn load_and_start_kernel(kernel: Kernel) -> ! {
    loop {}
}
