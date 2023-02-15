use core::arch::global_asm;
use core::sync::atomic::AtomicBool;

pub static AP_READY: AtomicBool = AtomicBool::new(false);

global_asm!(include_str!("ap_startup.s"));

#[no_mangle]
pub extern "C" fn kernel_ap_main(ap_id: u8) -> ! {
    AP_READY.store(true, core::sync::atomic::Ordering::SeqCst);
    log::info!("hello from AP CPU {}", ap_id);

    loop {
        x86_64::instructions::hlt();
    }
}
