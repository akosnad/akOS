use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};

#[inline(always)]
/// Signal End of Interrupt to the local APIC
fn eoi() {
    unsafe {
        super::LAPIC
            .try_get()
            .expect("tried to notify end of interrupt when local APIC was uninitialized")
            .lock_sync()
            .end_of_interrupt();
    }
}

pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    log::warn!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn page_fault_handler(
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

pub extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT\nerror code: {}, {:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: SATCK SEGMENT FAULT\nerror code: {}, {:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT\nerror code: {}, {:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn apic_error_handler(_stack_frame: InterruptStackFrame) {
    log::trace!("APIC ERROR CAUGHT");
    eoi();
}

pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::time::increment();
    eoi();
}

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::write(scancode);

    eoi();
}

pub extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let packet: u8 = unsafe { port.read() };
    crate::task::mouse::write(packet);

    eoi();
}
