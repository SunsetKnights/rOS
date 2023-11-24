use core::arch::asm;

// define 2 system call number
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_GET_TIME: usize = 169;

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
