use super::ThreadInfo;

core::arch::global_asm!(include_str!("context_switch.s"));

extern "C" {
    pub fn context_switch(current: *mut ThreadInfo, next: *mut ThreadInfo);
}

#[no_mangle]
pub extern "C" fn thread_entry_point() -> ! {
    log::debug!("thread entry point");
    super::cleanup();
    {
        let mut active = match super::swap_active(None) {
            Some(active) => active,
            None => panic!("no thread available in thread entry point"),
        };
        let task = active.get_work();
        super::swap_active(Some(active));
        task();
    }
    super::stop();
    log::debug!("thread entry point end");
    loop {}
}
