use alloc::{boxed::Box, string::String};
use core::{
    arch::asm,
    fmt::Display,
    sync::atomic::{AtomicU64, Ordering},
};
use x86_64::VirtAddr;

pub mod scheduler;
pub use scheduler::{exit, spawn, yield_now};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessID(u64);

const PROCESS_START_ADDR: VirtAddr = VirtAddr::new_truncate(0xF000);

impl ProcessID {
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

static CURRENT_PID: AtomicU64 = AtomicU64::new(0);

pub fn new_pid() -> ProcessID {
    let current = CURRENT_PID.load(Ordering::SeqCst);
    if current + 1 == u64::max_value() {
        panic!("out of process ids");
    }

    CURRENT_PID.store(current + 1, Ordering::SeqCst);
    ProcessID(current)
}

#[derive(Debug)]
pub enum ProcessState {
    Running,
    Waiting,
}

#[derive(Debug, Clone)]
pub enum Context {
    Initial(InitialState),
    Running(CPURegistersState),
}

#[derive(Debug, Clone)]
pub struct InitialState {
    pub rip_address: VirtAddr,
    pub cr3_base: u64,
    pub stack_end: VirtAddr,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct CPURegistersState {
    pub rbp: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}
impl CPURegistersState {
    // TODO: better way to save and restore registers

    #[inline(always)]
    pub fn get() -> *const Self {
        let state_repr: *const Self;
        unsafe {
            asm!(
                "push r15;
                push r14; 
                push r13;
                push r12;
                push r11;
                push r10;
                push r9;
                push r8;
                push rdi;
                push rsi;
                push rdx;
                push rcx;
                push rbx;
                push rax;
                push rbp;
                mov {}, rsp; 
                sub rsp, 0x400;",
                out(reg) state_repr
            );
        }

        state_repr
    }

    #[inline]
    pub fn load(state: &Self) {
        unsafe {
            asm!(
                "mov rsp, {};
                pop rbp;
                pop rax;
                pop rbx;
                pop rcx;
                pop rdx;
                pop rsi;
                pop rdi;
                pop r8;
                pop r9;
                pop r10;
                pop r11;
                pop r12;
                pop r13;
                pop r14;
                pop r15;
                iretq;",
                in(reg) state
            );
        }
    }
}

#[derive(Debug)]
pub enum ProcessError {}

#[derive(Debug)]
pub struct Process {
    pub id: ProcessID,
    pub name: String,
    pub state: ProcessState,
    pub context: Box<Context>,
}

impl Display for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}({:?})", self.name, self.id.0)
    }
}

impl Process {
    pub fn new(name: String, entry_point: VirtAddr) -> Result<Self, ProcessError> {
        let pid = new_pid();
        let mm = crate::mem::get_memory_manager();
        // TODO: create own page table, mappings, stack, entry point

        let stack = {
            let arr = Box::new([0u8; 0x1000]);
            let ptr = Box::into_raw(arr);
            let addr = ptr as *mut _ as u64;
            VirtAddr::new(addr)
        };

        let state = Context::Initial(InitialState {
            cr3_base: mm.lvl4_table_addr().as_u64(),
            stack_end: stack,
            rip_address: entry_point,
        });

        Ok(Self {
            id: pid,
            name,
            state: ProcessState::Waiting,
            context: Box::new(state),
        })
    }
}
