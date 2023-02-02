use super::{Task, TaskId};
use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use core::{task::{Context, Poll, Waker}, fmt::Debug};
use crossbeam_queue::ArrayQueue;

static mut DUMP_STATE: bool = false;

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already exists");
        }
        self.task_queue.push(task_id).expect("task queue full");
    }

    pub fn run_ready_tasks(&mut self) {
        // destructive self
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Some(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists
            };

            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new_waker(task_id, task_queue.clone()));

            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
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

    pub fn run(&mut self) -> ! {
        loop {
            unsafe {
                if DUMP_STATE {
                    self.dump_state_inner();
                }
            }
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
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(1024)),
            waker_cache: BTreeMap::new(),
        }
    }
}

impl Debug for Executor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Executor")
            .field("tasks", &self.tasks.values())
            .field("task_queue", &self.task_queue)
            .field("waker_cache", &self.waker_cache)
            .finish()
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
