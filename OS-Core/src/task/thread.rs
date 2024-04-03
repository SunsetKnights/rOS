use core::cell::RefMut;

use alloc::sync::{Arc, Weak};

use crate::{mm::address::PhysPageNum, sync::UPSafeCell, trap::TrapContext};

use super::{
    context::TaskContext,
    process::ProcessControlBlock,
    res::{kernel_stack_alloc, KernelStack, ThreadUserRes},
};

pub enum ThreadStatus {
    Ready,
    Running,
    Blocked,
}

pub struct ThreadControlBlock {
    pub process: Weak<ProcessControlBlock>,
    pub kernel_stack: KernelStack,
    inner: UPSafeCell<ThreadControlBlockInner>,
}
impl ThreadControlBlock {
    /// Create a new thread
    pub fn new(process: Arc<ProcessControlBlock>, user_stack_base: usize, alloc_res: bool) -> Self {
        let kernel_stack = kernel_stack_alloc();
        let res = ThreadUserRes::new(user_stack_base, process.clone(), alloc_res);
        let trap_context_ppn = res.trap_context_ppn();
        let kernel_stack_buttom = kernel_stack.get_bottom();
        Self {
            process: Arc::downgrade(&process),
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(ThreadControlBlockInner {
                    res: Some(res),
                    trap_context_ppn,
                    task_context: TaskContext::goto_trap_return(kernel_stack_buttom),
                    status: ThreadStatus::Ready,
                    exit_code: None,
                })
            },
        }
    }

    pub fn token(&self) -> usize {
        self.process.upgrade().unwrap().token()
    }

    pub fn tid(&self) -> usize {
        self.inner_exclusive_access().res.as_ref().unwrap().tid
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, ThreadControlBlockInner> {
        self.inner.exclusive_access()
    }
}

pub struct ThreadControlBlockInner {
    pub res: Option<ThreadUserRes>,
    pub trap_context_ppn: PhysPageNum,
    pub task_context: TaskContext,
    pub status: ThreadStatus,
    pub exit_code: Option<i32>,
}
impl ThreadControlBlockInner {
    pub fn trap_context(&self) -> &'static mut TrapContext {
        self.trap_context_ppn.get_mut()
    }

    pub fn trap_context_va(&self) -> usize {
        self.res.as_ref().unwrap().trap_context_va()
    }

    /// Reallocate user stack and trap page, called by exec or spawn.
    pub fn realloc_res(&mut self, new_user_stack_base: usize) {
        let resource = self.res.as_mut().unwrap();
        resource.rebase(new_user_stack_base);
        resource.alloc_res();
        self.trap_context_ppn = resource.trap_context_ppn();
    }

    pub fn task_context_ptr_mut(&mut self) -> *mut TaskContext {
        &mut self.task_context as *mut TaskContext
    }

    pub fn set_status(&mut self, status: ThreadStatus) {
        self.status = status;
    }
}
