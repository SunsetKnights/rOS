use alloc::vec::Vec;

// process manage mod
use crate::{
    fs::inode::{open_file, OpenFlags},
    mm::page_table::PageTable,
    task::{
        action::SignalAction,
        exit_current_and_run_next, get_pid,
        manager::add_task,
        processor::{current_task, current_user_token},
        signal::{SignalFlags, MAX_SIG},
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

pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let user_token = current_user_token();
    let path = PageTable::from_token(user_token).translated_str(path);
    let mut args_vec = Vec::new();
    loop {
        let arg_str_ptr = *PageTable::from_token(user_token).translated_ref(args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(PageTable::from_token(user_token).translated_str(arg_str_ptr as *const u8));
        args = unsafe { args.add(1) };
    }
    if let Some(inode) = open_file(&path, OpenFlags::READ_ONLY) {
        let argc = args_vec.len();
        let app_data = inode.read_all();
        let pcb = current_task().unwrap();
        pcb.exec(app_data.as_slice(), args_vec);
        argc as isize
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

pub fn sys_spawn(path: *const u8, mut args: *const usize) -> isize {
    let user_token = current_user_token();
    let path = PageTable::from_token(user_token).translated_str(path);
    let mut args_vec = Vec::new();
    loop {
        let arg_str_ptr = *PageTable::from_token(user_token).translated_ref(args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(PageTable::from_token(user_token).translated_str(arg_str_ptr as *const u8));
        args = unsafe { args.add(1) };
    }
    if let Some(inode) = open_file(&path, OpenFlags::READ_ONLY) {
        let app_data = inode.read_all();
        let pcb = current_task().unwrap();
        pcb.spawn(app_data.as_slice(), args_vec) as isize
    } else {
        -1
    }
}

pub fn sys_sigprocmask(mask: u32) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let old_mask = inner.signal_mask;
    if let Some(new_mask) = SignalFlags::from_bits(mask) {
        inner.signal_mask = new_mask;
        old_mask.bits() as isize
    } else {
        -1
    }
}

pub fn sys_sigaction(
    signum: i32,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    // Faild signal
    if signum as usize > MAX_SIG {
        return -1;
    }
    if let Some(flag) = SignalFlags::from_bits(1 << signum) {
        if flag == SignalFlags::SIGKILL
            || flag == SignalFlags::SIGSTOP
            || action as usize == 0
            || old_action as usize == 0
        {
            return -1;
        }
        let prev_action = inner.signal_actions.table[signum as usize];
        *PageTable::from_token(token).translated_refmut(old_action) = prev_action;
        inner.signal_actions.table[signum as usize] =
            *PageTable::from_token(token).translated_ref(action);
        0
    } else {
        -1
    }
}

pub fn sys_get_pid() -> isize {
    get_pid()
}
