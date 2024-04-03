use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::fs::inode::{open_file, OpenFlags};

use self::{
    manager::{add_proc, add_ready_thread, remove_proc, remove_thread},
    process::ProcessControlBlock,
    processor::{current_process, schedule, schedule_block, take_current_task},
    signal::SignalFlags,
};

pub mod action;
pub mod context;
pub mod manager;
pub mod process;
pub mod processor;
pub mod res;
pub mod signal;
pub mod switch;
pub mod thread;

lazy_static! {
    pub static ref INITPROC: Arc<ProcessControlBlock> = ProcessControlBlock::new(
        open_file("initproc", OpenFlags::READ_ONLY)
            .unwrap()
            .read_all()
            .as_slice()
    );
}

pub fn add_initproc() {
    add_proc(INITPROC.clone());
    add_ready_thread(INITPROC.clone().inner_exclusive_access().get_thread(0));
}

pub fn exit_current_and_run_next(exit_code: i32) {
    //let task = take_current_task().unwrap();
    //let mut inner = task.inner_exclusive_access();
    //inner.task_status = TaskStatus::Zombie;
    //inner.exit_code = exit_code;
    //let mut initproc_inner = INITPROC.inner_exclusive_access();
    //for child in inner.children.iter() {
    //    child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
    //    initproc_inner.children.push(child.clone());
    //}
    //inner.children.clear();
    //drop(initproc_inner);
    //inner.memory_set.recycle_data_pages();
    //drop(inner);

    let task = take_current_task().unwrap();
    let tid = task.tid();
    // Dealloc user resource.
    task.inner_exclusive_access().res.take();
    // Set exit code to current thread.
    task.inner_exclusive_access().exit_code = Some(exit_code);
    // If main thread exit, then add all child process to init process, recycle all thread resource.
    if tid == 0 {
        let proc = task.process.upgrade().unwrap();
        let mut proc_inner = proc.inner_exclusive_access();
        // Set current process to zombie process.
        proc_inner.is_zombie = true;
        proc_inner.exit_code = exit_code;
        // Add all child process to init process.
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in proc_inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
        drop(initproc_inner);
        proc_inner.children.clear();
        // Recycle all thread resource but main thread.
        let mut temp = proc_inner.threads.clone();
        proc_inner.threads.clear();
        // Delete fd_table
        proc_inner.fd_table.clear();
        drop(proc_inner);
        // Recycle data page.
        for i in 1..temp.len() {
            if let Some(tcb) = temp[i].take() {
                remove_thread(tcb);
            }
        }
        proc.inner_exclusive_access()
            .memory_set
            .recycle_data_pages();
        remove_proc(proc.pid());
        drop(proc);
    }
    schedule();
}

pub fn suspended_current_and_run_next() {
    schedule();
}

/// Block current thread and run next thread.
/// This function will put the reference of the current thread control block into the global task manager.
/// The caller only needs to save the weak reference of the thread control block.
pub fn block_current_and_run_next() {
    schedule_block();
}

pub fn get_pid() -> isize {
    current_process().pid() as isize
}

pub fn current_add_signal(signal: SignalFlags) {
    let pcb = current_process();
    pcb.inner_exclusive_access().signals.insert(signal);
}

pub fn check_current_signals_error() -> Option<(i32, &'static str)> {
    current_process()
        .inner_exclusive_access()
        .signals
        .check_error()
}
