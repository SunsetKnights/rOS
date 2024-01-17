// process manage mod
use crate::{
    loader::load_app_from_name,
    mm::page_table::PageTable,
    task::{
        exit_current_and_run_next, get_pid,
        manager::add_task,
        processor::{current_task, current_user_token},
        suspended_current_and_run_next,
    },
    timer::get_time_ms,
};

pub fn sys_exit(exit_code: i32) -> ! {
    // It may be necessary to delete the last UserContext saved in the kernel stack
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspended_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_fork() -> isize {
    let current_pcb = current_task().unwrap();
    let new_pcb = current_pcb.fork();
    new_pcb.inner_exclusive_access().get_trap_context().x[10] = 0;
    let new_pid = new_pcb.pid.0;
    add_task(new_pcb);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let user_token = current_user_token();
    let path = PageTable::from_token(user_token).translated_str(path);
    if let Some(app_data) = load_app_from_name(path.as_str()) {
        let pcb = current_task().unwrap();
        pcb.exec(app_data);
        0
    } else {
        -1
    }
}

/// Wait for the child process to exit and reclaim resources,
/// while collecting the exit code.
/// # Parameter
/// * 'pid' - The pid of the child process to be recycled, or -1 for any child process.
/// * 'exit_code_ptr' - Child process exit code address.
/// # Return
/// * -1 - Child process does not exist.
/// * -2 - The child process has not exited yet.
/// * pid - The pid of the child process that was successfully recycled.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let mut ret = -1;
    let mut idx = -1;
    for (index, child) in inner.children.iter().enumerate() {
        if pid == -1 || pid as usize == child.get_pid() {
            ret = -2;
            if child.inner_exclusive_access().is_zombie() {
                ret = child.get_pid() as isize;
                *(PageTable::from_token(inner.memory_set.token())
                    .translated_refmut::<i32>(exit_code_ptr)) =
                    child.inner_exclusive_access().exit_code;
                idx = index as isize;
                break;
            }
        }
    }
    if idx != -1 {
        inner.children.remove(idx as usize);
    }
    ret
}

pub fn sys_spawn(path: *const u8) -> isize {
    let user_token = current_user_token();
    let path = PageTable::from_token(user_token).translated_str(path);
    if let Some(app_data) = load_app_from_name(path.as_str()) {
        let pcb = current_task().unwrap();
        pcb.spawn(app_data) as isize
    } else {
        -1
    }
}

pub fn sys_get_pid() -> isize {
    get_pid()
}
