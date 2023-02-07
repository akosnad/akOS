use acpi::InterruptModel;
use conquer_once::spin::OnceCell;
use lazy_static::lazy_static;
use spin::{self, Mutex};
use x2apic::{
    ioapic::{IoApic, IrqFlags, RedirectionTableEntry},
    lapic::LocalApic,
};
use x86_64::structures::idt::InterruptDescriptorTable;

mod handlers;
use handlers::*;

pub static LAPIC: OnceCell<spin::Mutex<LocalApic>> = OnceCell::uninit();
pub static IOAPIC: OnceCell<spin::Mutex<IoApic>> = OnceCell::uninit();

lazy_static! {
    #[derive(Debug)]
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);

        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt[InterruptIndex::ApicError.into()].set_handler_fn(apic_error_handler);
        idt[InterruptIndex::Timer.into()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.into()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Mouse.into()].set_handler_fn(mouse_interrupt_handler);
        idt
    };
}

const IOAPIC_INTERRUPT_INDEX_OFFSET: u8 = 40;
const LAPIC_INTERRUPT_INDEX_OFFSET: u8 = 0x90;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    _IoApic = IOAPIC_INTERRUPT_INDEX_OFFSET, // we reserve this
    Keyboard,
    Mouse = IOAPIC_INTERRUPT_INDEX_OFFSET + 12,
    ApicError = LAPIC_INTERRUPT_INDEX_OFFSET,
    Timer,
}

impl From<InterruptIndex> for u8 {
    fn from(val: InterruptIndex) -> Self {
        val as u8
    }
}
impl From<InterruptIndex> for usize {
    fn from(val: InterruptIndex) -> Self {
        val as usize
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum IoApicTableIndex {
    Keyboard = 1,
    Mouse = 12,
}
impl From<IoApicTableIndex> for u8 {
    fn from(val: IoApicTableIndex) -> Self {
        val as u8
    }
}
impl From<IoApicTableIndex> for usize {
    fn from(val: IoApicTableIndex) -> Self {
        val as usize
    }
}

unsafe fn init_lapic(base_address: u64) {
    let mm = crate::mem::get_memory_manager();
    mm.identity_map_address(base_address)
        .unwrap_or_else(|e| panic!("can't map APIC base address: {:#?}", e));

    let mut lapic = x2apic::lapic::LocalApicBuilder::new()
        .set_xapic_base(base_address)
        .spurious_vector(0xff)
        .error_vector(InterruptIndex::ApicError.into())
        .timer_vector(InterruptIndex::Timer.into())
        .build()
        .unwrap_or_else(|e| panic!("{}", e));
    lapic.enable();

    #[cfg(feature = "dbg-interrupts")]
    log::debug!("apic id: {}, version: {}", lapic.id(), lapic.version());

    LAPIC.init_once(|| spin::Mutex::new(lapic));
}

unsafe fn register_io_apic_entry(ioapic: &mut IoApic, lapic_id: u8, int_index: u8, irq_index: u8) {
    let mut entry = RedirectionTableEntry::default();
    entry.set_mode(x2apic::ioapic::IrqMode::Fixed);
    entry.set_dest(lapic_id);
    entry.set_vector(int_index);
    entry.set_flags(IrqFlags::LEVEL_TRIGGERED | IrqFlags::LOW_ACTIVE | IrqFlags::MASKED);
    ioapic.set_table_entry(irq_index, entry);
    ioapic.enable_irq(irq_index);
}

unsafe fn init_io_apic(base_address: u64) {
    let lapic = LAPIC
        .get()
        .expect("should have the LAPIC initialized")
        .lock();

    let mm = crate::mem::get_memory_manager();
    mm.identity_map_address(base_address)
        .unwrap_or_else(|e| panic!("can't map IO-APIC base address: {:#?}", e));

    let mut ioapic = x2apic::ioapic::IoApic::new(base_address);
    ioapic.init(IOAPIC_INTERRUPT_INDEX_OFFSET);

    #[cfg(feature = "dbg-interrupts")]
    log::debug!("ioapic id: {}, version: {}", ioapic.id(), ioapic.version());

    register_io_apic_entry(
        &mut ioapic,
        lapic.id() as u8,
        InterruptIndex::Keyboard.into(),
        IoApicTableIndex::Keyboard.into(),
    );
    register_io_apic_entry(
        &mut ioapic,
        lapic.id() as u8,
        InterruptIndex::Mouse.into(),
        IoApicTableIndex::Mouse.into(),
    );

    // enable the keyboard and mouse
    // FIXME: this should be done in the keyboard and mouse driver
    // TODO: USB Legacy Suport would be a step up
    let mut cmd = x86_64::instructions::port::Port::<u8>::new(0x64);
    let mut data = x86_64::instructions::port::Port::<u8>::new(0x60);
    unsafe {
        cmd.write(0xae); // enable keyboard port
        cmd.write(0xa8); // enable mouse port
        cmd.write(0xd4); // tell mouse to expect a command
        data.write(0xf4); // enable mouse
    }

    IOAPIC.init_once(|| Mutex::new(ioapic));
}

pub fn init(interrupt_model: Option<InterruptModel>) {
    #[cfg(feature = "dbg-interrupts")]
    log::trace!("loading IDT at: {:p}", &IDT);

    IDT.load();
    if let Some(InterruptModel::Apic(model)) = interrupt_model {
        unsafe {
            init_lapic(model.local_apic_address);
            for ioapic in model.io_apics {
                init_io_apic(ioapic.address as u64);
            }
        }
        x86_64::instructions::interrupts::enable();
    } else {
        log::warn!("unsupported interrupt model, no APIC was found");
    }
}
