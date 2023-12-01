use core::arch::asm;

// define 2 system call number
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_GET_TIME: usize = 169;
const SYSCALL_TASK_INFO: usize = 410;
pub const SYSCALL_QUANTITY: usize = 5;

fn sys_call(call_id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm! {
            "ecall",
            inlateout("x10") args[0]=>ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") call_id,
        }
    }
    ret
}

// system call that emulate linux system call
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    sys_call(SYS_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}
pub fn sys_exit(xstate: i32) -> isize {
    sys_call(SYS_EXIT, [xstate as usize, 0, 0])
}
pub fn sys_yield() -> isize {
    sys_call(SYS_YIELD, [0, 0, 0])
}
pub fn sys_get_time() -> isize {
    sys_call(SYS_GET_TIME, [0, 0, 0])
}

#[derive(Clone, Copy, Debug)]
pub struct TaskInfo {
    pub id: usize,
    pub status: TaskStatus,
    pub call: [SyscallInfo; SYSCALL_QUANTITY],
    pub time: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SyscallInfo {
    pub id: usize,
    pub time: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

pub fn sys_task_info(task_id: usize, ti: *mut TaskInfo) -> isize {
    sys_call(SYSCALL_TASK_INFO, [task_id, ti as usize, 0])
}
