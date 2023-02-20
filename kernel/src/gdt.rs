//! Global Descriptor Table and Task State Segment

use alloc::boxed::Box;
use lazy_static::lazy_static;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::instructions::segmentation::{Segment, CS, DS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });

            stack_start + STACK_SIZE
        };
        tss
    };
}

lazy_static! {
    pub(crate) static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                data_selector,
                tss_selector,
            },
        )
    };
}

#[derive(Debug, Clone)]
pub(crate) struct Selectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

pub fn init() {
    GDT.0.load();
    unsafe {
        DS::set_reg(GDT.1.data_selector);
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }

    #[cfg(feature = "dbg-mem")]
    log::trace!("loaded GDT at {:p}, {:x?}", &GDT, GDT.0);
}

pub fn init_ap() {
    let tss = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });

            stack_start + STACK_SIZE
        };

        let b = Box::new(tss);
        Box::leak::<'static>(b)
    };

    let gdt = {
        let mut gdt = GDT.clone();
        gdt.0.add_entry(Descriptor::tss_segment(tss));

        let b = Box::new(gdt);
        Box::leak::<'static>(b)
    };

    without_interrupts(|| {
        gdt.0.load();
        unsafe {
            CS::set_reg(gdt.1.code_selector);
            DS::set_reg(gdt.1.data_selector);
        }
    });
}
