pub mod executor;
pub mod keyboard;

use core::sync::atomic::AtomicU64;
use core::{future::Future, pin::Pin};
use core::task::{Context, Poll};
use alloc::boxed::Box;
use alloc::string::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed))
    }
}

pub struct Task {
    id: TaskId,
    name: Option<String>,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Self {
        Self {
            id: TaskId::new(),
            name: None,
            future: Box::pin(future),
        }
    }

    pub fn new_with_name(name: &str, future: impl Future<Output = ()> + 'static) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::new(future)
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

impl core::fmt::Debug for Task {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}
