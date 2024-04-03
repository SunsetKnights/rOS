use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
};

use crate::task::{
    block_current_and_run_next, manager::wakeup_thread, processor::current_task,
    thread::ThreadControlBlock,
};

use super::{mutex::Mutex, UPSafeCell};

pub struct Condvar {
    inner: UPSafeCell<CondvarInner>,
}
pub struct CondvarInner {
    wait_queue: VecDeque<Weak<ThreadControlBlock>>,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(CondvarInner {
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    pub fn wait(&self, mutex: Arc<dyn Mutex>) {
        mutex.unlock();
        self.inner
            .exclusive_access()
            .wait_queue
            .push_back(Arc::downgrade(&current_task().unwrap()));
        block_current_and_run_next();
        mutex.lock();
    }

    pub fn signal(&self) {
        if let Some(tcb) = self.inner.exclusive_access().wait_queue.pop_front() {
            wakeup_thread(tcb);
        }
    }
}
