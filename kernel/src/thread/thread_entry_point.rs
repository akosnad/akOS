use super::ThreadInfo;

core::arch::global_asm!(include_str!("context_switch.s"));

extern "C" {
    pub fn context_switch(current: *mut ThreadInfo, next: *mut ThreadInfo);
}

#[no_mangle]
pub extern "C" fn thread_entry_point() -> ! {
    log::debug!("thread entry point");
    loop {}
}
