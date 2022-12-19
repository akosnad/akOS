use conquer_once::spin::OnceCell;
use spin;
use x2apic::lapic::LocalApic;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use lazy_static::lazy_static;

pub const INT_OFFSET: u8 = 32;

pub static LAPIC: OnceCell<spin::Mutex<LocalApic>> = OnceCell::uninit();

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    ApicError = INT_OFFSET,
    Timer,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

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

        idt[InterruptIndex::ApicError.as_usize()].set_handler_fn(apic_error_handler);
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

fn init_apic() {
    let xapic_base = unsafe { x2apic::lapic::xapic_base() };
    // FIXME: map the physical address dinamycally
    let xapic_virt_addr = xapic_base;
    let mut lapic = x2apic::lapic::LocalApicBuilder::new()
        .set_xapic_base(xapic_virt_addr)
        .spurious_vector(0xff)
        .error_vector(InterruptIndex::ApicError.as_usize())
        .timer_vector(InterruptIndex::Timer.as_usize())
        .build()
        .unwrap_or_else(|e| panic!("{}", e));
    unsafe {
        lapic.enable();
        //lapic.disable_timer();
        log::trace!("apic id: {}, version: {}", lapic.id(), lapic.version());
    }
    LAPIC.init_once(|| { spin::Mutex::new(lapic) });
}

pub fn init() {
    log::trace!("loading IDT at: {:p}", &IDT);
    IDT.load();
    init_apic();
    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    log::warn!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    log::error!("EXCEPTION: PAGE FAULT");
    log::trace!("Accessed Address: {:?}", Cr2::read());
    log::trace!("Error Code: {:?}", error_code);
    log::trace!("{:#?}", stack_frame);
    panic!("Unhandled page fault");
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("EXCEPTION: GENERAL PROTECTION FAULT\nerror code: {}, {:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!("EXCEPTION: SATCK SEGMENT FAULT\nerror code: {}, {:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\nerror code: {}, {:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn apic_error_handler(_stack_frame: InterruptStackFrame) {
    log::trace!("APIC ERROR CAUGHT");
    unsafe {
        LAPIC.try_get().expect("tried to notify end of interrupt when local APIC was uninitialized")
            .lock().end_of_interrupt();
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    //log::trace!(".");
    // TODO: track elapsed boot time
    unsafe {
        let mut lapic = LAPIC.try_get().expect("tried to notify end of interrupt when local APIC was uninitialized").lock();
        lapic.end_of_interrupt();
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        LAPIC.try_get().expect("tried to notify end of interrupt when local APIC was uninitialized")
            .lock().end_of_interrupt();
    }
}
