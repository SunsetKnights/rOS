use super::task::TaskStatus;
use crate::config::SYSCALL_QUANTITY;

#[derive(Clone, Copy)]
pub struct TaskInfo {
    pub id: usize,
    pub status: TaskStatus,
    pub call: [SyscallInfo; SYSCALL_QUANTITY],
    pub time: usize,
}

#[derive(Clone, Copy)]
pub struct SyscallInfo {
    pub id: usize,
    pub time: usize,
}

impl TaskInfo {
    pub fn init(id: usize) -> Self {
        TaskInfo {
            id,
            status: TaskStatus::UnInit,
            call: [
                SyscallInfo { id: 64, time: 0 },
                SyscallInfo { id: 93, time: 0 },
                SyscallInfo { id: 124, time: 0 },
                SyscallInfo { id: 169, time: 0 },
                SyscallInfo { id: 410, time: 0 },
            ],
            time: 0,
        }
    }
}
