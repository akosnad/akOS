use crate::util::Spinlock;

use super::{Task, TaskId};
use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use conquer_once::spin::OnceCell;
use core::{
    fmt::Debug,
    sync::atomic::AtomicBool,
    task::{Context, Poll, Waker},
};
use crossbeam_queue::ArrayQueue;

static mut DUMP_STATE: bool = false;

static CAN_SCHEDULE: AtomicBool = AtomicBool::new(false);
static mut EXECUTOR: OnceCell<Executor> = OnceCell::uninit();

/// This should be called from the main thread to initialize the kernel executor.
pub fn run() -> ! {
    unsafe {
        EXECUTOR
            .try_init_once(|| {
                let executor = Executor::default();
                executor.spawn(Task::new_with_name("logger", super::logger::process()));
                executor.spawn(Task::new_with_name("keyboard", super::keyboard::process()));
                executor.spawn(Task::new_with_name("mouse", super::mouse::process()));
                executor
            })
            .expect("executor already initialized");
        EXECUTOR.get().unwrap().run()
    }
}

/// This should be called from additional cores to signal that they are ready to run tasks.
pub fn schedule(id: u8) -> ! {
    while !CAN_SCHEDULE.load(core::sync::atomic::Ordering::SeqCst) {
        core::hint::spin_loop()
    }
    unsafe {
        EXECUTOR
            .try_get()
            .expect("executor not initialized")
            .schedule(id)
    }
}

#[derive(Debug)]
pub struct Executor {
    tasks: Spinlock<BTreeMap<TaskId, Task>>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: Spinlock<BTreeMap<TaskId, Waker>>,
}

impl Executor {
    pub fn spawn(&self, task: Task) {
        let task_id = task.id;
        if self.tasks.lock_sync().insert(task.id, task).is_some() {
            panic!("task with same ID already exists");
        }
        self.task_queue.push(task_id).expect("task queue full");
    }

    pub fn run_ready_tasks(&self) {
        // destructive self
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Some(task_id) = task_queue.pop() {
            let mut tasks = tasks.lock_sync();
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists
            };

            let mut waker_cache = waker_cache.lock_sync();
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new_waker(task_id, task_queue.clone()));

            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    #[cfg(feature = "dbg-executor")]
                    log::trace!("{:?} ready", task_id);

                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        if !self.task_queue.is_empty() {
            return;
        }

        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }

    pub fn run(&self) -> ! {
        CAN_SCHEDULE.store(true, core::sync::atomic::Ordering::SeqCst);
        loop {
            unsafe {
                if DUMP_STATE {
                    self.dump_state_inner();
                }
            }
            self.run_ready_tasks();
            self.sleep_if_idle();
            crate::time::wake_sleepers();
        }
    }

    fn schedule(&self, id: u8) -> ! {
        log::info!("core {} scheduled", id);
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    fn dump_state_inner(&self) {
        log::trace!("executor state dump:\n{:#?}", self);
        unsafe {
            DUMP_STATE = false;
        }
    }
    pub fn dump_state() {
        unsafe {
            DUMP_STATE = true;
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self {
            tasks: Spinlock::new(BTreeMap::new()),
            task_queue: Arc::new(ArrayQueue::new(1024)),
            waker_cache: Spinlock::new(BTreeMap::new()),
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}
impl TaskWaker {
    fn new_waker(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue
            .push(self.task_id)
            .expect("cannot wake task, task_queue full");
    }
}
impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
