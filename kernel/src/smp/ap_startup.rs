use core::sync::atomic::AtomicBool;

pub static AP_READY: AtomicBool = AtomicBool::new(false);

core::arch::global_asm!(include_str!("ap_startup.s"));

#[no_mangle]
pub extern "C" fn kernel_ap_main() -> ! {
    crate::gdt::init_ap();
    crate::interrupts::init_ap();

    let trampoline = unsafe { &*(super::TRAMPOLINE as *const super::ApTrampoline) };

    #[cfg(feature = "dbg-smp")]
    log::debug!("hello from AP CPU {}", trampoline.ap_id);

    AP_READY.store(true, core::sync::atomic::Ordering::SeqCst);

    crate::task::executor::schedule(trampoline.ap_id);
}
