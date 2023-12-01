// process manage mod

use crate::{
    println,
    task::{
        exit_current_and_run_next, get_task_info, info::TaskInfo, suspended_current_and_run_next,
    },
    timer::get_time_ms,
};

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    // It may be necessary to delete the last UserContext saved in the kernel stack
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspended_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_task_info(id: usize, ti: *mut TaskInfo) -> isize {
    get_task_info(id, ti);
    0
}
