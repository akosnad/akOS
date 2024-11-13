use alloc::{boxed::Box, collections::VecDeque, vec::Vec};
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;

use crate::{smp, util::Spinlock};

use super::{context_switch, Task, Thread, ThreadInfo, TCB};

static mut SCHEDULER: OnceCell<Scheduler> = OnceCell::uninit();

pub fn init() {
    let core_count = smp::core_count();
    unsafe {
        SCHEDULER
            .try_init_once(|| Scheduler::new(core_count))
            .expect("scheduler already initialized");
    }
}

pub fn surrender() {
    unsafe {
        SCHEDULER
            .try_get()
            .expect("scheduler not initialized")
            .surrender(false);
    }
}

pub fn stop() {
    unsafe {
        SCHEDULER
            .try_get()
            .expect("scheduler not initialized")
            .surrender(true);
    }
}

pub fn schedule(thread: Box<dyn TCB>) {
    unsafe {
        SCHEDULER
            .try_get()
            .expect("scheduler not initialized")
            .schedule(thread);
    }
}

pub fn cleanup() {
    unsafe {
        SCHEDULER
            .try_get()
            .expect("scheduler not initialized")
            .cleanup();
    }
}

pub fn swap_active(thread: Option<Box<dyn TCB>>) -> Option<Box<dyn TCB>> {
    unsafe {
        SCHEDULER
            .try_get()
            .expect("scheduler not initialized")
            .swap_active(thread)
    }
}

pub struct Scheduler {
    ready: ArrayQueue<Box<dyn TCB>>,
    active: Vec<Spinlock<Option<Box<dyn TCB>>>>,
    cleanup: Vec<Spinlock<Box<CleanupTaskHolder>>>,
}

impl Scheduler {
    fn new(core_count: usize) -> Self {
        let mut active: Vec<Spinlock<Option<Box<dyn TCB>>>> = Vec::with_capacity(core_count);
        active.resize_with(core_count, || {
            Spinlock::new(Some(Box::new(BootstrapThread::new())))
        });

        let mut cleanup: Vec<Spinlock<Box<CleanupTaskHolder>>> = Vec::with_capacity(core_count);
        cleanup.resize_with(core_count, || Spinlock::new(Box::default()));

        Self {
            ready: ArrayQueue::new(1024),
            active,
            cleanup,
        }
    }

    fn schedule(&self, thread: Box<dyn TCB>) {
        log::debug!("scheduling thread {:x?}", thread);
        self.ready.push(thread).expect("thread queue full");
    }

    fn surrender(&'static self, stop: bool) {
        let mut current_thread: Box<dyn TCB> = match self.swap_active(None) {
            Some(thread) => thread,
            None => {
                panic!("surrender called on no active thread")
            }
        };
        let current_thread_info = current_thread.get_info();

        let me = smp::me();
        if stop {
            log::trace!("stopping thread {:x?}", current_thread);
            let drop_current = move || {
                let x = current_thread;
                log::debug!("stopped thread {:x?}", x);
                drop(x);
            };
            self.cleanup[me as usize]
                .lock_sync()
                .add_task(Box::new(drop_current));
        } else {
            log::trace!("surrendering thread {:x?}", current_thread);
            let add_to_ready = move || {
                self.ready.push(current_thread).expect("thread queue full");
            };
            // The next thread adds us back to the ready queue
            self.cleanup[me as usize]
                .lock_sync()
                .add_task(Box::new(add_to_ready));
        }

        self.block(current_thread_info);
    }

    fn swap_active(&self, thread: Option<Box<dyn TCB>>) -> Option<Box<dyn TCB>> {
        let mut result = thread;
        core::mem::swap(
            &mut result,
            &mut self.active[smp::me() as usize].lock_sync(),
        );
        result
    }

    fn cleanup(&self) {
        let me = smp::me();
        let mut cleanup = self.cleanup[me as usize].lock_sync();

        while let Some(task) = cleanup.get_task() {
            task()
        }
    }

    fn block(&'static self, current_thread_info: *mut ThreadInfo) {
        let mut next_thread: Box<dyn TCB> = match self.ready.pop() {
            Some(thread) => thread,
            None => {
                let work = move || {};

                Box::new(Thread::new(Box::new(work)))
            }
        };

        let next_thread_info = next_thread.get_info();
        let assert_as_active = move || {
            self.swap_active(Some(next_thread));
        };
        self.cleanup[smp::me() as usize]
            .lock_sync()
            .add_task(Box::new(assert_as_active));
        unsafe {
            context_switch(current_thread_info, next_thread_info);
        }
        self.cleanup();
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct BootstrapThread {
    thread_info: ThreadInfo,
    stack_frame_start: Option<usize>,
}

impl core::fmt::Debug for BootstrapThread {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BootstrapThread").finish()
    }
}

impl BootstrapThread {
    pub fn new() -> Self {
        Self {
            thread_info: ThreadInfo::new(0),
            stack_frame_start: None,
        }
    }
}

impl TCB for BootstrapThread {
    fn get_info(&mut self) -> *mut ThreadInfo {
        &mut self.thread_info as *mut ThreadInfo
    }

    fn get_work(&mut self) -> Box<Task> {
        panic!("tried to get work from bootstrap thread")
    }
}

type Cleanup = dyn FnOnce() + Send + Sync;

#[derive(Default)]
pub struct CleanupTaskHolder {
    tasks: VecDeque<Box<Cleanup>>,
}

impl CleanupTaskHolder {
    pub fn add_task(&mut self, task: Box<Cleanup>) {
        self.tasks.push_back(task);
    }
    pub fn get_task(&mut self) -> Option<Box<Cleanup>> {
        self.tasks.pop_front()
    }
}
