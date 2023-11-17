#![no_std] // use core
#![feature(linkage)] // use customed link script?
#![feature(panic_info_message)]

#[macro_use]
pub mod console;
mod sys_call;
mod lang_runtimes;

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

use crate::sys_call::*;
pub fn write(fd: usize, buffer: &[u8]) -> isize {
    sys_write(fd, buffer)
}
pub fn exit(xstate: i32) -> isize {
    sys_exit(xstate)
}
