#![no_std] // use core
#![feature(linkage)] // use customed link script?
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
mod lang_runtimes;
mod sys_call;

use crate::sys_call::*;
use buddy_system_allocator::LockedHeap;
use core::borrow::BorrowMut;

const USER_HEAP_SIZE: usize = 0x4000;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::new();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[no_mangle]
// generation symbol use function name, no change
// set section name, in linker script, this section is top of all section. also, this symbol(function name) is the entry of program
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }
    exit(main()); //return exit code to os
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("no main find") //if not find main function in user program, use this main function
}

/// Read a piece of content from the file into the buffer.
/// # Parameter
/// * 'fd' - file descriptor.
/// * 'buffer' - buffer.
/// # Return
/// * -1 - if read error.
/// * length of bytes read.
pub fn read(fd: usize, buffer: &mut [u8]) -> isize {
    sys_read(fd, buffer)
}
pub fn write(fd: usize, buffer: &[u8]) -> isize {
    sys_write(fd, buffer)
}
pub fn exit(xstate: i32) -> isize {
    sys_exit(xstate)
}
/// yield is the rust key word, so this function named yield_
pub fn yield_() -> isize {
    sys_yield()
}
pub fn get_time() -> isize {
    sys_get_time()
}
/// The current process creates a child process.
/// # Return
/// * For the parent process: return the pid of the child process.
/// * For child processes: return 0.
pub fn fork() -> isize {
    sys_fork()
}
/// Clear the address space of the current process and load an executable file, then start execution.
/// # Parameter
/// * 'path' - path to executable file.
/// # Return
/// * -1 if something goes wrong, or no return.
pub fn exec(path: &str) -> isize {
    sys_exec(path)
}
/// Wait for any child process that becomes a zombie process, recycle resources and collect return values.
/// # Parameter
/// * 'exit_code' - child process return value.
/// # Return
/// * -1, If there is no child process.
/// * -2, If all child processes have not ended.
/// * pid, The pid of the child process that ended.
pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut i32) {
            -2 => {
                yield_();
            }
            exit_pid => return exit_pid,
        }
    }
}
/// Wait for specific child process that becomes a zombie process, recycle resources and collect return values.
/// # Parameter
/// * 'pid' - The pid of the child process waiting to end.
/// * 'exit_code' - child process return value.
/// # Return
/// * -1, If there is no child process.
/// * -2, If all child processes have not ended.
/// * pid, The pid of the child process that ended.
pub fn wait_pid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut i32) {
            -2 => {
                yield_();
            }
            exit_pid => return exit_pid,
        }
    }
}
pub fn get_task_info(id: usize) -> TaskInfo {
    let mut ret = TaskInfo {
        id,
        status: TaskStatus::UnInit,
        call: [SyscallInfo { id: 0, time: 0 }; SYSCALL_QUANTITY],
        time: 0,
    };
    sys_task_info(id, ret.borrow_mut() as *mut TaskInfo);
    ret
}
