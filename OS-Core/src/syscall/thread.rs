use alloc::sync::Arc;

use crate::{
    mm::memory_set::kernel_token,
    task::{
        manager::add_ready_thread,
        processor::{current_process, current_task},
        thread::ThreadControlBlock,
    },
    trap::{trap_handler, TrapContext},
};

pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
    let proc = current_process();
    // Create a new thread
    let tcb = Arc::new(ThreadControlBlock::new(
        proc.clone(),
        proc.user_stack_base,
        true,
    ));
    // Set thread trap context.
    let tcb_inner = tcb.inner_exclusive_access();
    let user_sp = tcb_inner.res.as_ref().unwrap().user_stack_bottom();
    let trap_context = tcb_inner.trap_context();
    *trap_context = TrapContext::init_app_context(
        entry,
        user_sp,
        kernel_token(),
        tcb.kernel_stack.get_bottom(),
        trap_handler as usize,
    );
    //Set arg
    trap_context.x[10] = arg;
    drop(tcb_inner);
    // Add thread to process.
    let tid = tcb.tid();
    let mut proc_inner = proc.inner_exclusive_access();
    if tid == proc_inner.thread_count() {
        proc_inner.threads.push(Some(tcb.clone()));
    } else {
        proc_inner.threads[tid] = Some(tcb.clone());
    }
    // Add thread to read queue.
    add_ready_thread(tcb);
    tid as isize
}

pub fn sys_waittid(tid: usize) -> isize {
    let proc = current_process();
    let mut inner = proc.inner_exclusive_access();
    if tid < inner.thread_count()
        && inner.threads[tid].is_some() // tid exist
        && current_task().unwrap().tid() != tid
    // thread can not wait itself
    {
        if inner.threads[tid]
            .as_ref()
            .unwrap()
            .inner_exclusive_access()
            .exit_code
            .is_some()
        {
            // Move out thread.
            let tcb = inner.threads[tid].take().unwrap();
            let exit_code = tcb.inner_exclusive_access().exit_code.take().unwrap();
            exit_code as isize
        } else {
            -2
        }
    } else {
        -1
    }
}

pub fn sys_get_tid() -> isize {
    current_task().unwrap().tid() as isize
}
