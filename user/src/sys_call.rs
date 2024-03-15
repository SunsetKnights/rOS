use core::arch::asm;

// system call numbers
const SYS_OPEN: usize = 56;
const SYS_CLOSE: usize = 57;
const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_GET_TIME: usize = 169;
const SYS_GET_PID: usize = 172;
const SYS_FORK: usize = 220;
const SYS_EXEC: usize = 221;
const SYS_WAITPID: usize = 260;
const SYS_SPAWN: usize = 400;

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

// system calls
pub fn sys_open(path: &str, flags: u32) -> isize {
    sys_call(SYS_OPEN, [path.as_ptr() as usize, flags as usize, 0])
}
pub fn sys_close(fd: usize) -> isize {
    sys_call(SYS_CLOSE, [fd, 0, 0])
}
pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    sys_call(SYS_READ, [fd, buffer.as_mut_ptr() as usize, buffer.len()])
}
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    sys_call(SYS_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}
pub fn sys_exit(xstate: i32) -> isize {
    sys_call(SYS_EXIT, [xstate as usize, 0, 0])
}
pub fn sys_yield() -> isize {
    sys_call(SYS_YIELD, [0; 3])
}
pub fn sys_get_time() -> isize {
    sys_call(SYS_GET_TIME, [0; 3])
}
pub fn sys_get_pid() -> isize {
    sys_call(SYS_GET_PID, [0; 3])
}
pub fn sys_fork() -> isize {
    sys_call(SYS_FORK, [0; 3])
}
pub fn sys_exec(path: &str) -> isize {
    sys_call(SYS_EXEC, [path.as_ptr() as usize, 0, 0])
}
pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    sys_call(SYS_WAITPID, [pid as usize, exit_code as usize, 0])
}
pub fn sys_spawn(path: &str) -> isize {
    sys_call(SYS_SPAWN, [path.as_ptr() as usize, 0, 0])
}
