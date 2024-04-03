use core::arch::asm;

use crate::SignalAction;

// system call numbers
// IO
const SYS_DUP: usize = 24;
const SYS_OPEN: usize = 56;
const SYS_CLOSE: usize = 57;
const SYS_PIPE: usize = 59;
const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
// Process
const SYS_EXIT: usize = 93;
const SYS_SLEEP: usize = 101;
const SYS_YIELD: usize = 124;
// Siganal
const SYS_KILL: usize = 129;
const SYS_SIGACTION: usize = 134;
const SYS_SIGPROCMASK: usize = 135;
const SYS_SIGRETURN: usize = 139;
const SYS_GET_TIME: usize = 169;
// Process
const SYS_GET_PID: usize = 172;
const SYS_FORK: usize = 220;
const SYS_EXEC: usize = 221;
const SYS_WAITPID: usize = 260;
const SYS_SPAWN: usize = 400;
// Thread
const SYS_THREAD_CREATE: usize = 1000;
const SYS_GET_TID: usize = 1001;
const SYS_WAITTID: usize = 1002;
// Mutex
const SYS_MUTEX_CREATE: usize = 1010;
const SYS_MUTEX_LOCK: usize = 1011;
const SYS_MUTEX_UNLOCK: usize = 1012;
// Semaphore
const SYS_SEMAPHORE_CREATE: usize = 1020;
const SYS_SEMAPHORE_UP: usize = 1021;
const SYS_SEMAPHORE_DOWN: usize = 1022;
// Condvar
const SYS_CONDVAR_CREATE: usize = 1030;
const SYS_CONDVAR_SIGNAL: usize = 1031;
const SYS_CONDVAR_WAIT: usize = 1032;

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
pub fn sys_dup(fd: usize) -> isize {
    sys_call(SYS_DUP, [fd, 0, 0])
}
pub fn sys_open(path: &str, flags: u32) -> isize {
    sys_call(SYS_OPEN, [path.as_ptr() as usize, flags as usize, 0])
}
pub fn sys_close(fd: usize) -> isize {
    sys_call(SYS_CLOSE, [fd, 0, 0])
}
pub fn sys_pipe(pipe: &mut [usize]) -> isize {
    sys_call(SYS_PIPE, [pipe.as_mut_ptr() as usize, 0, 0])
}
pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    sys_call(SYS_READ, [fd, buffer.as_mut_ptr() as usize, buffer.len()])
}
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    sys_call(SYS_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}
pub fn sys_exit(xstate: i32) -> ! {
    sys_call(SYS_EXIT, [xstate as usize, 0, 0]);
    panic!("Exit function should not return.");
}
pub fn sys_sleep(ms: usize) -> isize {
    sys_call(SYS_SLEEP, [ms, 0, 0])
}
pub fn sys_yield() -> isize {
    sys_call(SYS_YIELD, [0; 3])
}
pub fn sys_kill(pid: usize, signum: u32) -> isize {
    sys_call(SYS_KILL, [pid, signum as usize, 0])
}
pub fn sys_sigaction(
    signum: u32,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    sys_call(
        SYS_SIGACTION,
        [signum as usize, action as usize, old_action as usize],
    )
}
pub fn sys_sigprocmask(mask: u32) -> isize {
    sys_call(SYS_SIGPROCMASK, [mask as usize, 0, 0])
}
pub fn sys_sigreturn() -> isize {
    sys_call(SYS_SIGRETURN, [0; 3])
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
pub fn sys_exec(path: &str, args: &[*const u8]) -> isize {
    sys_call(
        SYS_EXEC,
        [path.as_ptr() as usize, args.as_ptr() as usize, 0],
    )
}
pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    sys_call(SYS_WAITPID, [pid as usize, exit_code as usize, 0])
}
pub fn sys_spawn(path: &str) -> isize {
    sys_call(SYS_SPAWN, [path.as_ptr() as usize, 0, 0])
}
pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
    sys_call(SYS_THREAD_CREATE, [entry, arg, 0])
}
pub fn sys_get_tid() -> isize {
    sys_call(SYS_GET_TID, [0; 3])
}
pub fn sys_waittid(tid: usize) -> isize {
    sys_call(SYS_WAITTID, [tid as usize, 0, 0])
}
pub fn sys_mutex_create(blocking: bool) -> isize {
    sys_call(SYS_MUTEX_CREATE, [blocking as usize, 0, 0])
}
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    sys_call(SYS_MUTEX_LOCK, [mutex_id, 0, 0])
}
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    sys_call(SYS_MUTEX_UNLOCK, [mutex_id, 0, 0])
}
pub fn sys_semaphore_create(res_count: usize) -> isize {
    sys_call(SYS_SEMAPHORE_CREATE, [res_count, 0, 0])
}
pub fn sys_semaphore_up(semaphore_id: usize) -> isize {
    sys_call(SYS_SEMAPHORE_UP, [semaphore_id, 0, 0])
}
pub fn sys_semaphore_down(semaphore_id: usize) -> isize {
    sys_call(SYS_SEMAPHORE_DOWN, [semaphore_id, 0, 0])
}
pub fn sys_condvar_create() -> isize {
    sys_call(SYS_CONDVAR_CREATE, [0; 3])
}
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    sys_call(SYS_CONDVAR_SIGNAL, [condvar_id, 0, 0])
}
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    sys_call(SYS_CONDVAR_WAIT, [condvar_id, mutex_id, 0])
}
