use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
};

use crate::task::{
    block_current_and_run_next, manager::wakeup_thread, processor::current_task,
    suspended_current_and_run_next, thread::ThreadControlBlock,
};

use super::UPSafeCell;

pub trait Mutex: Send + Sync {
    fn lock(&self);
    fn unlock(&self);
}

pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}
pub struct MutexBlockingInner {
    locked: bool,
    blocked_threads: VecDeque<Weak<ThreadControlBlock>>,
}
impl MutexBlocking {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    blocked_threads: VecDeque::new(),
                })
            },
        }
    }
}
impl Mutex for MutexBlocking {
    fn lock(&self) {
        let mut inner = self.inner.exclusive_access();
        if inner.locked {
            inner
                .blocked_threads
                .push_back(Arc::downgrade(&current_task().unwrap()));
            drop(inner);
            block_current_and_run_next();
        } else {
            inner.locked = true;
        }
    }

    fn unlock(&self) {
        let mut inner = self.inner.exclusive_access();
        assert!(inner.locked);
        if inner.blocked_threads.is_empty() {
            inner.locked = false;
        } else {
            wakeup_thread(inner.blocked_threads.pop_front().unwrap());
        }
    }
}

pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}
impl MutexSpin {
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}
impl Mutex for MutexSpin {
    fn lock(&self) {
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspended_current_and_run_next();
            } else {
                *locked = true;
                break;
            }
        }
    }

    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        assert!(*locked);
        *locked = false;
    }
}
