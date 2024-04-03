use crate::config::TICKS_PER_SEC;
use crate::platfrom::CLOCK_FREQ;
use crate::sbi_services::set_timer;
use crate::sync::UPSafeCell;
use crate::task::manager::wakeup_thread;
use crate::task::thread::ThreadControlBlock;
use alloc::collections::BinaryHeap;
use alloc::sync::{Arc, Weak};
use lazy_static::lazy_static;
use riscv::register::time;

pub fn get_time() -> usize {
    time::read()
}

pub fn get_time_ms() -> usize {
    time::read() / CLOCK_FREQ * 1000
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC)
}

pub struct TimerCondVar {
    pub expire_ms: usize,
    pub thread: Weak<ThreadControlBlock>,
}

impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire_ms == other.expire_ms
    }
}
impl Eq for TimerCondVar {}
impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        // The default heap is a large root heap,
        // but a small root heap is required,
        // so the comparison function returns the reverse order.
        match self.expire_ms.partial_cmp(&other.expire_ms) {
            Some(core::cmp::Ordering::Greater) => Some(core::cmp::Ordering::Less),
            Some(core::cmp::Ordering::Less) => Some(core::cmp::Ordering::Greater),
            equal => equal,
        }
    }
}
impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

lazy_static! {
    static ref TIMER: UPSafeCell<BinaryHeap<TimerCondVar>> =
        unsafe { UPSafeCell::new(BinaryHeap::new()) };
}

pub fn add_timer(expire_ms: usize, thread: Arc<ThreadControlBlock>) {
    TIMER.exclusive_access().push(TimerCondVar {
        expire_ms,
        thread: Arc::downgrade(&thread),
    });
}

pub fn check_timer() {
    let now = get_time_ms();
    let mut timers = TIMER.exclusive_access();
    while let Some(timer) = timers.peek() {
        if timer.expire_ms <= now {
            wakeup_thread(timers.pop().unwrap().thread);
        } else {
            break;
        }
    }
}
