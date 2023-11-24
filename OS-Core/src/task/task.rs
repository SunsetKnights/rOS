use super::{context::TaskContext, info::TaskInfo};

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

#[derive(Clone, Copy)]
pub struct TaskControlBlock {
    pub task_context: TaskContext,
    pub task_status: TaskStatus,
    pub task_info: TaskInfo,
}
