use alloc::{collections::BTreeMap, string::String, sync::Arc};
use conquer_once::spin::OnceCell;
use core::{arch::asm, ops::DerefMut};
use crossbeam_queue::ArrayQueue;
use x86_64::VirtAddr;

use crate::util::Spinlock;

use super::{Context, Process, ProcessID};

static mut SCHEDULER: OnceCell<Scheduler> = OnceCell::uninit();

pub fn init() {
    unsafe {
        SCHEDULER
            .try_init_once(Scheduler::new)
            .expect("scheduler already initialized");
    }
}

pub fn run() -> ! {
    unsafe { SCHEDULER.get().unwrap().run() }
}

pub fn spawn(name: impl Into<String>, entry_point: VirtAddr) {
    unsafe {
        SCHEDULER
            .try_get()
            .expect("scheduler not initialized")
            .spawn(name, entry_point)
    }
}

pub fn yield_now() {
    log::debug!("yielding");
    const INTERRUPT_IDX: u8 = crate::interrupts::InterruptIndex::ProcessYield as u8;
    unsafe {
        asm!("int {}", const INTERRUPT_IDX);
    }
}

pub fn exit(code: u64) {
    log::debug!("exiting");
    const INTERRUPT_IDX: u8 = crate::interrupts::InterruptIndex::ProcessExit as u8;
    unsafe {
        asm!(
        "push {};
        int {}",
        in(reg) code,
        const INTERRUPT_IDX
        );
    }
}

pub(crate) fn exit_impl(code: u64) {
    unsafe {
        SCHEDULER
            .try_get()
            .expect("scheduler not initialized")
            .exit(code)
    }
}

struct Scheduler {
    processes: Spinlock<BTreeMap<ProcessID, Process>>,
    process_queue: Arc<ArrayQueue<ProcessID>>,
    current_process: Spinlock<Option<ProcessID>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            processes: Spinlock::new(BTreeMap::new()),
            process_queue: Arc::new(ArrayQueue::new(1024)),
            current_process: Spinlock::new(None),
        }
    }

    pub fn run(&self) -> ! {
        loop {
            if let Some(id) = self.process_queue.pop() {
                {
                    let mut current_process = self.current_process.lock_sync();
                    if let Some(current_id) = *current_process {
                        self.process_queue
                            .push(current_id)
                            .expect("failed to push process to queue");
                    }
                    *current_process = Some(id);
                }

                let mut processes = self.processes.lock_sync();
                let process = processes.get_mut(&id).expect("process not found");
                let context = process.context.deref_mut();

                match context {
                    Context::Initial(initial_state) => unsafe {
                        let start_addr = initial_state.rip_address.as_u64();
                        let exit_addr = exit as *const () as u64;
                        let cr3 = initial_state.cr3_base;
                        let stack = initial_state.stack_end.as_u64();

                        log::debug!("running process: {}", process.name);
                        drop(processes); // release lock

                        asm!(
                            "mov cr3, {};
                            mov rsp, {};
                            push {};
                            push {};
                            push {};
                            ret;",
                            in(reg) cr3,
                            in(reg) stack,
                            const 0u64,
                            in(reg) exit_addr,
                            in(reg) start_addr,
                        );
                    },
                    Context::Running(_) => {}
                }
            }
        }
    }

    pub fn spawn(&self, name: impl Into<String>, entry_point: VirtAddr) {
        let process = Process::new(name.into(), entry_point).expect("failed to create process");

        log::debug!("spawned process: {}", process.name);

        let id = process.id;
        self.processes.lock_sync().insert(id, process);
        self.process_queue
            .push(id)
            .expect("failed to push process to queue");
    }

    pub fn exit(&self, code: u64) {
        let current_process = self
            .current_process
            .try_lock()
            .expect("tried to exit when current process is locked")
            .take()
            .expect("tried to exit without a current process");

        let process = self
            .processes
            .lock_sync()
            .remove(&current_process)
            .expect("process not found");

        log::debug!("process {} exited with code: {}", process, code);

        todo!("restore last context");
    }
}
