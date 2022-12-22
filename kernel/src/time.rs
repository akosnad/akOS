use crossbeam_utils::atomic::AtomicCell;


static BOOT_ELAPSED: AtomicCell<u64> = AtomicCell::new(0);
pub fn boot_elapsed() -> u64 {
    BOOT_ELAPSED.load()
}

pub(crate) fn increment() {
    BOOT_ELAPSED.fetch_add(1);
}
