use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use lazy_static::lazy_static;

use crate::sync::UPSafeCell;

use super::task::ProcessControlBlock;

pub struct TaskManager {
    ready_queue: VecDeque<Arc<ProcessControlBlock>>,
    all_pcb: BTreeMap<usize, Arc<ProcessControlBlock>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            all_pcb: BTreeMap::new(),
        }
    }
    pub fn add(&mut self, proc: Arc<ProcessControlBlock>) {
        if !self.all_pcb.contains_key(&proc.pid.0) {
            self.all_pcb.insert(proc.pid.0, proc.clone());
        }
        self.ready_queue.push_back(proc);
    }
    pub fn remove(&mut self, pid: usize) {
        self.all_pcb.remove(&pid);
    }
    pub fn fetch(&mut self) -> Option<Arc<ProcessControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

pub fn add_task(proc: Arc<ProcessControlBlock>) {
    TASK_MANAGER.exclusive_access().add(proc);
}

pub fn remove_task(pid: usize) {
    TASK_MANAGER.exclusive_access().remove(pid);
}

pub fn fetch_task() -> Option<Arc<ProcessControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

pub fn task_from_pid(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    TASK_MANAGER
        .exclusive_access()
        .all_pcb
        .get(&pid)
        .map(Arc::clone)
}
