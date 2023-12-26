#![no_std] //表示不用std，因为std需要system call，而是用core库
#![no_main]
//表示没有main函数，因为没有std库，所以也不存在main函数之前的初始化过程
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate bitflags;
extern crate alloc;

#[macro_use]
mod lang_runtimes; //完成核心（core）库里面的一些功能，例如panic宏
pub mod config;
mod console; //提供屏幕打印的功能
pub mod loader;
pub mod mm;
mod sbi_services; //提供调用sbi函数的功能
pub mod sync;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod trap;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

#[allow(unused)]
#[no_mangle] //告诉编译器不要乱改名字，不然entry.asm中找不到入口点
pub fn rust_main() -> ! {
    clean_bss();
    mm::init();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    task::run_first_task();
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
