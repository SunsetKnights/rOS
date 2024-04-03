use alloc::sync::Arc;

use crate::{
    sync::{condvar::Condvar, mutex::*, semaphore::Semaphore},
    task::{
        block_current_and_run_next,
        processor::{current_process, current_task},
    },
    timer::{add_timer, get_time_ms},
};

pub fn sys_sleep(ms: usize) -> isize {
    let expire_ms = get_time_ms() + ms;
    let thread = current_task().unwrap();
    add_timer(expire_ms, thread);
    block_current_and_run_next();
    0
}

pub fn sys_mutex_create(blocking: bool) -> isize {
    let mutex: Arc<dyn Mutex>;
    match blocking {
        true => {
            mutex = Arc::new(MutexBlocking::new());
        }
        false => {
            mutex = Arc::new(MutexSpin::new());
        }
    };
    current_process().inner_exclusive_access().add_mutex(mutex) as isize
}

pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let mutex = current_process()
        .inner_exclusive_access()
        .get_mutex(mutex_id);
    mutex.lock();
    0
}

pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let mutex = current_process()
        .inner_exclusive_access()
        .get_mutex(mutex_id);
    mutex.unlock();
    0
}

pub fn sys_semaphore_create(res_count: usize) -> isize {
    let semaphore = Arc::new(Semaphore::new(res_count));
    current_process()
        .inner_exclusive_access()
        .add_semaphore(semaphore) as isize
}

pub fn sys_semaphore_up(semaphore_id: usize) -> isize {
    let semaphore = current_process()
        .inner_exclusive_access()
        .get_semaphore(semaphore_id);
    semaphore.up();
    0
}

pub fn sys_semaphore_down(semaphore_id: usize) -> isize {
    let semaphore = current_process()
        .inner_exclusive_access()
        .get_semaphore(semaphore_id);
    semaphore.down();
    0
}

pub fn sys_condvar_create() -> isize {
    current_process()
        .inner_exclusive_access()
        .add_condvar(Arc::new(Condvar::new())) as isize
}

pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    current_process()
        .inner_exclusive_access()
        .get_condvar(condvar_id)
        .signal();
    0
}

pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    let proc = current_process();
    let proc_inner = proc.inner_exclusive_access();
    let mutex = proc_inner.get_mutex(mutex_id);
    let condvar = proc_inner.get_condvar(condvar_id);
    drop(proc_inner);
    drop(proc);
    condvar.wait(mutex);
    0
}
