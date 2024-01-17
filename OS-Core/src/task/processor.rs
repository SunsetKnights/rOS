use crate::{sync::UPSafeCell, trap::TrapContext};

use super::{
    context::TaskContext,
    manager::{add_task, fetch_task},
    switch::__switch,
    task::{ProcessControlBlock, TaskStatus},
};
use alloc::sync::Arc;
use lazy_static::lazy_static;

pub struct Processor {
    current: Option<Arc<ProcessControlBlock>>,
    processor_task_context: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            processor_task_context: TaskContext::zero_init(),
        }
    }

    pub fn take_current(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.current.take()
    }

    pub fn set_current(&mut self, pcb: Option<Arc<ProcessControlBlock>>) {
        self.current = pcb;
    }

    pub fn current(&self) -> Option<Arc<ProcessControlBlock>> {
        self.current.as_ref().map(|pcb| pcb.clone())
    }

    pub fn get_processor_task_context(&mut self) -> *mut TaskContext {
        &mut self.processor_task_context as *mut TaskContext
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

pub fn take_current_task() -> Option<Arc<ProcessControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

pub fn current_task() -> Option<Arc<ProcessControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

pub fn current_user_token() -> usize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_user_token()
}

pub fn current_trap_context() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_context()
}

/// Take out a task from the task scheduling queue,
/// set the task status to Running,
/// and then switch to the task running.
pub fn run_tasks() {
    loop {
        if let Some(pcb) = fetch_task() {
            let mut next_pcb_inner = pcb.inner_exclusive_access();
            let next_task = next_pcb_inner.get_task_context();
            let mut processor = PROCESSOR.exclusive_access();
            let processor_task = processor.get_processor_task_context();
            next_pcb_inner.set_task_status(TaskStatus::Running);
            drop(next_pcb_inner);
            processor.set_current(Some(pcb));
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
    if let Some(curr_pcb) = take_current_task() {
        let mut pcb_inner = curr_pcb.inner_exclusive_access();
        pcb_inner.set_task_status(TaskStatus::Ready);
        curr_task = pcb_inner.get_task_context();
        drop(pcb_inner);
        add_task(curr_pcb);
    } else {
        curr_task = (&mut TaskContext::zero_init()) as *mut TaskContext;
    }
    let mut processor = PROCESSOR.exclusive_access();
    let processor_task = processor.get_processor_task_context();
    drop(processor);
    unsafe { __switch(curr_task, processor_task) };
}
