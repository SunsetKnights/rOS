use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Weak},
};
use lazy_static::lazy_static;

use crate::sync::UPSafeCell;

use super::{
    process::ProcessControlBlock,
    thread::{ThreadControlBlock, ThreadStatus},
};

pub struct TaskManager {
    ready_queue: VecDeque<Arc<ThreadControlBlock>>,
    blocked_thread: BTreeMap<usize, Arc<ThreadControlBlock>>,
    all_pcb: BTreeMap<usize, Arc<ProcessControlBlock>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            blocked_thread: BTreeMap::new(),
            all_pcb: BTreeMap::new(),
        }
    }
    pub fn add_process(&mut self, proc: Arc<ProcessControlBlock>) {
        self.all_pcb.insert(proc.pid.0, proc.clone());
    }
    pub fn add_ready_thread(&mut self, thread: Arc<ThreadControlBlock>) {
        self.ready_queue.push_back(thread);
    }
    pub fn add_block_thread(&mut self, thread: Arc<ThreadControlBlock>) {
        let ptr = Arc::as_ptr(&thread) as usize;
        self.blocked_thread.insert(ptr, thread);
    }
    pub fn remove_process(&mut self, pid: usize) {
        self.all_pcb.remove(&pid);
    }
    pub fn remove_thread(&mut self, thread: Arc<ThreadControlBlock>) {
        self.ready_queue
            .retain(|ptr| Arc::as_ptr(&ptr) != Arc::as_ptr(&thread));
        self.blocked_thread.remove(&(Arc::as_ptr(&thread) as usize));
    }
    pub fn move_out_blocked_thread(&mut self, ptr: usize) -> Arc<ThreadControlBlock> {
        self.blocked_thread.remove(&ptr).unwrap()
    }
    pub fn fetch(&mut self) -> Option<Arc<ThreadControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

pub fn add_proc(proc: Arc<ProcessControlBlock>) {
    TASK_MANAGER.exclusive_access().add_process(proc);
}
pub fn add_ready_thread(thread: Arc<ThreadControlBlock>) {
    TASK_MANAGER.exclusive_access().add_ready_thread(thread);
}
pub fn add_block_thread(thread: Arc<ThreadControlBlock>) {
    TASK_MANAGER.exclusive_access().add_block_thread(thread);
}
pub fn remove_proc(pid: usize) {
    TASK_MANAGER.exclusive_access().remove_process(pid);
}
pub fn remove_thread(thread: Arc<ThreadControlBlock>) {
    TASK_MANAGER.exclusive_access().remove_thread(thread);
}
pub fn move_out_blocked_thread(ptr: usize) -> Arc<ThreadControlBlock> {
    TASK_MANAGER.exclusive_access().move_out_blocked_thread(ptr)
}

pub fn fetch_task() -> Option<Arc<ThreadControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

pub fn wakeup_thread(thread: Weak<ThreadControlBlock>) {
    if thread.strong_count() != 0 {
        let ptr = thread.as_ptr() as usize;
        let thread = move_out_blocked_thread(ptr);
        thread
            .inner_exclusive_access()
            .set_status(ThreadStatus::Ready);
        add_ready_thread(thread);
    }
}

pub fn proc_from_pid(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    TASK_MANAGER
        .exclusive_access()
        .all_pcb
        .get(&pid)
        .map(Arc::clone)
}
