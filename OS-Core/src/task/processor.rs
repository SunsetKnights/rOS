use crate::{sync::UPSafeCell, trap::TrapContext};

use super::{
    context::TaskContext,
    manager::{add_block_thread, add_ready_thread, fetch_task},
    process::ProcessControlBlock,
    switch::__switch,
    thread::{ThreadControlBlock, ThreadStatus},
};
use alloc::sync::Arc;
use lazy_static::lazy_static;

pub struct Processor {
    current: Option<Arc<ThreadControlBlock>>,
    processor_task_context: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            processor_task_context: TaskContext::zero_init(),
        }
    }

    pub fn take_current(&mut self) -> Option<Arc<ThreadControlBlock>> {
        self.current.take()
    }

    pub fn set_current(&mut self, tcb: Option<Arc<ThreadControlBlock>>) {
        self.current = tcb;
    }

    pub fn current(&self) -> Option<Arc<ThreadControlBlock>> {
        self.current.as_ref().map(|tcb| tcb.clone())
    }

    pub fn get_processor_task_context(&mut self) -> *mut TaskContext {
        &mut self.processor_task_context as *mut TaskContext
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

/// Take current thread from processor
pub fn take_current_task() -> Option<Arc<ThreadControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}
/// Get a current tcb copy
pub fn current_task() -> Option<Arc<ThreadControlBlock>> {
    PROCESSOR.exclusive_access().current()
}
/// Get current process.
pub fn current_process() -> Arc<ProcessControlBlock> {
    current_task().unwrap().process.upgrade().unwrap()
}
/// Get current user token.
pub fn current_user_token() -> usize {
    current_task().unwrap().token()
}
/// Get the phsical address of trap context of current thread.
pub fn current_trap_context() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .trap_context()
}
/// Get the virtual address of trap context of current thread.
pub fn current_trap_context_va() -> usize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .trap_context_va()
}

/// Take out a task from the task scheduling queue,
/// set the task status to Running,
/// and then switch to the task running.
pub fn run_tasks() {
    loop {
        if let Some(tcb) = fetch_task() {
            let mut next_task_inner = tcb.inner_exclusive_access();
            let next_task = next_task_inner.task_context_ptr_mut();
            let mut processor = PROCESSOR.exclusive_access();
            let processor_task = processor.get_processor_task_context();
            next_task_inner.set_status(ThreadStatus::Running);
            drop(next_task_inner);
            processor.set_current(Some(tcb));
            drop(processor);
            unsafe { __switch(processor_task, next_task) };
        }
    }
}

/// Switch the currently running task to the Ready status,
/// put it in the task scheduling queue,
/// and then switch to the processor task (run_tasks function).
pub fn schedule() {
    let curr_task;
    if let Some(curr_tcb) = take_current_task() {
        let mut tcb_inner = curr_tcb.inner_exclusive_access();
        tcb_inner.set_status(ThreadStatus::Ready);
        curr_task = tcb_inner.task_context_ptr_mut();
        drop(tcb_inner);
        add_ready_thread(curr_tcb);
    } else {
        curr_task = (&mut TaskContext::zero_init()) as *mut TaskContext;
    }
    let mut processor = PROCESSOR.exclusive_access();
    let processor_task = processor.get_processor_task_context();
    drop(processor);
    unsafe { __switch(curr_task, processor_task) };
}

pub fn schedule_block() {
    let curr_task;
    if let Some(curr_tcb) = take_current_task() {
        let mut tcb_inner = curr_tcb.inner_exclusive_access();
        tcb_inner.set_status(ThreadStatus::Blocked);
        curr_task = tcb_inner.task_context_ptr_mut();
        drop(tcb_inner);
        add_block_thread(curr_tcb);
    } else {
        curr_task = (&mut TaskContext::zero_init()) as *mut TaskContext;
    }
    let mut processor = PROCESSOR.exclusive_access();
    let processor_task = processor.get_processor_task_context();
    drop(processor);
    unsafe { __switch(curr_task, processor_task) };
}
