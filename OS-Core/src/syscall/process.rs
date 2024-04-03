use core::usize;

use alloc::vec::Vec;

// process manage mod
use crate::{
    fs::inode::{open_file, OpenFlags},
    mm::page_table::PageTable,
    println,
    task::{
        exit_current_and_run_next, get_pid,
        manager::{add_proc, proc_from_pid, remove_proc},
        processor::{current_process, current_user_token},
        signal::SignalFlags,
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
    let curr_proc = current_process();
    let new_proc = curr_proc.fork();
    new_proc
        .inner_exclusive_access()
        .get_thread(0)
        .inner_exclusive_access()
        .trap_context()
        .x[10] = 0;
    let new_pid = new_proc.pid();
    add_proc(new_proc);
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
        let proc = current_process();
        proc.exec(app_data.as_slice(), args_vec);
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
    let proc = current_process();
    let mut inner = proc.inner_exclusive_access();
    let mut ret = -1;
    let mut idx = -1;
    for (index, child) in inner.children.iter().enumerate() {
        if pid == -1 || pid as usize == child.pid() {
            ret = -2;
            if child.inner_exclusive_access().is_zombie {
                ret = child.pid() as isize;
                *(PageTable::from_token(inner.memory_set.token())
                    .translated_refmut::<i32>(exit_code_ptr)) =
                    child.inner_exclusive_access().exit_code;
                idx = index as isize;
                break;
            }
        }
    }
    if idx != -1 {
        let child_pid = inner.children.remove(idx as usize).pid();
        remove_proc(child_pid);
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
        let proc = current_process();
        proc.spawn(app_data.as_slice(), args_vec) as isize
    } else {
        -1
    }
}

pub fn sys_kill(pid: usize, signum: u32) -> isize {
    if let Some(pcb) = proc_from_pid(pid) {
        if let Some(signal) = SignalFlags::from_bits(1 << signum) {
            let mut inner = pcb.inner_exclusive_access();
            if inner.signals.contains(signal) {
                return -1;
            }
            inner.signals.insert(signal);
            0
        } else {
            println!("[kernel] Can not general SignalFlags from {}", signum);
            -1
        }
    } else {
        println!("[kernel] Not found pcb which pid is {}", pid);
        -1
    }
}

pub fn sys_get_pid() -> isize {
    get_pid()
}
