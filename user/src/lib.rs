#![no_std] // use core
#![feature(linkage)] // use customed link script?
#![feature(panic_info_message)]

#[macro_use]
pub mod console;
mod lang_runtimes;
mod sys_call;

#[no_mangle]
// generation symbol use function name, no change
// set section name, in linker script, this section is top of all section. also, this symbol(function name) is the entry of program
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss(); //clear bss section
    let result = main(); //run user program and get return
    exit(result); //return exit code to os
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("no main find") //if not find main function in user program, use this main function
}

fn clear_bss() {
    extern "C" {
        fn u_sbss();
        fn u_ebss();
    }
    // u_sbss和u_ebss为用户程序bss段开始和结束的位置，按字节为单位清零
    for c in u_sbss as usize..u_ebss as usize {
        unsafe { (c as *mut u8).write_volatile(0) }
    }
}

use core::borrow::BorrowMut;

use crate::sys_call::*;
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
