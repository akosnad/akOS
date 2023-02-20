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

pub extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: InterruptStackFrame) {
    if crate::PANICKING.load(core::sync::atomic::Ordering::SeqCst) {
        x86_64::instructions::interrupts::disable();
        loop {
            x86_64::instructions::hlt();
        }
    }
    panic!(
        "EXCEPTION: NON-MASKABLE INTERRUPT

{:#?}",
        stack_frame
    );
}

pub extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: SEGMENT NOT PRESENT\nerror code: {}, {:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    panic!(
        "EXCEPTION: DIVIDE ERROR

{:#?}",
        stack_frame
    );
}

pub extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!(
        "EXCEPTION: INVALID OPCODE

{:#?}",
        stack_frame
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
    unsafe {
        let lapic = super::LAPIC
            .try_get()
            .expect("tried to get LAPIC while it was uninitialized")
            .lock_sync();
        let flags = lapic.error_flags();
        panic!("EXCEPTION: APIC ERROR: {:#?}", flags);
    }
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
