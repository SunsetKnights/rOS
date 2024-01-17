use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::loader::load_app_from_name;

use self::{
    manager::add_task,
    processor::{current_task, schedule, take_current_task},
    task::{ProcessControlBlock, TaskStatus},
};

pub mod context;
pub mod manager;
pub mod pid;
pub mod processor;
pub mod switch;
pub mod task;

lazy_static! {
    pub static ref INITPROC: Arc<ProcessControlBlock> = Arc::new(ProcessControlBlock::new(
        load_app_from_name("initproc").unwrap()
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;
    let mut initproc_inner = INITPROC.inner_exclusive_access();
    for child in inner.children.iter() {
        child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
        initproc_inner.children.push(child.clone());
    }
    inner.children.clear();
    drop(initproc_inner);
    inner.memory_set.recycle_data_pages();
    drop(inner);
    schedule();
}

pub fn suspended_current_and_run_next() {
    schedule();
}

pub fn get_pid() -> isize {
    current_task().unwrap().get_pid() as isize
}
