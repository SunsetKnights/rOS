#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate bitflags;
extern crate alloc;

#[macro_use]
mod lang_runtimes;
pub mod config;
mod console;
pub mod loader;
pub mod mm;
mod sbi_services;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod trap;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

#[allow(unused)]
#[no_mangle]
pub fn rust_main() -> ! {
    clean_bss();
    mm::init();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    task::add_initproc();
    loader::list_app();
    task::processor::run_tasks();
    panic!("Unreachable in rust_main!");
}

fn clean_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    // sbss和ebss为bss段开始和结束的位置，按字节为单位清零
    for c in sbss as usize..ebss as usize {
        unsafe { (c as *mut u8).write_volatile(0) }
    }
}
