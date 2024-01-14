//! Kernel threads
//!
//! Threads are used to run preemtively scheduled code.

mod thread_entry_point;

use alloc::boxed::Box;
use core::sync::atomic::AtomicU64;

pub use thread_entry_point::context_switch;
use thread_entry_point::thread_entry_point;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadId(u64);

impl ThreadId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self(NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::SeqCst))
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        Self::new()
    }
}

type Task = dyn 'static + FnOnce() + Send + Sync;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ThreadInfo {
    stack_pointer: usize,
}
impl ThreadInfo {
    pub fn new(stack_pointer: usize) -> Self {
        Self { stack_pointer }
    }
}

/// Thread Control Block
pub trait TCB: Send + Sync {
    fn get_info(&mut self) -> *mut ThreadInfo;
    fn get_work(&mut self) -> Box<Task>;
}

#[repr(C)]
pub struct Thread {
    id: ThreadId,
    thread_info: ThreadInfo,
    stack: Box<[u64]>,
    work: Option<Box<Task>>,
}

impl Thread {
    const NUM_CALLEE_SAVED: usize = 6;

    pub fn new(work: Box<Task>) -> Self {
        let mut stack: Box<[u64]> = Box::new([0; 512]);
        let end_of_stack = 511;

        stack[end_of_stack] = thread_entry_point as *const () as u64;
        let index: usize = end_of_stack - Self::NUM_CALLEE_SAVED - 1; // Callee saved registers
        stack[index] = 0; // CR2
        stack[index - 1] = 0; // RFLAGS
        let stack_ptr = Box::into_raw(stack);
        let stack_ptr_usize = stack_ptr as *mut u64 as usize;
        stack = unsafe { Box::from_raw(stack_ptr) };
        let stack_ptr_start = stack_ptr_usize + (index - 1) * core::mem::size_of::<usize>();
        let thread_info = ThreadInfo::new(stack_ptr_start);

        Self {
            id: ThreadId::new(),
            thread_info,
            stack,
            work: Some(work),
        }
    }
}

impl TCB for Thread {
    fn get_info(&mut self) -> *mut ThreadInfo {
        &mut self.thread_info as *mut ThreadInfo
    }

    fn get_work(&mut self) -> Box<Task> {
        let mut work = None;
        core::mem::swap(&mut work, &mut self.work);
        match work {
            Some(task) => task,
            None => panic!("Thread had no work!"),
        }
    }
}
