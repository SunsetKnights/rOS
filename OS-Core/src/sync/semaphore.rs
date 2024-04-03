use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
};

use crate::task::{
    block_current_and_run_next, manager::wakeup_thread, processor::current_task,
    thread::ThreadControlBlock,
};

use super::UPSafeCell;

pub struct Semaphore {
    inner: UPSafeCell<SemaphoreInner>,
}
pub struct SemaphoreInner {
    count: isize,
    blocked_threads: VecDeque<Weak<ThreadControlBlock>>,
}
impl Semaphore {
    pub fn new(res_count: usize) -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    blocked_threads: VecDeque::new(),
                })
            },
        }
    }
    pub fn up(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            wakeup_thread(inner.blocked_threads.pop_front().unwrap());
        }
    }
    pub fn down(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            let curr_thread = current_task().unwrap();
            inner
                .blocked_threads
                .push_back(Arc::downgrade(&curr_thread));
            drop(inner);
            block_current_and_run_next();
        }
    }
}
